//! 多模块链接阶段。
//!
//! `LinkStage` 将多个模块的局部 CPS 合并为一个全局 CpsModule：
//! 1. 构建全局索引映射（func_remap, struct_remap, const_remap）
//! 2. 遍历所有模块，将 `CallExternal` 解析为 `Call(global_idx)`
//! 3. 重映射所有 struct_id 和 const_idx 引用
//! 4. 合并函数表、结构体、常量、vtable
//! 5. 生成 symbol_map 和 func_owners

use crate::export_table::{ExportEntry, ExportTable, GlobalRef};
use crate::protocol::BuildError;
use kaubo_ir::cps::{
    CpsBlock, CpsFunction, CpsInstr, CpsModule, CpsTerminator, Constant, EnumDef, StructDef,
    VtableDef,
};
use std::collections::HashMap;

/// 多模块链接器。
pub struct LinkStage;

impl LinkStage {
    /// 链接多个已编译模块为一个全局 CpsModule。
    ///
    /// `built`: module_path → ExportTable（包含局部 CPS + 导入表）
    /// `order`: 拓扑序（叶子→根）
    pub fn link(
        built: &HashMap<String, ExportTable>,
        order: &[String],
    ) -> Result<CpsModule, BuildError> {
        if order.is_empty() {
            return Ok(CpsModule {
                functions: vec![],
                constants: vec![],
                structs: vec![],
                enums: vec![],
                vtables: vec![],
                symbol_map: HashMap::new(),
                func_owners: vec![],
            });
        }

        // ── 1. 构建全局索引映射 ──
        let (func_remap, struct_remap, const_remap) = Self::build_remap_tables(built, order);

        // ── 2. 构建全局 struct 表 ──
        let mut global_structs: Vec<StructDef> = Vec::new();
        for path in order {
            let export = &built[path];
            let offset = global_structs.len();
            for (i, sd) in export.cps_module.structs.iter().enumerate() {
                struct_remap
                    .borrow_mut()
                    .insert((path.clone(), sd.id), offset + i);
                let mut new_sd = sd.clone();
                new_sd.id = offset + i;
                global_structs.push(new_sd);
            }
        }

        // ── 3. 计算总函数数并分配全局函数表 ──
        let total_funcs: usize = order
            .iter()
            .map(|path| built[path].cps_module.functions.len())
            .sum();
        let mut linked_funcs: Vec<Option<CpsFunction>> = vec![None; total_funcs];

        // ── 4. 遍历每个模块，重映射并写入全局表 ──
        for path in order {
            let export = &built[path];
            let import_table = &export.import_table;
            let mut module_cps = (*export.cps_module).clone();

            for func in &mut module_cps.functions {
                for block in &mut func.blocks {
                    // 4a. 重映射本地 Call/TailCall → global_idx（必须在 CallExternal 之前）
                    match &block.term {
                        CpsTerminator::Call(local_idx, _, _)
                        | CpsTerminator::TailCall(local_idx, _) => {
                            let global_idx = func_remap
                                .get(&(path.to_string(), *local_idx))
                                .copied()
                                .unwrap_or(*local_idx);
                            match &mut block.term {
                                CpsTerminator::Call(ref mut fi, _, _) => *fi = global_idx,
                                CpsTerminator::TailCall(ref mut fi, _) => *fi = global_idx,
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                    // 4b. 解析 CallExternal → Call(global_idx)（本地 Call 已重映射完毕）
                    if let CpsTerminator::CallExternal {
                        import_handle,
                        args,
                        ret_block,
                    } = &block.term
                    {
                        let resolved =
                            import_table.entries.get(*import_handle).ok_or_else(|| {
                                BuildError::Bug(format!(
                                    "link: invalid import handle {import_handle} in {path}"
                                ))
                            })?;

                        let global_idx = match &resolved.entry {
                            ExportEntry::Function { func_idx, .. } => func_remap
                                .get(&(resolved.source_path.clone(), *func_idx))
                                .copied()
                                .ok_or_else(|| {
                                    BuildError::Bug(format!(
                                        "link: func_remap missing for ({}, {})",
                                        resolved.source_path, func_idx
                                    ))
                                })?,
                            _ => {
                                return Err(BuildError::Bug(format!(
                                    "link: import handle {import_handle} in {path} is not a function"
                                )));
                            }
                        };

                        block.term = CpsTerminator::Call(global_idx, args.clone(), *ret_block);
                    }

                    // 4c. 跳过 CallExternalDynamic（原样保留，运行时解析）

                    // 4d. 重映射 struct_id 引用
                    Self::remap_struct_ids(block, &struct_remap, path);

                    // 4d. 重映射 const_idx 引用（LoadConst）——必须在 LoadExternalConst 之前
                    Self::remap_const_refs(block, &const_remap, path);

                    // 4e. 解析 LoadExternalConst → LoadConst(global_idx)
                    // ★ 必须在 remap_const_refs 之后，避免已解析的全局 idx 被再次重映射
                    for instr in &mut block.instrs {
                        if let CpsInstr::LoadExternalConst(dst, import_handle) = instr {
                            let resolved =
                                import_table.entries.get(*import_handle).ok_or_else(|| {
                                    BuildError::Bug(format!(
                                        "link: invalid import handle {import_handle} in {path}"
                                    ))
                                })?;
                            let global_idx = match &resolved.entry {
                                ExportEntry::Const { const_idx, .. } => const_remap
                                    .borrow()
                                    .get(&(resolved.source_path.clone(), *const_idx))
                                    .copied()
                                    .ok_or_else(|| {
                                        BuildError::Bug(format!(
                                            "link: const_remap missing for ({}, {})",
                                            resolved.source_path, const_idx
                                        ))
                                    })?,
                                _ => {
                                    return Err(BuildError::Bug(format!(
                                        "link: LoadExternalConst handle {import_handle} in {path} is not a Const"
                                    )));
                                }
                            };
                            *instr = CpsInstr::LoadConst(*dst, global_idx);
                        }
                    }
                }
            }

            // ── 写入全局函数表 ──
            for (i, func) in module_cps.functions.into_iter().enumerate() {
                let global_idx = func_remap[&(path.clone(), i)];
                linked_funcs[global_idx] = Some(func);
            }
        }

        // ── 5. 收集全局函数 ──
        let global_functions: Vec<CpsFunction> = linked_funcs
            .into_iter()
            .map(|f| f.expect("link: all function slots should be filled"))
            .collect();

        // ── 6. 合并常量表 ──
        let mut global_constants: Vec<Constant> = Vec::new();
        for path in order {
            let export = &built[path];
            let offset = global_constants.len();
            let old_len = export.cps_module.constants.len();
            global_constants.extend(export.cps_module.constants.clone());
            // 更新 const_remap
            for i in 0..old_len {
                const_remap
                    .borrow_mut()
                    .insert((path.clone(), i), offset + i);
            }
        }

        // ── 7. 合并 vtable（重映射 func_idx） ──
        let mut global_vtables: Vec<VtableDef> = Vec::new();
        for path in order {
            for vt in &built[path].cps_module.vtables {
                let mut new_vt = vt.clone();
                for (_, func_idx) in &mut new_vt.methods {
                    let key = (path.clone(), *func_idx);
                    if let Some(&global_idx) = func_remap.get(&key) {
                        *func_idx = global_idx;
                    }
                }
                global_vtables.push(new_vt);
            }
        }

        // ── 8. 合并 enum ──
        let global_enums: Vec<EnumDef> = order
            .iter()
            .flat_map(|path| built[path].cps_module.enums.clone())
            .collect();

        // ── 9. 构建 symbol_map ──
        let mut symbol_map: HashMap<(String, String), usize> = HashMap::new();
        for path in order {
            let export = &built[path];
            for entry in &export.entries {
                let global_ref = match entry {
                    ExportEntry::Function { func_idx, .. } => {
                        func_remap.get(&(path.clone(), *func_idx)).map(|&g| GlobalRef::Func(g))
                    }
                    ExportEntry::Struct { struct_id, .. } => struct_remap
                        .borrow()
                        .get(&(path.clone(), *struct_id))
                        .map(|&g| GlobalRef::Struct(g)),
                    ExportEntry::Const { const_idx, .. } => const_remap
                        .borrow()
                        .get(&(path.clone(), *const_idx))
                        .map(|&g| GlobalRef::Const(g)),
                    _ => None,
                };
                if let Some(GlobalRef::Func(global_idx)) = global_ref {
                    symbol_map
                        .insert((path.clone(), entry.export_name().to_string()), global_idx);
                }
            }
        }

        // ── 10. 构建 func_owners ──
        let mut func_owners = vec![String::new(); total_funcs];
        for path in order {
            let n = built[path].cps_module.functions.len();
            for i in 0..n {
                let global_idx = func_remap[&(path.clone(), i)];
                func_owners[global_idx] = path.clone();
            }
        }

        Ok(CpsModule {
            functions: global_functions,
            constants: global_constants,
            structs: global_structs,
            enums: global_enums,
            vtables: global_vtables,
            symbol_map,
            func_owners,
        })
    }

    // ── 内部辅助 ──

    /// 构建全局索引映射表。
    fn build_remap_tables(
        built: &HashMap<String, ExportTable>,
        order: &[String],
    ) -> (
        HashMap<(String, usize), usize>,
        std::cell::RefCell<HashMap<(String, usize), usize>>,
        std::cell::RefCell<HashMap<(String, usize), usize>>,
    ) {
        let mut func_remap = HashMap::new();
        let mut struct_remap = HashMap::new();
        let mut const_remap = HashMap::new();
        let mut func_offset = 0;
        let mut const_offset = 0;

        for path in order {
            let cps = &built[path].cps_module;
            let n_func = cps.functions.len();
            let n_const = cps.constants.len();
            for i in 0..n_func {
                func_remap.insert((path.clone(), i), func_offset + i);
            }
            for i in 0..n_const {
                const_remap.insert((path.clone(), i), const_offset + i);
            }
            func_offset += n_func;
            const_offset += n_const;
        }

        (
            func_remap,
            // struct_remap 和 const_remap 在后续步骤中动态补充，使用 RefCell 允许延迟填充
            std::cell::RefCell::new(struct_remap),
            std::cell::RefCell::new(const_remap),
        )
    }

    /// 重映射 block 中所有 struct_id 引用。
    fn remap_struct_ids(
        block: &mut CpsBlock,
        struct_remap: &std::cell::RefCell<HashMap<(String, usize), usize>>,
        module_path: &str,
    ) {
        for instr in &mut block.instrs {
            if let CpsInstr::NewStruct(_, sid, _) = instr {
                if let Some(&global_id) = struct_remap
                    .borrow()
                    .get(&(module_path.to_string(), *sid))
                {
                    *sid = global_id;
                }
            }
        }
    }

    /// 重映射 block 中所有 const_idx 引用。
    fn remap_const_refs(
        block: &mut CpsBlock,
        const_remap: &std::cell::RefCell<HashMap<(String, usize), usize>>,
        module_path: &str,
    ) {
        for instr in &mut block.instrs {
            if let CpsInstr::LoadConst(_, idx) = instr {
                if let Some(&global_idx) = const_remap
                    .borrow()
                    .get(&(module_path.to_string(), *idx))
                {
                    *idx = global_idx;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export_table::ImportTable;
    use std::sync::Arc;

    fn make_cps(functions: Vec<CpsFunction>) -> CpsModule {
        CpsModule {
            functions,
            constants: vec![],
            structs: vec![],
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        }
    }

    fn make_entry(name: &str) -> ExportTable {
        ExportTable {
            source_path: name.to_string(),
            entries: vec![],
            import_table: ImportTable::empty(),
            cps_module: Arc::new(make_cps(vec![])),
        }
    }

    #[test]
    fn link_empty_is_ok() {
        let built: HashMap<String, ExportTable> = HashMap::new();
        let order: Vec<String> = vec![];
        let result = LinkStage::link(&built, &order).unwrap();
        assert!(result.functions.is_empty());
    }

    #[test]
    fn link_single_module_passthrough() {
        let mut built = HashMap::new();
        let cps = make_cps(vec![CpsFunction {
            name: "main".into(),
            blocks: vec![CpsBlock {
                id: 0,
                params: vec![],
                instrs: vec![CpsInstr::LoadConst(0, 0)],
                term: CpsTerminator::Return(0),
            }],
            entry: 0,
            reg_count: 1,
        }]);
        built.insert(
            "main.kb".to_string(),
            ExportTable {
                source_path: "main.kb".into(),
                entries: vec![],
                import_table: ImportTable::empty(),
                cps_module: Arc::new(cps),
            },
        );

        let order = vec!["main.kb".to_string()];
        let result = LinkStage::link(&built, &order).unwrap();
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "main");
    }
}
