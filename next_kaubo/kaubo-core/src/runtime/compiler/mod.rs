//! AST → Bytecode 编译器

pub mod context;
pub mod error;
pub mod expr;
pub mod stmt;
pub mod var;

// 重新导出 public API
pub use context::{ModuleInfo, StructInfo, VarType};
pub use error::CompileError;
pub use var::{Local, Upvalue, Variable};

use crate::compiler::parser::{Expr, ExprKind, Module, Stmt};
use crate::core::{Chunk, OpCode};
use kaubo_log::{trace, Logger};
use std::collections::HashMap;
use std::sync::Arc;

/// AST 编译器
pub struct Compiler {
    pub(crate) chunk: Chunk,
    pub(crate) locals: Vec<Local>,
    pub(crate) upvalues: Vec<Upvalue>,
    pub(crate) scope_depth: usize,
    pub(crate) max_locals: usize,
    pub(crate) enclosing: *mut Compiler,
    pub(crate) current_module: Option<ModuleInfo>,
    pub(crate) modules: Vec<ModuleInfo>,
    pub(crate) imported_modules: Vec<String>,
    pub(crate) module_aliases: HashMap<String, String>,
    pub(crate) struct_infos: HashMap<String, StructInfo>,
    pub(crate) var_types: HashMap<String, VarType>,
    pub(crate) logger: Arc<Logger>,
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
                        field_types: Vec::new(),
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
    /// infos: name -> (shape_id, field_names, field_types)
    pub fn new_with_struct_infos(infos: HashMap<String, (u16, Vec<String>, Vec<String>)>) -> Self {
        Self::new_with_struct_infos_and_logger(infos, Logger::noop())
    }

    pub fn new_with_struct_infos_and_logger(
        infos: HashMap<String, (u16, Vec<String>, Vec<String>)>,
        logger: Arc<Logger>,
    ) -> Self {
        let struct_infos: HashMap<String, StructInfo> = infos
            .into_iter()
            .map(|(name, (shape_id, field_names, field_types))| {
                (
                    name,
                    StructInfo {
                        shape_id,
                        field_names,
                        field_types,
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

    /// 编译语句（委托给 stmt 模块）
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        stmt::compile_stmt(self, stmt)
    }

    /// 编译表达式（委托给 expr 模块）
    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        expr::compile_expr(self, expr)
    }

    /// 发射常量加载指令 (优化：使用 LoadConst0-15)
    fn emit_constant(&mut self, idx: u8) {
        var::emit_constant(self, idx);
    }

    /// 发射局部变量加载指令
    fn emit_load_local(&mut self, idx: u8) {
        var::emit_load_local(self, idx);
    }

    /// 发射局部变量存储指令
    fn emit_store_local(&mut self, idx: u8) {
        var::emit_store_local(self, idx);
    }

    /// 发射 upvalue 加载指令
    fn emit_load_upvalue(&mut self, idx: u8) {
        var::emit_load_upvalue(self, idx);
    }

    /// 发射 upvalue 存储指令
    fn emit_store_upvalue(&mut self, idx: u8) {
        var::emit_store_upvalue(self, idx);
    }

    /// 查找模块导出项的 ShapeID
    /// 返回 Some(shape_id) 如果找到模块和导出项
    pub(crate) fn find_module_shape_id(&self, module_name: &str, export_name: &str) -> Option<u16> {
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
    pub(crate) fn is_module_name(&self, name: &str) -> bool {
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

    /// 获取表达式的类型（扩展版，支持变量引用、函数调用、struct 字面量、unary/binary 运算符等）
    pub(crate) fn get_expr_type(&self, expr: &Expr) -> Option<VarType> {
        match expr.as_ref() {
            ExprKind::VarRef(var_ref) => {
                self.var_types.get(&var_ref.name).cloned()
            }
            ExprKind::StructLiteral(struct_lit) => {
                Some(VarType::Struct(struct_lit.name.clone()))
            }
            ExprKind::LiteralInt(_) => {
                Some(VarType::Int)
            }
            ExprKind::LiteralFloat(_) => {
                Some(VarType::Float)
            }
            ExprKind::LiteralString(_) => {
                Some(VarType::String)
            }
            ExprKind::LiteralTrue(_) | ExprKind::LiteralFalse(_) => {
                Some(VarType::Bool)
            }
            ExprKind::LiteralList(_) => {
                // List 字面量：推导为 List<any>（元素类型暂时不精确推导）
                Some(VarType::List(Box::new(VarType::Int))) // 简化：假设 List<int>
            }
            ExprKind::JsonLiteral(_) => {
                Some(VarType::Json)
            }
            ExprKind::FunctionCall(call) => {
                // 尝试推断函数返回类型
                self.infer_call_return_type(call)
            }
            ExprKind::Unary(unary) => {
                // 尝试推断 unary 运算符返回类型
                self.infer_unary_return_type(unary)
            }
            ExprKind::Binary(binary) => {
                // 尝试推断 binary 运算符返回类型
                self.infer_binary_return_type(binary)
            }
            _ => None,
        }
    }

    /// 推断 unary 运算符的返回类型
    fn infer_unary_return_type(&self, unary: &crate::compiler::parser::expr::Unary) -> Option<VarType> {
        match unary.op {
            crate::compiler::lexer::token_kind::KauboTokenKind::Minus => {
                // 取负运算符：返回类型与操作数相同
                self.get_expr_type(&unary.operand)
            }
            _ => None,
        }
    }

    /// 推断函数调用的返回类型
    fn infer_call_return_type(&self, call: &crate::compiler::parser::expr::FunctionCall) -> Option<VarType> {
        // 检查是否是方法调用：obj.method(args)
        if let ExprKind::MemberAccess(member) = call.function_expr.as_ref() {
            if let Some(obj_type) = self.get_expr_type(&member.object) {
                if let VarType::Struct(struct_name) = obj_type {
                    // 查找方法返回类型
                    return self.find_method_return_type(&struct_name, &member.member);
                }
            }
        }
        
        // 检查是否是运算符调用（如 -v）
        // 这里简化处理：如果是单个参数且是 struct 类型，查找 operator
        if call.arguments.len() == 1 {
            if let Some(arg_type) = self.get_expr_type(&call.arguments[0]) {
                if let VarType::Struct(struct_name) = arg_type {
                    // 检查是否有匹配的 operator
                    if let Some(ret_type) = self.find_operator_return_type(&struct_name, "neg") {
                        return Some(ret_type);
                    }
                }
            }
        }
        
        None
    }

    /// 查找方法的返回类型
    fn find_method_return_type(&self, struct_name: &str, method_name: &str) -> Option<VarType> {
        // 对于 operator，返回类型通常是 struct 自身
        if method_name.starts_with("operator ") {
            return Some(VarType::Struct(struct_name.to_string()));
        }
        // TODO: 从方法定义中获取实际返回类型
        None
    }

    /// 查找运算符的返回类型
    fn find_operator_return_type(&self, struct_name: &str, op: &str) -> Option<VarType> {
        // 简化：运算符通常返回相同的 struct 类型
        // 实际应该从 impl 块定义中查找
        Some(VarType::Struct(struct_name.to_string()))
    }

    /// 推断 binary 运算符的返回类型
    fn infer_binary_return_type(&self, binary: &crate::compiler::parser::expr::Binary) -> Option<VarType> {
        let left_type = self.get_expr_type(&binary.left)?;
        let right_type = self.get_expr_type(&binary.right)?;
        
        // 获取运算符名称
        let op_name = match binary.op {
            crate::compiler::lexer::token_kind::KauboTokenKind::Plus => "add",
            crate::compiler::lexer::token_kind::KauboTokenKind::Minus => "sub",
            crate::compiler::lexer::token_kind::KauboTokenKind::Asterisk => "mul",
            crate::compiler::lexer::token_kind::KauboTokenKind::Slash => "div",
            crate::compiler::lexer::token_kind::KauboTokenKind::Percent => "mod",
            // 比较运算符返回 bool，但我们不跟踪 bool 类型，返回 None
            crate::compiler::lexer::token_kind::KauboTokenKind::LessThan => return None,
            crate::compiler::lexer::token_kind::KauboTokenKind::LessThanEqual => return None,
            crate::compiler::lexer::token_kind::KauboTokenKind::GreaterThan => return None,
            crate::compiler::lexer::token_kind::KauboTokenKind::GreaterThanEqual => return None,
            crate::compiler::lexer::token_kind::KauboTokenKind::DoubleEqual => return None,
            crate::compiler::lexer::token_kind::KauboTokenKind::ExclamationEqual => return None,
            _ => return None,
        };
        
        // 如果左操作数是 struct，查找 operator 的返回类型
        if let VarType::Struct(struct_name) = left_type {
            return self.find_operator_return_type(&struct_name, op_name);
        }
        
        // 如果右操作数是 struct（尝试 rmul/rsub/rdiv 等反向运算符）
        // 注意：基础类型（Int, Float, String, Bool）在左边时也需要检查反向运算符
        if let VarType::Struct(struct_name) = right_type {
            // 只有当左操作数是基础类型或 struct 时才检查反向运算符
            let should_check_reverse = matches!(left_type, 
                VarType::Int | VarType::Float | VarType::String | VarType::Bool
            );
            if should_check_reverse {
                let reverse_op = match op_name {
                    "add" => "radd",
                    "sub" => "rsub",
                    "mul" => "rmul",
                    "div" => "rdiv",
                    "mod" => "rmod",
                    _ => op_name,
                };
                return self.find_operator_return_type(&struct_name, reverse_op);
            }
        }
        
        // 基础类型的运算返回左操作数类型（简化处理）
        Some(left_type)
    }

    // ==================== 以下方法供其他模块使用（内部 API） ====================

    /// 进入新作用域
    fn begin_scope(&mut self) {
        var::begin_scope(self);
    }

    /// 退出作用域，返回弹出的变量数量
    fn end_scope(&mut self) -> usize {
        var::end_scope(self)
    }

    /// 添加局部变量，返回其在栈中的索引
    fn add_local(&mut self, name: &str) -> Result<u8, CompileError> {
        var::add_local(self, name)
    }

    /// 标记最后一个变量为已初始化
    fn mark_initialized(&mut self) {
        var::mark_initialized(self);
    }

    /// 解析变量名，返回其在局部变量表中的索引
    fn resolve_local(&self, name: &str) -> Option<u8> {
        var::resolve_local(self, name)
    }

    /// 标记局部变量被捕获
    fn mark_captured(&mut self, index: usize) {
        var::mark_captured(self, index);
    }

    /// 添加 upvalue 描述，返回其索引
    fn add_upvalue(&mut self, name: &str, index: u8, is_local: bool) -> u8 {
        var::add_upvalue(self, name, index, is_local)
    }

    /// 递归解析 upvalue
    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        var::resolve_upvalue(self, name)
    }

    /// 统一变量解析：Local 或 Upvalue
    fn resolve_variable(&mut self, name: &str) -> Option<Variable> {
        var::resolve_variable(self, name)
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
/// struct_infos: name -> (shape_id, field_names, field_types)
pub fn compile_with_struct_info(
    module: &Module,
    struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)>,
) -> Result<(Chunk, usize), CompileError> {
    compile_with_struct_info_and_logger(module, struct_infos, Logger::noop())
}

/// 带完整字段信息和 logger 的编译
/// struct_infos: name -> (shape_id, field_names, field_types)
pub fn compile_with_struct_info_and_logger(
    module: &Module,
    struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)>,
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
    use crate::core::{InterpretResult, Value, VM};
    use crate::core::object::ObjShape;

    fn compile_code(code: &str) -> Result<(Chunk, usize, HashMap<String, (u16, Vec<String>, Vec<String>)>), CompileError> {
        let mut lexer = build_lexer();
        let _ = lexer.feed(code.as_bytes());
        let _ = lexer.terminate();

        let mut parser = Parser::new(lexer);
        let ast = parser
            .parse()
            .map_err(|e| CompileError::Unimplemented(format!("Parse error: {e:?}")))?;

        // 收集 struct 信息（包括字段名和类型）
        let mut struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)> = HashMap::new();
        // 基础类型使用 0-99，struct 从 100 开始避免冲突
        let mut next_shape_id: u16 = 100;
        
        for stmt in &ast.statements {
            if let crate::compiler::parser::StmtKind::Struct(struct_stmt) = stmt.as_ref() {
                let field_names: Vec<String> = struct_stmt
                    .fields
                    .iter()
                    .map(|f| f.name.clone())
                    .collect();
                let field_types: Vec<String> = struct_stmt
                    .fields
                    .iter()
                    .map(|f| f.type_annotation.to_string())
                    .collect();
                struct_infos.insert(struct_stmt.name.clone(), (next_shape_id, field_names, field_types));
                next_shape_id += 1;
            }
        }

        // 创建带日志的编译器
        let logger = kaubo_log::LogConfig::new(kaubo_log::Level::Trace)
            .with_stdout()
            .init()
            .0;
        let (chunk, local_count) = compile_with_struct_info_and_logger(&ast, struct_infos.clone(), logger)?;
        Ok((chunk, local_count, struct_infos))
    }

    fn run_code(code: &str) -> Result<Value, String> {
        let (chunk, local_count, struct_infos) =
            compile_code(code).map_err(|e| format!("Compile error: {e:?}"))?;
        
        // 创建带日志的 VM
        let config = kaubo_log::LogConfig::new(kaubo_log::Level::Trace)
            .with_stdout()
            .with_ring_buffer(1000);
        let (logger, _ring) = config.init();
        let mut vm = VM::with_config_and_logger(crate::core::VMConfig::default(), logger);
        
        // 注册 shapes 到 VM（包含字段类型）
        for (name, (shape_id, field_names, field_types)) in struct_infos {
            let shape = Box::into_raw(Box::new(ObjShape::new_with_types(
                shape_id,
                name,
                field_names,
                field_types,
            )));
            unsafe {
                vm.register_shape(shape);
            }
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
        assert!(!chunk.code.is_empty());
    }

    #[test]
    fn test_compile_binary() {
        let (chunk, _, _) = compile_code("1 + 2;").unwrap();
        assert!(!chunk.code.is_empty());
    }

    #[test]
    fn test_compile_complex() {
        let (chunk, _, _) = compile_code("1 + 2 * 3;").unwrap();
        assert!(!chunk.code.is_empty());
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
        assert!(!chunk.code.is_empty());
    }

    #[test]
    fn test_compile_lambda_no_params() {
        // 测试无参数 lambda
        let (chunk, _, _) = compile_code("| | { return 42; };").unwrap();
        assert!(!chunk.code.is_empty());
    }

    #[test]
    fn test_compile_lambda_multi_params() {
        // 测试多参数 lambda
        let (chunk, _, _) = compile_code("|a, b| { return a + b; };").unwrap();
        assert!(!chunk.code.is_empty());
    }

    #[test]
    fn test_compile_function_call() {
        // 测试函数调用
        let (chunk, _, _) = compile_code("var f = |x| { return x + 1; }; f(5);").unwrap();
        assert!(!chunk.code.is_empty());
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

    #[test]
    fn test_operator_add_struct_field_order() {
        // 回归测试：验证 operator add 返回 struct 时字段顺序正确
        // Bug: call_operator_closure 中的 BuildStruct 多了 reverse()，导致 x/y 互换
        // 修复: 2026-02-14，移除多余的 reverse()
        let result = run_code(
            "
            struct Vec2 {
                x: float,
                y: float
            };
            
            impl Vec2 {
                operator add: |self, other: Vec2| -> Vec2 {
                    return Vec2 {
                        x: self.x + other.x,
                        y: self.y + other.y
                    };
                }
            };
            
            var v1 = Vec2 { x: 1.0, y: 2.0 };
            var v2 = Vec2 { x: 3.0, y: 4.0 };
            var v3 = v1 + v2;
            
            // 分别验证 x 和 y，确保顺序正确
            // 如果顺序错了，会得到 x=6, y=4 而不是 x=4, y=6
            var x_correct = v3.x == 4.0;
            var y_correct = v3.y == 6.0;
            
            if x_correct and y_correct {
                return 1;  // 成功
            } else {
                return 0;  // 失败
            }
        ",
        )
        .unwrap();

        assert!(result.is_smi());
        assert_eq!(result.as_smi(), Some(1), "operator add 返回的 struct 字段顺序错误");
    }

    #[test]
    fn test_operator_neg() {
        // 测试 operator neg（一元负号）- 简化版
        let result = run_code(
            "
            struct Vec2 {
                x: float,
                y: float
            };
            
            impl Vec2 {
                operator neg: |self| -> Vec2 {
                    return Vec2 { x: self.x, y: self.y };
                }
            };
            
            var v = Vec2 { x: 3.0, y: 4.0 };
            var neg_v = -v;
            return neg_v.x;
        ",
        )
        .unwrap();

        assert!(result.is_float());
        assert_eq!(result.as_float(), 3.0);
    }

    #[test]
    fn test_operator_lt() {
        // 测试 operator lt（小于比较）
        let result = run_code(
            "
            struct Point {
                x: float,
                y: float
            };
            
            impl Point {
                // 按 x 坐标比较
                operator lt: |self, other: Point| -> bool {
                    return self.x < other.x;
                }
            };
            
            var p1 = Point { x: 1.0, y: 5.0 };
            var p2 = Point { x: 3.0, y: 2.0 };
            
            // p1.x < p2.x，所以 p1 < p2 应该为 true
            if p1 < p2 {
                return 1;
            } else {
                return 0;
            }
        ",
        )
        .unwrap();

        assert!(result.is_smi());
        assert_eq!(result.as_smi(), Some(1), "operator lt 结果错误");
    }

    #[test]
    fn test_operator_get() {
        // 测试 operator get（索引访问）
        let result = run_code(
            "
            struct Vector {
                data: List<float>
            };
            
            impl Vector {
                operator get: |self, index: int| -> float {
                    return self.data[index];
                }
            };
            
            var v = Vector { data: [1.0, 2.0, 3.0] };
            var first = v[0];
            var second = v[1];
            
            if first == 1.0 and second == 2.0 {
                return 1;
            } else {
                return 0;
            }
        ",
        )
        .unwrap();

        assert!(result.is_smi());
        assert_eq!(result.as_smi(), Some(1), "operator get 结果错误");
    }

    #[test]
    fn test_simple_struct_field_access() {
        // 简单测试：不使用 operator，只测试 struct 字段访问
        let result = run_code(
            "
            struct Point {
                x: float,
                y: float
            };
            
            var p = Point { x: 3.0, y: 4.0 };
            return p.x + p.y;
        ",
        )
        .unwrap();

        assert!(result.is_float());
        assert_eq!(result.as_float(), 7.0);
    }

    #[test]
    fn test_operator_rmul() {
        // 测试反向运算符 rmul: 2.0 * vector
        // float 没有 operator mul for Vector，但 Vector 有 operator rmul
        let result = run_code(
            "
            struct Vector {
                x: float,
                y: float
            };
            
            impl Vector {
                // vector * scalar
                operator mul: |self, scalar: float| -> Vector {
                    return Vector { 
                        x: self.x * scalar, 
                        y: self.y * scalar 
                    };
                },
                // scalar * vector (反向)
                operator rmul: |self, scalar: float| -> Vector {
                    return Vector { 
                        x: scalar * self.x, 
                        y: scalar * self.y 
                    };
                }
            };
            
            var v = Vector { x: 1.0, y: 2.0 };
            var scaled = 3.0 * v;  // 应该调用 v.rmul(3.0)
            
            return scaled.x + scaled.y;  // 3.0 + 6.0 = 9.0
        ",
        )
        .unwrap();

        assert!(result.is_float());
        assert_eq!(result.as_float(), 9.0);
    }

    #[test]
    fn test_operator_call() {
        // 测试 operator call（可调用对象）
        let result = run_code(
            "
            struct Adder {
                offset: int
            };
            
            impl Adder {
                // 让 Adder 可以像函数一样被调用
                operator call: |self, x: int| -> int {
                    return x + self.offset;
                }
            };
            
            var add5 = Adder { offset: 5 };
            var result = add5(10);  // 调用 operator call
            
            return result;  // 应该返回 15
        ",
        )
        .unwrap();

        assert!(result.is_smi());
        assert_eq!(result.as_smi(), Some(15));
    }
}
