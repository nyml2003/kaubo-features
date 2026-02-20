//! 编译错误定义

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
                write!(f, "Variable '{name}' already exists")
            }
            CompileError::UninitializedVariable(name) => {
                write!(f, "Variable '{name}' is not initialized")
            }
            CompileError::Unimplemented(s) => write!(f, "Unimplemented: {s}"),
        }
    }
}
