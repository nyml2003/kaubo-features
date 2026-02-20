//! Pass trait and types
//!
//! Passes are compilation stages that transform intermediate representation (IR).
//! Examples include parser, type checker, and code generator.

use crate::component::Component;
use crate::adaptive_parser::{IR, DataFormat};
use crate::error::PassError;
use crate::output::{OutputHandle, new_output_buffer};
use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;

/// Input to a pass
#[derive(Debug, Clone)]
pub struct Input {
    /// The IR data
    pub data: IR,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl Input {
    /// Create new input from IR
    pub fn new(data: IR) -> Self {
        Self {
            data,
            metadata: HashMap::new(),
        }
    }
    
    /// Create input with metadata
    pub fn with_metadata(data: IR, metadata: HashMap<String, Value>) -> Self {
        Self { data, metadata }
    }
    
    /// Get the IR format
    pub fn format(&self) -> DataFormat {
        match &self.data {
            IR::Source(_) => DataFormat::Source,
            IR::Tokens(_) => DataFormat::Tokens,
            IR::Ast(_) => DataFormat::Ast,
            IR::TypedAst(_) => DataFormat::TypedAst,
            IR::Bytecode(_) => DataFormat::Bytecode,
            IR::Result(_) => DataFormat::Result,
        }
    }
    
    /// Try to get as source
    pub fn as_source(&self) -> Result<&String, PassError> {
        match &self.data {
            IR::Source(s) => Ok(s),
            _ => Err(PassError::InvalidInput {
                message: format!("Expected Source, got {:?}", self.format()),
            }),
        }
    }
    
    /// Try to get as tokens
    pub fn as_tokens(&self) -> Result<&Vec<crate::adaptive_parser::Token>, PassError> {
        match &self.data {
            IR::Tokens(t) => Ok(t),
            _ => Err(PassError::InvalidInput {
                message: format!("Expected Tokens, got {:?}", self.format()),
            }),
        }
    }
    
    /// Try to get as AST (kaubo-core Module)
    pub fn as_ast(&self) -> Result<&crate::passes::parser::Module, PassError> {
        match &self.data {
            IR::Ast(a) => Ok(a),
            _ => Err(PassError::InvalidInput {
                message: format!("Expected Ast, got {:?}", self.format()),
            }),
        }
    }
    
    /// Try to get as Typed AST
    pub fn as_typed_ast(&self) -> Result<&crate::adaptive_parser::TypedAstNode, PassError> {
        match &self.data {
            IR::TypedAst(t) => Ok(t),
            _ => Err(PassError::InvalidInput {
                message: format!("Expected TypedAst, got {:?}", self.format()),
            }),
        }
    }
    
    /// Try to get as Bytecode (kaubo-core Chunk)
    pub fn as_bytecode(&self) -> Result<&crate::vm::core::Chunk, PassError> {
        match &self.data {
            IR::Bytecode(b) => Ok(b),
            _ => Err(PassError::InvalidInput {
                message: format!("Expected Bytecode, got {:?}", self.format()),
            }),
        }
    }
}

/// Output from a pass
#[derive(Debug, Clone)]
pub struct Output {
    /// The IR data
    pub data: IR,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl Output {
    /// Create new output from IR
    pub fn new(data: IR) -> Self {
        Self {
            data,
            metadata: HashMap::new(),
        }
    }
    
    /// Get the IR format
    pub fn format(&self) -> DataFormat {
        match &self.data {
            IR::Source(_) => DataFormat::Source,
            IR::Tokens(_) => DataFormat::Tokens,
            IR::Ast(_) => DataFormat::Ast,
            IR::TypedAst(_) => DataFormat::TypedAst,
            IR::Bytecode(_) => DataFormat::Bytecode,
            IR::Result(_) => DataFormat::Result,
        }
    }
}

impl From<IR> for Output {
    fn from(data: IR) -> Self {
        Self::new(data)
    }
}

/// Context for pass execution
pub struct PassContext {
    /// The configuration
    pub config: Arc<kaubo_config::VmConfig>,
    /// Virtual file system
    pub vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
    /// Logger
    pub log: Arc<kaubo_log::Logger>,
    /// Output buffer for capturing print/show_source etc.
    pub output: OutputHandle,
    /// Compilation options
    pub options: PassOptions,
    /// Previous pass metadata
    pub previous_metadata: HashMap<String, Value>,
}

impl std::fmt::Debug for PassContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassContext")
            .field("config", &self.config)
            .field("options", &self.options)
            .field("previous_metadata", &self.previous_metadata)
            .field("has_output", &!self.output.is_empty())
            .finish_non_exhaustive()
    }
}

impl Clone for PassContext {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            vfs: self.vfs.clone(),
            log: self.log.clone(),
            output: self.output.clone(),
            options: self.options.clone(),
            previous_metadata: self.previous_metadata.clone(),
        }
    }
}

/// Options for pass execution
#[derive(Debug, Clone, Default)]
pub struct PassOptions {
    /// Whether to enable optimizations
    pub optimize: bool,
    /// Debug level
    pub debug: bool,
    /// Target platform
    pub target: String,
}

impl PassContext {
    /// Create a new pass context
    pub fn new(
        config: Arc<kaubo_config::VmConfig>,
        vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
        log: Arc<kaubo_log::Logger>,
    ) -> Self {
        Self {
            config,
            vfs,
            log,
            output: new_output_buffer(),
            options: PassOptions::default(),
            previous_metadata: HashMap::new(),
        }
    }
    
    /// Create a new pass context with custom output buffer
    pub fn with_output(
        config: Arc<kaubo_config::VmConfig>,
        vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
        log: Arc<kaubo_log::Logger>,
        output: OutputHandle,
    ) -> Self {
        Self {
            config,
            vfs,
            log,
            output,
            options: PassOptions::default(),
            previous_metadata: HashMap::new(),
        }
    }
    
    /// Set options
    pub fn with_options(mut self, options: PassOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Get a config value
    pub fn config_val(&self, _key: &str) -> Option<&Value> {
        // TODO: implement config access
        None
    }
}

/// The Pass trait for compilation stages
///
/// Passes transform intermediate representation (IR) from one form to another.
/// Examples: lexer, parser, type checker, code generator.
pub trait Pass: Component {
    /// Get the input IR format
    fn input_format(&self) -> DataFormat;
    
    /// Get the output IR format
    fn output_format(&self) -> DataFormat;
    
    /// Execute the pass
    ///
    /// # Arguments
    /// * `input` - The input IR and metadata
    /// * `ctx` - The execution context
    ///
    /// # Returns
    /// The output IR and metadata
    fn run(&self, input: Input, ctx: &PassContext) -> Result<Output, PassError>;
}

/// Helper methods for Passes
pub trait PassExt: Pass {
    /// Check if this pass can process the given format
    fn accepts(&self, format: &DataFormat) -> bool {
        &self.input_format() == format
    }
    
    /// Get the pass name
    fn pass_name(&self) -> &'static str {
        self.metadata().name
    }
}

impl<T: Pass + ?Sized> PassExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_creation() {
        let input = Input::new(IR::Source("test".to_string()));
        assert!(matches!(input.data, IR::Source(_)));
        assert_eq!(input.format(), DataFormat::Source);
    }

    #[test]
    fn test_input_as_source() {
        let input = Input::new(IR::Source("hello".to_string()));
        assert_eq!(input.as_source().unwrap(), "hello");
        
        // 测试非 Source 类型的输入返回错误
        use crate::passes::parser::ModuleKind;
        let module = Box::new(ModuleKind { statements: vec![] });
        let input = Input::new(IR::Ast(module));
        assert!(input.as_source().is_err());
    }

    #[test]
    fn test_output_creation() {
        let output: Output = IR::Source("result".to_string()).into();
        assert!(matches!(output.data, IR::Source(_)));
    }
}
