//! AdaptiveParser trait and types
//!
//! AdaptiveParsers parse raw input data into initial IR format.
//! This is the entry point of the pipeline: RawData → IR

use crate::component::{Component, ComponentKind, ComponentMetadata, Capabilities};
use crate::loader::RawData;
use crate::error::AdaptiveParserError;
use serde::{Serialize, Deserialize};

// 引入 kaubo-core 类型用于 IR
use crate::vm::core::Chunk;
use crate::passes::parser::Module;

/// The format of data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// The AdaptiveParser trait for parsing raw data into initial IR
///
/// Implementors parse raw input data (from Loader) into the initial IR format.
/// This is the first stage of the pipeline: RawData → IR
pub trait AdaptiveParser: Component {
    /// Get the output IR format this parser produces
    fn output_format(&self) -> DataFormat;
    
    /// Parse raw data to IR
    ///
    /// # Arguments
    /// * `input` - The raw data loaded from source
    ///
    /// # Returns
    /// The parsed IR
    fn parse(&self, input: RawData) -> Result<IR, AdaptiveParserError>;
}

/// Helper methods for AdaptiveParsers
pub trait AdaptiveParserExt: AdaptiveParser {
    /// Check if this parser can produce the given format
    fn can_parse_to(&self, format: &DataFormat) -> bool {
        &self.output_format() == format
    }
}

impl<T: AdaptiveParser + ?Sized> AdaptiveParserExt for T {}

/// SourceParser - 将原始数据直接转换为 Source IR
/// 
/// 这是一个简单的适配器，不做实际解析，只是将 RawData 包装为 IR::Source
pub struct SourceParser;

impl SourceParser {
    /// Create a new SourceParser
    pub fn new() -> Self {
        Self
    }
}

impl Default for SourceParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for SourceParser {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "source_parser",
            "0.1.0",
            ComponentKind::AdaptiveParser,
            Some("将原始数据直接转换为 Source IR"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![], vec![DataFormat::Source])
    }
}

impl AdaptiveParser for SourceParser {
    fn output_format(&self) -> DataFormat {
        DataFormat::Source
    }

    fn parse(&self, input: RawData) -> Result<IR, AdaptiveParserError> {
        // 直接将 RawData 转换为 IR::Source
        let text = match input {
            RawData::Text(s) => s,
            RawData::Binary(b) => String::from_utf8_lossy(&b).to_string(),
        };
        Ok(IR::Source(text))
    }
}

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
