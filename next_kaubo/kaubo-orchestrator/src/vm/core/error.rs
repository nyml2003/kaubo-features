//! 错误类型 (Core 层)
//!
//! 由于错误类型依赖 lexer 的位置类型，这里先导出 InterpretResult
//! 其他错误类型保持原位，由上层模块重新导出

pub use super::vm::InterpretResult;

/// 统一的运行时错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// 类型错误
    TypeError(String),
    /// 未定义变量
    UndefinedVariable(String),
    /// 索引越界
    IndexOutOfBounds,
    /// 除零错误
    DivisionByZero,
    /// 栈溢出
    StackOverflow,
    /// 其他错误
    Other(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::TypeError(msg) => write!(f, "TypeError: {msg}"),
            RuntimeError::UndefinedVariable(name) => write!(f, "UndefinedVariable: {name}"),
            RuntimeError::IndexOutOfBounds => write!(f, "IndexOutOfBounds"),
            RuntimeError::DivisionByZero => write!(f, "DivisionByZero"),
            RuntimeError::StackOverflow => write!(f, "StackOverflow"),
            RuntimeError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}
