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
    Unimplemented(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::InvalidOperator => write!(f, "Invalid operator"),
            CompileError::TooManyConstants => write!(f, "Too many constants in one chunk"),
            CompileError::Unimplemented(s) => write!(f, "Unimplemented: {}", s),
        }
    }
}

/// AST 编译器
pub struct Compiler {
    chunk: Chunk,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
        }
    }

    /// 编译整个模块（视为匿名函数体）
    pub fn compile(&mut self, module: &Module) -> Result<Chunk, CompileError> {
        for stmt in &module.statements {
            self.compile_stmt(stmt)?;
        }
        
        // 默认返回 null
        self.chunk.write_op(OpCode::LoadNull, 0);
        self.chunk.write_op(OpCode::Return, 0);
        
        Ok(self.chunk.clone())
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
                self.compile_expr(&decl.initializer)?;
                // TODO: 存储到局部变量
                self.chunk.write_op(OpCode::Pop, 0);
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
            
            ExprKind::VarRef(_) => {
                return Err(CompileError::Unimplemented("Variable reference".to_string()));
            }
            
            ExprKind::FunctionCall(_) => {
                return Err(CompileError::Unimplemented("Function call".to_string()));
            }
            
            ExprKind::Assign(_) => {
                return Err(CompileError::Unimplemented("Assignment".to_string()));
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
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// 编译模块的便捷函数
pub fn compile(module: &Module) -> Result<Chunk, CompileError> {
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

    fn compile_code(code: &str) -> Result<Chunk, CompileError> {
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
        let chunk = compile_code(code).map_err(|e| format!("Compile error: {:?}", e))?;
        let mut vm = VM::new();
        let result = vm.interpret(&chunk);
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
        let chunk = compile_code("42;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_binary() {
        let chunk = compile_code("1 + 2;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_complex() {
        let chunk = compile_code("1 + 2 * 3;").unwrap();
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
}

