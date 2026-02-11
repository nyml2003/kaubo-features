//! 测试辅助工具
//!
//! 提供端到端测试的辅助函数

use next_kaubo::core::compiler::lexer::builder::build_lexer;
use next_kaubo::core::compiler::parser::parser::Parser;
use next_kaubo::core::runtime::{compile, InterpretResult, VM};

/// 执行 Kaubo 代码并返回结果
///
/// # Example
/// ```
/// let output = run_code("var x = 5; return x;");
/// assert!(output.is_ok());
/// ```
pub fn run_code(code: &str) -> Result<ExecResult, ExecError> {
    // 词法分析
    let mut lexer = build_lexer();
    let _ = lexer.feed(&code.as_bytes().to_vec());
    let _ = lexer.terminate();

    // 语法分析
    let mut parser = Parser::new(lexer);
    let ast = parser
        .parse()
        .map_err(|e| ExecError::Parser(format!("{:?}", e)))?;

    // 编译
    let (chunk, local_count) = compile(&ast).map_err(|e| ExecError::Compiler(format!("{:?}", e)))?;

    // 执行
    let mut vm = VM::new();
    let result = vm.interpret_with_locals(&chunk, local_count);

    match result {
        InterpretResult::Ok => {
            let return_value = vm.stack_top();
            Ok(ExecResult {
                return_value,
                output: String::new(), // TODO: 捕获 stdout
            })
        }
        InterpretResult::RuntimeError(msg) => Err(ExecError::Runtime(msg)),
        InterpretResult::CompileError(msg) => Err(ExecError::Compiler(msg)),
    }
}

/// 执行结果
#[derive(Debug)]
pub struct ExecResult {
    /// 返回值
    pub return_value: Option<next_kaubo::Value>,
    /// 标准输出
    pub output: String,
}

/// 执行错误
#[derive(Debug)]
pub enum ExecError {
    Lexer(String),
    Parser(String),
    Compiler(String),
    Runtime(String),
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::Lexer(msg) => write!(f, "Lexer error: {}", msg),
            ExecError::Parser(msg) => write!(f, "Parser error: {}", msg),
            ExecError::Compiler(msg) => write!(f, "Compiler error: {}", msg),
            ExecError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for ExecError {}

/// 获取整数值
pub fn get_int(result: &ExecResult) -> Option<i32> {
    result.return_value.as_ref()?.as_int()
}

/// 获取浮点数值
pub fn get_float(result: &ExecResult) -> Option<f64> {
    Some(result.return_value.as_ref()?.as_float())
}

/// 获取字符串值
pub fn get_string(result: &ExecResult) -> Option<String> {
    let ptr = result.return_value.as_ref()?.as_string()?;
    unsafe { Some((&*ptr).chars.clone()) }
}
