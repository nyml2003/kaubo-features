//! Pipeline types
//!
//! This module provides types for pipeline execution requests and results.
//! The actual execution logic is now in the Orchestrator.

use crate::loader::Source;
use crate::adaptive_parser::{IR, DataFormat};
use crate::emitter::{Target, SerializedOutput};
use crate::output::OutputEntry;
use crate::pass::PassOptions;
use std::collections::HashMap;

/// Execution request
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    /// Input source
    pub source: Source,
    /// Pipeline start stage
    pub from: DataFormat,
    /// Pipeline end stage
    pub to: DataFormat,
    /// Output target
    pub target: Target,
    /// Pass options
    pub options: PassOptions,
    /// Preferred pass name (for selecting between multiple passes with same input/output)
    pub preferred_pass: Option<String>,
}

impl ExecutionRequest {
    /// Create a new execution request
    pub fn new(source: Source) -> Self {
        Self {
            source,
            from: DataFormat::Source,
            to: DataFormat::Result,
            target: Target::stdout(),
            options: PassOptions::default(),
            preferred_pass: None,
        }
    }
    
    /// Set pipeline range
    pub fn from_to(mut self, from: DataFormat, to: DataFormat) -> Self {
        self.from = from;
        self.to = to;
        self
    }
    
    /// Set output target
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = target;
        self
    }
    
    /// Set pass options
    pub fn with_options(mut self, options: PassOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Set preferred pass name (for selecting specific pass when multiple match)
    pub fn with_preferred_pass(mut self, pass_name: impl Into<String>) -> Self {
        self.preferred_pass = Some(pass_name.into());
        self
    }
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final IR (if applicable)
    pub final_ir: Option<IR>,
    /// Serialized output (if emitted)
    pub serialized: Option<SerializedOutput>,
    /// Captured output entries (print, show_source, etc.)
    pub output_entries: Vec<OutputEntry>,
    /// Execution metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::Source;
    use crate::emitter::TargetKind;

    #[test]
    fn test_execution_request() {
        let source = Source::file("test.kaubo");
        let request = ExecutionRequest::new(source)
            .from_to(DataFormat::Source, DataFormat::Ast)
            .with_target(Target::stdout());
        
        assert_eq!(request.from, DataFormat::Source);
        assert_eq!(request.to, DataFormat::Ast);
        assert_eq!(request.target.kind, TargetKind::Stdout);
    }
}
