//! Pipeline execution engine
//!
//! This module provides the pipeline engine that orchestrates the execution
//! of loaders, converters, passes, and emitters.

use crate::registry::{LoaderRegistry, ConverterRegistry, PassRegistry, EmitterRegistry};
use crate::loader::{Source, RawData};
use crate::converter::{Converter, IR, DataFormat};
use crate::pass::{Pass, PassContext, Input as PassInput, Output as PassOutput, PassOptions};
use crate::emitter::{Emitter, Target, SerializedOutput};
use crate::error::{OrchestratorError, LoaderError, ConverterError, PassError, EmitterError};
use crate::context::Context;
use crate::output::{OutputHandle, new_output_buffer, OutputEntry};
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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

/// Pipeline engine
pub struct PipelineEngine {
    config: Arc<kaubo_config::VmConfig>,
    vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
    log: Arc<kaubo_log::Logger>,
    output: OutputHandle,
}

impl std::fmt::Debug for PipelineEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineEngine")
            .field("config", &self.config)
            .field("has_output", &!self.output.is_empty())
            .finish_non_exhaustive()
    }
}

impl PipelineEngine {
    /// Create a new pipeline engine
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
        }
    }
    
    /// Create with custom output buffer
    pub fn with_output(
        config: Arc<kaubo_config::VmConfig>,
        vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
        log: Arc<kaubo_log::Logger>,
        output: OutputHandle,
    ) -> Self {
        Self { config, vfs, log, output }
    }
    
    /// Execute the pipeline
    pub fn execute(
        &self,
        request: ExecutionRequest,
        loaders: &LoaderRegistry,
        converters: &ConverterRegistry,
        passes: &PassRegistry,
        emitters: &EmitterRegistry,
    ) -> Result<ExecutionResult, OrchestratorError> {
        let start_time = std::time::Instant::now();
        
        // Step 1: Load input
        let raw_data = self.load(&request.source, loaders)?;
        
        // Step 2: Convert to initial IR
        let mut ir = self.convert(raw_data, &request.from, converters)?;
        
        // Step 3: Run passes
        ir = self.transform(ir, &request.from, &request.to, passes, &request.options)?;
        
        // Step 4: Emit output
        let serialized = self.emit(ir.clone(), &request.to, &request.target, emitters)?;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Collect output entries
        let output_entries = self.output.drain();
        
        Ok(ExecutionResult {
            final_ir: Some(ir),
            serialized,
            output_entries,
            metadata: HashMap::new(),
            execution_time_ms,
        })
    }
    
    /// Load data from source
    fn load(
        &self,
        source: &Source,
        loaders: &LoaderRegistry,
    ) -> Result<RawData, OrchestratorError> {
        // Find appropriate loader
        let loader_name = match source.kind {
            crate::loader::SourceKind::File => "file",
            crate::loader::SourceKind::Stdin => "stdin",
            crate::loader::SourceKind::Memory => "memory",
            crate::loader::SourceKind::Network => "network",
        };
        
        let loader = loaders.get(loader_name)
            .ok_or_else(|| OrchestratorError::ComponentNotFound {
                kind: "loader".to_string(),
                name: loader_name.to_string(),
            })?;
        
        loader.load(source)
            .map_err(|e| OrchestratorError::LoaderError {
                name: loader_name.to_string(),
                source: e,
            })
    }
    
    /// Convert raw data to IR
    fn convert(
        &self,
        raw: RawData,
        target_format: &DataFormat,
        converters: &ConverterRegistry,
    ) -> Result<IR, OrchestratorError> {
        // Find converter that can produce target format
        let converter = converters.all()
            .find(|c| c.output_format() == *target_format)
            .ok_or_else(|| OrchestratorError::PipelineError {
                message: format!("No converter found for format: {:?}", target_format),
            })?;
        
        converter.convert_raw(raw)
            .map_err(|e| OrchestratorError::ConverterError {
                name: converter.metadata().name.to_string(),
                source: e,
            })
    }
    
    /// Transform IR through passes
    fn transform(
        &self,
        initial_ir: IR,
        from: &DataFormat,
        to: &DataFormat,
        passes: &PassRegistry,
        options: &PassOptions,
    ) -> Result<IR, OrchestratorError> {
        let mut current_ir = initial_ir;
        let mut current_format = from.clone();
        
        // Build pass chain
        let pass_chain = self.build_pass_chain(from, to, passes)?;
        
        // Create pass context with output buffer
        let ctx = PassContext::with_output(
            self.config.clone(),
            self.vfs.clone(),
            self.log.clone(),
            self.output.clone(),
        ).with_options(options.clone());
        
        // Execute each pass
        for pass in pass_chain {
            let input = PassInput::new(current_ir);
            let output = pass.run(input, &ctx)
                .map_err(|e| OrchestratorError::PassError {
                    name: pass.metadata().name.to_string(),
                    source: e,
                })?;
            
            current_ir = output.data;
            current_format = pass.output_format();
        }
        
        // Verify final format
        if &current_format != to {
            return Err(OrchestratorError::IncompleteChain {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        
        Ok(current_ir)
    }
    
    /// Build the chain of passes from `from` to `to`
    fn build_pass_chain<'a>(
        &'a self,
        from: &DataFormat,
        to: &DataFormat,
        passes: &'a PassRegistry,
    ) -> Result<Vec<&'a dyn Pass>, OrchestratorError> {
        let mut chain = Vec::new();
        let mut current = from.clone();
        
        // Simple greedy algorithm - find passes that can progress toward target
        while &current != to {
            let pass = passes.all()
                .find(|p| p.input_format() == current)
                .ok_or_else(|| OrchestratorError::IncompleteChain {
                    from: from.to_string(),
                    to: to.to_string(),
                })?;
            
            current = pass.output_format();
            chain.push(pass);
        }
        
        Ok(chain)
    }
    
    /// Emit IR to target
    fn emit(
        &self,
        ir: IR,
        format: &DataFormat,
        target: &Target,
        emitters: &EmitterRegistry,
    ) -> Result<Option<SerializedOutput>, OrchestratorError> {
        // Find emitter for this format
        let emitter = emitters.all()
            .find(|e| e.format() == format.to_string())
            .ok_or_else(|| OrchestratorError::ComponentNotFound {
                kind: "emitter".to_string(),
                name: format.to_string(),
            })?;
        
        let output = PassOutput::new(ir);
        
        emitter.emit(&output, target)
            .map_err(|e| OrchestratorError::EmitterError {
                name: emitter.metadata().name.to_string(),
                source: e,
            })?;
        
        // Also return serialized data
        let serialized = emitter.serialize(&output)
            .map_err(|e| OrchestratorError::EmitterError {
                name: emitter.metadata().name.to_string(),
                source: e,
            })?;
        
        Ok(Some(serialized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // Use the existing types
    use crate::loader::Source;
    use crate::emitter::TargetKind;
}
