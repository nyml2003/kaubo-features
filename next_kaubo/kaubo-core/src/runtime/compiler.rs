//! AST → Bytecode 编译器

use crate::compiler::parser::expr::{AsExpr, FunctionCall, Lambda, VarRef};
use crate::compiler::parser::stmt::{ForStmt, IfStmt, ModuleStmt, WhileStmt};
use crate::compiler::parser::{Binary, Expr, ExprKind, Module, Stmt, StmtKind, TypeExpr};
use crate::runtime::{
    bytecode::{chunk::Chunk, OpCode},
    object::{ObjFunction, ObjString},
    Value,
};
use kaubo_log::{trace, Logger};
use std::collections::HashMap;
use std::sync::Arc;

/// 编译错误
#[derive(Debug, Clone)]
pub enum CompileError {
    InvalidOperator,
    TooManyConstants,
    TooManyLocals,
    VariableAlreadyExists(String),
    UninitializedVariable(String),
    Unimplemented(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::InvalidOperator => write!(f, "Invalid operator"),
            CompileError::TooManyConstants => write!(f, "Too many constants in one chunk"),
            CompileError::TooManyLocals => write!(f, "Too many local variables"),
            CompileError::VariableAlreadyExists(name) => {
                write!(f, "Variable '{}' already exists", name)
            }
            CompileError::UninitializedVariable(name) => {
                write!(f, "Variable '{}' is not initialized", name)
            }
            CompileError::Unimplemented(s) => write!(f, "Unimplemented: {}", s),
        }
    }
}

/// 局部变量信息
#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: usize,
    is_initialized: bool,
    is_captured: bool, // 是否被内层闭包捕获
}

/// Upvalue 描述（编译时）
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Upvalue {
    name: String,
    index: u8,      // upvalue 索引
    is_local: bool, // true=捕获外层局部变量, false=继承外层的 upvalue
}

/// 导出项信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Export {
    name: String,
    is_public: bool, // pub 修饰
    local_idx: u8,   // 对应的局部变量索引
    shape_id: u16,   // ShapeID（编译期确定的静态索引）
}

/// 模块信息（编译时）
#[derive(Debug, Clone)]
struct ModuleInfo {
    name: String,
    exports: Vec<Export>,
    export_name_to_shape_id: HashMap<String, u16>, // 名称到 ShapeID 的映射
}

/// 变量类型（解析结果）
#[derive(Debug, Clone, Copy)]
enum Variable {
    Local(u8),   // 局部变量索引
    Upvalue(u8), // upvalue 索引
}

/// Struct 编译期信息
#[derive(Debug, Clone)]
struct StructInfo {
    shape_id: u16,
    field_names: Vec<String>,  // 字段名列表（索引即字段位置）
    method_names: Vec<String>, // 方法名列表（来自 impl 块）
}

/// 变量类型信息
#[derive(Debug, Clone)]
enum VarType {
    Struct(String), // struct 类型名
}

/// AST 编译器
pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,                        // 局部变量表
    upvalues: Vec<Upvalue>,                    // upvalue 描述表
    scope_depth: usize,                        // 当前作用域深度
    max_locals: usize,                         // 最大局部变量数量（用于栈预分配）
    enclosing: *mut Compiler,                  // 指向父编译器（用于解析 upvalue）
    current_module: Option<ModuleInfo>,        // 当前正在编译的模块
    modules: Vec<ModuleInfo>,                  // 文件内所有模块
    imported_modules: Vec<String>,             // 当前导入的模块名列表
    module_aliases: HashMap<String, String>,   // 局部变量名 -> 模块名（用于 import x as y）
    struct_infos: HashMap<String, StructInfo>, // struct 名称 -> 编译期信息
    var_types: HashMap<String, VarType>,       // 变量名 -> 类型信息
    logger: Arc<Logger>,                       // Logger
}

impl Compiler {
    pub fn new() -> Self {
        Self::new_with_shapes(HashMap::new())
    }

    pub fn new_with_shapes(shape_ids: HashMap<String, u16>) -> Self {
        Self::new_with_shapes_and_logger(shape_ids, Logger::noop())
    }

    pub fn new_with_logger(logger: Arc<Logger>) -> Self {
        Self::new_with_shapes_and_logger(HashMap::new(), logger)
    }

    pub fn new_with_shapes_and_logger(
        shape_ids: HashMap<String, u16>,
        logger: Arc<Logger>,
    ) -> Self {
        // 转换 shape_ids 为 struct_infos（字段信息暂时为空，需要额外传递）
        let struct_infos: HashMap<String, StructInfo> = shape_ids
            .into_iter()
            .map(|(name, shape_id)| {
                (
                    name,
                    StructInfo {
                        shape_id,
                        field_names: Vec::new(),
                        method_names: Vec::new(),
                    },
                )
            })
            .collect();

        Self {
            chunk: Chunk::with_logger(logger.clone()),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            max_locals: 0,
            enclosing: std::ptr::null_mut(),
            current_module: None,
            modules: Vec::new(),
            imported_modules: Vec::new(),
            module_aliases: HashMap::new(),
            struct_infos,
            var_types: HashMap::new(),
            logger,
        }
    }

    /// 使用完整的 struct 信息创建编译器（支持字段索引优化）
    pub fn new_with_struct_infos(infos: HashMap<String, (u16, Vec<String>)>) -> Self {
        Self::new_with_struct_infos_and_logger(infos, Logger::noop())
    }

    pub fn new_with_struct_infos_and_logger(
        infos: HashMap<String, (u16, Vec<String>)>,
        logger: Arc<Logger>,
    ) -> Self {
        let struct_infos: HashMap<String, StructInfo> = infos
            .into_iter()
            .map(|(name, (shape_id, field_names))| {
                (
                    name,
                    StructInfo {
                        shape_id,
                        field_names,
                        method_names: Vec::new(),
                    },
                )
            })
            .collect();

        Self {
            chunk: Chunk::with_logger(logger.clone()),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            max_locals: 0,
            enclosing: std::ptr::null_mut(),
            current_module: None,
            modules: Vec::new(),
            imported_modules: Vec::new(),
            module_aliases: HashMap::new(),
            struct_infos,
            var_types: HashMap::new(),
            logger,
        }
    }

    /// 创建子编译器（用于编译嵌套函数）
    fn new_child(enclosing: *mut Compiler) -> Self {
        // 从父编译器继承 struct_infos 和 logger
        let (struct_infos, logger) = unsafe {
            if enclosing.is_null() {
                (HashMap::new(), Logger::noop())
            } else {
                (
                    (*enclosing).struct_infos.clone(),
                    (*enclosing).logger.clone(),
                )
            }
        };

        Self {
            chunk: Chunk::with_logger(logger.clone()),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            max_locals: 0,
            enclosing,
            current_module: None,
            modules: Vec::new(),
            imported_modules: Vec::new(),
            module_aliases: HashMap::new(),
            struct_infos,              // 继承父编译器的 struct_infos
            var_types: HashMap::new(), // 子编译器创建新的类型环境
            logger,
        }
    }

    /// 编译整个模块（视为匿名函数体）
    /// 返回 (Chunk, 最大局部变量数量)
    pub fn compile(&mut self, module: &Module) -> Result<(Chunk, usize), CompileError> {
        trace!(
            self.logger,
            "compile: starting with {} statements",
            module.statements.len()
        );
        for stmt in &module.statements {
            self.compile_stmt(stmt)?;
        }

        // 默认返回 null
        self.chunk.write_op(OpCode::LoadNull, 0);
        self.chunk.write_op(OpCode::Return, 0);

        Ok((self.chunk.clone(), self.max_locals))
    }

    /// 编译语句
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        trace!(self.logger, "compile_stmt: {:?}", stmt.as_ref());
        match stmt.as_ref() {
            StmtKind::Expr(expr) => {
                self.compile_expr(&expr.expression)?;
                // 表达式语句的结果丢弃
                self.chunk.write_op(OpCode::Pop, 0);
            }

            StmtKind::VarDecl(decl) => {
                // 先声明变量（占位）
                let idx = self.add_local(&decl.name)?;

                // 编译初始化表达式
                self.compile_expr(&decl.initializer)?;

                // 标记为已初始化并存储
                self.mark_initialized();
                self.emit_store_local(idx);

                // 记录变量类型（如果是 struct 实例）
                if let ExprKind::StructLiteral(struct_lit) = decl.initializer.as_ref() {
                    self.var_types
                        .insert(decl.name.clone(), VarType::Struct(struct_lit.name.clone()));
                }

                // 如果是 pub 且在当前模块中，记录导出
                if decl.is_public {
                    if let Some(ref mut module) = self.current_module {
                        let shape_id = module.exports.len() as u16;
                        module.exports.push(Export {
                            name: decl.name.clone(),
                            is_public: true,
                            local_idx: idx,
                            shape_id,
                        });
                        module
                            .export_name_to_shape_id
                            .insert(decl.name.clone(), shape_id);
                    }
                }
            }

            StmtKind::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.compile_expr(value)?;
                    self.chunk.write_op(OpCode::ReturnValue, 0);
                } else {
                    self.chunk.write_op(OpCode::LoadNull, 0);
                    self.chunk.write_op(OpCode::Return, 0);
                }
            }

            StmtKind::Print(print) => {
                self.compile_expr(&print.expression)?;
                self.chunk.write_op(OpCode::Print, 0);
            }

            StmtKind::Empty(_) => {}

            StmtKind::Block(block) => {
                self.begin_scope();
                for stmt in &block.statements {
                    self.compile_stmt(stmt)?;
                }
                self.end_scope();
            }

            StmtKind::If(if_stmt) => {
                self.compile_if(if_stmt)?;
            }

            StmtKind::While(while_stmt) => {
                self.compile_while(while_stmt)?;
            }

            StmtKind::For(for_stmt) => {
                self.compile_for(for_stmt)?;
            }

            StmtKind::Module(module_stmt) => {
                self.compile_module(module_stmt)?;
            }

            StmtKind::Import(import_stmt) => {
                self.compile_import(import_stmt)?;
            }

            StmtKind::Struct(_) => {
                // Struct 定义是编译期类型信息，不生成运行时代码
                // shape 信息已在 type checker 中生成
            }

            StmtKind::Impl(impl_stmt) => {
                self.compile_impl_block(impl_stmt)?;
            }
        }
        Ok(())
    }

    /// 编译 impl 块
    /// 将每个方法编译为函数，并记录到 Chunk.method_table 供 VM 初始化时使用
    fn compile_impl_block(
        &mut self,
        impl_stmt: &crate::compiler::parser::stmt::ImplStmt,
    ) -> Result<(), CompileError> {
        use crate::compiler::parser::expr::ExprKind;
        use crate::runtime::bytecode::chunk::{MethodTableEntry, OperatorTableEntry};

        // 获取 shape_id（必须在 struct_infos 中存在）
        let shape_id = self
            .struct_infos
            .get(&impl_stmt.struct_name)
            .map(|info| info.shape_id)
            .unwrap_or(0);

        // 确保 struct_info 存在
        let struct_info = self
            .struct_infos
            .entry(impl_stmt.struct_name.clone())
            .or_insert_with(|| StructInfo {
                shape_id: 0,
                field_names: Vec::new(),
                method_names: Vec::new(),
            });

        // 先记录所有方法名，确定 method_idx
        let start_idx = struct_info.method_names.len();
        for method in &impl_stmt.methods {
            struct_info.method_names.push(method.name.clone());
        }

        // 编译每个方法
        for (i, method) in impl_stmt.methods.iter().enumerate() {
            let method_idx = (start_idx + i) as u8;
            let function_name = format!("{}_{}", impl_stmt.struct_name, method.name);

            if let ExprKind::Lambda(lambda) = method.lambda.as_ref() {
                // 创建独立编译器编译方法体（方法不捕获 upvalues）
                let mut method_compiler = Compiler::new();
                // 复制 struct_infos 用于类型推断
                method_compiler.struct_infos = self.struct_infos.clone();
                // 继承模块信息（用于访问 std 等模块）
                method_compiler.module_aliases = self.module_aliases.clone();
                method_compiler.imported_modules = self.imported_modules.clone();

                // 添加参数作为局部变量（self 已经在 lambda.params 中）
                for (param_name, param_type) in &lambda.params {
                    let _idx = method_compiler.add_local(param_name)?;
                    method_compiler.mark_initialized();
                    // 记录参数类型（用于字段访问优化）
                    if param_name == "self" {
                        // self 总是当前 struct 类型
                        method_compiler.var_types.insert(
                            param_name.clone(),
                            VarType::Struct(impl_stmt.struct_name.clone()),
                        );
                    } else if let Some(type_expr) = param_type {
                        if let crate::compiler::parser::type_expr::TypeExpr::Named(named_type) = type_expr {
                            method_compiler.var_types.insert(
                                param_name.clone(),
                                VarType::Struct(named_type.name.clone()),
                            );
                        }
                    }
                }

                // 编译方法体
                method_compiler.compile_stmt(&lambda.body)?;
                method_compiler.chunk.write_op(OpCode::LoadNull, 0);
                method_compiler.chunk.write_op(OpCode::Return, 0);

                // 创建函数对象
                let arity = lambda.params.len() as u8;
                let function = Box::new(crate::runtime::object::ObjFunction::new(
                    method_compiler.chunk.clone(),
                    arity,
                    Some(function_name),
                ));
                let function_ptr = Box::into_raw(function);
                let function_value = Value::function(function_ptr);

                // 添加到常量池，获取索引
                let const_idx = self.chunk.add_constant(function_value);

                // 添加到方法表（VM 初始化时会注册到 Shape）
                self.chunk.method_table.push(MethodTableEntry {
                    shape_id,
                    method_idx,
                    const_idx,
                });
                
                // 如果是运算符方法，添加到运算符表
                if method.name.starts_with("operator ") {
                    let op_name = &method.name[9..]; // 去掉 "operator " 前缀
                    self.chunk.operator_table.push(OperatorTableEntry {
                        shape_id,
                        operator_name: op_name.to_string(),
                        const_idx,
                    });
                }
            }
        }

        Ok(())
    }

    /// 编译表达式
    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr.as_ref() {
            ExprKind::LiteralInt(lit) => {
                let value = Value::smi(lit.value as i32);
                let idx = self.chunk.add_constant(value);
                self.emit_constant(idx);
            }

            ExprKind::LiteralFloat(lit) => {
                let value = Value::float(lit.value);
                let idx = self.chunk.add_constant(value);
                self.emit_constant(idx);
            }

            ExprKind::LiteralString(lit) => {
                // 创建字符串对象
                let string_obj = Box::new(ObjString::new(lit.value.clone()));
                let string_ptr = Box::into_raw(string_obj) as *mut ObjString;
                let value = Value::string(string_ptr);
                let idx = self.chunk.add_constant(value);
                self.emit_constant(idx);
            }

            ExprKind::LiteralTrue(_) => {
                self.chunk.write_op(OpCode::LoadTrue, 0);
            }

            ExprKind::LiteralFalse(_) => {
                self.chunk.write_op(OpCode::LoadFalse, 0);
            }

            ExprKind::LiteralNull(_) => {
                self.chunk.write_op(OpCode::LoadNull, 0);
            }

            ExprKind::LiteralList(list) => {
                // 编译所有元素（从左到右，依次压栈）
                let count = list.elements.len();
                if count > 255 {
                    return Err(CompileError::Unimplemented(
                        "List too long (max 255 elements)".to_string(),
                    ));
                }

                for elem in &list.elements {
                    self.compile_expr(elem)?;
                }

                // 生成 BuildList 指令
                self.chunk.write_op_u8(OpCode::BuildList, count as u8, 0);
            }

            ExprKind::Binary(bin) => {
                self.compile_binary(bin)?;
            }

            ExprKind::Unary(un) => {
                self.compile_expr(&un.operand)?;
                let op = match un.op {
                    crate::compiler::lexer::token_kind::KauboTokenKind::Minus => OpCode::Neg,
                    crate::compiler::lexer::token_kind::KauboTokenKind::Not => OpCode::Not,
                    _ => return Err(CompileError::InvalidOperator),
                };
                self.chunk.write_op(op, 0);
            }

            ExprKind::Grouping(g) => {
                self.compile_expr(&g.expression)?;
            }

            ExprKind::VarRef(var_ref) => {
                match self.resolve_variable(&var_ref.name) {
                    Some(Variable::Local(idx)) => self.emit_load_local(idx),
                    Some(Variable::Upvalue(idx)) => self.emit_load_upvalue(idx),
                    None => {
                        // 检查是否是模块名
                        if self.is_module_name(&var_ref.name) {
                            // 模块作为全局变量访问
                            let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                            let name_ptr = Box::into_raw(name_obj) as *mut ObjString;
                            let name_val = Value::string(name_ptr);
                            let name_idx = self.chunk.add_constant(name_val);
                            self.emit_constant(name_idx);
                            self.chunk.write_op(OpCode::LoadGlobal, 0);
                        } else {
                            // 作为全局变量访问（如 std 函数）
                            let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                            let name_ptr = Box::into_raw(name_obj) as *mut ObjString;
                            let name_val = Value::string(name_ptr);
                            let name_idx = self.chunk.add_constant(name_val);
                            self.chunk.write_op_u8(OpCode::LoadGlobal, name_idx, 0);
                        }
                    }
                }
            }

            ExprKind::FunctionCall(call) => {
                self.compile_function_call(call)?;
            }

            ExprKind::Lambda(lambda) => {
                self.compile_lambda(lambda)?;
            }

            ExprKind::MemberAccess(member) => {
                // 成员访问语法糖: obj.name 等价于 obj["name"]
                // 对于模块访问，使用 ModuleGet 指令（编译期确定的 ShapeID）

                if let ExprKind::VarRef(var_ref) = member.object.as_ref() {
                    if let Some(shape_id) = self.find_module_shape_id(&var_ref.name, &member.member)
                    {
                        // 检查是否是模块别名（局部变量）
                        if self.module_aliases.contains_key(&var_ref.name) {
                            // 模块别名是局部变量：LoadLocal + ModuleGet(shape_id)
                            // 查找局部变量索引
                            if let Some(local_idx) = self.resolve_local(&var_ref.name) {
                                self.emit_load_local(local_idx);
                            } else {
                                return Err(CompileError::Unimplemented(format!(
                                    "Module alias '{}' not found as local variable",
                                    var_ref.name
                                )));
                            }
                        } else {
                            // 模块是全局变量：LoadGlobal + ModuleGet(shape_id)
                            let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                            let name_ptr = Box::into_raw(name_obj) as *mut ObjString;
                            let name_val = Value::string(name_ptr);
                            let name_idx = self.chunk.add_constant(name_val);
                            self.chunk.write_op_u8(OpCode::LoadGlobal, name_idx, 0);
                        }

                        // ModuleGet 指令（ShapeID 作为 u16 操作数）
                        self.chunk.write_op(OpCode::ModuleGet, 0);
                        self.chunk.write_u16(shape_id, 0);
                        return Ok(());
                    }
                }

                // 尝试获取对象类型，检查是否是 struct
                let obj_type = self.get_expr_type(&member.object);
                let is_struct_field = if let Some(VarType::Struct(struct_name)) = obj_type {
                    // 查找 struct 的字段索引
                    if let Some(struct_info) = self.struct_infos.get(&struct_name) {
                        struct_info.field_names.iter().position(|f| f == &member.member)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                if let Some(field_idx) = is_struct_field {
                    // Struct 字段访问：使用字段索引
                    self.compile_expr(&member.object)?;
                    self.chunk.write_op_u8(OpCode::GetField, field_idx as u8, 0);
                } else {
                    // 普通对象访问（JSON）：编译对象 + 字符串键 + IndexGet
                    self.compile_expr(&member.object)?;
                    let key_obj = Box::new(ObjString::new(member.member.clone()));
                    let key_ptr = Box::into_raw(key_obj) as *mut ObjString;
                    let key_val = Value::string(key_ptr);
                    let idx = self.chunk.add_constant(key_val);
                    self.emit_constant(idx);
                    self.chunk.write_op(OpCode::IndexGet, 0);
                }
            }

            ExprKind::IndexAccess(index) => {
                // 普通索引访问: list[index]
                // 注意：字符串字面量索引已在类型检查阶段报错
                self.compile_expr(&index.object)?; // list
                self.compile_expr(&index.index)?; // index
                self.chunk.write_op(OpCode::IndexGet, 0);
            }

            ExprKind::JsonLiteral(json) => {
                // 编译 JSON 字面量: json { "key": value, ... }
                // 栈布局: [value1, key1, value2, key2, ...]（从后往前压栈）
                let count = json.entries.len();
                if count > 255 {
                    return Err(CompileError::Unimplemented(
                        "JSON literal too large (max 255 entries)".to_string(),
                    ));
                }

                // 逆序处理，这样 BuildJson 可以按正确顺序弹出
                for (key, value) in json.entries.iter().rev() {
                    // 值先入栈
                    self.compile_expr(value)?;
                    // 键（字符串）入栈
                    let key_obj = Box::new(ObjString::new(key.clone()));
                    let key_ptr = Box::into_raw(key_obj) as *mut ObjString;
                    let key_val = Value::string(key_ptr);
                    let idx = self.chunk.add_constant(key_val);
                    self.emit_constant(idx);
                }

                self.chunk.write_op_u8(OpCode::BuildJson, count as u8, 0);
            }

            ExprKind::Yield(y) => {
                // 编译 yield 表达式
                if let Some(value) = &y.value {
                    self.compile_expr(value)?;
                } else {
                    // yield; 等价于 yield null;
                    self.chunk.write_op(OpCode::LoadNull, 0);
                }
                self.chunk.write_op(OpCode::Yield, 0);
            }

            ExprKind::StructLiteral(struct_lit) => {
                // 编译 struct 字面量
                let struct_info = self.struct_infos.get(&struct_lit.name).ok_or_else(|| {
                    CompileError::Unimplemented(format!(
                        "Struct '{}' not found in shape table",
                        struct_lit.name
                    ))
                })?;

                let shape_id = struct_info.shape_id;
                let field_count = struct_lit.fields.len();

                // 将 Vec 转换为 HashMap 便于查找
                let field_map: std::collections::HashMap<String, Expr> = struct_lit
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // 按 struct 定义的顺序编译字段
                // 如果 struct_info 有字段信息，按那个顺序；否则按字母排序
                let field_order: Vec<String> = if struct_info.field_names.is_empty() {
                    //  fallback：按字母排序
                    let mut names: Vec<_> = field_map.keys().cloned().collect();
                    names.sort();
                    names
                } else {
                    struct_info.field_names.clone()
                };

                // 按字段定义顺序逆序入栈
                for field_name in field_order.iter().rev() {
                    if let Some(value_expr) = field_map.get(field_name) {
                        self.compile_expr(value_expr)?;
                    } else {
                        return Err(CompileError::Unimplemented(format!(
                            "Missing field '{}' in struct '{}'",
                            field_name, struct_lit.name
                        )));
                    }
                }

                // 生成 BuildStruct 指令
                self.chunk
                    .write_op_u16_u8(OpCode::BuildStruct, shape_id, field_count as u8, 0);
            }

            ExprKind::As(as_expr) => {
                // 编译源表达式
                self.compile_expr(&as_expr.expr)?;
                // 生成类型转换指令
                self.compile_cast(&as_expr.target_type)?;
            }
        }
        Ok(())
    }

    /// 编译类型转换指令
    fn compile_cast(&mut self, target_type: &TypeExpr) -> Result<(), CompileError> {
        // 根据目标类型生成相应的转换指令
        match target_type {
            TypeExpr::Named(named) => {
                match named.name.as_str() {
                    "int" => self.chunk.write_op(OpCode::CastToInt, 0),
                    "float" => self.chunk.write_op(OpCode::CastToFloat, 0),
                    "string" => self.chunk.write_op(OpCode::CastToString, 0),
                    "bool" => self.chunk.write_op(OpCode::CastToBool, 0),
                    _ => {
                        return Err(CompileError::Unimplemented(format!(
                            "Cast to type '{}' not supported",
                            named.name
                        )))
                    }
                }
            }
            _ => {
                return Err(CompileError::Unimplemented(
                    "Cast to complex types not supported".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// 编译二元运算
    fn compile_binary(&mut self, bin: &Binary) -> Result<(), CompileError> {
        use crate::compiler::lexer::token_kind::KauboTokenKind;

        // 特殊处理赋值运算符：=
        if bin.op == KauboTokenKind::Equal {
            return self.compile_assignment(&bin.left, &bin.right);
        }

        // 短路求值：and
        if bin.op == KauboTokenKind::And {
            return self.compile_and(&bin.left, &bin.right);
        }

        // 短路求值：or
        if bin.op == KauboTokenKind::Or {
            return self.compile_or(&bin.left, &bin.right);
        }

        // 先编译左操作数
        self.compile_expr(&bin.left)?;

        // 再编译右操作数
        self.compile_expr(&bin.right)?;

        // 生成运算指令
        let op = match bin.op {
            KauboTokenKind::Plus => OpCode::Add,
            KauboTokenKind::Minus => OpCode::Sub,
            KauboTokenKind::Asterisk => OpCode::Mul,
            KauboTokenKind::Slash => OpCode::Div,
            KauboTokenKind::Percent => OpCode::Mod,
            KauboTokenKind::DoubleEqual => OpCode::Equal,
            KauboTokenKind::ExclamationEqual => OpCode::NotEqual,
            KauboTokenKind::GreaterThan => OpCode::Greater,
            KauboTokenKind::GreaterThanEqual => OpCode::GreaterEqual,
            KauboTokenKind::LessThan => OpCode::Less,
            KauboTokenKind::LessThanEqual => OpCode::LessEqual,
            _ => return Err(CompileError::InvalidOperator),
        };

        self.chunk.write_op(op, 0);
        Ok(())
    }

    /// 编译逻辑与 (and) - 短路求值
    /// a and b 等价于: if a then b else a
    fn compile_and(&mut self, left: &Expr, right: &Expr) -> Result<(), CompileError> {
        // 编译左操作数
        self.compile_expr(left)?;

        // 复制左操作数用于条件判断（因为 JumpIfFalse 会弹出值）
        self.chunk.write_op(OpCode::Dup, 0);

        // 如果左操作数为假，跳转到结束（保留左操作数作为结果）
        let jump_offset = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 左操作数为真，需要计算右操作数
        // 弹出复制的左操作数值
        self.chunk.write_op(OpCode::Pop, 0);

        // 编译右操作数
        self.compile_expr(right)?;

        // 修补跳转偏移量
        self.chunk.patch_jump(jump_offset);

        Ok(())
    }

    /// 编译逻辑或 (or) - 短路求值
    /// a or b 等价于: if a then a else b
    fn compile_or(&mut self, left: &Expr, right: &Expr) -> Result<(), CompileError> {
        // 编译左操作数
        self.compile_expr(left)?;

        // 复制左操作数用于条件判断
        self.chunk.write_op(OpCode::Dup, 0);

        // 如果为假，跳转到右操作数计算
        let jump_if_false = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 左操作数为真：直接短路，保留左操作数
        // 不需要 Pop，因为 JumpIfFalse 已经弹出了复制的值
        // 原始左操作数仍在栈上作为结果
        let end_jump = self.chunk.write_jump(OpCode::Jump, 0);

        // 修补跳转到右操作数计算的位置
        self.chunk.patch_jump(jump_if_false);

        // 左操作数为假：弹出 falsy 的左操作数，计算右操作数
        self.chunk.write_op(OpCode::Pop, 0);
        self.compile_expr(right)?;

        // 修补结束跳转
        self.chunk.patch_jump(end_jump);

        Ok(())
    }

    /// 编译赋值表达式 (处理 Binary 形式的赋值)
    /// 赋值表达式返回 null（语句级别的副作用）
    fn compile_assignment(&mut self, left: &Expr, right: &Expr) -> Result<(), CompileError> {
        // 左值可能是变量引用或索引访问
        match left.as_ref() {
            ExprKind::VarRef(var_ref) => {
                // 编译右值
                self.compile_expr(right)?;

                match self.resolve_variable(&var_ref.name) {
                    Some(Variable::Local(idx)) => self.emit_store_local(idx),
                    Some(Variable::Upvalue(idx)) => self.emit_store_upvalue(idx),
                    None => {
                        return Err(CompileError::Unimplemented(format!(
                            "Undefined variable: {}",
                            var_ref.name
                        )));
                    }
                }
                // 赋值表达式返回 null（不是被赋的值）
                self.chunk.write_op(OpCode::LoadNull, 0);
                Ok(())
            }

            ExprKind::IndexAccess(index) => {
                // 索引赋值: list[index] = value
                // 栈布局: [value, index, list]
                self.compile_expr(right)?; // value
                self.compile_expr(&index.index)?; // index
                self.compile_expr(&index.object)?; // list
                self.chunk.write_op(OpCode::IndexSet, 0);
                // 返回 null
                self.chunk.write_op(OpCode::LoadNull, 0);
                Ok(())
            }

            ExprKind::MemberAccess(member) => {
                // 成员赋值: obj.name = value
                // 等价于 obj["name"] = value
                // 栈布局: [value, "name", obj]
                self.compile_expr(right)?; // value
                                           // 成员名作为字符串
                let key_obj = Box::new(ObjString::new(member.member.clone()));
                let key_ptr = Box::into_raw(key_obj) as *mut ObjString;
                let key_val = Value::string(key_ptr);
                let idx = self.chunk.add_constant(key_val);
                self.emit_constant(idx); // key
                self.compile_expr(&member.object)?; // obj
                self.chunk.write_op(OpCode::IndexSet, 0);
                // 返回 null
                self.chunk.write_op(OpCode::LoadNull, 0);
                Ok(())
            }

            _ => Err(CompileError::Unimplemented(
                "Left side of assignment must be a variable, index access, or member access"
                    .to_string(),
            )),
        }
    }

    /// 发射常量加载指令 (优化：使用 LoadConst0-15)
    fn emit_constant(&mut self, idx: u8) {
        let op = match idx {
            0 => OpCode::LoadConst0,
            1 => OpCode::LoadConst1,
            2 => OpCode::LoadConst2,
            3 => OpCode::LoadConst3,
            4 => OpCode::LoadConst4,
            5 => OpCode::LoadConst5,
            6 => OpCode::LoadConst6,
            7 => OpCode::LoadConst7,
            8 => OpCode::LoadConst8,
            9 => OpCode::LoadConst9,
            10 => OpCode::LoadConst10,
            11 => OpCode::LoadConst11,
            12 => OpCode::LoadConst12,
            13 => OpCode::LoadConst13,
            14 => OpCode::LoadConst14,
            15 => OpCode::LoadConst15,
            _ => {
                self.chunk.write_op_u8(OpCode::LoadConst, idx, 0);
                return;
            }
        };
        self.chunk.write_op(op, 0);
    }

    // ==================== 局部变量管理 ====================

    /// 进入新作用域
    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// 退出作用域，返回弹出的变量数量
    fn end_scope(&mut self) -> usize {
        self.scope_depth -= 1;

        let mut popped = 0;
        while let Some(local) = self.locals.last() {
            if local.depth <= self.scope_depth {
                break;
            }
            self.locals.pop();
            popped += 1;
        }
        popped
    }

    /// 添加局部变量，返回其在栈中的索引
    fn add_local(&mut self, name: &str) -> Result<u8, CompileError> {
        // 检查局部变量数量上限
        if self.locals.len() >= 256 {
            return Err(CompileError::TooManyLocals);
        }

        // 检查同作用域内是否已有同名变量
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if local.name == name {
                return Err(CompileError::VariableAlreadyExists(name.to_string()));
            }
        }

        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scope_depth,
            is_initialized: false,
            is_captured: false,
        });

        // 更新最大局部变量数
        self.max_locals = self.max_locals.max(self.locals.len());

        Ok((self.locals.len() - 1) as u8)
    }

    /// 标记最后一个变量为已初始化
    fn mark_initialized(&mut self) {
        if let Some(local) = self.locals.last_mut() {
            local.is_initialized = true;
        }
    }

    /// 解析变量名，返回其在局部变量表中的索引
    fn resolve_local(&self, name: &str) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                if !local.is_initialized {
                    // 不能在初始化完成前使用
                    return None;
                }
                return Some(i as u8);
            }
        }
        None
    }

    /// 标记局部变量被捕获
    fn mark_captured(&mut self, index: usize) {
        if let Some(local) = self.locals.get_mut(index) {
            local.is_captured = true;
        }
    }

    /// 添加 upvalue 描述，返回其索引
    fn add_upvalue(&mut self, name: &str, index: u8, is_local: bool) -> u8 {
        // 检查是否已存在相同的 upvalue
        for (i, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i as u8;
            }
        }

        let upvalue = Upvalue {
            name: name.to_string(),
            index,
            is_local,
        };
        self.upvalues.push(upvalue);
        (self.upvalues.len() - 1) as u8
    }

    /// 递归解析 upvalue
    /// 返回 Some(idx) 如果是 upvalue，None 如果未找到
    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        // 没有外层编译器，无法解析
        if self.enclosing.is_null() {
            return None;
        }

        unsafe {
            // 1. 在外层编译器查找局部变量
            if let Some(local_idx) = (*self.enclosing).resolve_local(name) {
                // 找到了！标记为被捕获
                (*self.enclosing).mark_captured(local_idx as usize);
                // 添加 upvalue 描述（指向外层局部变量）
                return Some(self.add_upvalue(name, local_idx, true));
            }

            // 2. 递归在外层查找 upvalue
            if let Some(upvalue_idx) = (*self.enclosing).resolve_upvalue(name) {
                // 继承外层的 upvalue
                return Some(self.add_upvalue(name, upvalue_idx, false));
            }
        }

        None
    }

    /// 统一变量解析：Local 或 Upvalue
    fn resolve_variable(&mut self, name: &str) -> Option<Variable> {
        // 1. 先查找局部变量
        if let Some(idx) = self.resolve_local(name) {
            return Some(Variable::Local(idx));
        }

        // 2. 查找 upvalue（需要可变引用，因为可能添加 upvalue 描述）
        if let Some(idx) = self.resolve_upvalue(name) {
            return Some(Variable::Upvalue(idx));
        }

        None
    }

    /// 查找模块导出项的 ShapeID
    /// 返回 Some(shape_id) 如果找到模块和导出项
    fn find_module_shape_id(&self, module_name: &str, export_name: &str) -> Option<u16> {
        // 检查是否是模块别名（如 import std as s; 中的 s）
        let actual_module_name = self
            .module_aliases
            .get(module_name)
            .map(|s| s.as_str())
            .unwrap_or(module_name);

        // 在已编译的模块中查找
        for module in &self.modules {
            if module.name == actual_module_name {
                return module.export_name_to_shape_id.get(export_name).copied();
            }
        }
        // 在当前正在编译的模块中查找
        if let Some(ref current) = self.current_module {
            if current.name == actual_module_name {
                return current.export_name_to_shape_id.get(export_name).copied();
            }
        }
        // 在标准库模块中查找
        self.find_std_module_shape_id(actual_module_name, export_name)
    }

    /// 查找标准库模块的 ShapeID
    fn find_std_module_shape_id(&self, module_name: &str, export_name: &str) -> Option<u16> {
        match module_name {
            "std" => match export_name {
                // 核心函数 (0-3)
                "print" => Some(0),
                "assert" => Some(1),
                "type" => Some(2),
                "to_string" => Some(3),
                // 数学函数 (4-8)
                "sqrt" => Some(4),
                "sin" => Some(5),
                "cos" => Some(6),
                "floor" => Some(7),
                "ceil" => Some(8),
                // 数学常量 (9-10)
                "PI" => Some(9),
                "E" => Some(10),
                // 协程函数 (11-13)
                "create_coroutine" => Some(11),
                "resume" => Some(12),
                "coroutine_status" => Some(13),
                _ => None,
            },
            _ => None,
        }
    }

    /// 检查名称是否是已定义的模块名（包括导入的）
    fn is_module_name(&self, name: &str) -> bool {
        // 检查是否是已导入的模块
        if self.imported_modules.iter().any(|m| m == name) {
            return true;
        }

        // 检查标准库模块（硬编码，启动时自动加载）
        if name == "std" {
            return true;
        }

        // 在已编译的模块中查找
        for module in &self.modules {
            if module.name == name {
                return true;
            }
        }
        // 在当前正在编译的模块中查找
        if let Some(ref current) = self.current_module {
            if current.name == name {
                return true;
            }
        }
        false
    }

    /// 获取表达式的类型（简化版，仅支持变量引用）
    fn get_expr_type(&self, expr: &Expr) -> Option<VarType> {
        match expr.as_ref() {
            ExprKind::VarRef(var_ref) => {
                self.var_types.get(&var_ref.name).cloned()
            }
            _ => None,
        }
    }

    /// 发射局部变量加载指令
    fn emit_load_local(&mut self, idx: u8) {
        match idx {
            0 => self.chunk.write_op(OpCode::LoadLocal0, 0),
            1 => self.chunk.write_op(OpCode::LoadLocal1, 0),
            2 => self.chunk.write_op(OpCode::LoadLocal2, 0),
            3 => self.chunk.write_op(OpCode::LoadLocal3, 0),
            4 => self.chunk.write_op(OpCode::LoadLocal4, 0),
            5 => self.chunk.write_op(OpCode::LoadLocal5, 0),
            6 => self.chunk.write_op(OpCode::LoadLocal6, 0),
            7 => self.chunk.write_op(OpCode::LoadLocal7, 0),
            _ => self.chunk.write_op_u8(OpCode::LoadLocal, idx, 0),
        }
    }

    /// 发射局部变量存储指令
    fn emit_store_local(&mut self, idx: u8) {
        match idx {
            0 => self.chunk.write_op(OpCode::StoreLocal0, 0),
            1 => self.chunk.write_op(OpCode::StoreLocal1, 0),
            2 => self.chunk.write_op(OpCode::StoreLocal2, 0),
            3 => self.chunk.write_op(OpCode::StoreLocal3, 0),
            4 => self.chunk.write_op(OpCode::StoreLocal4, 0),
            5 => self.chunk.write_op(OpCode::StoreLocal5, 0),
            6 => self.chunk.write_op(OpCode::StoreLocal6, 0),
            7 => self.chunk.write_op(OpCode::StoreLocal7, 0),
            _ => self.chunk.write_op_u8(OpCode::StoreLocal, idx, 0),
        }
    }

    /// 发射 upvalue 加载指令
    fn emit_load_upvalue(&mut self, idx: u8) {
        self.chunk.write_op_u8(OpCode::GetUpvalue, idx, 0);
    }

    /// 发射 upvalue 存储指令
    fn emit_store_upvalue(&mut self, idx: u8) {
        self.chunk.write_op_u8(OpCode::SetUpvalue, idx, 0);
    }

    // ==================== 控制流编译 ====================

    /// 编译 if/elif/else 语句
    fn compile_if(&mut self, if_stmt: &IfStmt) -> Result<(), CompileError> {
        // 编译 if 条件
        self.compile_expr(&if_stmt.if_condition)?;

        // 如果条件为假，跳过 then 分支
        let then_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 编译 then 分支
        self.compile_stmt(&if_stmt.then_body)?;

        // then 分支执行完后，跳过所有 elif 和 else 分支
        let else_jump = self.chunk.write_jump(OpCode::Jump, 0);

        // 修补 then_jump，指向 elif 或 else 分支的开始位置
        self.chunk.patch_jump(then_jump);

        // 保存所有 elif 分支的跳转，最后统一修补到 else 分支之后
        let mut elif_jumps = Vec::new();

        // 编译 elif 分支
        for (elif_cond, elif_body) in if_stmt
            .elif_conditions
            .iter()
            .zip(if_stmt.elif_bodies.iter())
        {
            // 编译 elif 条件
            self.compile_expr(elif_cond)?;

            // 如果条件为假，跳过这个 elif 的 body
            let elif_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

            // 编译 elif body
            self.compile_stmt(elif_body)?;

            // elif body 执行完后，跳过剩余的分支
            let next_jump = self.chunk.write_jump(OpCode::Jump, 0);
            elif_jumps.push(next_jump);

            // 修补 elif_jump，指向下一个 elif 或 else 分支
            self.chunk.patch_jump(elif_jump);
        }

        // 编译 else 分支（如果存在）
        if let Some(else_body) = &if_stmt.else_body {
            self.compile_stmt(else_body)?;
        } else {
            // 如果没有 else 分支，加载 null
            self.chunk.write_op(OpCode::LoadNull, 0);
        }

        // 修补所有跳转到 else 分支之后的跳转
        if if_stmt.elif_conditions.is_empty() {
            // 只有 if-else，没有 elif
            self.chunk.patch_jump(else_jump);
        } else {
            // 有 elif 分支，修补 if 的 else_jump 和所有 elif 的 next_jump
            self.chunk.patch_jump(else_jump);
            for jump in elif_jumps {
                self.chunk.patch_jump(jump);
            }
        }

        Ok(())
    }

    /// 编译 while 循环
    fn compile_while(&mut self, while_stmt: &WhileStmt) -> Result<(), CompileError> {
        // 记录循环开始位置
        let loop_start = self.chunk.code.len();

        // 编译循环条件
        self.compile_expr(&while_stmt.condition)?;

        // 如果条件为假，跳出循环
        let exit_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 编译循环体
        self.compile_stmt(&while_stmt.body)?;

        // 跳回循环开始位置
        self.chunk.write_loop(loop_start, 0);

        // 修补退出跳转
        self.chunk.patch_jump(exit_jump);

        Ok(())
    }

    /// 编译 for-in 循环（基于迭代器协议）
    /// 语法: for var item in iterable { body }
    fn compile_for(&mut self, for_stmt: &ForStmt) -> Result<(), CompileError> {
        // 解析迭代变量声明
        let var_name = match for_stmt.iterator.as_ref() {
            ExprKind::VarRef(VarRef { name }) => name.clone(),
            _ => {
                return Err(CompileError::Unimplemented(
                    "For loop iterator must be a variable".to_string(),
                ));
            }
        };

        // 进入 for 循环作用域
        self.begin_scope();

        // 1. 获取迭代器: var $iter = iterable.iter();
        self.compile_expr(&for_stmt.iterable)?;
        self.chunk.write_op(OpCode::GetIter, 0);
        let iter_idx = self.add_local("$iter")?;
        self.mark_initialized();
        self.emit_store_local(iter_idx);

        // 2. 声明迭代变量（只声明一次，循环内赋值）
        let var_idx = self.add_local(&var_name)?;
        self.mark_initialized();

        // 3. 循环开始
        let loop_start = self.chunk.code.len();

        // 4. 获取下一个值
        self.emit_load_local(iter_idx);
        self.chunk.write_op(OpCode::IterNext, 0);

        // 5. 复制值用于 null 检查
        self.chunk.write_op(OpCode::Dup, 0);

        // 6. 检查是否为 null（结束标记）
        self.chunk.write_op(OpCode::LoadNull, 0);
        self.chunk.write_op(OpCode::Equal, 0);
        let exit_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 是 null，退出循环
        // 注意：JumpIfFalse 已经弹出 true，栈上是 [null]
        self.chunk.write_op(OpCode::Pop, 0); // 弹出 null 值
        let exit_patch = self.chunk.write_jump(OpCode::Jump, 0); // 跳到循环外

        // 7. 不是 null，赋值给迭代变量
        self.chunk.patch_jump(exit_jump);
        // 注意：JumpIfFalse 已经弹出 false，栈上只剩 next 值
        self.emit_store_local(var_idx); // 弹出 next 值，存入 item

        // 8. 编译循环体
        self.compile_stmt(&for_stmt.body)?;

        // 9. 跳回循环开始
        self.chunk.write_loop(loop_start, 0);

        // 10. 修补退出跳转
        self.chunk.patch_jump(exit_patch);

        // 11. 退出作用域（item 和 $iter 被清理）
        self.end_scope();

        Ok(())
    }

    // ==================== 模块编译 ====================

    /// 编译模块定义
    /// module name { ... }
    /// 模块在运行时是一个 ObjModule 对象，导出项按索引存储
    fn compile_module(&mut self, module_stmt: &ModuleStmt) -> Result<(), CompileError> {
        // 进入模块作用域
        self.begin_scope();

        // 创建模块信息
        let module_info = ModuleInfo {
            name: module_stmt.name.clone(),
            exports: Vec::new(),
            export_name_to_shape_id: HashMap::new(),
        };

        // 设置当前模块
        let prev_module = self.current_module.take();
        self.current_module = Some(module_info);

        // 编译模块体
        self.compile_stmt(&module_stmt.body)?;

        // 收集导出信息并生成模块对象
        if let Some(info) = self.current_module.take() {
            let export_count = info.exports.len();

            // 对于每个导出项，加载其值（正序压栈，BuildModule 会 reverse）
            for export in info.exports.iter() {
                // 加载导出值（从局部变量）
                self.emit_load_local(export.local_idx);
            }

            // 生成 BuildModule 指令创建模块对象
            if export_count > 255 {
                return Err(CompileError::Unimplemented(
                    "Module has too many exports (max 255)".to_string(),
                ));
            }
            self.chunk
                .write_op_u8(OpCode::BuildModule, export_count as u8, 0);

            // 定义全局变量：模块名
            let module_name_obj = Box::new(ObjString::new(module_stmt.name.clone()));
            let module_name_ptr = Box::into_raw(module_name_obj) as *mut ObjString;
            let module_name_val = Value::string(module_name_ptr);
            let module_name_idx = self.chunk.add_constant(module_name_val);
            self.chunk
                .write_op_u8(OpCode::DefineGlobal, module_name_idx, 0);

            // 保存模块信息
            self.modules.push(info);
        }

        // 恢复之前的模块上下文
        self.current_module = prev_module;

        // 退出模块作用域
        self.end_scope();

        Ok(())
    }

    /// 编译导入语句
    /// import module; 或 from module import item;
    fn compile_import(
        &mut self,
        import_stmt: &crate::compiler::parser::stmt::ImportStmt,
    ) -> Result<(), CompileError> {
        use crate::runtime::object::ObjString;

        // 检查模块是否存在（同文件内模块或标准库模块）
        let module_name = &import_stmt.module_path;

        if !self.is_module_name(module_name) && !self.is_std_module(module_name) {
            return Err(CompileError::Unimplemented(format!(
                "Module '{}' not found",
                module_name
            )));
        }

        // 将模块名加入导入列表
        self.imported_modules.push(module_name.clone());

        // 处理 from...import 语句：为每个导入的项创建局部变量
        if !import_stmt.items.is_empty() {
            // from module import item1, item2, ...
            for item_name in &import_stmt.items {
                // 为导入的项创建局部变量
                self.add_local(item_name)?;

                // 加载模块并获取导出项
                // 1. 加载模块名（字符串常量）
                let module_str_obj = Box::new(ObjString::new(module_name.clone()));
                let module_str_ptr = Box::into_raw(module_str_obj) as *mut ObjString;
                let module_name_val = Value::string(module_str_ptr);
                let module_name_constant = self.chunk.add_constant(module_name_val);
                self.emit_constant(module_name_constant);

                // 2. 获取模块对象
                self.chunk.write_op(OpCode::GetModule, 0);

                // 3. 添加导出项名称到常量池（用于 GetModuleExport）
                let item_str_obj = Box::new(ObjString::new(item_name.clone()));
                let item_str_ptr = Box::into_raw(item_str_obj) as *mut ObjString;
                let item_name_val = Value::string(item_str_ptr);
                let item_name_constant = self.chunk.add_constant(item_name_val);

                if item_name_constant > 254 {
                    return Err(CompileError::TooManyConstants);
                }

                // 4. 使用 GetModuleExport 从模块获取导出项
                // 操作数: u8 常量池索引
                self.chunk.write_op(OpCode::GetModuleExport, 0);
                self.chunk.code.push(item_name_constant as u8);
                self.chunk.lines.push(0);

                // 5. 存储到局部变量
                let local_idx = (self.locals.len() - 1) as u8;
                self.emit_store_local(local_idx);

                // 6. 标记为已初始化
                self.mark_initialized();
            }
        } else if let Some(alias) = &import_stmt.alias {
            // import module as alias
            // 记录别名到模块名的映射（用于后续成员访问解析）
            self.module_aliases
                .insert(alias.clone(), module_name.clone());

            // 为别名创建局部变量
            self.add_local(alias)?;

            // 加载模块名
            let module_str_obj = Box::new(ObjString::new(module_name.clone()));
            let module_str_ptr = Box::into_raw(module_str_obj) as *mut ObjString;
            let module_name_val = Value::string(module_str_ptr);
            let module_name_constant = self.chunk.add_constant(module_name_val);
            self.emit_constant(module_name_constant);

            // 获取模块对象
            self.chunk.write_op(OpCode::GetModule, 0);

            // 存储到别名变量
            let local_idx = (self.locals.len() - 1) as u8;
            self.emit_store_local(local_idx);
            self.mark_initialized();
        }
        // 否则是简单的 import module;，不做特殊处理（运行时会加载模块到全局）

        Ok(())
    }

    /// 检查是否是标准库模块
    fn is_std_module(&self, name: &str) -> bool {
        name == "std"
    }

    // ==================== 函数编译 ====================

    /// 编译 lambda 表达式
    fn compile_lambda(&mut self, lambda: &Lambda) -> Result<(), CompileError> {
        // 创建子编译器，传递 self 作为 enclosing（用于解析 upvalue）
        let mut function_compiler = Compiler::new_child(self);

        // 为每个参数添加局部变量
        for (param_name, _param_type) in &lambda.params {
            function_compiler.add_local(param_name)?;
            function_compiler.mark_initialized();
        }

        // 编译函数体
        function_compiler.compile_stmt(&lambda.body)?;

        // 函数体末尾添加返回 null（如果没有显式返回）
        function_compiler.chunk.write_op(OpCode::LoadNull, 0);
        function_compiler.chunk.write_op(OpCode::Return, 0);

        // 获取 upvalue 信息
        let upvalues = std::mem::take(&mut function_compiler.upvalues);

        // 创建函数对象
        let function = Box::new(ObjFunction::new(
            function_compiler.chunk,
            lambda.params.len() as u8,
            None, // 暂时不支持函数名
        ));

        // 将函数对象作为常量添加到当前 chunk
        let function_ptr = Box::into_raw(function) as *mut ObjFunction;
        let function_value = Value::function(function_ptr);
        let idx = self.chunk.add_constant(function_value);

        // 发射 Closure 指令：函数索引 + upvalue数量 + upvalue描述符
        self.chunk.write_op(OpCode::Closure, 0);
        self.chunk.code.push(idx as u8);
        self.chunk.lines.push(0);
        self.chunk.code.push(upvalues.len() as u8);
        self.chunk.lines.push(0);

        // 输出 upvalue 描述符
        for upvalue in &upvalues {
            self.chunk.code.push(if upvalue.is_local { 1 } else { 0 });
            self.chunk.lines.push(0);
            self.chunk.code.push(upvalue.index);
            self.chunk.lines.push(0);
        }

        Ok(())
    }

    /// 编译函数调用
    fn compile_function_call(&mut self, call: &FunctionCall) -> Result<(), CompileError> {
        // 检测是否是方法调用：obj.method(args)
        if let ExprKind::MemberAccess(member) = call.function_expr.as_ref() {
            // 尝试编译为方法调用
            if let Some(struct_name) = self.try_infer_struct_type(&member.object) {
                // 获取方法索引
                let method_idx = self.struct_infos.get(&struct_name).and_then(|info| {
                    info.method_names
                        .iter()
                        .position(|name| name == &member.member)
                        .map(|idx| idx as u8)
                });

                if let Some(idx) = method_idx {
                    // 编译参数（先编译 self）
                    self.compile_expr(&member.object)?; // self
                    for arg in call.arguments.iter() {
                        self.compile_expr(arg)?;
                    }

                    // 从 Shape 加载方法函数
                    self.chunk.write_op_u8(OpCode::LoadMethod, idx, 0);

                    // 调用方法（参数数包含 self）
                    let arg_count = (call.arguments.len() + 1) as u8;
                    self.chunk.write_op_u8(OpCode::Call, arg_count, 0);
                    return Ok(());
                }
            }
        }

        // 普通函数调用
        // 先编译参数（参数从左到右压栈）
        for arg in call.arguments.iter() {
            self.compile_expr(arg)?;
        }

        // 编译函数表达式
        self.compile_expr(&call.function_expr)?;

        // 发射 Call 指令
        let arg_count = call.arguments.len() as u8;
        self.chunk.write_op_u8(OpCode::Call, arg_count, 0);

        Ok(())
    }

    /// 尝试推断表达式的 struct 类型
    fn try_infer_struct_type(&self, expr: &Expr) -> Option<String> {
        // 从变量类型表中查找
        if let ExprKind::VarRef(var_ref) = expr.as_ref() {
            if let Some(var_type) = self.var_types.get(&var_ref.name) {
                let VarType::Struct(struct_name) = var_type;
                return Some(struct_name.clone());
            }
        }
        None
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// 编译模块的便捷函数
/// 返回 (Chunk, 局部变量数量)
pub fn compile(module: &Module) -> Result<(Chunk, usize), CompileError> {
    compile_with_shapes(module, HashMap::new())
}

pub fn compile_with_shapes(
    module: &Module,
    shape_ids: HashMap<String, u16>,
) -> Result<(Chunk, usize), CompileError> {
    let mut compiler = Compiler::new_with_shapes(shape_ids);
    compiler.compile(module)
}

/// 带完整字段信息的编译（支持编译期字段索引优化）
pub fn compile_with_struct_info(
    module: &Module,
    struct_infos: HashMap<String, (u16, Vec<String>)>, // name -> (shape_id, field_names)
) -> Result<(Chunk, usize), CompileError> {
    compile_with_struct_info_and_logger(module, struct_infos, Logger::noop())
}

/// 带完整字段信息和 logger 的编译
pub fn compile_with_struct_info_and_logger(
    module: &Module,
    struct_infos: HashMap<String, (u16, Vec<String>)>, // name -> (shape_id, field_names)
    logger: Arc<Logger>,
) -> Result<(Chunk, usize), CompileError> {
    let mut compiler = Compiler::new_with_struct_infos_and_logger(struct_infos, logger);
    compiler.compile(module)
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::builder::build_lexer;
    use crate::compiler::parser::parser::Parser;
    use crate::runtime::{InterpretResult, VM};
    use crate::runtime::object::ObjShape;

    fn compile_code(code: &str) -> Result<(Chunk, usize, HashMap<String, (u16, Vec<String>)>), CompileError> {
        let mut lexer = build_lexer();
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let mut parser = Parser::new(lexer);
        let ast = parser
            .parse()
            .map_err(|e| CompileError::Unimplemented(format!("Parse error: {:?}", e)))?;

        // 收集 struct 信息
        let mut struct_infos: HashMap<String, (u16, Vec<String>)> = HashMap::new();
        let mut next_shape_id: u16 = 1; // 从 1 开始，0 保留
        
        for stmt in &ast.statements {
            if let StmtKind::Struct(struct_stmt) = stmt.as_ref() {
                let field_names: Vec<String> = struct_stmt
                    .fields
                    .iter()
                    .map(|f| f.name.clone())
                    .collect();
                struct_infos.insert(struct_stmt.name.clone(), (next_shape_id, field_names));
                next_shape_id += 1;
            }
        }

        let (chunk, local_count) = compile_with_struct_info(&ast, struct_infos.clone())?;
        Ok((chunk, local_count, struct_infos))
    }

    fn run_code(code: &str) -> Result<Value, String> {
        let (chunk, local_count, struct_infos) =
            compile_code(code).map_err(|e| format!("Compile error: {:?}", e))?;
        
        let mut vm = VM::new();
        
        // 注册 shapes 到 VM
        for (name, (shape_id, field_names)) in struct_infos {
            let shape = Box::into_raw(Box::new(ObjShape::new(
                shape_id,
                name,
                field_names,
            )));
            vm.register_shape(shape);
        }
        
        let result = vm.interpret_with_locals(&chunk, local_count);
        match result {
            InterpretResult::Ok => {
                // 返回值在栈顶
                vm.stack_top().ok_or("Empty stack".to_string())
            }
            InterpretResult::RuntimeError(msg) => Err(msg),
            InterpretResult::CompileError(msg) => Err(msg),
        }
    }

    #[test]
    fn test_compile_literal() {
        let (chunk, _, _) = compile_code("42;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_binary() {
        let (chunk, _, _) = compile_code("1 + 2;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_complex() {
        let (chunk, _, _) = compile_code("1 + 2 * 3;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    // ===== End-to-End 测试 =====
    // 设计理念：代码文件 = 匿名函数体
    // - 表达式语句结果被 Pop（无副作用）
    // - 使用 return 显式返回值

    #[test]
    fn test_run_literal() {
        // 表达式语句结果丢弃，返回 null
        let result = run_code("42;").unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_run_addition() {
        // 使用 return 获取结果
        let result = run_code("return 1 + 2;").unwrap();
        assert_eq!(result.as_smi(), Some(3));
    }

    #[test]
    fn test_run_complex() {
        // 1 + 2 * 3 = 7
        let result = run_code("return 1 + 2 * 3;").unwrap();
        assert_eq!(result.as_smi(), Some(7));
    }

    #[test]
    fn test_run_division() {
        // 5 / 2 = 2.5
        let result = run_code("return 5 / 2;").unwrap();
        assert!(result.is_float());
        assert_eq!(result.as_float(), 2.5);
    }

    #[test]
    fn test_run_comparison() {
        let result = run_code("return 2 > 1;").unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_run_true() {
        let result = run_code("return true;").unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_run_false() {
        let result = run_code("return false;").unwrap();
        assert!(result.is_false());
    }

    #[test]
    fn test_run_null() {
        let result = run_code("return null;").unwrap();
        assert!(result.is_null());
    }

    // ===== 变量测试 =====

    #[test]
    fn test_run_variable_declaration() {
        // var x = 5; return x;
        let result = run_code("var x = 5; return x;").unwrap();
        assert_eq!(result.as_smi(), Some(5));
    }

    #[test]
    fn test_run_variable_expression() {
        // var x = 5; var y = x + 3; return y;
        let result = run_code("var x = 5; var y = x + 3; return y;").unwrap();
        assert_eq!(result.as_smi(), Some(8));
    }

    #[test]
    fn test_run_variable_assignment() {
        // var x = 5; x = 10; return x;
        // 注意：赋值表达式返回 null，所以要用分号分隔语句
        let result = run_code("var x = 5; x = 10; return x;").unwrap();
        assert_eq!(result.as_smi(), Some(10));
    }

    #[test]
    fn test_assignment_returns_null() {
        // 赋值表达式本身返回 null
        let result = run_code("var x = 5; return x = 10;").unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_run_multiple_variables() {
        // var a = 1; var b = 2; var c = 3; return a + b + c;
        let result = run_code("var a = 1; var b = 2; var c = 3; return a + b + c;").unwrap();
        assert_eq!(result.as_smi(), Some(6));
    }

    // ===== Lambda 测试 =====

    #[test]
    fn test_compile_lambda() {
        // 测试基本的 lambda 编译
        let (chunk, _, _) = compile_code("|x| { return x + 1; };").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_lambda_no_params() {
        // 测试无参数 lambda
        let (chunk, _, _) = compile_code("| | { return 42; };").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_lambda_multi_params() {
        // 测试多参数 lambda
        let (chunk, _, _) = compile_code("|a, b| { return a + b; };").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_function_call() {
        // 测试函数调用
        let (chunk, _, _) = compile_code("var f = |x| { return x + 1; }; f(5);").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_run_lambda() {
        // 测试基本的 lambda 调用
        let result = run_code("var f = |x| { return x + 1; }; return f(5);").unwrap();
        assert_eq!(result.as_smi(), Some(6));
    }

    #[test]
    fn test_run_lambda_no_params() {
        // 测试无参数 lambda
        let result = run_code("var f = | | { return 42; }; return f();").unwrap();
        assert_eq!(result.as_smi(), Some(42));
    }

    #[test]
    fn test_run_lambda_multi_params() {
        // 测试多参数 lambda
        let result = run_code("var add = |a, b| { return a + b; }; return add(3, 4);").unwrap();
        assert_eq!(result.as_smi(), Some(7));
    }

    #[test]
    fn test_closure_capture() {
        // 测试基础闭包捕获
        let result = run_code("var x = 5; var f = || { return x; }; return f();").unwrap();
        assert_eq!(result.as_smi(), Some(5));
    }

    #[test]
    fn test_closure_capture_modify() {
        // 测试闭包捕获并修改外部变量
        // 第一次调用: y=10, y=11, 返回 11
        // 第二次调用: y=11, y=12, 返回 12
        // r1 + r2 = 11 + 12 = 23
        let result = run_code(
            "
            var y = 10;
            var g = || { y = y + 1; return y; };
            var r1 = g();
            var r2 = g();
            return r1 + r2;
        ",
        )
        .unwrap();
        assert_eq!(result.as_smi(), Some(23));
    }

    #[test]
    fn test_closure_multi_capture() {
        // 测试多变量捕获
        let result = run_code(
            "
            var a = 1;
            var b = 2;
            var h = || { return a + b; };
            return h();
        ",
        )
        .unwrap();
        assert_eq!(result.as_smi(), Some(3));
    }

    // TODO: 嵌套闭包测试
    // #[test]
    // fn test_nested_closure() {
    //     let result = run_code("
    //         var outer = 100;
    //         var f1 = || {
    //             var inner = 10;
    //             var f2 = || { return outer + inner; };
    //             return f2();
    //         };
    //         return f1();
    //     ").unwrap();
    //     assert_eq!(result.as_smi(), Some(110));
    // }

    // ===== 运算符重载测试 =====

    #[test]
    fn test_struct_basic() {
        // 测试基本的 struct 创建和字段访问
        let result = run_code(
            "
            struct Vector {
                x: float,
                y: float
            };
            
            var v = Vector { x: 1.0, y: 2.0 };
            return v.x;
        ",
        )
        .unwrap();
        
        assert!(result.is_float());
        assert_eq!(result.as_float(), 1.0);
    }

    #[test]
    fn test_operator_overloading_add() {
        // 测试 Vector 的 operator add - 只访问 self.x
        // 完整测试：创建新的 Vector
        let result = run_code(
            "
            struct Vector {
                x: float,
                y: float
            };
            
            impl Vector {
                operator add: |self, other: Vector| -> Vector {
                    return Vector {
                        x: self.x,
                        y: other.y
                    };
                }
            };
            
            var v1 = Vector { x: 1.0, y: 2.0 };
            var v2 = Vector { x: 3.0, y: 4.0 };
            var v3 = v1 + v2;
            return v3.x + v3.y;
        ",
        )
        .unwrap();
        
        assert!(result.is_float());
        assert_eq!(result.as_float(), 5.0);  // 1.0 + 4.0
    }
}
