//! 错误类型 (Core 层)

pub use crate::vm::InterpretResult;

/// 源码位置信息
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    /// 字节码偏移
    pub ip_offset: usize,
    /// 源码行号
    pub source_line: Option<u32>,
    /// 函数名
    pub function_name: Option<String>,
}

/// 统一的运行时错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// 类型错误
    TypeError { message: String, location: Option<SourceLocation> },
    /// 未定义变量
    UndefinedVariable { name: String, location: Option<SourceLocation> },
    /// 索引越界
    IndexOutOfBounds { location: Option<SourceLocation> },
    /// 除零错误
    DivisionByZero { location: Option<SourceLocation> },
    /// 栈溢出
    StackOverflow { location: Option<SourceLocation> },
    /// 栈下溢
    StackUnderflow { location: Option<SourceLocation> },
    /// 无效操作数
    InvalidOperand { message: String, location: Option<SourceLocation> },
    /// 协程 yield 信号
    Yield,
    /// 其他错误
    Other { message: String, location: Option<SourceLocation> },
}

impl RuntimeError {
    /// 创建不带位置信息的错误
    pub fn type_error(msg: impl Into<String>) -> Self {
        RuntimeError::TypeError { message: msg.into(), location: None }
    }

    pub fn undefined_variable(name: impl Into<String>) -> Self {
        RuntimeError::UndefinedVariable { name: name.into(), location: None }
    }

    pub fn index_out_of_bounds() -> Self {
        RuntimeError::IndexOutOfBounds { location: None }
    }

    pub fn division_by_zero() -> Self {
        RuntimeError::DivisionByZero { location: None }
    }

    pub fn stack_overflow() -> Self {
        RuntimeError::StackOverflow { location: None }
    }

    pub fn stack_underflow() -> Self {
        RuntimeError::StackUnderflow { location: None }
    }

    pub fn invalid_operand(msg: impl Into<String>) -> Self {
        RuntimeError::InvalidOperand { message: msg.into(), location: None }
    }

    pub fn other(msg: impl Into<String>) -> Self {
        RuntimeError::Other { message: msg.into(), location: None }
    }

    pub fn r#yield() -> Self {
        RuntimeError::Yield
    }

    /// 获取错误消息
    pub fn message(&self) -> &str {
        match self {
            RuntimeError::TypeError { message, .. } => message,
            RuntimeError::UndefinedVariable { name, .. } => name,
            RuntimeError::IndexOutOfBounds { .. } => "IndexOutOfBounds",
            RuntimeError::DivisionByZero { .. } => "DivisionByZero",
            RuntimeError::StackOverflow { .. } => "StackOverflow",
            RuntimeError::StackUnderflow { .. } => "StackUnderflow",
            RuntimeError::InvalidOperand { message, .. } => message,
            RuntimeError::Yield => "yield",
            RuntimeError::Other { message, .. } => message,
        }
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::TypeError { message, location } => {
                write!(f, "TypeError: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::UndefinedVariable { name, location } => {
                write!(f, "UndefinedVariable: {name}")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::IndexOutOfBounds { location } => {
                write!(f, "IndexOutOfBounds")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::DivisionByZero { location } => {
                write!(f, "DivisionByZero")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::StackOverflow { location } => {
                write!(f, "StackOverflow")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::StackUnderflow { location } => {
                write!(f, "StackUnderflow")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::InvalidOperand { message, location } => {
                write!(f, "InvalidOperand: {message}")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
            RuntimeError::Yield => write!(f, "yield"),
            RuntimeError::Other { message, location } => {
                write!(f, "{message}")?;
                if let Some(loc) = location {
                    write!(f, " at offset {}", loc.ip_offset)?;
                    if let Some(line) = loc.source_line {
                        write!(f, " line {line}")?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RuntimeError {}
