//! cps_build — block-level CPS construction
//!
//! Orchestrates: register allocation, block creation, chaining, scope management.
//! Uses cps_emit for individual instruction generation.
//!
//! Supports multi-function compilation: lambda expressions create separate CpsFunctions.
//! Every build_* returns (entry, continu, reg):
//!   entry   — first block to jump INTO
//!   continu — block whose terminator is Return (can be chained FROM)
//!   reg     — register holding the result value

use crate::cps::*;
use crate::cps_emit;
use kaubo_ast::*;
use kaubo_log::emit;
use std::collections::{HashMap, HashSet};

pub fn build_module(
    module: &Module,
    events: Option<&dyn kaubo_log::EventHandler>,
) -> Result<CpsModule, String> {
    build_module_with_imports(module, events, None, &HashSet::new(), None).map(|(cps, _, _)| cps)
}

/// 带导入表和导出集的 CPS 构建。
///
/// `import_table`: local_name → import_handle（CPS 层使用整数句柄）
/// `is_exported`: 本模块导出符号名集合
pub fn build_module_with_imports(
    module: &Module,
    events: Option<&dyn kaubo_log::EventHandler>,
    import_table: Option<&HashMap<String, usize>>,
    is_exported: &HashSet<String>,
    import_structs: Option<&HashMap<String, (String, usize, Vec<(String, String)>)>>,
) -> Result<(CpsModule, HashMap<String, usize>, HashMap<String, usize>), String> {
    let mut b = CpsBuilder::new(events);
    b.import_table = import_table.cloned();
    b.is_exported = is_exported.clone();
    b.import_structs = import_structs.cloned();

    // ★ 注册导入的结构体——必须在 collect_signatures 之前，
    // 这样后续的 type_expr_hint 才能识别导入 struct 的类型名
    if let Some(import_structs) = &b.import_structs {
        for (name, (_src_path, src_id, fields)) in import_structs {
            let mut bitmap: u64 = 0;
            for (i, (_, field_ty)) in fields.iter().enumerate() {
                if is_heap_type_name(field_ty) {
                    bitmap |= 1 << i;
                }
            }
            b.structs.push(StructDef {
                id: *src_id,
                name: name.clone(),
                fields: fields.clone(),
                type_bitmap: bitmap,
            });
            b.struct_names.insert(name.clone());
            let field_hints: HashMap<String, ValueHint> = fields
                .iter()
                .map(|(fn_name, fn_ty)| (fn_name.clone(), type_name_to_hint(fn_ty)))
                .collect();
            b.struct_field_types.insert(name.clone(), field_hints);
        }
    }

    b.collect_signatures(module);
    b.ctx.new_block(); // entry block id 0
    let mut tail: Option<usize> = None;

    for stmt in &module.stmts {
        let (entry, continu, _) = b.build_top_stmt(stmt)?;
        if entry == usize::MAX {
            continue;
        }
        if let Some(t) = tail {
            b.ctx.chain(t, entry)?;
        } else {
            b.ctx.set_block(0, block_jump(0, entry));
        }
        tail = Some(continu);
    }

    b.finalize(0);
    dump_blocks("main", &b.ctx);
    let export_func_map = b.export_funcs.clone();
    let export_const_map = b.export_consts.clone();
    Ok((
        CpsModule {
            functions: b.functions,
            constants: b.constants,
            structs: b.structs,
            enums: b.enums,
            vtables: b.vtables,
            symbol_map: std::collections::HashMap::new(),
            func_owners: vec![],
        },
        export_func_map,
        export_const_map,
    ))
}

fn block_jump(id: usize, target: usize) -> CpsBlock {
    CpsBlock {
        id,
        params: vec![],
        instrs: vec![],
        term: CpsTerminator::Jump(target, vec![]),
    }
}

/// 将类型名字符串转换为 ValueHint（用于导入 struct 的字段类型推断）。
fn type_name_to_hint(type_name: &str) -> ValueHint {
    match type_name {
        "Int64" => ValueHint::Int,
        "Float64" => ValueHint::Float,
        "String" => ValueHint::String,
        "Bool" => ValueHint::Bool,
        "Null" => ValueHint::Null,
        "List" => ValueHint::List,
        other => ValueHint::Struct(other.to_string()),
    }
}

/// 判断类型名字符串是否表示堆类型（用于 GC bitmap 计算）。
fn is_heap_type_name(type_name: &str) -> bool {
    matches!(type_name, "String" | "List")
        || (!matches!(type_name, "Int64" | "Float64" | "Bool" | "Null"))
    // Struct 和 Interface 名称也是堆类型
}

fn dump_blocks(label: &str, ctx: &FuncCtx) {
    if cfg!(debug_assertions) {
        eprintln!(
            "[CPS {}] regs={} blocks={}",
            label,
            ctx.next_reg,
            ctx.blocks.len()
        );
        for b in &ctx.blocks {
            eprintln!("  blk{} {:?} | {:?}", b.id, b.instrs, b.term);
        }
    }
}

// ── Per-function compilation context ──

pub struct FuncCtx {
    pub name: String,
    pub blocks: Vec<CpsBlock>,
    pub next_reg: usize,
    pub var_map: HashMap<String, usize>,
    type_map: HashMap<String, ValueHint>,
    pub func_map: HashMap<String, usize>, // function name → func_idx
    pub loop_stack: Vec<(usize, usize)>,
}

impl FuncCtx {
    pub fn new(name: String) -> Self {
        FuncCtx {
            name,
            blocks: vec![],
            next_reg: 1,
            var_map: HashMap::new(),
            type_map: HashMap::new(),
            func_map: HashMap::new(),
            loop_stack: vec![],
        }
    }

    fn alloc(&mut self) -> usize {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    pub fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(CpsBlock {
            id,
            params: vec![],
            instrs: vec![],
            term: CpsTerminator::Return(0),
        });
        id
    }

    pub fn set_block(&mut self, id: usize, block: CpsBlock) {
        if id < self.blocks.len() {
            self.blocks[id] = block;
        }
    }

    /// Chain block `from` → `to`. Fails if `from` terminator is not Return.
    pub fn chain(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.blocks.len() {
            return Ok(());
        }
        if !matches!(self.blocks[from].term, CpsTerminator::Return(_)) {
            return Err(format!("chain: block {from} not Return (already chained?)"));
        }
        self.blocks[from].term = CpsTerminator::Jump(to, vec![]);
        Ok(())
    }

    /// Rewire `from` block's Return → Jump(target, args).
    fn rewire_return_args(
        &mut self,
        from: usize,
        target: usize,
        args: &[usize],
    ) -> Result<(), String> {
        if from >= self.blocks.len() {
            return Ok(());
        }
        if !matches!(self.blocks[from].term, CpsTerminator::Return(_)) {
            return Err(format!("rewire: block {from} not Return"));
        }
        self.blocks[from].term = CpsTerminator::Jump(target, args.to_vec());
        Ok(())
    }

    fn leaf_block(&mut self, reg: usize, const_idx: usize) -> (usize, usize) {
        let (instrs, term) = cps_emit::emit_literal(reg, const_idx);
        let id = self.new_block();
        self.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term,
            },
        );
        (id, id)
    }

    fn finalize(&self, entry_block: usize) -> CpsFunction {
        if self.blocks.is_empty() {
            return CpsFunction {
                name: self.name.clone(),
                blocks: vec![],
                entry: 0,
                reg_count: 0,
            };
        }
        if cfg!(debug_assertions) {
            eprintln!(
                "[CPS FINALIZE {}] entry_in={} total_blocks={}",
                self.name,
                entry_block,
                self.blocks.len()
            );
        }
        let mut id_map = HashMap::new();
        let mut new_blocks = Vec::new();
        for (i, b) in self.blocks.iter().enumerate() {
            id_map.insert(b.id, i);
            new_blocks.push(CpsBlock {
                id: i,
                params: b.params.clone(),
                instrs: b.instrs.clone(),
                term: b.term.clone(),
            });
        }
        for b in &mut new_blocks {
            remap_term_ids(b, &id_map);
        }
        let entry = *id_map.get(&entry_block).unwrap_or(&0);
        if cfg!(debug_assertions) {
            eprintln!("[CPS FINALIZE {}] entry_out={}", self.name, entry);
        }
        CpsFunction {
            name: self.name.clone(),
            blocks: new_blocks,
            entry,
            reg_count: self.next_reg,
        }
    }
}

// ── Module-level builder ──

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueHint {
    Int,
    Float,
    String,
    Bool,
    Null,
    Struct(String),
    Interface(String),
    List,
    Tuple,
    Unknown,
}

impl ValueHint {
    fn is_float(&self) -> bool {
        matches!(self, ValueHint::Float)
    }
}

pub struct CpsBuilder<'a> {
    pub functions: Vec<CpsFunction>,
    pub constants: Vec<Constant>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub vtables: Vec<VtableDef>,
    const_map: HashMap<String, usize>,
    pub ctx: FuncCtx,
    struct_names: HashSet<String>,
    struct_field_types: HashMap<String, HashMap<String, ValueHint>>,
    function_returns: HashMap<String, ValueHint>,
    method_returns: HashMap<String, ValueHint>,
    enum_names: HashSet<String>,
    variant_to_enum: HashMap<String, String>,
    variant_field_map: HashMap<String, Vec<(String, String)>>,
    /// interface_name → VtableDef (built during build_top_stmt)
    interface_vtables: HashMap<String, VtableDef>,
    /// struct_name → [interface_names] that this struct implements
    struct_impl_for: HashMap<String, Vec<String>>,
    /// func_idx → param hints (for generating Struct→Interface wrapping at call sites)
    fn_param_hints: HashMap<usize, Vec<ValueHint>>,
    /// Optional event handler for structured logging.
    /// Passed through from the driver; stages only emit events, never touch output.
    events: Option<&'a dyn kaubo_log::EventHandler>,
    /// 当前模块的导入表：local_name → import_handle
    import_table: Option<HashMap<String, usize>>,
    /// 本模块导出符号名集合
    is_exported: HashSet<String>,
    /// 导出函数名 → local func_idx 映射
    export_funcs: HashMap<String, usize>,
    /// 导出常量名 → local const_idx 映射
    export_consts: HashMap<String, usize>,
    /// 导入结构体：struct_name → (source_module_path, original_struct_id, fields)
    /// type_bitmap 在注册时从字段类型计算
    import_structs: Option<HashMap<String, (String, usize, Vec<(String, String)>)>>,
}

impl<'a> Default for CpsBuilder<'a> {
    fn default() -> Self {
        Self::new(None)
    }
}

impl<'a> CpsBuilder<'a> {
    pub fn new(events: Option<&'a dyn kaubo_log::EventHandler>) -> Self {
        CpsBuilder {
            functions: vec![],
            constants: vec![],
            structs: vec![],
            enums: vec![],
            vtables: vec![],
            const_map: HashMap::new(),
            ctx: FuncCtx::new("main".into()),
            struct_names: HashSet::new(),
            struct_field_types: HashMap::new(),
            function_returns: HashMap::new(),
            method_returns: HashMap::new(),
            enum_names: HashSet::new(),
            variant_to_enum: HashMap::new(),
            variant_field_map: HashMap::new(),
            interface_vtables: HashMap::new(),
            struct_impl_for: HashMap::new(),
            fn_param_hints: HashMap::new(),
            events,
            import_table: None,
            is_exported: HashSet::new(),
            export_funcs: HashMap::new(),
            export_consts: HashMap::new(),
            import_structs: None,
        }
    }

    fn collect_signatures(&mut self, module: &Module) {
        // Inject builtin interface definitions so `impl Add for ...` works without
        // needing explicit `interface Add { ... }` in source.
        for iface_name in &[
            "Add",
            "Subtract",
            "Multiply",
            "Divide",
            "Modulo",
            "Compare",
            "Display",
            "IntoFloat",
            "IntoInt",
        ] {
            self.interface_vtables
                .entry(iface_name.to_string())
                .or_insert_with(|| VtableDef {
                    interface_name: iface_name.to_string(),
                    struct_name: String::new(),
                    methods: vec![],
                });
        }

        for stmt in &module.stmts {
            // Unwrap ExportStmt to reach inner definitions
            let inner = match stmt {
                Stmt::ExportStmt(inner) => inner.as_ref(),
                other => other,
            };
            match inner {
                Stmt::StructDef { name, .. } => {
                    self.struct_names.insert(name.clone());
                }
                Stmt::ConstDecl { name, value, .. }
                | Stmt::VarDecl {
                    name,
                    value: Some(value),
                    ..
                } => {
                    if let Expr::Lambda {
                        ret_ty: Some(ret_ty),
                        ..
                    } = value
                    {
                        self.function_returns
                            .insert(name.clone(), self.type_expr_hint(ret_ty));
                    }
                }
                _ => {}
            }
        }

        for stmt in &module.stmts {
            let inner = match stmt {
                Stmt::ExportStmt(inner) => inner.as_ref(),
                other => other,
            };
            match inner {
                Stmt::StructDef { name, fields, .. } => {
                    let fields = fields
                        .iter()
                        .map(|field| (field.name.clone(), self.type_expr_hint(&field.ty)))
                        .collect();
                    self.struct_field_types.insert(name.clone(), fields);
                }
                Stmt::EnumDef { name, variants, .. } => {
                    self.enum_names.insert(name.clone());
                    for v in variants {
                        self.variant_to_enum.insert(v.name.clone(), name.clone());
                        let fields: Vec<(String, String)> = v
                            .fields
                            .iter()
                            .map(|f| (f.name.clone(), f.ty.to_string()))
                            .collect();
                        self.variant_field_map.insert(v.name.clone(), fields);
                    }
                }
                Stmt::ImplBlock {
                    struct_name,
                    interface_name,
                    methods,
                    ..
                } => {
                    for method in methods {
                        if let Expr::Lambda {
                            ret_ty: Some(ret_ty),
                            ..
                        } = &method.body
                        {
                            self.method_returns.insert(
                                format!("{}.{}", struct_name, method.name),
                                self.type_expr_hint(ret_ty),
                            );
                        }
                    }
                    if let Some(ref iface_name) = interface_name {
                        self.struct_impl_for
                            .entry(struct_name.clone())
                            .or_default()
                            .push(iface_name.clone());
                    }
                }
                Stmt::InterfaceDef { name, .. } => {
                    self.interface_vtables
                        .entry(name.clone())
                        .or_insert_with(|| VtableDef {
                            interface_name: name.clone(),
                            struct_name: String::new(),
                            methods: vec![],
                        });
                }
                _ => {}
            }
        }
    }

    fn type_expr_hint(&self, ty: &TypeExpr) -> ValueHint {
        match ty {
            TypeExpr::Named(name) => match name.as_str() {
                "Int64" => ValueHint::Int,
                "Float64" => ValueHint::Float,
                "String" => ValueHint::String,
                "Bool" => ValueHint::Bool,
                "Null" => ValueHint::Null,
                "List" => ValueHint::List,
                _ if self.struct_names.contains(name) => ValueHint::Struct(name.clone()),
                _ if self.interface_vtables.contains_key(name) => {
                    ValueHint::Interface(name.clone())
                }
                _ => ValueHint::Unknown,
            },
            TypeExpr::List(_) => ValueHint::List,
            TypeExpr::Tuple(_) => ValueHint::Tuple,
            TypeExpr::Arrow { .. } => ValueHint::Unknown,
        }
    }

    fn set_value_hint(&mut self, reg: usize, hint: ValueHint) {
        if !matches!(hint, ValueHint::Unknown) {
            self.ctx.type_map.insert(format!("__r{reg}"), hint);
        }
    }

    fn reg_hint(&self, reg: usize) -> ValueHint {
        self.ctx
            .type_map
            .get(&format!("__r{reg}"))
            .cloned()
            .unwrap_or(ValueHint::Unknown)
    }

    fn value_hint(&self, expr: &Expr) -> ValueHint {
        match expr {
            Expr::VarRef { name, .. } => self
                .ctx
                .type_map
                .get(name)
                .cloned()
                .unwrap_or(ValueHint::Unknown),
            _ => self.expr_hint(expr),
        }
    }

    fn decl_hint(&self, ty_ann: Option<&TypeExpr>, value: Option<&Expr>) -> ValueHint {
        ty_ann
            .map(|ty| self.type_expr_hint(ty))
            .unwrap_or_else(|| value.map_or(ValueHint::Unknown, |expr| self.expr_hint(expr)))
    }

    fn expr_hint(&self, expr: &Expr) -> ValueHint {
        match expr {
            Expr::LitInt(_) => ValueHint::Int,
            Expr::LitFloat(_) => ValueHint::Float,
            Expr::LitString(_) => ValueHint::String,
            Expr::LitTrue | Expr::LitFalse => ValueHint::Bool,
            Expr::LitNull => ValueHint::Null,
            Expr::VarRef { name, .. } => self
                .ctx
                .type_map
                .get(name)
                .cloned()
                .unwrap_or(ValueHint::Unknown),
            Expr::StructLit { name, .. } => ValueHint::Struct(name.clone()),
            Expr::VariantLit {
                enum_name,
                variant_name,
                ..
            } => ValueHint::Struct(format!("{enum_name}::{variant_name}")),
            Expr::GetVariantTag(_) => ValueHint::Int,
            Expr::GetVariantField { .. } => ValueHint::Unknown,
            Expr::ListLit(_) => ValueHint::List,
            Expr::Tuple(_) => ValueHint::Tuple,
            Expr::Binary { left, op, right } => match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    let lh = self.expr_hint(left);
                    let rh = self.expr_hint(right);
                    if lh.is_float() || rh.is_float() {
                        ValueHint::Float
                    } else if let ValueHint::Struct(name) = &lh {
                        ValueHint::Struct(name.clone())
                    } else {
                        ValueHint::Int
                    }
                }
                BinOp::Mod => ValueHint::Int,
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    ValueHint::Bool
                }
                BinOp::And | BinOp::Or => ValueHint::Bool,
                BinOp::SAdd => ValueHint::String,
                BinOp::Pipe | BinOp::GtGt => self.expr_hint(right),
            },
            Expr::Unary { op, right } => match op {
                UnOp::Neg => self.expr_hint(right),
                UnOp::Not => ValueHint::Bool,
            },
            Expr::Member { object, field } => {
                if let ValueHint::Struct(struct_name) = self.expr_hint(object) {
                    self.struct_field_types
                        .get(&struct_name)
                        .and_then(|fields| fields.get(field))
                        .cloned()
                        .unwrap_or(ValueHint::Unknown)
                } else {
                    ValueHint::Unknown
                }
            }
            Expr::Call { func, arg } => {
                let args: Vec<Expr> = arg.as_args().into_iter().cloned().collect();
                self.call_hint(func, &args)
            }
            Expr::Block(stmts) => stmts
                .iter()
                .rev()
                .find_map(|stmt| match stmt {
                    Stmt::ConstDecl { value, .. }
                    | Stmt::VarDecl {
                        value: Some(value), ..
                    } => Some(self.expr_hint(value)),
                    Stmt::ExprStmt(expr) => Some(self.expr_hint(expr)),
                    _ => None,
                })
                .unwrap_or(ValueHint::Null),
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_hint = self.expr_hint(then_branch);
                let else_hint = else_branch
                    .as_ref()
                    .map(|expr| self.expr_hint(expr))
                    .unwrap_or(ValueHint::Null);
                if then_hint == else_hint {
                    then_hint
                } else if then_hint.is_float() || else_hint.is_float() {
                    ValueHint::Float
                } else {
                    ValueHint::Unknown
                }
            }
            Expr::Return(Some(value)) => self.expr_hint(value),
            Expr::Return(None) => ValueHint::Null,
            Expr::Index { .. } => ValueHint::Unknown,
            Expr::Assign { value, .. } => self.expr_hint(value),
            Expr::Lambda { .. } => ValueHint::Unknown,
            Expr::While { .. } | Expr::For { .. } | Expr::Break | Expr::Continue => ValueHint::Null,
            Expr::Async(body) | Expr::Await(body) => self.expr_hint(body),
        }
    }

    fn call_hint(&self, func: &Expr, args: &[Expr]) -> ValueHint {
        match func {
            Expr::Member { object: _, field } if field == "to_string" && args.is_empty() => {
                ValueHint::String
            }
            Expr::Member { object: _, field } if field == "to_float" && args.is_empty() => {
                ValueHint::Float
            }
            Expr::Member { object, field } => {
                if let ValueHint::Struct(struct_name) = self.expr_hint(object) {
                    self.method_returns
                        .get(&format!("{struct_name}.{field}"))
                        .cloned()
                        .unwrap_or(ValueHint::Unknown)
                } else {
                    ValueHint::Unknown
                }
            }
            Expr::VarRef { name, .. } => match name.as_str() {
                "sqrt" | "sin" | "cos" | "floor" | "ceil" => ValueHint::Float,
                "print" => ValueHint::Null,
                "assert" => ValueHint::Null,
                _ => self
                    .function_returns
                    .get(name)
                    .cloned()
                    .unwrap_or(ValueHint::Unknown),
            },
            _ => ValueHint::Unknown,
        }
    }

    fn is_heap_type(&self, ty: &TypeExpr) -> bool {
        match self.type_expr_hint(ty) {
            ValueHint::String
            | ValueHint::Struct(_)
            | ValueHint::List
            | ValueHint::Interface(_)
            | ValueHint::Tuple => true,
            ValueHint::Int | ValueHint::Float | ValueHint::Bool | ValueHint::Null => false,
            ValueHint::Unknown => false,
        }
    }

    pub fn add_const(&mut self, c: Constant) -> usize {
        let key = format!("{c:?}");
        *self.const_map.entry(key).or_insert_with(|| {
            let i = self.constants.len();
            self.constants.push(c);
            i
        })
    }

    fn finalize(&mut self, entry: usize) {
        let f = self.ctx.finalize(entry);
        self.functions.push(f);
    }

    // ── Top-level statement ──

    fn build_top_stmt(&mut self, stmt: &Stmt) -> Result<(usize, usize, usize), String> {
        match stmt {
            Stmt::ConstDecl {
                name,
                ty_ann,
                value,
                ..
            } => {
                if matches!(value, Expr::Lambda { .. }) {
                    let func_idx = self.build_lambda_as_function(value)?;
                    self.ctx.func_map.insert(name.clone(), func_idx);
                    Ok((usize::MAX, usize::MAX, 0))
                } else {
                    let (entry, continu, reg) = self.build_expr(value)?;
                    self.ctx.var_map.insert(name.clone(), reg);
                    self.ctx
                        .type_map
                        .insert(name.clone(), self.decl_hint(ty_ann.as_ref(), Some(value)));
                    Ok((entry, continu, reg))
                }
            }
            Stmt::VarDecl {
                name,
                ty_ann,
                value,
                ..
            } => {
                if let Some(v) = value {
                    if matches!(v, Expr::Lambda { .. }) {
                        let func_idx = self.build_lambda_as_function(v)?;
                        self.ctx.func_map.insert(name.clone(), func_idx);
                        Ok((usize::MAX, usize::MAX, 0))
                    } else {
                        let (entry, continu, reg) = self.build_expr(v)?;
                        self.ctx.var_map.insert(name.clone(), reg);
                        self.ctx
                            .type_map
                            .insert(name.clone(), self.decl_hint(ty_ann.as_ref(), Some(v)));
                        Ok((entry, continu, reg))
                    }
                } else {
                    let r = self.ctx.alloc();
                    self.ctx.var_map.insert(name.clone(), r);
                    self.ctx
                        .type_map
                        .insert(name.clone(), self.decl_hint(ty_ann.as_ref(), None));
                    Ok((usize::MAX, usize::MAX, r))
                }
            }
            Stmt::ExprStmt(e) => self.build_expr(e),
            Stmt::StructDef { name, fields, .. } => {
                let mut bitmap: u64 = 0;
                for (i, f) in fields.iter().enumerate() {
                    if self.is_heap_type(&f.ty) {
                        bitmap |= 1 << i;
                    }
                }
                self.structs.push(StructDef {
                    id: self.structs.len(),
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|f| (f.name.clone(), f.ty.to_string()))
                        .collect(),
                    type_bitmap: bitmap,
                });
                Ok((usize::MAX, usize::MAX, 0))
            }
            Stmt::EnumDef { name, variants, .. } => {
                let id = self.enums.len();
                let mut variant_type_bitmaps = Vec::new();
                for v in variants {
                    let mut bitmap: u64 = 0;
                    for (i, f) in v.fields.iter().enumerate() {
                        if self.is_heap_type(&f.ty) {
                            bitmap |= 1 << i;
                        }
                    }
                    variant_type_bitmaps.push(bitmap);
                }
                self.enums.push(EnumDef {
                    id,
                    name: name.clone(),
                    variants: variants
                        .iter()
                        .enumerate()
                        .map(|(tag, v)| {
                            (
                                v.name.clone(),
                                tag as u16,
                                v.fields
                                    .iter()
                                    .map(|f| (f.name.clone(), f.ty.to_string()))
                                    .collect(),
                            )
                        })
                        .collect(),
                    variant_type_bitmaps,
                });
                Ok((usize::MAX, usize::MAX, 0))
            }
            Stmt::ImplBlock {
                struct_name,
                interface_name,
                methods,
                ..
            } => {
                // Build lambda functions for all methods
                let mut method_funcs: Vec<(String, usize)> = Vec::new();
                for m in methods {
                    let func_idx = self.build_lambda_as_function(&m.body)?;
                    let full_name = format!("{}.{}", struct_name, m.name);
                    method_funcs.push((m.name.clone(), func_idx));
                    // Only register in func_map for non-interface impls (static dispatch)
                    if interface_name.is_none() {
                        self.ctx.func_map.insert(full_name, func_idx);
                    }
                }
                // If implementing an interface, build a vtable for this (struct, interface) pair
                if let Some(ref iface_name) = interface_name {
                    let vdef = VtableDef {
                        interface_name: iface_name.clone(),
                        struct_name: struct_name.clone(),
                        methods: method_funcs,
                    };
                    let _vtable_idx = self.vtables.len();
                    self.vtables.push(vdef.clone());
                    self.interface_vtables.insert(iface_name.clone(), vdef);
                    // Update struct_impl_for to map struct → interface
                    self.struct_impl_for
                        .entry(struct_name.clone())
                        .or_default()
                        .push(iface_name.clone());
                }
                Ok((usize::MAX, usize::MAX, 0))
            }
            Stmt::ExportStmt(inner) => {
                // 记录构建前的常量表长度，用于检测新增常量
                let consts_before = self.constants.len();
                // 递归构建内部声明——export 在 CPS 层是透明的
                let result = self.build_top_stmt(inner)?;
                // 如果是导出函数，记录 name → func_idx 映射
                if let Stmt::ConstDecl {
                    name,
                    value: Expr::Lambda { .. },
                    ..
                }
                | Stmt::VarDecl {
                    name,
                    value: Some(Expr::Lambda { .. }),
                    ..
                } = inner.as_ref()
                {
                    if self.is_exported.contains(name) {
                        if let Some(&func_idx) = self.ctx.func_map.get(name) {
                            self.export_funcs.insert(name.clone(), func_idx);
                        }
                    }
                }
                // 如果是导出常量（非 lambda），记录 name → const_idx 映射
                if let Stmt::ConstDecl { name, value, .. } = inner.as_ref() {
                    if self.is_exported.contains(name) && !matches!(value, Expr::Lambda { .. }) {
                        // 取最后一个新增的常量 idx（如果有的话）
                        if self.constants.len() > consts_before {
                            let const_idx = self.constants.len() - 1;
                            self.export_consts.insert(name.clone(), const_idx);
                        }
                    }
                }
                Ok(result)
            }
            _ => Ok((usize::MAX, usize::MAX, 0)),
        }
    }

    // ── Expression dispatch ──

    fn build_expr(&mut self, expr: &Expr) -> Result<(usize, usize, usize), String> {
        let result = match expr {
            Expr::LitInt(n) => {
                let r = self.ctx.alloc();
                let c = self.add_const(Constant::Int(*n));
                let (e, l) = self.ctx.leaf_block(r, c);
                Ok((e, l, r))
            }
            Expr::LitFloat(n) => {
                let r = self.ctx.alloc();
                let c = self.add_const(Constant::Float(*n));
                let (e, l) = self.ctx.leaf_block(r, c);
                Ok((e, l, r))
            }
            Expr::LitString(s) => {
                let r = self.ctx.alloc();
                let c = self.add_const(Constant::String(s.clone()));
                let (e, l) = self.ctx.leaf_block(r, c);
                Ok((e, l, r))
            }
            Expr::LitTrue => {
                let r = self.ctx.alloc();
                let c = self.add_const(Constant::Int(1));
                let (e, l) = self.ctx.leaf_block(r, c);
                Ok((e, l, r))
            }
            Expr::LitFalse | Expr::LitNull => {
                let r = self.ctx.alloc();
                let c = self.add_const(Constant::Int(0));
                let (e, l) = self.ctx.leaf_block(r, c);
                Ok((e, l, r))
            }
            Expr::VarRef { name, .. } => {
                if let Some(&reg) = self.ctx.var_map.get(name) {
                    let id = self.ctx.new_block();
                    self.ctx.set_block(
                        id,
                        CpsBlock {
                            id,
                            params: vec![],
                            instrs: vec![],
                            term: cps_emit::emit_varref(reg).1,
                        },
                    );
                    Ok((id, id, reg))
                } else if let Some(ref import_table) = self.import_table {
                    if let Some(&handle) = import_table.get(name) {
                        // 导入常量：生成 LoadExternalConst，LinkStage 解析
                        let dst = self.ctx.alloc();
                        let id = self.ctx.new_block();
                        self.ctx.set_block(
                            id,
                            CpsBlock {
                                id,
                                params: vec![],
                                instrs: vec![CpsInstr::LoadExternalConst(dst, handle)],
                                term: cps_emit::emit_return(dst),
                            },
                        );
                        Ok((id, id, dst))
                    } else {
                        Err(format!("undefined variable '{name}'"))
                    }
                } else {
                    Err(format!("undefined variable '{name}'"))
                }
            }
            Expr::Binary { left, op, right } => self.build_binary(left, *op, right),
            Expr::Unary { op, right } => self.build_unary(op, right),
            Expr::Lambda { params, body, .. } => self.build_lambda(params, body),
            Expr::Block(stmts) => self.build_block(stmts),
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => self.build_if(cond, then_branch, else_branch.as_deref()),
            Expr::While { cond, body } => self.build_while(cond, body),
            Expr::For {
                var,
                iterable,
                body,
            } => self.build_for(var, iterable, body),
            Expr::Break => self.build_break(),
            Expr::Continue => self.build_continue(),
            Expr::Return(val) => self.build_return(val.as_deref()),
            Expr::Member { object, field } => self.build_member(object, field),
            Expr::Call { func, arg } => {
                let args: Vec<Expr> = arg.as_args().into_iter().cloned().collect();
                self.build_call(func, &args)
            }
            Expr::ListLit(items) => self.build_list(items),
            Expr::Tuple(items) => self.build_tuple(items),
            Expr::StructLit {
                name,
                fields,
                spread,
            } => {
                if let Some(spread_expr) = spread {
                    self.build_struct_lit_with_spread(name, fields, spread_expr)
                } else {
                    self.build_struct_lit(name, fields)
                }
            }
            Expr::VariantLit {
                enum_name,
                variant_name,
                fields,
                ..
            } => self.build_variant_lit(enum_name, variant_name, fields),
            Expr::GetVariantTag(inner) => {
                let (entry, continu, obj_reg) = self.build_expr(inner)?;
                let dst = self.ctx.alloc();
                let id = self.ctx.new_block();
                let (instrs, _) = cps_emit::emit_get_variant_tag(dst, obj_reg);
                self.ctx.set_block(
                    id,
                    CpsBlock {
                        id,
                        params: vec![],
                        instrs,
                        term: cps_emit::emit_return(dst),
                    },
                );
                self.ctx.chain(continu, id)?;
                Ok((entry, id, dst))
            }
            Expr::GetVariantField { object, field_idx } => {
                let (entry, continu, obj_reg) = self.build_expr(object)?;
                let dst = self.ctx.alloc();
                let id = self.ctx.new_block();
                let (instrs, _) = cps_emit::emit_get_variant_field(dst, obj_reg, *field_idx);
                self.ctx.set_block(
                    id,
                    CpsBlock {
                        id,
                        params: vec![],
                        instrs,
                        term: cps_emit::emit_return(dst),
                    },
                );
                self.ctx.chain(continu, id)?;
                Ok((entry, id, dst))
            }
            Expr::Index { object, index } => self.build_index(object, index),
            Expr::Assign { target, value } => self.build_assign(target, value),
            Expr::Async(body) | Expr::Await(body) => self.build_expr(body),
        }?;
        self.set_value_hint(result.2, self.expr_hint(expr));
        Ok(result)
    }

    fn build_stmt(&mut self, stmt: &Stmt) -> Result<(usize, usize, usize), String> {
        match stmt {
            Stmt::ConstDecl {
                name,
                ty_ann,
                value,
                ..
            } => {
                if matches!(value, Expr::Lambda { .. }) {
                    let func_idx = self.build_lambda_as_function(value)?;
                    self.ctx.func_map.insert(name.clone(), func_idx);
                    Ok((usize::MAX, usize::MAX, 0))
                } else {
                    let (entry, continu, reg) = self.build_expr(value)?;
                    self.ctx.var_map.insert(name.clone(), reg);
                    self.ctx
                        .type_map
                        .insert(name.clone(), self.decl_hint(ty_ann.as_ref(), Some(value)));
                    Ok((entry, continu, reg))
                }
            }
            Stmt::VarDecl {
                name,
                ty_ann,
                value,
                ..
            } => {
                if let Some(v) = value {
                    if matches!(v, Expr::Lambda { .. }) {
                        let func_idx = self.build_lambda_as_function(v)?;
                        self.ctx.func_map.insert(name.clone(), func_idx);
                        Ok((usize::MAX, usize::MAX, 0))
                    } else {
                        let (entry, continu, reg) = self.build_expr(v)?;
                        self.ctx.var_map.insert(name.clone(), reg);
                        self.ctx
                            .type_map
                            .insert(name.clone(), self.decl_hint(ty_ann.as_ref(), Some(v)));
                        Ok((entry, continu, reg))
                    }
                } else {
                    let r = self.ctx.alloc();
                    self.ctx.var_map.insert(name.clone(), r);
                    self.ctx
                        .type_map
                        .insert(name.clone(), self.decl_hint(ty_ann.as_ref(), None));
                    Ok((usize::MAX, usize::MAX, r))
                }
            }
            Stmt::ExprStmt(e) => self.build_expr(e),
            _ => Ok((usize::MAX, usize::MAX, 0)),
        }
    }

    // ── Lambda — creates a separate CpsFunction ──

    /// For top-level lambda bindings: swap ctx, compile body, swap back, return func_idx.
    fn build_lambda_as_function(&mut self, value: &Expr) -> Result<usize, String> {
        if let Expr::Lambda { params, body, .. } = value {
            let mut callee = FuncCtx::new(format!("lambda_{}", self.functions.len()));
            for (i, p) in params.iter().enumerate() {
                callee.var_map.insert(p.name.clone(), i);
                if let Some(ty) = &p.ty_ann {
                    callee
                        .type_map
                        .insert(p.name.clone(), self.type_expr_hint(ty));
                }
            }
            callee.next_reg = params.len().max(1);

            // Swap ctx — build_expr operates on callee
            std::mem::swap(&mut self.ctx, &mut callee);
            let (entry, continu, result_reg) = self.build_expr(body)?;
            // Ensure body ends with Return(result_reg)
            if !matches!(self.ctx.blocks[continu].term, CpsTerminator::Return(_)) {
                let ri = self.ctx.new_block();
                self.ctx.set_block(
                    ri,
                    CpsBlock {
                        id: ri,
                        params: vec![],
                        instrs: vec![],
                        term: CpsTerminator::Return(result_reg),
                    },
                );
                self.ctx.chain(continu, ri)?;
            }
            // Swap back — callee now has the lambda blocks
            std::mem::swap(&mut self.ctx, &mut callee);

            let func = callee.finalize(entry);
            let func_idx = self.functions.len();
            // Record parameter hints for Struct→Interface wrapping at call sites
            let hints: Vec<ValueHint> = params
                .iter()
                .map(|p| {
                    p.ty_ann
                        .as_ref()
                        .map(|t| self.type_expr_hint(t))
                        .unwrap_or(ValueHint::Unknown)
                })
                .collect();
            self.fn_param_hints.insert(func_idx, hints);
            dump_blocks(&format!("lambda_{func_idx}"), &callee);
            self.functions.push(func);
            Ok(func_idx)
        } else {
            Err("expected lambda".into())
        }
    }

    // ── build_lambda for expression position ──

    fn build_lambda(
        &mut self,
        params: &[Param],
        body: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        let mut callee = FuncCtx::new(format!("lambda_{}", self.functions.len()));
        for (i, p) in params.iter().enumerate() {
            callee.var_map.insert(p.name.clone(), i + 1);
            if let Some(ty) = &p.ty_ann {
                callee
                    .type_map
                    .insert(p.name.clone(), self.type_expr_hint(ty));
            }
        }
        callee.next_reg = params.len() + 1;
        callee.next_reg = params.len();

        std::mem::swap(&mut self.ctx, &mut callee);
        let (entry, continu, result_reg) = self.build_expr(body)?;
        if !matches!(self.ctx.blocks[continu].term, CpsTerminator::Return(_)) {
            let ri = self.ctx.new_block();
            self.ctx.set_block(
                ri,
                CpsBlock {
                    id: ri,
                    params: vec![],
                    instrs: vec![],
                    term: CpsTerminator::Return(result_reg),
                },
            );
            self.ctx.chain(continu, ri)?;
        }
        std::mem::swap(&mut self.ctx, &mut callee);

        let func = callee.finalize(entry);
        let func_idx = self.functions.len();
        dump_blocks(&format!("lambda_expr_{func_idx}"), &callee);
        self.functions.push(func);
        let r = self.ctx.alloc();
        let cidx = self.add_const(Constant::Int(func_idx as i64));
        self.ctx
            .func_map
            .insert(format!("lambda_{func_idx}"), func_idx);
        let (e, l) = self.ctx.leaf_block(r, cidx);
        Ok((e, l, r))
    }

    // ── Operator method dispatch helpers ──

    /// Map a built-in method name to the corresponding BinOp for rewrite.
    /// Returns `None` if the method is not a recognized builtin operator.
    fn builtin_method_to_binop(&self, method: &str, _arg_count: usize) -> Option<BinOp> {
        match method {
            "add" => Some(BinOp::Add),
            "subtract" => Some(BinOp::Sub),
            "multiply" => Some(BinOp::Mul),
            "divide" => Some(BinOp::Div),
            "modulo" => Some(BinOp::Mod),
            "less" => Some(BinOp::Lt),
            "less_equal" => Some(BinOp::Le),
            "greater" => Some(BinOp::Gt),
            "greater_equal" => Some(BinOp::Ge),
            "equal" => Some(BinOp::Eq),
            "not_equal" => Some(BinOp::Ne),
            _ => None,
        }
    }

    /// Handle unary-like builtin method calls (neg, not) on builtin types.
    fn build_unary_for_method(
        &mut self,
        method: &str,
        object: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        let (entry, continu, obj_reg) = self.build_expr(object)?;
        let dst = self.ctx.alloc();
        let id = self.ctx.new_block();
        let hint = self.value_hint(object);
        let instrs = match method {
            "neg" if hint.is_float() => {
                vec![CpsInstr::UnOp(dst, CpsUnOp::FNeg, obj_reg)]
            }
            "neg" => {
                vec![CpsInstr::UnOp(dst, CpsUnOp::NegInt, obj_reg)]
            }
            "not" => {
                vec![CpsInstr::UnOp(dst, CpsUnOp::Not, obj_reg)]
            }
            _ => return Err(format!("unknown unary method '{method}'")),
        };
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        self.ctx.chain(continu, id)?;
        Ok((entry, id, dst))
    }

    /// Build operator dispatch for user struct types: a + b → CallIndirect
    #[allow(clippy::too_many_arguments)]
    fn build_operator_dispatch(
        &mut self,
        struct_name: &str,
        method: &str,
        _op: BinOp,
        left_continu: usize,
        left_reg: usize,
        _right_entry: usize,
        right_continu: usize,
        right_reg: usize,
        bl: usize,
        r: usize,
    ) -> Result<(usize, usize, usize), String> {
        // Find vtable for this struct that contains the operator method
        let mut vtable_idx: Option<usize> = None;
        let mut slot: Option<usize> = None;
        for (vi, vdef) in self.vtables.iter().enumerate() {
            if let Some((s, _)) = vdef
                .methods
                .iter()
                .enumerate()
                .find(|(_, (mname, _))| mname == method)
            {
                vtable_idx = Some(vi);
                slot = Some(s);
                break;
            }
        }
        let (vtable_idx, slot) = match (vtable_idx, slot) {
            (Some(vi), Some(s)) => (vi, s),
            _ => {
                // Fallback: try static dispatch via func_map
                let full_name = format!("{struct_name}.{method}");
                let func_idx =
                    self.ctx.func_map.get(&full_name).copied().ok_or_else(|| {
                        format!("operator '{method}' not found for '{struct_name}'")
                    })?;
                // Use the existing build_call_with_idx but with right_reg already built
                // Simpler: chain right_continu then do call
                let result_reg = self.ctx.alloc();
                let cont_block = self.ctx.new_block();
                let move_block0 = self.ctx.new_block();
                self.ctx.set_block(
                    cont_block,
                    CpsBlock {
                        id: cont_block,
                        params: vec![],
                        instrs: vec![],
                        term: CpsTerminator::Jump(move_block0, vec![]),
                    },
                );
                self.ctx.set_block(
                    move_block0,
                    CpsBlock {
                        id: move_block0,
                        params: vec![],
                        instrs: vec![CpsInstr::Move(result_reg, 0)],
                        term: cps_emit::emit_return(result_reg),
                    },
                );
                let call_block = self.ctx.new_block();
                self.ctx.set_block(
                    call_block,
                    CpsBlock {
                        id: call_block,
                        params: vec![],
                        instrs: vec![],
                        term: cps_emit::emit_call(func_idx, vec![left_reg, right_reg], cont_block),
                    },
                );
                self.ctx.chain(left_continu, right_continu)?;
                self.ctx.chain(right_continu, call_block)?;
                return Ok((bl, move_block0, result_reg));
            }
        };

        // LoadVtable
        let vt_r = self.ctx.alloc();
        let (vt_instrs, _) = cps_emit::emit_load_vtable(vt_r, vtable_idx);
        let vt_id = self.ctx.new_block();
        self.ctx.set_block(
            vt_id,
            CpsBlock {
                id: vt_id,
                params: vec![],
                instrs: vt_instrs,
                term: cps_emit::emit_return(vt_r),
            },
        );
        self.ctx.chain(left_continu, vt_id)?;

        // NewInterfaceObj
        let iface_r = self.ctx.alloc();
        let (iface_instrs, _) = cps_emit::emit_new_interface_obj(iface_r, vt_r, left_reg);
        let iface_id = self.ctx.new_block();
        self.ctx.set_block(
            iface_id,
            CpsBlock {
                id: iface_id,
                params: vec![],
                instrs: iface_instrs,
                term: cps_emit::emit_return(iface_r),
            },
        );
        self.ctx.chain(vt_id, iface_id)?;

        // Chain right operand after iface block
        self.ctx.chain(iface_id, right_continu)?;

        // Continuation block
        let cont_block = self.ctx.new_block();
        let move_block = self.ctx.new_block();
        self.ctx.set_block(
            cont_block,
            CpsBlock {
                id: cont_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(move_block, vec![]),
            },
        );
        self.ctx.set_block(
            move_block,
            CpsBlock {
                id: move_block,
                params: vec![],
                instrs: vec![CpsInstr::Move(r, 0)],
                term: cps_emit::emit_return(r),
            },
        );

        // CallIndirect
        let call_block = self.ctx.new_block();
        self.ctx.set_block(
            call_block,
            CpsBlock {
                id: call_block,
                params: vec![],
                instrs: vec![],
                term: cps_emit::emit_call_indirect(slot, vec![iface_r, right_reg], cont_block),
            },
        );
        self.ctx.chain(right_continu, call_block)?;

        self.set_value_hint(r, ValueHint::Struct(struct_name.to_string()));
        Ok((bl, move_block, r))
    }

    // ── build_call — uses func_map to find function index ──

    fn build_call(&mut self, func: &Expr, args: &[Expr]) -> Result<(usize, usize, usize), String> {
        // Builtin operator method rewrite: x.add(y) → x + y → build_binary
        if let Expr::Member { object, field } = func {
            if let Some(binop) = self.builtin_method_to_binop(field, args.len()) {
                if args.len() == 1 {
                    return self.build_binary(object, binop, &args[0]);
                }
                // For unary-like operators (neg, not) or zero-arg
                if args.is_empty() {
                    return self.build_unary_for_method(field, object);
                }
            }
            // to_string() — compile-time rewrite for builtin types only
            // Struct types go through normal method/interface dispatch
            if field == "to_string" && args.is_empty() {
                let hint = self.value_hint(object);
                let is_builtin = matches!(
                    hint,
                    ValueHint::Int
                        | ValueHint::Float
                        | ValueHint::String
                        | ValueHint::Bool
                        | ValueHint::Unknown
                );
                if is_builtin {
                    let (entry, continu, obj_reg) = self.build_expr(object)?;
                    let dst = self.ctx.alloc();
                    let id = self.ctx.new_block();
                    let instrs = if hint == ValueHint::String {
                        vec![CpsInstr::Move(dst, obj_reg)]
                    } else if hint.is_float() {
                        vec![CpsInstr::BinOp(dst, CpsBinOp::FToS, obj_reg, 0)]
                    } else if hint == ValueHint::Bool {
                        vec![CpsInstr::BinOp(dst, CpsBinOp::BToS, obj_reg, 0)]
                    } else {
                        vec![CpsInstr::BinOp(dst, CpsBinOp::IToS, obj_reg, 0)]
                    };
                    self.ctx.set_block(
                        id,
                        CpsBlock {
                            id,
                            params: vec![],
                            instrs,
                            term: cps_emit::emit_return(dst),
                        },
                    );
                    self.ctx.chain(continu, id)?;
                    self.set_value_hint(dst, ValueHint::String);
                    return Ok((entry, id, dst));
                }
                // Struct: fall through to interface dispatch below
            }
            // to_float() — builtin rewrite for Int64/Float64 only
            if field == "to_float" && args.is_empty() {
                let hint = self.value_hint(object);
                if matches!(hint, ValueHint::Int | ValueHint::Float | ValueHint::String) {
                    let (entry, continu, obj_reg) = self.build_expr(object)?;
                    let dst = self.ctx.alloc();
                    let id = self.ctx.new_block();
                    self.ctx.set_block(
                        id,
                        CpsBlock {
                            id,
                            params: vec![],
                            instrs: vec![CpsInstr::BinOp(dst, CpsBinOp::IToF, obj_reg, 0)],
                            term: cps_emit::emit_return(dst),
                        },
                    );
                    self.ctx.chain(continu, id)?;
                    self.set_value_hint(dst, ValueHint::Float);
                    return Ok((entry, id, dst));
                }
                // Struct: fall through to method dispatch
            }
            // to_int() — builtin rewrite for String/Float64 only
            if field == "to_int" && args.is_empty() {
                let hint = self.value_hint(object);
                if matches!(hint, ValueHint::Int | ValueHint::Float | ValueHint::String) {
                    let (entry, continu, obj_reg) = self.build_expr(object)?;
                    let dst = self.ctx.alloc();
                    let id = self.ctx.new_block();
                    self.ctx.set_block(
                        id,
                        CpsBlock {
                            id,
                            params: vec![],
                            instrs: vec![CpsInstr::BinOp(dst, CpsBinOp::SToI, obj_reg, 0)],
                            term: cps_emit::emit_return(dst),
                        },
                    );
                    self.ctx.chain(continu, id)?;
                    self.set_value_hint(dst, ValueHint::Int);
                    return Ok((entry, id, dst));
                }
                // Struct: fall through to method dispatch
            }
            // Try struct method call: obj.method(args)
            for sd in &self.structs.clone() {
                let full_name = format!("{}.{}", sd.name, field);
                if let Some(&func_idx) = self.ctx.func_map.get(&full_name) {
                    // Build args: [self_obj, ...user_args]
                    let mut all_args: Vec<Expr> = vec![object.as_ref().clone()];
                    all_args.extend_from_slice(args);
                    let result = self.build_call_with_idx(func_idx, &all_args)?;
                    self.set_value_hint(
                        result.2,
                        self.method_returns
                            .get(&full_name)
                            .cloned()
                            .unwrap_or(ValueHint::Unknown),
                    );
                    return Ok(result);
                }
            }
            // Interface method dispatch: try each vtable for matching method
            {
                // Quick path: object already has Interface type → direct CallIndirect
                let obj_hint = self.value_hint(object);
                if let ValueHint::Interface(iface_name) = &obj_hint {
                    let (obj_entry, obj_continu, obj_reg) = self.build_expr(object)?;
                    let vtables = self.vtables.clone();
                    for (vi, vdef) in vtables.iter().enumerate() {
                        if vdef.interface_name == *iface_name {
                            if let Some((slot, _)) = vdef
                                .methods
                                .iter()
                                .enumerate()
                                .find(|(_, (mname, _))| mname == field)
                            {
                                let mut arg_regs = vec![obj_reg];
                                let mut prev_c = Some(obj_continu);
                                for arg in args {
                                    let (e, c, r) = self.build_expr(arg)?;
                                    if let Some(t) = prev_c {
                                        self.ctx.chain(t, e)?;
                                    }
                                    prev_c = Some(c);
                                    arg_regs.push(r);
                                }
                                let result_reg = self.ctx.alloc();
                                let cont_block = self.ctx.new_block();
                                let mb = self.ctx.new_block();
                                self.ctx.set_block(
                                    cont_block,
                                    CpsBlock {
                                        id: cont_block,
                                        params: vec![],
                                        instrs: vec![],
                                        term: CpsTerminator::Jump(mb, vec![]),
                                    },
                                );
                                self.ctx.set_block(
                                    mb,
                                    CpsBlock {
                                        id: mb,
                                        params: vec![],
                                        instrs: vec![CpsInstr::Move(result_reg, 0)],
                                        term: cps_emit::emit_return(result_reg),
                                    },
                                );
                                let call_block = self.ctx.new_block();
                                self.ctx.set_block(
                                    call_block,
                                    CpsBlock {
                                        id: call_block,
                                        params: vec![],
                                        instrs: vec![],
                                        term: cps_emit::emit_call_indirect(
                                            slot, arg_regs, cont_block,
                                        ),
                                    },
                                );
                                if let Some(t) = prev_c {
                                    self.ctx.chain(t, call_block)?;
                                }
                                return Ok((obj_entry, mb, result_reg));
                            }
                        }
                    }
                    return Err(format!(
                        "method '{field}' not found on interface {iface_name}"
                    ));
                }
                let vtables = self.vtables.clone();
                for (vi, vdef) in vtables.iter().enumerate() {
                    if let Some((slot, _)) = vdef
                        .methods
                        .iter()
                        .enumerate()
                        .find(|(_, (mname, _))| mname == field)
                    {
                        // Build object expression
                        let (obj_entry, obj_continu, obj_reg) = self.build_expr(object)?;
                        // LoadVtable
                        let vt_r = self.ctx.alloc();
                        let (vt_instrs, _) = cps_emit::emit_load_vtable(vt_r, vi);
                        let vt_id = self.ctx.new_block();
                        self.ctx.set_block(
                            vt_id,
                            CpsBlock {
                                id: vt_id,
                                params: vec![],
                                instrs: vt_instrs,
                                term: cps_emit::emit_return(vt_r),
                            },
                        );
                        self.ctx.chain(obj_continu, vt_id)?;
                        // NewInterfaceObj
                        let iface_r = self.ctx.alloc();
                        let (iface_instrs, _) =
                            cps_emit::emit_new_interface_obj(iface_r, vt_r, obj_reg);
                        let iface_id = self.ctx.new_block();
                        self.ctx.set_block(
                            iface_id,
                            CpsBlock {
                                id: iface_id,
                                params: vec![],
                                instrs: iface_instrs,
                                term: cps_emit::emit_return(iface_r),
                            },
                        );
                        self.ctx.chain(vt_id, iface_id)?;
                        // Build args chaining from iface_id
                        let mut prev_c = Some(iface_id);
                        let mut arg_regs = vec![iface_r]; // first arg = InterfaceObj handle
                        for arg in args {
                            let (e, c, r) = self.build_expr(arg)?;
                            if let Some(t) = prev_c {
                                self.ctx.chain(t, e)?;
                            }
                            prev_c = Some(c);
                            arg_regs.push(r);
                        }
                        // Continuation: cont_block → move_block → Return
                        let result_reg = self.ctx.alloc();
                        let cont_block = self.ctx.new_block();
                        let move_block2 = self.ctx.new_block();
                        self.ctx.set_block(
                            cont_block,
                            CpsBlock {
                                id: cont_block,
                                params: vec![],
                                instrs: vec![],
                                term: CpsTerminator::Jump(move_block2, vec![]),
                            },
                        );
                        self.ctx.set_block(
                            move_block2,
                            CpsBlock {
                                id: move_block2,
                                params: vec![],
                                instrs: vec![CpsInstr::Move(result_reg, 0)],
                                term: cps_emit::emit_return(result_reg),
                            },
                        );
                        // CallIndirect
                        let call_block = self.ctx.new_block();
                        self.ctx.set_block(
                            call_block,
                            CpsBlock {
                                id: call_block,
                                params: vec![],
                                instrs: vec![],
                                term: cps_emit::emit_call_indirect(slot, arg_regs, cont_block),
                            },
                        );
                        if let Some(t) = prev_c {
                            self.ctx.chain(t, call_block)?;
                        }
                        return Ok((obj_entry, move_block2, result_reg));
                    }
                }
            }
        }

        if let Expr::VarRef { name, .. } = func {
            // Check if this is a variant constructor call: Some(42) → NewVariant
            if let Some(enum_name) = self.variant_to_enum.get(name).cloned() {
                return self.build_variant_construct(&enum_name, name, args);
            }
            // Look up user-defined function first (allows shadowing builtins)
            if let Some(&func_idx) = self.ctx.func_map.get(name) {
                let result = self.build_call_with_idx(func_idx, args)?;
                self.set_value_hint(
                    result.2,
                    self.function_returns
                        .get(name)
                        .cloned()
                        .unwrap_or(ValueHint::Unknown),
                );
                return Ok(result);
            }
            // ── builtins dispatch (fallback) ──
            if let Some(bi) = get_builtin(name) {
                let hint = builtin_return_hint(name);
                return match bi {
                    Builtin::Print => {
                        if args.len() != 1 {
                            return Err("print expects 1 argument".into());
                        }
                        let (entry, continu, reg) = self.build_expr(&args[0])?;
                        let id = self.ctx.new_block();
                        self.ctx.set_block(
                            id,
                            CpsBlock {
                                id,
                                params: vec![],
                                instrs: vec![CpsInstr::Print(reg)],
                                term: cps_emit::emit_return(reg),
                            },
                        );
                        self.ctx.chain(continu, id)?;
                        self.set_value_hint(reg, hint);
                        Ok((entry, id, reg))
                    }
                    Builtin::Native(ni) => {
                        let result = self.build_native_call(ni, args)?;
                        self.set_value_hint(result.2, hint);
                        Ok(result)
                    }
                    Builtin::Inline(instr) => {
                        let _ = instr;
                        Err("inline builtins not yet routed".into())
                    }
                };
            }
            // ★ 检查导入表：如果名字在 import_table 中，生成 CallExternal
            if let Some(ref import_table) = self.import_table {
                if let Some(&import_handle) = import_table.get(name) {
                    return self.build_call_external(import_handle, args);
                }
            }
            return Err(format!("undefined function '{name}'"));
        }
        Err("call target is not a simple name".to_string())
    }

    /// 构建对外部模块函数的调用——生成 `CallExternal` 终结器。
    fn build_call_external(
        &mut self,
        import_handle: usize,
        args: &[Expr],
    ) -> Result<(usize, usize, usize), String> {
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut arg_regs = Vec::new();
        for arg in args {
            let (e, c, r) = self.build_expr(arg)?;
            if entry == 0 {
                entry = e;
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(c);
            arg_regs.push(r);
        }
        let result_reg = self.ctx.alloc();
        let cont_block = self.ctx.new_block();
        let move_block = self.ctx.new_block();
        self.ctx.set_block(
            cont_block,
            CpsBlock {
                id: cont_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(move_block, vec![]),
            },
        );
        self.ctx.set_block(
            move_block,
            CpsBlock {
                id: move_block,
                params: vec![],
                instrs: vec![CpsInstr::Move(result_reg, 0)],
                term: cps_emit::emit_return(result_reg),
            },
        );
        let call_block = self.ctx.new_block();
        self.ctx.set_block(
            call_block,
            CpsBlock {
                id: call_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::CallExternal {
                    import_handle,
                    args: arg_regs,
                    ret_block: cont_block,
                },
            },
        );
        if let Some(t) = prev_c {
            self.ctx.chain(t, call_block)?;
        }
        Ok((entry, move_block, result_reg))
    }

    fn build_call_with_idx(
        &mut self,
        func_idx: usize,
        args: &[Expr],
    ) -> Result<(usize, usize, usize), String> {
        let param_hints = self.fn_param_hints.get(&func_idx).cloned();
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut arg_regs = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let (e, c, r) = self.build_expr(arg)?;
            if entry == 0 {
                entry = e;
            }
            let mut arg_r = r;
            let mut arg_continu = c;
            // Struct→Interface wrapping: if param expects Interface but arg is Struct
            if let Some(ref hints) = param_hints {
                if i < hints.len() {
                    if let ValueHint::Interface(ref iface_name) = hints[i] {
                        let arg_hint = self.reg_hint(r);
                        if let ValueHint::Struct(ref struct_name) = arg_hint {
                            if let Some(vi) = self.vtables.iter().position(|v| {
                                v.interface_name == *iface_name && v.struct_name == *struct_name
                            }) {
                                let vt_r = self.ctx.alloc();
                                let vt_id = self.ctx.new_block();
                                self.ctx.set_block(
                                    vt_id,
                                    CpsBlock {
                                        id: vt_id,
                                        params: vec![],
                                        instrs: (cps_emit::emit_load_vtable(vt_r, vi)).0,
                                        term: cps_emit::emit_return(vt_r),
                                    },
                                );
                                self.ctx.chain(arg_continu, vt_id)?;
                                let iface_r = self.ctx.alloc();
                                let iface_id = self.ctx.new_block();
                                self.ctx.set_block(
                                    iface_id,
                                    CpsBlock {
                                        id: iface_id,
                                        params: vec![],
                                        instrs: (cps_emit::emit_new_interface_obj(
                                            iface_r, vt_r, r,
                                        ))
                                        .0,
                                        term: cps_emit::emit_return(iface_r),
                                    },
                                );
                                self.ctx.chain(vt_id, iface_id)?;
                                arg_r = iface_r;
                                arg_continu = iface_id;
                            }
                        }
                    }
                }
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(arg_continu);
            arg_regs.push(arg_r);
        }
        let result_reg = self.ctx.alloc();
        let cont_block = self.ctx.new_block();
        let move_block = self.ctx.new_block();
        self.ctx.set_block(
            cont_block,
            CpsBlock {
                id: cont_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(move_block, vec![]),
            },
        );
        self.ctx.set_block(
            move_block,
            CpsBlock {
                id: move_block,
                params: vec![],
                instrs: vec![CpsInstr::Move(result_reg, 0)],
                term: cps_emit::emit_return(result_reg),
            },
        );
        let call_block = self.ctx.new_block();
        self.ctx.set_block(
            call_block,
            CpsBlock {
                id: call_block,
                params: vec![],
                instrs: vec![],
                term: cps_emit::emit_call(func_idx, arg_regs, cont_block),
            },
        );
        if let Some(t) = prev_c {
            self.ctx.chain(t, call_block)?;
        }
        let entry = if entry != 0 { entry } else { call_block };
        Ok((entry, move_block, result_reg))
    }

    fn build_native_call(
        &mut self,
        native_idx: usize,
        args: &[Expr],
    ) -> Result<(usize, usize, usize), String> {
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut arg_regs = Vec::new();
        for arg in args {
            let (e, c, r) = self.build_expr(arg)?;
            if entry == 0 {
                entry = e;
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(c);
            arg_regs.push(r);
        }
        let result_reg = self.ctx.alloc();
        let move_block = self.ctx.new_block();
        self.ctx.set_block(
            move_block,
            CpsBlock {
                id: move_block,
                params: vec![],
                instrs: vec![CpsInstr::Move(result_reg, 0)],
                term: cps_emit::emit_return(result_reg),
            },
        );
        let call_block = self.ctx.new_block();
        self.ctx.set_block(
            call_block,
            CpsBlock {
                id: call_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::CallNative(native_idx, arg_regs, move_block),
            },
        );
        if let Some(t) = prev_c {
            self.ctx.chain(t, call_block)?;
        }
        if entry == 0 {
            entry = call_block;
        }
        Ok((entry, move_block, result_reg))
    }

    // ── Complex expressions (delegate to ctx) ──

    fn build_binary(
        &mut self,
        left: &Expr,
        op: BinOp,
        right: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        // and / or: short-circuit branching
        if op == BinOp::And || op == BinOp::Or {
            return self.build_logical(left, op, right);
        }
        // pipe: desugar a |> f → f(a)
        if op == BinOp::Pipe {
            return self.build_call(right, &[left.clone()]);
        }
        let (bl, cl, rl) = self.build_expr(left)?;
        let (br, cr, rr) = self.build_expr(right)?;
        let r = self.ctx.alloc();
        let lhs_hint = self.reg_hint(rl);
        let rhs_hint = self.reg_hint(rr);

        // Check if operands are user struct types → try operator dispatch
        let lhs_struct = match &lhs_hint {
            ValueHint::Struct(name) => Some(name.clone()),
            _ => None,
        };
        if let Some(struct_name) = lhs_struct {
            // Try to find an operator method for this struct
            let op_method = match op {
                BinOp::Add => Some("add"),
                BinOp::Sub => Some("subtract"),
                BinOp::Mul => Some("multiply"),
                BinOp::Div => Some("divide"),
                BinOp::Mod => Some("modulo"),
                BinOp::Eq => Some("equal"),
                BinOp::Ne => Some("not_equal"),
                BinOp::Lt => Some("less"),
                BinOp::Le => Some("less_equal"),
                BinOp::Gt => Some("greater"),
                BinOp::Ge => Some("greater_equal"),
                _ => None,
            };
            if let Some(method) = op_method {
                let full_name = format!("{struct_name}.{method}");
                if self
                    .interface_vtables
                    .values()
                    .any(|v| v.methods.iter().any(|(mname, _)| mname == method))
                    || self.ctx.func_map.contains_key(&full_name)
                {
                    // Found operator method — use interface dispatch path
                    // Build the left operand (self) then the right operand, then CallIndirect
                    return self.build_operator_dispatch(
                        &struct_name,
                        method,
                        op,
                        cl,
                        rl,
                        br,
                        cr,
                        rr,
                        bl,
                        r,
                    );
                }
            }
        }

        let is_float = lhs_hint.is_float() || rhs_hint.is_float();
        let is_string = lhs_hint == ValueHint::String || rhs_hint == ValueHint::String;
        let (binop, sl, sr) = match op {
            BinOp::Add if is_string => (CpsBinOp::SAdd, rl, rr),
            BinOp::Gt if !is_float => (CpsBinOp::GtInt, rl, rr),
            BinOp::Ge if !is_float => (CpsBinOp::GeInt, rl, rr),
            _ => (bin_op_to_cps(op, is_float)?, rl, rr),
        };
        let (instrs, _) = cps_emit::emit_binary(r, binop, sl, sr);
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(r),
            },
        );
        if br != 0 {
            self.ctx.chain(cl, br)?;
            self.ctx.chain(cr, id)?;
        } else {
            self.ctx.chain(cl, id)?;
        }
        let entry = bl;
        let result_hint = self.expr_hint(&Expr::Binary {
            left: Box::new(left.clone()),
            op,
            right: Box::new(right.clone()),
        });
        self.set_value_hint(r, result_hint);
        Ok((entry, id, r))
    }

    fn build_unary(&mut self, op: &UnOp, right: &Expr) -> Result<(usize, usize, usize), String> {
        let (entry, continu, r) = self.build_expr(right)?;
        let dst = self.ctx.alloc();
        let unop = match op {
            UnOp::Neg => CpsUnOp::NegInt,
            UnOp::Not => CpsUnOp::Not,
        };
        let (instrs, _) = cps_emit::emit_unary(dst, unop, r);
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        self.ctx.chain(continu, id)?;
        Ok((entry, id, dst))
    }

    fn build_block(&mut self, stmts: &[Stmt]) -> Result<(usize, usize, usize), String> {
        let mut first: Option<usize> = None;
        let mut last_reg: usize = 0;
        let mut last_continu: Option<usize> = None;
        for stmt in stmts {
            let (entry, continu, reg) = self.build_stmt(stmt)?;
            if entry == usize::MAX {
                continue;
            }
            if first.is_none() {
                first = Some(entry);
            }
            if let Some(t) = last_continu {
                self.ctx.chain(t, entry)?;
            }
            last_continu = Some(continu);
            last_reg = reg;
        }
        Ok((first.unwrap_or(0), last_continu.unwrap_or(0), last_reg))
    }

    fn build_logical(
        &mut self,
        left: &Expr,
        op: BinOp,
        right: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        let (cond_entry, cond_continu, cond_reg) = self.build_expr(left)?;
        let (right_entry, right_continu, right_reg) = self.build_expr(right)?;

        let short_val = if op == BinOp::And { 0 } else { 1 };
        let short_const = self.add_const(Constant::Int(short_val));
        let short_reg = self.ctx.alloc();
        let short_block = self.ctx.new_block();
        self.ctx.set_block(
            short_block,
            CpsBlock {
                id: short_block,
                params: vec![],
                instrs: vec![CpsInstr::LoadConst(short_reg, short_const)],
                term: CpsTerminator::Return(short_reg),
            },
        );

        let branch = self.ctx.new_block();
        let (true_target, false_target) = if op == BinOp::And {
            (right_entry, short_block)
        } else {
            (short_block, right_entry)
        };
        self.ctx.set_block(
            branch,
            CpsBlock {
                id: branch,
                params: vec![],
                instrs: vec![],
                term: cps_emit::emit_branch(cond_reg, true_target, false_target),
            },
        );

        let merge_reg = self.ctx.alloc();
        let merge = self.ctx.new_block();
        self.ctx.set_block(
            merge,
            CpsBlock {
                id: merge,
                params: vec![merge_reg],
                instrs: vec![],
                term: CpsTerminator::Return(merge_reg),
            },
        );

        self.ctx.chain(cond_continu, branch)?;
        self.ctx
            .rewire_return_args(right_continu, merge, &[right_reg])?;
        self.ctx
            .rewire_return_args(short_block, merge, &[short_reg])?;

        let entry = if cond_entry != 0 { cond_entry } else { branch };
        self.set_value_hint(merge_reg, ValueHint::Bool);
        Ok((entry, merge, merge_reg))
    }

    fn build_if(
        &mut self,
        cond: &Expr,
        then_b: &Expr,
        else_b: Option<&Expr>,
    ) -> Result<(usize, usize, usize), String> {
        let (cond_entry, cond_continu, cond_reg) = self.build_expr(cond)?;
        let (then_entry, then_continu, then_reg) = self.build_expr(then_b)?;
        if let Some(eb) = else_b {
            let (else_entry, else_continu, else_reg) = self.build_expr(eb)?;
            let branch = self.ctx.new_block();
            self.ctx.set_block(
                branch,
                CpsBlock {
                    id: branch,
                    params: vec![],
                    instrs: vec![],
                    term: cps_emit::emit_branch(cond_reg, then_entry, else_entry),
                },
            );
            let merge_reg = self.ctx.alloc();
            let merge = self.ctx.new_block();
            self.ctx.set_block(
                merge,
                CpsBlock {
                    id: merge,
                    params: vec![merge_reg],
                    instrs: vec![],
                    term: CpsTerminator::Return(merge_reg),
                },
            );
            self.ctx
                .rewire_return_args(then_continu, merge, &[then_reg])?;
            self.ctx
                .rewire_return_args(else_continu, merge, &[else_reg])?;
            if cond_entry != 0 {
                self.ctx.chain(cond_continu, branch)?;
            }
            let entry = if cond_entry != 0 { cond_entry } else { branch };
            Ok((entry, merge, merge_reg))
        } else {
            let skip_block = self.ctx.new_block();
            self.ctx.set_block(
                skip_block,
                CpsBlock {
                    id: skip_block,
                    params: vec![],
                    instrs: vec![],
                    term: CpsTerminator::Return(0),
                },
            );
            let branch = self.ctx.new_block();
            self.ctx.set_block(
                branch,
                CpsBlock {
                    id: branch,
                    params: vec![],
                    instrs: vec![],
                    term: cps_emit::emit_branch(cond_reg, then_entry, skip_block),
                },
            );
            self.ctx.chain(then_continu, skip_block)?;
            if cond_entry != 0 {
                self.ctx.chain(cond_continu, branch)?;
            }
            let entry = if cond_entry != 0 { cond_entry } else { branch };
            Ok((entry, skip_block, then_reg))
        }
    }

    fn build_for(
        &mut self,
        var: &Param,
        iterable: &Expr,
        body: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        // Build the iterable (list)
        let (list_entry, list_continu, list_reg) = self.build_expr(iterable)?;

        // Get list length
        let len_reg = self.ctx.alloc();
        let len_block = self.ctx.new_block();
        self.ctx.set_block(
            len_block,
            CpsBlock {
                id: len_block,
                params: vec![],
                instrs: vec![CpsInstr::ListLen(len_reg, list_reg)],
                term: CpsTerminator::Return(0),
            },
        );
        self.ctx.chain(list_continu, len_block)?;

        // Initialize index counter to 0
        let idx_reg = self.ctx.alloc();
        let idx_const = self.add_const(Constant::Int(0));
        let init_idx_block = self.ctx.new_block();
        self.ctx.set_block(
            init_idx_block,
            CpsBlock {
                id: init_idx_block,
                params: vec![],
                instrs: vec![CpsInstr::LoadConst(idx_reg, idx_const)],
                term: CpsTerminator::Return(0),
            },
        );
        // Chain: len_block → init_idx_block
        self.ctx.chain(len_block, init_idx_block)?;

        // Emit condition: idx < len
        let cmp_reg = self.ctx.alloc();
        let header_block = self.ctx.new_block();
        let body_entry_block = self.ctx.new_block();
        let exit_block = self.ctx.new_block();
        self.ctx.loop_stack.push((header_block, exit_block));

        self.ctx.set_block(
            header_block,
            CpsBlock {
                id: header_block,
                params: vec![],
                instrs: vec![CpsInstr::BinOp(cmp_reg, CpsBinOp::LtInt, idx_reg, len_reg)],
                term: cps_emit::emit_branch(cmp_reg, body_entry_block, exit_block),
            },
        );

        // Get element: elem = list[idx]
        let elem_reg = self.ctx.alloc();
        let get_elem_block = self.ctx.new_block();
        self.ctx.set_block(
            get_elem_block,
            CpsBlock {
                id: get_elem_block,
                params: vec![],
                instrs: vec![CpsInstr::IndexGet(elem_reg, list_reg, idx_reg)],
                term: CpsTerminator::Return(elem_reg),
            },
        );

        // Bind loop variable to element
        let old_var = self.ctx.var_map.insert(var.name.clone(), elem_reg);

        // Build body
        let (body_entry, body_continu, _) = self.build_expr(body)?;

        // Restore variable shadowing
        if let Some(old) = old_var {
            self.ctx.var_map.insert(var.name.clone(), old);
        } else {
            self.ctx.var_map.remove(&var.name);
        }

        // Increment index: idx = idx + 1
        let one_reg = self.ctx.alloc();
        let one_const = self.add_const(Constant::Int(1));
        let inc_block = self.ctx.new_block();
        self.ctx.set_block(
            inc_block,
            CpsBlock {
                id: inc_block,
                params: vec![],
                instrs: vec![
                    CpsInstr::LoadConst(one_reg, one_const),
                    CpsInstr::BinOp(idx_reg, CpsBinOp::AddInt, idx_reg, one_reg),
                ],
                term: CpsTerminator::Return(0),
            },
        );

        // Chain: init_idx → header
        self.ctx.chain(init_idx_block, header_block)?;
        // Chain body: body_entry → get_elem
        self.ctx.set_block(
            body_entry_block,
            CpsBlock {
                id: body_entry_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(get_elem_block, vec![]),
            },
        );
        // Chain: get_elem → body entry
        self.ctx.chain(get_elem_block, body_entry)?;
        // Chain: body_continu → inc
        self.ctx.chain(body_continu, inc_block)?;
        // Chain: inc → header (loop back)
        self.ctx.chain(inc_block, header_block)?;

        self.ctx.set_block(
            exit_block,
            CpsBlock {
                id: exit_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Return(0),
            },
        );

        self.ctx.loop_stack.pop();

        // Entry path: list_entry → len → init_idx → header
        let entry = if list_entry != 0 {
            list_entry
        } else {
            len_block
        };
        Ok((entry, exit_block, 0))
    }

    fn build_while(&mut self, cond: &Expr, body: &Expr) -> Result<(usize, usize, usize), String> {
        let loop_header = self.ctx.new_block();
        let body_block = self.ctx.new_block();
        let exit_block = self.ctx.new_block();
        self.ctx.loop_stack.push((loop_header, exit_block));

        let (cond_entry, cond_continu, cond_reg) = self.build_expr(cond)?;
        let (body_entry, body_continu, _) = self.build_expr(body)?;

        self.ctx.set_block(
            loop_header,
            CpsBlock {
                id: loop_header,
                params: vec![],
                instrs: vec![],
                term: cps_emit::emit_branch(cond_reg, body_block, exit_block),
            },
        );
        self.ctx.set_block(
            body_block,
            CpsBlock {
                id: body_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(body_entry, vec![]),
            },
        );
        self.ctx.set_block(
            exit_block,
            CpsBlock {
                id: exit_block,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Return(0),
            },
        );

        emit!(
            self.events,
            kaubo_log::ToolchainEvent::Cps(kaubo_log::CpsEvent::WhileLowered {
                header: loop_header,
                body: body_block,
                exit: exit_block,
            })
        );

        self.ctx.chain(cond_continu, loop_header)?;
        self.ctx.chain(body_continu, cond_entry)?;
        self.ctx.loop_stack.pop();
        Ok((cond_entry, exit_block, 0))
    }

    fn build_break(&mut self) -> Result<(usize, usize, usize), String> {
        let (_, brk) = self
            .ctx
            .loop_stack
            .last()
            .copied()
            .ok_or("break outside loop")?;
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(brk, vec![]),
            },
        );
        // Return MAX as continu — break terminates the chain, caller must not chain it
        Ok((id, usize::MAX, 0))
    }

    fn build_continue(&mut self) -> Result<(usize, usize, usize), String> {
        let (cont, _) = self
            .ctx
            .loop_stack
            .last()
            .copied()
            .ok_or("continue outside loop")?;
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs: vec![],
                term: CpsTerminator::Jump(cont, vec![]),
            },
        );
        Ok((id, id, 0))
    }

    fn build_return(&mut self, val: Option<&Expr>) -> Result<(usize, usize, usize), String> {
        if let Some(v) = val {
            if let Expr::LitInt(n) = v {
                let r = self.ctx.alloc();
                let c = self.add_const(Constant::Int(*n));
                let id = self.ctx.new_block();
                self.ctx.set_block(
                    id,
                    CpsBlock {
                        id,
                        params: vec![],
                        instrs: vec![CpsInstr::LoadConst(r, c)],
                        term: cps_emit::emit_return(r),
                    },
                );
                return Ok((id, id, r));
            }
            let (entry, continu, r) = self.build_expr(v)?;
            let id = self.ctx.new_block();
            self.ctx.set_block(
                id,
                CpsBlock {
                    id,
                    params: vec![],
                    instrs: vec![],
                    term: cps_emit::emit_return(r),
                },
            );
            self.ctx.chain(continu, id)?;
            Ok((entry, id, r))
        } else {
            let id = self.ctx.new_block();
            self.ctx.set_block(
                id,
                CpsBlock {
                    id,
                    params: vec![],
                    instrs: vec![],
                    term: CpsTerminator::Return(0),
                },
            );
            Ok((id, id, 0))
        }
    }

    fn build_member(
        &mut self,
        object: &Expr,
        field: &str,
    ) -> Result<(usize, usize, usize), String> {
        let (entry, continu, obj_reg) = self.build_expr(object)?;
        let dst = self.ctx.alloc();
        let struct_name = match self.value_hint(object) {
            ValueHint::Struct(name) => name,
            other => {
                return Err(format!(
                    "cannot access field '{field}' on non-struct value {other:?}"
                ))
            }
        };
        let sd = self
            .structs
            .iter()
            .find(|s| s.name == struct_name)
            .ok_or_else(|| format!("unknown struct '{struct_name}'"))?;
        let fi = sd
            .fields
            .iter()
            .position(|(n, _)| n == field)
            .ok_or_else(|| format!("field '{field}' not found on struct '{struct_name}'"))?
            as u16;
        let (instrs, _) = cps_emit::emit_get_field(dst, obj_reg, fi);
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        self.ctx.chain(continu, id)?;
        Ok((entry, id, dst))
    }

    fn build_list(&mut self, items: &[Expr]) -> Result<(usize, usize, usize), String> {
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut regs = Vec::new();
        for item in items {
            let (e, c, r) = self.build_expr(item)?;
            if entry == 0 {
                entry = e;
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(c);
            regs.push(r);
        }
        let dst = self.ctx.alloc();
        let id = self.ctx.new_block();
        // 元素类型一致时选择对应的 TypedArray 指令
        let all_int = items.iter().all(|e| matches!(e, Expr::LitInt(_)));
        let all_float = items.iter().all(|e| matches!(e, Expr::LitFloat(_)));
        let instr = if all_float {
            CpsInstr::NewFloat64Array(dst, regs.clone())
        } else if all_int {
            CpsInstr::NewInt64Array(dst, regs.clone())
        } else {
            CpsInstr::NewList(dst, regs.clone())
        };
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: regs.clone(),
                instrs: vec![instr],
                term: cps_emit::emit_return(dst),
            },
        );
        if let Some(t) = prev_c {
            // Rewire to pass element regs as jump args (not chain's empty args)
            if let Some(block) = self.ctx.blocks.get_mut(t) {
                block.term = CpsTerminator::Jump(id, regs.clone());
            }
        }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_tuple(&mut self, items: &[Expr]) -> Result<(usize, usize, usize), String> {
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut regs = Vec::new();
        for item in items {
            let (e, c, r) = self.build_expr(item)?;
            if entry == 0 {
                entry = e;
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(c);
            regs.push(r);
        }
        let dst = self.ctx.alloc();
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: regs.clone(),
                instrs: vec![CpsInstr::NewTuple(dst, regs.clone())],
                term: cps_emit::emit_return(dst),
            },
        );
        if let Some(t) = prev_c {
            if let Some(block) = self.ctx.blocks.get_mut(t) {
                block.term = CpsTerminator::Jump(id, regs.clone());
            }
        }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_struct_lit(
        &mut self,
        struct_name: &str,
        fields: &[(String, Expr)],
    ) -> Result<(usize, usize, usize), String> {
        let sd = self
            .structs
            .iter()
            .find(|s| s.name == struct_name)
            .cloned()
            .ok_or_else(|| format!("unknown struct '{struct_name}'"))?;
        for (name, _) in fields {
            if !sd.fields.iter().any(|(declared, _)| declared == name) {
                return Err(format!(
                    "field '{name}' not found on struct '{struct_name}'"
                ));
            }
        }
        for (declared, _) in &sd.fields {
            if !fields.iter().any(|(name, _)| name == declared) {
                return Err(format!(
                    "missing field '{declared}' for struct '{struct_name}'"
                ));
            }
        }

        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut regs = Vec::new();
        let mut ordered_values: Vec<&Expr> = Vec::new();
        for (declared, _) in &sd.fields {
            let (_, val) = fields
                .iter()
                .find(|(name, _)| name == declared)
                .expect("field presence checked above");
            ordered_values.push(val);
        }
        for val in &ordered_values {
            let (e, c, r) = self.build_expr(val)?;
            if entry == 0 {
                entry = e;
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(c);
            regs.push(r);
        }
        let dst = self.ctx.alloc();
        let mut instrs = vec![CpsInstr::NewStruct(dst, sd.id, regs.clone())];
        for (i, (&reg, value)) in regs.iter().zip(ordered_values.iter()).enumerate() {
            let field_reg = if self.value_hint(value).is_float() {
                let boxed = self.ctx.alloc();
                instrs.push(CpsInstr::BinOp(boxed, CpsBinOp::FToI, reg, 0));
                boxed
            } else {
                reg
            };
            instrs.push(CpsInstr::SetField(field_reg, dst, i as u16, 0));
        }
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        if let Some(t) = prev_c {
            self.ctx.chain(t, id)?;
        }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_struct_lit_with_spread(
        &mut self,
        struct_name: &str,
        fields: &[(String, Expr)],
        spread_expr: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        let sd = self
            .structs
            .iter()
            .find(|s| s.name == struct_name)
            .cloned()
            .ok_or_else(|| format!("unknown struct '{struct_name}'"))?;

        // Check no unknown fields
        for (name, _) in fields {
            if !sd.fields.iter().any(|(declared, _)| declared == name) {
                return Err(format!(
                    "field '{name}' not found on struct '{struct_name}'"
                ));
            }
        }

        // Build spread source
        let (s_entry, s_continu, spread_reg) = self.build_expr(spread_expr)?;

        let mut entry = s_entry;
        let mut prev_c = Some(s_continu);
        let mut regs = Vec::new();
        let mut instrs = Vec::new();

        // For each declared field, use explicit value or get from spread
        for (i, (declared, _)) in sd.fields.iter().enumerate() {
            if let Some((_, val)) = fields.iter().find(|(name, _)| name == declared) {
                // Explicit field value
                let (e, c, r) = self.build_expr(val)?;
                if entry == 0 {
                    entry = e;
                }
                if let Some(t) = prev_c {
                    self.ctx.chain(t, e)?;
                }
                prev_c = Some(c);
                regs.push(r);
            } else {
                // Get field from spread source
                let field_reg = self.ctx.alloc();
                instrs.push(CpsInstr::GetField(field_reg, spread_reg, i as u16));
                regs.push(field_reg);
            }
        }

        let dst = self.ctx.alloc();
        instrs.push(CpsInstr::NewStruct(dst, sd.id, regs.clone()));
        // SetField for float boxing (simplified)
        for (i, &reg) in regs.iter().enumerate() {
            instrs.push(CpsInstr::SetField(reg, dst, i as u16, 0));
        }

        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        if let Some(t) = prev_c {
            self.ctx.chain(t, id)?;
        }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_variant_lit(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        fields: &[Expr],
    ) -> Result<(usize, usize, usize), String> {
        let ed = self
            .enums
            .iter()
            .find(|e| e.name == enum_name)
            .cloned()
            .ok_or_else(|| format!("unknown enum '{enum_name}'"))?;
        let (tag, _expected_fields): (u16, &Vec<(String, String)>) = ed
            .variants
            .iter()
            .find(|(name, _, _)| name == variant_name)
            .map(|(_, t, flds)| (*t, flds))
            .ok_or_else(|| format!("unknown variant '{variant_name}'"))?;

        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut field_regs = Vec::new();
        for val in fields {
            let (e, c, r) = self.build_expr(val)?;
            if entry == 0 {
                entry = e;
            }
            if let Some(t) = prev_c {
                self.ctx.chain(t, e)?;
            }
            prev_c = Some(c);
            field_regs.push(r);
        }
        let dst = self.ctx.alloc();
        let id = self.ctx.new_block();
        let mut instrs = vec![CpsInstr::NewVariant(dst, ed.id, tag, vec![])];
        // Set each field value
        for (i, &reg) in field_regs.iter().enumerate() {
            instrs.push(CpsInstr::SetVariantField(reg, dst, i as u16, 0));
        }
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        if let Some(t) = prev_c {
            self.ctx.chain(t, id)?;
        }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_variant_construct(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        args: &[Expr],
    ) -> Result<(usize, usize, usize), String> {
        // payload variant: Some(42) — parsed as Call, redirected here
        self.build_variant_lit(enum_name, variant_name, args)
    }

    fn build_index(
        &mut self,
        object: &Expr,
        index: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        let (e1, c1, obj) = self.build_expr(object)?;
        let (e2, c2, idx) = self.build_expr(index)?;
        let dst = self.ctx.alloc();
        // Use TupleIndex for tuples, IndexGet for lists/strings
        let hint = self.value_hint(object);
        let (instrs, _) = if matches!(hint, ValueHint::Tuple) {
            let lit = match index {
                Expr::LitInt(n) => *n as usize,
                _ => return Err("tuple index must be a literal integer".into()),
            };
            cps_emit::emit_tuple_index(dst, obj, lit as u16)
        } else {
            cps_emit::emit_index_get(dst, obj, idx)
        };
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs,
                term: cps_emit::emit_return(dst),
            },
        );
        self.ctx.chain(c1, e2)?;
        self.ctx.chain(c2, id)?;
        Ok((e1, id, dst))
    }

    fn build_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<(usize, usize, usize), String> {
        if let Expr::Index { object, index } = target {
            // xs[i] = val
            let (val_entry, val_continu, val_reg) = self.build_expr(value)?;
            let (idx_entry, idx_continu, idx_reg) = self.build_expr(index)?;
            // Look up the list variable
            let obj_reg = if let Expr::VarRef { name, .. } = object.as_ref() {
                *self
                    .ctx
                    .var_map
                    .get(name)
                    .ok_or_else(|| format!("undefined variable '{name}'"))?
            } else {
                return Err("index assignment target must be a variable".into());
            };
            // Chain: val → idx (if idx has its own entry)
            if idx_entry != 0 {
                self.ctx.chain(val_continu, idx_entry)?;
            }
            let id = self.ctx.new_block();
            self.ctx.set_block(
                id,
                CpsBlock {
                    id,
                    params: vec![],
                    instrs: vec![CpsInstr::IndexSet(val_reg, obj_reg, idx_reg, val_reg)],
                    term: cps_emit::emit_return(val_reg),
                },
            );
            // Chain: idx_continu (or val_continu) → id
            let chain_from = if idx_entry != 0 {
                idx_continu
            } else {
                val_continu
            };
            self.ctx.chain(chain_from, id)?;
            return Ok((val_entry, id, val_reg));
        }

        let (val_entry, val_continu, val_reg) = self.build_expr(value)?;
        let target_reg = if let Expr::VarRef { name, .. } = target {
            if let Some(&reg) = self.ctx.var_map.get(name) {
                reg
            } else {
                return Err(format!("undefined assignment target '{name}'"));
            }
        } else {
            return Err("only variable assignment is implemented".into());
        };
        let id = self.ctx.new_block();
        self.ctx.set_block(
            id,
            CpsBlock {
                id,
                params: vec![],
                instrs: vec![CpsInstr::Move(target_reg, val_reg)],
                term: cps_emit::emit_return(target_reg),
            },
        );
        self.ctx.chain(val_continu, id)?;
        Ok((val_entry, id, target_reg))
    }
}

// ── builtins dispatch ──

enum Builtin {
    /// Emit a CPS instruction directly — reserved for @intToString, @floatToString, etc.
    #[allow(dead_code)]
    Inline(CpsInstr),
    /// Build args + emit CallNative(native_index)
    Native(usize),
    /// Print: build arg → CpsInstr::Print
    Print,
}

/// Look up a function name in the builtins table.
/// Returns None if not a builtin — caller falls through to user-defined functions.
fn get_builtin(name: &str) -> Option<Builtin> {
    match name {
        "print" => Some(Builtin::Print),
        "assert" => Some(Builtin::Native(2)),
        "type_of" => Some(Builtin::Native(1)),
        "sqrt" => Some(Builtin::Native(3)),
        "sin" => Some(Builtin::Native(4)),
        "cos" => Some(Builtin::Native(5)),
        "floor" => Some(Builtin::Native(6)),
        "ceil" => Some(Builtin::Native(7)),
        _ => None,
    }
}

fn builtin_return_hint(name: &str) -> ValueHint {
    match name {
        "print" => ValueHint::Null,
        "assert" => ValueHint::Null,
        "type_of" => ValueHint::Int,
        "sqrt" | "sin" | "cos" | "floor" | "ceil" => ValueHint::Float,
        _ => ValueHint::Unknown,
    }
}

fn bin_op_to_cps(op: BinOp, is_float: bool) -> Result<CpsBinOp, String> {
    match (op, is_float) {
        (BinOp::Add, true) => Ok(CpsBinOp::FAdd),
        (BinOp::Sub, true) => Ok(CpsBinOp::FSub),
        (BinOp::Mul, true) => Ok(CpsBinOp::FMul),
        (BinOp::Div, true) => Ok(CpsBinOp::FDiv),
        (BinOp::Eq, true) => Ok(CpsBinOp::FEq),
        (BinOp::Ne, true) => Ok(CpsBinOp::FNe),
        (BinOp::Lt, true) => Ok(CpsBinOp::FLt),
        (BinOp::Le, true) => Ok(CpsBinOp::FLe),
        (BinOp::Gt, true) => Ok(CpsBinOp::FGt),
        (BinOp::Ge, true) => Ok(CpsBinOp::FGe),
        (BinOp::Add, false) => Ok(CpsBinOp::AddInt),
        (BinOp::Sub, false) => Ok(CpsBinOp::SubInt),
        (BinOp::Mul, false) => Ok(CpsBinOp::MulInt),
        (BinOp::Div, false) => Ok(CpsBinOp::DivInt),
        (BinOp::Mod, false) => Ok(CpsBinOp::ModInt),
        (BinOp::Eq, false) => Ok(CpsBinOp::EqInt),
        (BinOp::Ne, false) => Ok(CpsBinOp::NeInt),
        (BinOp::Lt, false) => Ok(CpsBinOp::LtInt),
        (BinOp::Le, false) => Ok(CpsBinOp::LeInt),
        (BinOp::Gt, false) => Ok(CpsBinOp::GtInt),
        (BinOp::Ge, false) => Ok(CpsBinOp::GeInt),
        (BinOp::Mod, true) => Err("modulo is not supported for Float64".into()),
        (BinOp::SAdd, _) => Ok(CpsBinOp::SAdd),
        (BinOp::And | BinOp::Or, _) => Err("logical binary operators are not implemented".into()),
        (BinOp::Pipe | BinOp::GtGt, _) => Err("pipe operators are not implemented".into()),
    }
}

fn remap_term_ids(block: &mut CpsBlock, map: &HashMap<usize, usize>) {
    match &mut block.term {
        CpsTerminator::Jump(b, _) => {
            if let Some(&n) = map.get(b) {
                *b = n;
            }
        }
        CpsTerminator::Branch(_, tb, _, fb, _) => {
            if let Some(&n) = map.get(tb) {
                *tb = n;
            }
            if let Some(&n) = map.get(fb) {
                *fb = n;
            }
        }
        CpsTerminator::Call(_, _, ret)
        | CpsTerminator::CallNative(_, _, ret)
        | CpsTerminator::CallIndirect(_, _, ret) => {
            if let Some(&n) = map.get(ret) {
                *ret = n;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;

    const S: Span = Span { line: 0, col: 0 };

    fn build_src(src: &str) -> CpsModule {
        build_module(&fixture_module(src), None).unwrap()
    }

    fn fixture_module(src: &str) -> Module {
        Module {
            stmts: match src {
                "const x = 42;" => vec![const_decl("x", Expr::LitInt(42))],
                "const x = 10; const y = 32;" => {
                    vec![
                        const_decl("x", Expr::LitInt(10)),
                        const_decl("y", Expr::LitInt(32)),
                    ]
                }
                "var x = 10; const y = x;" => vec![
                    var_decl("x", Expr::LitInt(10)),
                    const_decl("y", Expr::VarRef { name: "x".to_string(), span: S }),
                ],
                "var x = 10; var y = 32; const z = x + y;" => vec![
                    var_decl("x", Expr::LitInt(10)),
                    var_decl("y", Expr::LitInt(32)),
                    const_decl(
                        "z",
                        Expr::Binary {
                            left: Box::new(Expr::VarRef { name: "x".to_string(), span: S }),
                            op: BinOp::Add,
                            right: Box::new(Expr::VarRef { name: "y".to_string(), span: S }),
                        },
                    ),
                ],
                "const x = if true { 1 } else { 2 };" => vec![const_decl(
                    "x",
                    Expr::If {
                        cond: Box::new(Expr::LitTrue),
                        then_branch: Box::new(Expr::LitInt(1)),
                        else_branch: Some(Box::new(Expr::LitInt(2))),
                    },
                )],
                "var n = 0; while n < 3 { n = n + 1; };" => vec![
                    var_decl("n", Expr::LitInt(0)),
                    Stmt::ExprStmt(while_assign("n", BinOp::Lt, Expr::LitInt(3), BinOp::Add)),
                ],
                "const r = { var x = 1; x + 1; };" => vec![const_decl(
                    "r",
                    Expr::Block(vec![
                        var_decl("x", Expr::LitInt(1)),
                        Stmt::ExprStmt(binary_var_int("x", BinOp::Add, 1)),
                    ]),
                )],
                "const f = |x| { x + 1 };" => vec![const_decl(
                    "f",
                    lambda(
                        "x",
                        Expr::Block(vec![Stmt::ExprStmt(binary_var_int("x", BinOp::Add, 1))]),
                    ),
                )],
                "const f = |x| { x + 1 }; f(41);" => vec![
                    const_decl(
                        "f",
                        lambda(
                            "x",
                            Expr::Block(vec![Stmt::ExprStmt(binary_var_int("x", BinOp::Add, 1))]),
                        ),
                    ),
                    call_stmt("f", vec![Expr::LitInt(41)]),
                ],
                "const f = |n| { while n > 0 { n = n - 1; } }; f(5);" => vec![
                    const_decl(
                        "f",
                        lambda(
                            "n",
                            Expr::Block(vec![Stmt::ExprStmt(while_assign(
                                "n",
                                BinOp::Gt,
                                Expr::LitInt(0),
                                BinOp::Sub,
                            ))]),
                        ),
                    ),
                    call_stmt("f", vec![Expr::LitInt(5)]),
                ],
                "const xs = [1, 2, 3];" => vec![const_decl(
                    "xs",
                    Expr::ListLit(vec![Expr::LitInt(1), Expr::LitInt(2), Expr::LitInt(3)]),
                )],
                "const f = async |x| { x + 1 };" => vec![const_decl(
                    "f",
                    Expr::Async(Box::new(lambda(
                        "x",
                        Expr::Block(vec![Stmt::ExprStmt(binary_var_int("x", BinOp::Add, 1))]),
                    ))),
                )],
                "const s = 42.to_string();" => {
                    vec![const_decl("s", call_member(Expr::LitInt(42), "to_string"))]
                }
                "print(42.to_string());" => {
                    vec![call_stmt(
                        "print",
                        vec![call_member(Expr::LitInt(42), "to_string")],
                    )]
                }
                "const f = |x| { var r = 42; return r; }; f(0);" => vec![
                    const_decl(
                        "f",
                        lambda(
                            "x",
                            Expr::Block(vec![
                                var_decl("r", Expr::LitInt(42)),
                                Stmt::ExprStmt(Expr::Return(Some(Box::new(Expr::VarRef {
                                    name: "r".to_string(),
                                    span: S,
                                })))),
                            ]),
                        ),
                    ),
                    call_stmt("f", vec![Expr::LitInt(0)]),
                ],
                "const f = |x| { print(\"hi\"); return x; }; f(0);" => vec![
                    const_decl(
                        "f",
                        lambda(
                            "x",
                            Expr::Block(vec![
                                call_stmt("print", vec![Expr::LitString("hi".to_string())]),
                                Stmt::ExprStmt(Expr::Return(Some(Box::new(Expr::VarRef {
                                    name: "x".to_string(),
                                    span: S,
                                })))),
                            ]),
                        ),
                    ),
                    call_stmt("f", vec![Expr::LitInt(0)]),
                ],
                "const f = |x| { print(x.to_string()); return x; }; f(99);" => vec![
                    const_decl(
                        "f",
                        lambda(
                            "x",
                            Expr::Block(vec![
                                call_stmt(
                                    "print",
                                    vec![call_member(Expr::VarRef { name: "x".to_string(), span: S }, "to_string")],
                                ),
                                Stmt::ExprStmt(Expr::Return(Some(Box::new(Expr::VarRef {
                                    name: "x".to_string(),
                                    span: S,
                                })))),
                            ]),
                        ),
                    ),
                    call_stmt("f", vec![Expr::LitInt(99)]),
                ],
                "const f = |n| { var i = 0; while i < n { i = i + 1; }; return i; }; f(5);" => {
                    vec![
                        const_decl(
                            "f",
                            lambda(
                                "n",
                                Expr::Block(vec![
                                    var_decl("i", Expr::LitInt(0)),
                                    Stmt::ExprStmt(while_assign(
                                        "i",
                                        BinOp::Lt,
                                        Expr::VarRef { name: "n".to_string(), span: S },
                                        BinOp::Add,
                                    )),
                                    Stmt::ExprStmt(Expr::Return(Some(Box::new(Expr::VarRef {
                                        name: "i".to_string(),
                                        span: S,
                                    })))),
                                ]),
                            ),
                        ),
                        call_stmt("f", vec![Expr::LitInt(5)]),
                    ]
                }
                "print(\"hi\");" => {
                    vec![call_stmt("print", vec![Expr::LitString("hi".to_string())])]
                }
                "const x = sqrt(4.0);" => vec![const_decl(
                    "x",
                    Expr::Call {
                        func: Box::new(Expr::VarRef { name: "sqrt".to_string(), span: S }),
                        arg: Box::new(Expr::LitFloat(4.0)),
                    },
                )],
                _ => panic!("missing AST fixture for {src}"),
            },
        }
    }

    fn const_decl(name: &str, value: Expr) -> Stmt {
        Stmt::ConstDecl {
            name: name.to_string(),
            span: S,
            ty_ann: None,
            value,
        }
    }

    fn var_decl(name: &str, value: Expr) -> Stmt {
        Stmt::VarDecl {
            name: name.to_string(),
            span: S,
            ty_ann: None,
            value: Some(value),
        }
    }

    fn lambda(param: &str, body: Expr) -> Expr {
        Expr::Lambda {
            params: vec![Param {
                name: param.to_string(),
                span: S,
                ty_ann: None,
            }],
            ret_ty: None,
            body: Box::new(body),
        }
    }

    fn binary_var_int(name: &str, op: BinOp, value: i64) -> Expr {
        Expr::Binary {
            left: Box::new(Expr::VarRef { name: name.to_string(), span: S }),
            op,
            right: Box::new(Expr::LitInt(value)),
        }
    }

    fn while_assign(name: &str, cmp: BinOp, rhs: Expr, update: BinOp) -> Expr {
        Expr::While {
            cond: Box::new(Expr::Binary {
                left: Box::new(Expr::VarRef { name: name.to_string(), span: S }),
                op: cmp,
                right: Box::new(rhs),
            }),
            body: Box::new(Expr::Block(vec![Stmt::ExprStmt(Expr::Assign {
                target: Box::new(Expr::VarRef { name: name.to_string(), span: S }),
                value: Box::new(binary_var_int(name, update, 1)),
            })])),
        }
    }

    fn member(object: Expr, field: &str) -> Expr {
        Expr::Member {
            object: Box::new(object),
            field: field.to_string(),
        }
    }

    fn call_member(object: Expr, field: &str) -> Expr {
        Expr::Call {
            func: Box::new(member(object, field)),
            arg: Expr::call_arg(vec![]),
        }
    }

    fn call_stmt(name: &str, args: Vec<Expr>) -> Stmt {
        Stmt::ExprStmt(Expr::Call {
            func: Box::new(Expr::VarRef { name: name.to_string(), span: S }),
            arg: Expr::call_arg(args),
        })
    }

    #[test]
    fn build_single_const() {
        let c = build_src("const x = 42;");
        assert!(c.functions[0].blocks.len() >= 2);
    }
    #[test]
    fn build_two_consts() {
        let c = build_src("const x = 10; const y = 32;");
        assert!(c.functions[0].blocks.len() >= 3);
    }
    #[test]
    fn build_var() {
        let c = build_src("var x = 10; const y = x;");
        assert!(c.functions[0].blocks.len() >= 2);
    }
    #[test]
    fn build_multi_var() {
        let c = build_src("var x = 10; var y = 32; const z = x + y;");
        assert!(c.functions[0].blocks.len() >= 5);
    }
    #[test]
    fn build_if_else() {
        let c = build_src("const x = if true { 1 } else { 2 };");
        assert!(c.functions[0].blocks.len() >= 4);
    }
    #[test]
    fn build_while_struct() {
        let c = build_src("var n = 0; while n < 3 { n = n + 1; };");
        assert!(c.functions[0].blocks.len() >= 3);
    }
    #[test]
    fn build_block() {
        let c = build_src("const r = { var x = 1; x + 1; };");
        assert!(c.functions[0].blocks.len() >= 2);
    }

    #[test]
    fn build_lambda_creates_separate_function() {
        let c = build_src("const f = |x| { x + 1 };");
        assert!(
            c.functions.len() >= 2,
            "lambda should create separate function, got {}",
            c.functions.len()
        );
    }

    #[test]
    fn build_lambda_call_emits_call_terminator() {
        let c = build_src("const f = |x| { x + 1 }; f(41);");
        // The main function should have a block with Call terminator
        let main = c.functions.last().unwrap();
        let has_call = main
            .blocks
            .iter()
            .any(|b| matches!(b.term, CpsTerminator::Call(..)));
        assert!(has_call, "main function should contain a Call terminator");
    }

    #[test]
    fn build_lambda_with_while_body() {
        let c = build_src("const f = |n| { while n > 0 { n = n - 1; } }; f(5);");
        assert!(c.functions.len() >= 2);
    }

    #[test]
    fn build_list_not_empty() {
        let cps = build_module(&fixture_module("const xs = [1, 2, 3];"), None).unwrap();
        assert!(cps.functions.len() >= 1);
    }

    #[test]
    fn build_async_ok() {
        let c = build_src("const f = async |x| { x + 1 };");
        assert!(c.functions.len() >= 2);
    }

    #[test]
    fn build_to_string_emits_itos() {
        let c = build_src("const s = 42.to_string();");
        let main = c.functions.last().unwrap();
        let has_itos = main.blocks.iter().any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::IToS, _, _)))
        });
        assert!(has_itos, "42.to_string() should emit IToS instruction");
    }

    #[test]
    fn build_float_to_string_emits_ftos() {
        let module = Module {
            stmts: vec![const_decl(
                "s",
                call_member(Expr::LitFloat(3.14), "to_string"),
            )],
        };
        let c = build_module(&module, None).unwrap();
        let main = c.functions.last().unwrap();
        let has_ftos = main.blocks.iter().any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::FToS, _, _)))
        });
        assert!(has_ftos, "3.14.to_string() should emit FToS instruction");
    }

    #[test]
    fn build_float_add_uses_float_instruction() {
        let module = Module {
            stmts: vec![const_decl(
                "n",
                Expr::Binary {
                    left: Box::new(Expr::LitFloat(1.5)),
                    op: BinOp::Add,
                    right: Box::new(Expr::LitFloat(2.5)),
                },
            )],
        };
        let c = build_module(&module, None).unwrap();
        let main = c.functions.last().unwrap();
        let has_fadd = main.blocks.iter().any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::FAdd, _, _)))
        });
        assert!(has_fadd, "Float64 addition should emit FAdd instruction");
    }

    #[test]
    fn build_print_int_handled() {
        let c = build_src("print(42.to_string());");
        let main = c.functions.last().unwrap();
        let has_print = main
            .blocks
            .iter()
            .any(|b| b.instrs.iter().any(|i| matches!(i, CpsInstr::Print(_))));
        assert!(
            has_print,
            "print(42.to_string()) should emit Print instruction"
        );
    }

    #[test]
    fn build_lambda_var_return() {
        let c = build_src("const f = |x| { var r = 42; return r; }; f(0);");
        assert!(c.functions.len() >= 2, "should have main + lambda");
        let lambda = &c.functions[c.functions.len() - 2]; // lambda before main
        let has_loadconst = lambda.blocks.iter().any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::LoadConst(_, _)))
        });
        assert!(has_loadconst, "lambda should have LoadConst for var r = 42");
    }

    #[test]
    fn build_lambda_print_literal() {
        let c = build_src("const f = |x| { print(\"hi\"); return x; }; f(0);");
        assert!(c.functions.len() >= 2);
        let lambda = &c.functions[c.functions.len() - 2];
        let has_print = lambda
            .blocks
            .iter()
            .any(|b| b.instrs.iter().any(|i| matches!(i, CpsInstr::Print(_))));
        assert!(has_print, "lambda should contain Print instruction");
    }

    #[test]
    fn build_lambda_to_string() {
        let c = build_src("const f = |x| { print(x.to_string()); return x; }; f(99);");
        assert!(c.functions.len() >= 2);
        let lambda = &c.functions[c.functions.len() - 2];
        let has_itos = lambda.blocks.iter().any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::IToS, _, _)))
        });
        assert!(has_itos, "lambda should contain IToS for x.to_string()");
    }

    #[test]
    fn build_lambda_while_loop() {
        let c =
            build_src("const f = |n| { var i = 0; while i < n { i = i + 1; }; return i; }; f(5);");
        assert!(c.functions.len() >= 2);
        let lambda = &c.functions[c.functions.len() - 2];
        assert!(
            lambda.blocks.len() >= 4,
            "while should create header+body+exit+cond blocks, got {}",
            lambda.blocks.len()
        );
    }

    // ── builtin dispatch ──

    #[test]
    fn build_print_emits_print_opcode() {
        let c = build_src("print(\"hi\");");
        let has_print = c.functions[0]
            .blocks
            .iter()
            .any(|b| b.instrs.iter().any(|i| matches!(i, CpsInstr::Print(_))));
        assert!(has_print, "print should emit CpsInstr::Print");
    }

    #[test]
    fn build_sqrt_calls_native() {
        let cps = build_src("const x = sqrt(4.0);");
        let has_call_native = cps.functions[0].blocks.iter().any(|b| {
            matches!(b.term, CpsTerminator::CallNative(3, _, _)) // sqrt = native idx 3
        });
        assert!(has_call_native, "sqrt should emit CallNative(3)");
    }

    #[test]
    fn get_builtin_table_returns_all_eight() {
        let builtins = [
            ("print", true),
            ("assert", true),
            ("type_of", true),
            ("sqrt", true),
            ("sin", true),
            ("cos", true),
            ("floor", true),
            ("ceil", true),
        ];
        for (name, expected) in builtins {
            assert_eq!(
                get_builtin(name).is_some(),
                expected,
                "{name}: builtin lookup mismatch"
            );
        }
        assert!(get_builtin("unknown_fn").is_none());
        assert!(get_builtin("").is_none());
    }

    #[test]
    fn builtin_return_hints_match_expected() {
        assert_eq!(builtin_return_hint("print"), ValueHint::Null);
        assert_eq!(builtin_return_hint("assert"), ValueHint::Null);
        assert_eq!(builtin_return_hint("type_of"), ValueHint::Int);
        assert_eq!(builtin_return_hint("sqrt"), ValueHint::Float);
        assert_eq!(builtin_return_hint("sin"), ValueHint::Float);
        assert_eq!(builtin_return_hint("cos"), ValueHint::Float);
        assert_eq!(builtin_return_hint("floor"), ValueHint::Float);
        assert_eq!(builtin_return_hint("ceil"), ValueHint::Float);
        assert_eq!(builtin_return_hint("unknown"), ValueHint::Unknown);
    }

    #[test]
    fn build_native_call_multi_arg() {
        // Test that build_native_call handles 0, 1, 2 args without panicking
        let mut b = CpsBuilder::new(None);
        b.ctx = FuncCtx::new("test".into());
        let no_args: Vec<Expr> = vec![];
        let result = b.build_native_call(0, &no_args);
        assert!(
            result.is_ok(),
            "0 args should not panic, got {:?}",
            result.err()
        );

        let mut b = CpsBuilder::new(None);
        b.ctx = FuncCtx::new("test".into());
        let one_arg = vec![Expr::LitInt(42)];
        let result = b.build_native_call(0, &one_arg);
        assert!(result.is_ok(), "1 arg should work, got {:?}", result.err());

        let mut b = CpsBuilder::new(None);
        b.ctx = FuncCtx::new("test".into());
        let two_args = vec![Expr::LitInt(1), Expr::LitInt(2)];
        let result = b.build_native_call(0, &two_args);
        assert!(result.is_ok(), "2 args should work, got {:?}", result.err());
    }
}
