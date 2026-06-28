//! 模块编译器——按拓扑序编译模块并链接。
//!
//! `ModuleCompiler` 使用 `ModuleGraph` 的拓扑序，逐个编译模块，
//! 解析导入依赖，管理缓存失效，最后调用 `LinkStage` 产生全局 CPS。

use crate::export_table::{ExportEntry, ExportTable, ImportTable, RawImport, ResolvedImport};
use crate::link_stage::LinkStage;
use crate::module_graph::ModuleGraph;
use crate::module_loader::ModuleLoader;
use crate::protocol::BuildError;
use crate::stages::FrontendStage;
use kaubo_infer::types::{ImportKind, ImportSpec};
use kaubo_infer::Type;
use kaubo_ir::cps::CpsModule;
use kaubo_ir::cps_build::build_module_with_imports;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// 模块编译缓存条目。
#[derive(Debug, Clone)]
struct ModuleCacheEntry {
    export_table: ExportTable,
    /// 源码的 SHA-256 哈希（十六进制）
    content_hash: String,
    /// 直接依赖的 content_hash 快照（用于缓存失效）
    dep_hashes: HashMap<String, String>,
}

/// 模块编译器。
///
/// 消费 `ModuleGraph` 产生链接后的全局 `CpsModule`。
pub struct ModuleCompiler<'a> {
    loader: &'a dyn ModuleLoader,
    built: HashMap<String, ModuleCacheEntry>,
}

impl<'a> ModuleCompiler<'a> {
    pub fn new(loader: &'a dyn ModuleLoader) -> Self {
        Self {
            loader,
            built: HashMap::new(),
        }
    }

    /// 按拓扑序编译所有模块，最后链接为一个全局 CpsModule。
    pub fn compile_all(&mut self, graph: &ModuleGraph) -> Result<CpsModule, BuildError> {
        for path in &graph.order {
            // 检查缓存
            if self.is_cache_fresh(path, graph)? {
                continue;
            }

            let source = graph
                .sources
                .get(path)
                .ok_or_else(|| BuildError::Bug(format!("source not found for {path}")))?;

            let raw_imports = graph.imports.get(path).map_or(&[][..], |v| v);

            // 1. 解析导入——依赖模块已全部编译完毕
            let import_table = self.resolve_imports(path, raw_imports)?;

            // 2. 前端：解析（预注册导入 struct 名称以支持 `Name { ... }` 语法）
            let module = {
                let mut parser = kaubo_syntax::parser::Parser::new(source);
                // 预注册导入的 struct 名称
                for ri in &import_table.entries {
                    if matches!(ri.entry, ExportEntry::Struct { .. }) {
                        parser.register_struct_name(&ri.local_name);
                    }
                }
                parser.parse().map_err(|e| BuildError::Parse(format!("{path}: {e}")))?
            };

            // 3. 转换为 infer 能消费的导入格式
            let import_specs: Option<Vec<ImportSpec>> = if import_table.is_empty() {
                None
            } else {
                Some(
                    import_table
                        .entries
                        .iter()
                        .map(|ri| export_entry_to_import_spec(&ri.entry, &ri.local_name, &ri.source_path))
                        .collect(),
                )
            };

            // 4. 语义分析（带导入）
            let (type_env, struct_fields, exports) =
                kaubo_infer::infer_module_with_imports(
                    &module,
                    import_specs.as_deref(),
                )
                .map_err(|e| BuildError::Infer(format!("{path}: {}", e.msg)))?;

            // 5. CPS 构建（带导入表 + 导出集 + 导入结构体）
            let import_map: Option<HashMap<String, usize>> = if import_table.is_empty() {
                None
            } else {
                Some(import_table.by_name.clone())
            };

            // 提取导入结构体的定义（源模块路径 + 原始 struct_id + 字段列表）
            let import_structs: Option<HashMap<String, (String, usize, Vec<(String, String)>)>> =
                if import_table.is_empty() {
                    None
                } else {
                    let mut s = HashMap::new();
                    for ri in &import_table.entries {
                        if let ExportEntry::Struct {
                            name,
                            fields,
                            struct_id,
                        } = &ri.entry
                        {
                            let field_strs: Vec<(String, String)> = fields
                                .iter()
                                .map(|(fn_name, fn_ty)| {
                                    (fn_name.clone(), type_to_string(fn_ty))
                                })
                                .collect();
                            s.insert(
                                name.clone(),
                                (ri.source_path.clone(), *struct_id, field_strs),
                            );
                        }
                    }
                    if s.is_empty() { None } else { Some(s) }
                };

            let (cps, export_funcs, export_consts) = build_module_with_imports(
                &module, None, import_map.as_ref(), &exports, import_structs.as_ref(),
            )
            .map_err(|e| BuildError::Build(format!("{path}: {e}")))?;

            // 6. 构建导出表
            let export_table =
                build_export_table(path, &cps, type_env, struct_fields, &exports, &export_funcs, &export_consts, import_table);

            // 7. 计算哈希并缓存
            let content_hash = sha256_hex(source.as_bytes());
            let dep_hashes = self.snapshot_dep_hashes(path, raw_imports);

            self.built.insert(
                path.clone(),
                ModuleCacheEntry {
                    export_table,
                    content_hash,
                    dep_hashes,
                },
            );
        }

        // 8. 链接所有模块
        let built_map: HashMap<String, ExportTable> = self
            .built
            .iter()
            .map(|(k, v)| (k.clone(), v.export_table.clone()))
            .collect();

        LinkStage::link(&built_map, &graph.order)
    }

    /// 解析当前模块的导入，生成 ImportTable。
    fn resolve_imports(
        &self,
        path: &str,
        raw: &[RawImport],
    ) -> Result<ImportTable, BuildError> {
        let mut entries = Vec::new();
        let mut by_name = HashMap::new();

        for raw_imp in raw {
            let (dep_path, _) = self
                .loader
                .resolve(path, &raw_imp.source_path)
                .map_err(|e| BuildError::Build(format!("resolve failed for {path}: {e}")))?;

            let dep = self.built.get(&dep_path).ok_or_else(|| {
                BuildError::ImportNotFound {
                    path: dep_path.clone(),
                    name: raw_imp.names.first().cloned().unwrap_or_default(),
                }
            })?;

            for name in &raw_imp.names {
                let entry = dep
                    .export_table
                    .find_export(name)
                    .cloned()
                    .ok_or_else(|| BuildError::ExportNotFound {
                        name: name.clone(),
                        path: dep_path.clone(),
                    })?;

                // 冲突检测
                if by_name.contains_key(name) {
                    let existing: &ResolvedImport = &entries[*by_name.get(name).unwrap()];
                    return Err(BuildError::SymbolConflict {
                        name: name.clone(),
                        path1: existing.source_path.clone(),
                        path2: dep_path.clone(),
                    });
                }

                by_name.insert(name.clone(), entries.len());
                entries.push(ResolvedImport {
                    local_name: name.clone(),
                    source_path: dep_path.clone(),
                    entry,
                });
            }
        }

        Ok(ImportTable { entries, by_name })
    }

    /// 检查缓存是否仍然有效。
    fn is_cache_fresh(&self, path: &str, graph: &ModuleGraph) -> Result<bool, BuildError> {
        let Some(source) = graph.sources.get(path) else {
            return Ok(false);
        };
        let hash = sha256_hex(source.as_bytes());

        let Some(entry) = self.built.get(path) else {
            return Ok(false);
        };

        if entry.content_hash != hash {
            return Ok(false);
        }

        let raw_imports = graph.imports.get(path).map_or(&[][..], |v| v);
        for imp in raw_imports {
            let (dep, _) = self
                .loader
                .resolve(path, &imp.source_path)
                .map_err(|e| BuildError::Build(format!("resolve failed: {e}")))?;
            let Some(dep_entry) = self.built.get(&dep) else {
                return Ok(false);
            };
            if entry.dep_hashes.get(&dep) != Some(&dep_entry.content_hash) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 收集所有直接依赖的 content_hash 快照。
    fn snapshot_dep_hashes(
        &self,
        path: &str,
        raw: &[RawImport],
    ) -> HashMap<String, String> {
        let mut hashes = HashMap::new();
        for imp in raw {
            if let Ok((dep, _)) = self.loader.resolve(path, &imp.source_path) {
                if let Some(dep_entry) = self.built.get(&dep) {
                    hashes.insert(dep, dep_entry.content_hash.clone());
                }
            }
        }
        hashes
    }
}

// ── 辅助函数 ──

/// 从 ExportEntry 转换为 infer 用的 ImportSpec。
fn export_entry_to_import_spec(
    entry: &ExportEntry,
    local_name: &str,
    _source_path: &str,
) -> ImportSpec {
    match entry {
        ExportEntry::Const { ty, .. } => ImportSpec {
            local_name: local_name.to_string(),
            source_path: _source_path.to_string(),
            kind: ImportKind::Const { ty: ty.clone() },
        },
        ExportEntry::Function { ty, .. } => ImportSpec {
            local_name: local_name.to_string(),
            source_path: _source_path.to_string(),
            kind: ImportKind::Function { ty: ty.clone() },
        },
        ExportEntry::Struct { fields, .. } => ImportSpec {
            local_name: local_name.to_string(),
            source_path: _source_path.to_string(),
            kind: ImportKind::Struct {
                fields: fields.clone(),
            },
        },
        ExportEntry::Interface { methods, .. } => ImportSpec {
            local_name: local_name.to_string(),
            source_path: _source_path.to_string(),
            kind: ImportKind::Interface {
                methods: methods.clone(),
            },
        },
    }
}

/// 从语义分析和 CPS 结果构建导出表。
fn build_export_table(
    path: &str,
    cps: &CpsModule,
    type_env: kaubo_infer::TypeEnv,
    struct_fields: HashMap<usize, Vec<(String, Type)>>,
    exports: &HashSet<String>,
    export_funcs: &HashMap<String, usize>,
    export_consts: &HashMap<String, usize>,
    import_table: ImportTable,
) -> ExportTable {
    let mut entries = Vec::new();

    for name in exports {
        // 从 type_env 获取类型
        let ty = type_env
            .get(name)
            .map(|s| {
                // 从 Scheme 提取 monomorphic 类型
                if s.bound.is_empty() {
                    (*s.body).clone()
                } else {
                    // 多态类型暂不支持导出——使用 Null 占位
                    Type::Null
                }
            })
            .unwrap_or(Type::Null);

        // 根据类型推断种类
        let entry = match &ty {
            Type::Arrow(_, _) => {
                // 使用 export_funcs 映射获取准确的 local func_idx
                let func_idx = export_funcs
                    .get(name)
                    .copied()
                    .unwrap_or_else(|| {
                        // fallback: 按名称匹配
                        cps.functions
                            .iter()
                            .position(|f| f.name == *name)
                            .unwrap_or(0)
                    });
                ExportEntry::Function {
                    name: name.clone(),
                    ty,
                    func_idx,
                }
            }
            Type::Record(struct_id, fields) => ExportEntry::Struct {
                name: name.clone(),
                fields: fields.clone(),
                struct_id: *struct_id,
            },
            _ => {
                // 常量——从 export_consts 获取正确的 local const_idx
                let const_idx = export_consts
                    .get(name)
                    .copied()
                    .unwrap_or(0);
                ExportEntry::Const {
                    name: name.clone(),
                    ty,
                    const_idx,
                }
            }
        };
        entries.push(entry);
    }

    // 同时扫描 type_env 中所有以 "struct_name.method_name" 命名的 impl 方法
    // （这些不在 exports 中但需要可通过导入表访问）
    for (key, scheme) in &type_env {
        if key.contains('.') && scheme.bound.is_empty() {
            if let Type::Arrow(_, _) = *scheme.body {
                let func_idx = cps
                    .functions
                    .iter()
                    .position(|f| f.name == *key)
                    .unwrap_or(usize::MAX);
                if func_idx != usize::MAX {
                    entries.push(ExportEntry::Function {
                        name: key.clone(),
                        ty: (*scheme.body).clone(),
                        func_idx,
                    });
                }
            }
        }
    }

    ExportTable {
        source_path: path.to_string(),
        entries,
        import_table,
        cps_module: Arc::new(cps.clone()),
    }
}

/// 将 `kaubo_infer::Type` 转换为类型名字符串（用于 CPS StructDef 的字段类型）。
fn type_to_string(ty: &kaubo_infer::Type) -> String {
    match ty {
        kaubo_infer::Type::Int64 => "Int64".into(),
        kaubo_infer::Type::Float64 => "Float64".into(),
        kaubo_infer::Type::String => "String".into(),
        kaubo_infer::Type::Bool => "Bool".into(),
        kaubo_infer::Type::Null => "Null".into(),
        kaubo_infer::Type::List(_) => "List".into(),
        kaubo_infer::Type::Record(_, _) => "Struct".into(),
        kaubo_infer::Type::Arrow(_, _) => "Arrow".into(),
        kaubo_infer::Type::Var(_) => "Unknown".into(),
        kaubo_infer::Type::Variant(_, _, _) => "Variant".into(),
        kaubo_infer::Type::Interface(_) => "Interface".into(),
    }
}

/// SHA-256 哈希（十六进制编码）。
///
/// 使用 std 的 DefaultHasher 作为轻量替代，生产环境应使用真正的 SHA-256。
fn sha256_hex(data: &[u8]) -> String {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash_slice(data, &mut hasher);
    format!("{:016x}", hasher.finish())
}
