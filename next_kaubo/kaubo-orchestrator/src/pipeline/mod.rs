//! Pass implementations

pub mod codegen;
pub mod lexer;
pub mod module;
pub mod parser;

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::adaptive_parser::{DataFormat, IR};
use crate::emitter::{SerializedOutput, Target};
use crate::error::PassError;
use crate::loader::Source;
use crate::pass::{Input, Output, Pass, PassContext, PassOptions};
use crate::output::OutputEntry;
use std::collections::HashMap;

/// Execution request for the pipeline
pub struct ExecutionRequest {
    /// Source to load from
    pub source: Source,
    /// Target to emit to
    pub target: Target,
    /// Source data format
    pub from: DataFormat,
    /// Target data format
    pub to: DataFormat,
    /// Pass options
    pub options: PassOptions,
    /// Preferred pass name (optional)
    pub preferred_pass: Option<String>,
}

impl ExecutionRequest {
    /// Create a new execution request with the given source
    pub fn new(source: Source) -> Self {
        Self {
            source,
            target: Target::memory(),
            from: DataFormat::Source,
            to: DataFormat::Bytecode,
            options: PassOptions::default(),
            preferred_pass: None,
        }
    }
    
    /// Set the source and target formats
    pub fn from_to(mut self, from: DataFormat, to: DataFormat) -> Self {
        self.from = from;
        self.to = to;
        self
    }
    
    /// Set the target
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }
    
    /// Set pass options
    pub fn with_options(mut self, options: PassOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Set preferred pass name
    pub fn with_preferred_pass(mut self, name: impl Into<String>) -> Self {
        self.preferred_pass = Some(name.into());
        self
    }
}

/// Execution result from the pipeline
pub struct ExecutionResult {
    /// Final IR after all transformations
    pub final_ir: Option<IR>,
    /// Serialized output
    pub serialized: Option<SerializedOutput>,
    /// Output entries (from print/show_source etc.)
    pub output_entries: Vec<OutputEntry>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// A simple pass that just passes through the input (for testing)
pub struct NoOpPass {
    name: &'static str,
    input_format: DataFormat,
    output_format: DataFormat,
}

impl NoOpPass {
    pub fn new(name: &'static str, input_format: DataFormat, output_format: DataFormat) -> Self {
        Self {
            name,
            input_format,
            output_format,
        }
    }
}

impl Component for NoOpPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            self.name,
            "0.1.0",
            ComponentKind::Pass,
            Some("No-op pass for testing"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![self.input_format.clone()], vec![self.output_format.clone()])
    }
}

impl Pass for NoOpPass {
    fn input_format(&self) -> DataFormat {
        self.input_format.clone()
    }

    fn output_format(&self) -> DataFormat {
        self.output_format.clone()
    }

    fn run(&self, input: Input, _ctx: &PassContext) -> Result<Output, PassError> {
        Ok(Output::new(input.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_pass() {
        let pass = NoOpPass::new("test", DataFormat::Source, DataFormat::Source);

        assert_eq!(pass.metadata().name, "test");
        assert_eq!(pass.input_format(), DataFormat::Source);
        assert_eq!(pass.output_format(), DataFormat::Source);
    }
}
