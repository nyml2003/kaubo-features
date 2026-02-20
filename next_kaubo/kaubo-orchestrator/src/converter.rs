//! Converter trait and types
//!
//! Converters transform data between different formats,
//! such as parsing source code into AST, or deserializing JSON.

use crate::component::{Component, ComponentKind, ComponentMetadata, Capabilities};
use crate::loader::RawData;
use crate::error::ConverterError;
use serde::{Serialize, Deserialize};

// 引入 kaubo-core 类型用于 IR
use crate::vm::core::Chunk;
use crate::passes::parser::Module;

/// The format of data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataFormat {
    /// Raw source text
    Source,
    /// Token stream
    Tokens,
    /// Abstract Syntax Tree (AST)
    Ast,
    /// Typed AST
    TypedAst,
    /// Bytecode
    Bytecode,
    /// Execution result
    Result,
    /// JSON format
    Json,
    /// Binary format
    Binary,
    /// Text format
    Text,
    /// Custom format with name
    Custom(String),
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFormat::Source => write!(f, "source"),
            DataFormat::Tokens => write!(f, "tokens"),
            DataFormat::Ast => write!(f, "ast"),
            DataFormat::TypedAst => write!(f, "typed_ast"),
            DataFormat::Bytecode => write!(f, "bytecode"),
            DataFormat::Result => write!(f, "result"),
            DataFormat::Json => write!(f, "json"),
            DataFormat::Binary => write!(f, "binary"),
            DataFormat::Text => write!(f, "text"),
            DataFormat::Custom(name) => write!(f, "{}", name),
        }
    }
}

use std::fmt;

/// Intermediate representation (IR) data
#[derive(Debug, Clone)]
pub enum IR {
    /// Source code text
    Source(String),
    /// Token stream
    Tokens(Vec<Token>),
    /// Abstract Syntax Tree (使用 kaubo-core 的 Module)
    Ast(Module),
    /// Typed AST
    TypedAst(TypedAstNode),
    /// Bytecode (使用 kaubo-core 的 Chunk)
    Bytecode(Chunk),
    /// Execution result
    Result(ExecutionResult),
}

/// Token representation (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub kind: String,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

/// AST node representation (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub kind: String,
    pub children: Vec<AstNode>,
    pub value: Option<String>,
}

/// Typed AST node representation (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedAstNode {
    pub kind: String,
    pub ty: Type,
    pub children: Vec<TypedAstNode>,
}

/// Type representation (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Type {
    pub name: String,
}

/// Bytecode representation (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bytecode {
    pub instructions: Vec<u8>,
    pub constants: Vec<Constant>,
}

/// Constant value (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Int(i64),
    Float(f64),
    String(String),
}

/// Execution result (placeholder)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// The Converter trait for transforming data between formats
///
/// Implementors convert raw data or IR from one format to another.
pub trait Converter: Component {
    /// Get the input format this converter accepts
    fn input_format(&self) -> DataFormat;
    
    /// Get the output format this converter produces
    fn output_format(&self) -> DataFormat;
    
    /// Convert raw data to IR
    ///
    /// Default implementation returns an error.
    /// Override this if your converter accepts raw data.
    fn convert_raw(&self, _input: RawData) -> Result<IR, ConverterError> {
        Err(ConverterError::InvalidInputFormat {
            expected: "IR".to_string(),
            actual: "RawData".to_string(),
        })
    }
    
    /// Convert IR to IR
    ///
    /// Default implementation returns an error.
    /// Override this if your converter transforms IR.
    fn convert_ir(&self, _input: IR) -> Result<IR, ConverterError> {
        Err(ConverterError::InvalidInputFormat {
            expected: "RawData".to_string(),
            actual: "IR".to_string(),
        })
    }
}

/// Helper methods for Converters
pub trait ConverterExt: Converter {
    /// Check if this converter can convert from the given format
    fn can_convert_from(&self, format: &DataFormat) -> bool {
        &self.input_format() == format
    }
    
    /// Check if this converter can convert to the given format
    fn can_convert_to(&self, format: &DataFormat) -> bool {
        &self.output_format() == format
    }
}

impl<T: Converter + ?Sized> ConverterExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_format_display() {
        assert_eq!(DataFormat::Source.to_string(), "source");
        assert_eq!(DataFormat::Ast.to_string(), "ast");
        assert_eq!(DataFormat::Custom("myformat".to_string()).to_string(), "myformat");
    }

    #[test]
    fn test_ir_creation() {
        let source = IR::Source("print(1)".to_string());
        match source {
            IR::Source(s) => assert_eq!(s, "print(1)"),
            _ => panic!("Expected Source variant"),
        }
    }
}
