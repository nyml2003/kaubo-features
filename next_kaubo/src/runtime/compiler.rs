//! AST → Bytecode 编译器

use crate::compiler::parser::{
    Expr, ExprKind, Binary, Unary, LiteralInt,
    Stmt, StmtKind, PrintStmt,
    Module,
};
use crate::runtime::{
    bytecode::{chunk::Chunk, OpCode},
    Value,
};

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
            CompileError::VariableAlreadyExists(name) => write!(f, "Variable '{}' already exists", name),
            CompileError::UninitializedVariable(name) => write!(f, "Variable '{}' is not initialized", name),
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
}

/// AST 编译器
pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,      // 局部变量表
    scope_depth: usize,      // 当前作用域深度
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    /// 编译整个模块（视为匿名函数体）
    /// 返回 (Chunk, 局部变量数量)
    pub fn compile(&mut self, module: &Module) -> Result<(Chunk, usize), CompileError> {
        for stmt in &module.statements {
            self.compile_stmt(stmt)?;
        }
        
        // 默认返回 null
        self.chunk.write_op(OpCode::LoadNull, 0);
        self.chunk.write_op(OpCode::Return, 0);
        
        Ok((self.chunk.clone(), self.locals.len()))
    }

    /// 编译语句
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
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
            
            _ => {
                return Err(CompileError::Unimplemented(format!(
                    "Statement type: {:?}",
                    std::mem::discriminant(stmt.as_ref())
                )));
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
            
            ExprKind::LiteralString(_) => {
                return Err(CompileError::Unimplemented("String literal".to_string()));
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
            
            ExprKind::LiteralList(_) => {
                return Err(CompileError::Unimplemented("List literal".to_string()));
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
                if let Some(idx) = self.resolve_local(&var_ref.name) {
                    self.emit_load_local(idx);
                } else {
                    return Err(CompileError::Unimplemented(
                        format!("Undefined variable: {}", var_ref.name)
                    ));
                }
            }
            
            ExprKind::FunctionCall(_) => {
                return Err(CompileError::Unimplemented("Function call".to_string()));
            }
            
            ExprKind::Assign(assign) => {
                // 编译右值
                self.compile_expr(&assign.value)?;
                
                // 存储到变量
                if let Some(idx) = self.resolve_local(&assign.name) {
                    self.emit_store_local(idx);
                    // 赋值表达式返回 null（语句级别的副作用）
                    self.chunk.write_op(OpCode::LoadNull, 0);
                } else {
                    return Err(CompileError::Unimplemented(
                        format!("Undefined variable: {}", assign.name)
                    ));
                }
            }
            
            ExprKind::Lambda(_) => {
                return Err(CompileError::Unimplemented("Lambda".to_string()));
            }
            
            ExprKind::MemberAccess(_) => {
                return Err(CompileError::Unimplemented("Member access".to_string()));
            }
        }
        Ok(())
    }

    /// 编译二元运算
    fn compile_binary(&mut self, bin: &Binary) -> Result<(), CompileError> {
        // 特殊处理赋值运算符：= 
        if bin.op == crate::compiler::lexer::token_kind::KauboTokenKind::Equal {
            return self.compile_assignment(&bin.left, &bin.right);
        }
        
        // 先编译左操作数
        self.compile_expr(&bin.left)?;
        
        // 再编译右操作数
        self.compile_expr(&bin.right)?;
        
        // 生成运算指令
        let op = match bin.op {
            crate::compiler::lexer::token_kind::KauboTokenKind::Plus => OpCode::Add,
            crate::compiler::lexer::token_kind::KauboTokenKind::Minus => OpCode::Sub,
            crate::compiler::lexer::token_kind::KauboTokenKind::Asterisk => OpCode::Mul,
            crate::compiler::lexer::token_kind::KauboTokenKind::Slash => OpCode::Div,
            crate::compiler::lexer::token_kind::KauboTokenKind::DoubleEqual => OpCode::Equal,
            crate::compiler::lexer::token_kind::KauboTokenKind::ExclamationEqual => OpCode::NotEqual,
            crate::compiler::lexer::token_kind::KauboTokenKind::GreaterThan => OpCode::Greater,
            crate::compiler::lexer::token_kind::KauboTokenKind::GreaterThanEqual => OpCode::GreaterEqual,
            crate::compiler::lexer::token_kind::KauboTokenKind::LessThan => OpCode::Less,
            crate::compiler::lexer::token_kind::KauboTokenKind::LessThanEqual => OpCode::LessEqual,
            _ => return Err(CompileError::InvalidOperator),
        };
        
        self.chunk.write_op(op, 0);
        Ok(())
    }
    
    /// 编译赋值表达式 (处理 Binary 形式的赋值)
    /// 赋值表达式返回 null（语句级别的副作用）
    fn compile_assignment(&mut self, left: &Expr, right: &Expr) -> Result<(), CompileError> {
        // 编译右值
        self.compile_expr(right)?;
        
        // 左值必须是变量引用
        match left.as_ref() {
            ExprKind::VarRef(var_ref) => {
                if let Some(idx) = self.resolve_local(&var_ref.name) {
                    self.emit_store_local(idx);
                    // 赋值表达式返回 null（不是被赋的值）
                    self.chunk.write_op(OpCode::LoadNull, 0);
                    Ok(())
                } else {
                    Err(CompileError::Unimplemented(
                        format!("Undefined variable: {}", var_ref.name)
                    ))
                }
            }
            _ => Err(CompileError::Unimplemented(
                "Left side of assignment must be a variable".to_string()
            ))
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
        });
        
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
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// 编译模块的便捷函数
/// 返回 (Chunk, 局部变量数量)
pub fn compile(module: &Module) -> Result<(Chunk, usize), CompileError> {
    let mut compiler = Compiler::new();
    compiler.compile(module)
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::builder::build_lexer;
    use crate::compiler::parser::parser::Parser;
    use crate::runtime::{VM, InterpretResult};

    fn compile_code(code: &str) -> Result<(Chunk, usize), CompileError> {
        let mut lexer = build_lexer();
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();
        
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().map_err(|e| {
            CompileError::Unimplemented(format!("Parse error: {:?}", e))
        })?;
        
        compile(&ast)
    }

    fn run_code(code: &str) -> Result<Value, String> {
        let (chunk, local_count) = compile_code(code)
            .map_err(|e| format!("Compile error: {:?}", e))?;
        let mut vm = VM::new();
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
        let (chunk, _) = compile_code("42;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_binary() {
        let (chunk, _) = compile_code("1 + 2;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_complex() {
        let (chunk, _) = compile_code("1 + 2 * 3;").unwrap();
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

    #[test]
    fn test_run_print() {
        // print 语句返回 null
        let result = run_code("print 42;").unwrap();
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
}

