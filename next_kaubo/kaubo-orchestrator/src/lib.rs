//! Kaubo Orchestrator
//! 
//! Pipeline execution engine and component management.

pub mod component;
pub mod loader;
pub mod adaptive_parser;
pub mod pass;
pub mod emitter;
pub mod registry;
pub mod pipeline;
pub mod context;
pub mod error;
pub mod output;

// Component implementations
pub mod adapters;
pub mod loaders;
pub mod emitters;
pub mod passes;

// VM runtime and kit (migrated from kaubo-core)
pub mod vm;
pub mod kit;

pub use component::{Component, ComponentKind, ComponentMetadata, Capabilities};
pub use loader::{Loader, Source, SourceKind, RawData};
pub use adaptive_parser::{AdaptiveParser, IR, DataFormat};
pub use pass::{Pass, PassContext, Input as PassInput, Output as PassOutput, PassOptions};
pub use emitter::{Emitter, Target, TargetKind, SerializedOutput};
pub use registry::{LoaderRegistry, AdaptiveParserRegistry, PassRegistry, EmitterRegistry};
pub use pipeline::{ExecutionRequest, ExecutionResult};
pub use context::Context;
pub use error::{OrchestratorError, PassError, LoaderError, AdaptiveParserError, EmitterError};
pub use output::{OutputHandle, OutputEntry, OutputBuffer, new_output_buffer};

// Re-export component implementations
pub use adapters::{CodeGenPass, CompilePass, MultiModulePass, ParserPass};
pub use loaders::FileLoader;
pub use emitters::{BytecodeEmitter, FileEmitter, StdoutEmitter};
pub use passes::NoOpPass;
pub use adaptive_parser::SourceParser;

use std::sync::Arc;
use std::collections::HashSet;
use kaubo_config::VmConfig;
use kaubo_vfs::{VirtualFileSystem, MemoryFileSystem};
use kaubo_log::Logger;

/// The main orchestrator that manages components and executes pipelines
/// 
/// Orchestrator is the central coordinator that owns all component registries
/// and executes the full pipeline: Load → Parse → Transform → Emit
pub struct Orchestrator {
    config: Arc<VmConfig>,
    vfs: Arc<dyn VirtualFileSystem + Send + Sync>,
    log: Arc<Logger>,
    output: OutputHandle,
    
    // Component registries
    loaders: LoaderRegistry,
    adaptive_parsers: AdaptiveParserRegistry,
    passes: PassRegistry,
    emitters: EmitterRegistry,
}

impl std::fmt::Debug for Orchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Orchestrator")
            .field("loaders", &self.loaders)
            .field("adaptive_parsers", &self.adaptive_parsers)
            .field("passes", &self.passes)
            .field("emitters", &self.emitters)
            .finish()
    }
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration
    pub fn new(config: VmConfig) -> Self {
        let vfs: Arc<dyn VirtualFileSystem + Send + Sync> = Arc::new(MemoryFileSystem::new());
        let log: Arc<Logger> = Logger::new(kaubo_log::Level::Info);
        let config_arc = Arc::new(config);
        let output = new_output_buffer();
        
        Self {
            config: config_arc,
            vfs,
            log,
            output,
            loaders: LoaderRegistry::new(),
            adaptive_parsers: AdaptiveParserRegistry::new(),
            passes: PassRegistry::new(),
            emitters: EmitterRegistry::new(),
        }
    }
    
    /// Create with custom VFS
    pub fn with_vfs<VFS: VirtualFileSystem + Send + Sync + 'static>(
        config: VmConfig,
        vfs: Arc<VFS>,
    ) -> Self {
        let log: Arc<Logger> = Logger::new(kaubo_log::Level::Info);
        let vfs_dyn: Arc<dyn VirtualFileSystem + Send + Sync> = vfs.clone();
        let output = new_output_buffer();
        
        Self {
            config: Arc::new(config),
            vfs: vfs_dyn,
            log,
            output,
            loaders: LoaderRegistry::new(),
            adaptive_parsers: AdaptiveParserRegistry::new(),
            passes: PassRegistry::new(),
            emitters: EmitterRegistry::new(),
        }
    }
    
    /// Register a loader component
    pub fn register_loader(&mut self, loader: Box<dyn Loader>) {
        self.loaders.register(loader);
    }
    
    /// Register an adaptive parser component
    pub fn register_adaptive_parser(&mut self, parser: Box<dyn AdaptiveParser>) {
        self.adaptive_parsers.register(parser);
    }
    
    /// Register a pass component
    pub fn register_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.register(pass);
    }
    
    /// Register an emitter component
    pub fn register_emitter(&mut self, emitter: Box<dyn Emitter>) {
        self.emitters.register(emitter);
    }
    
    /// Execute the pipeline with the given request
    pub fn run(&self, request: ExecutionRequest) -> Result<ExecutionResult, OrchestratorError> {
        let start_time = std::time::Instant::now();
        
        // Step 1: Load input
        let raw_data = self.load(&request.source)?;
        
        // Step 2: Parse raw data to initial IR
        let ir = self.parse(raw_data, &request.from)?;
        
        // Step 3: Run passes to transform IR
        let ir = self.transform(ir, &request.from, &request.to, &request.options, request.preferred_pass.as_deref())?;
        
        // Step 4: Emit output
        let serialized = self.emit(ir.clone(), &request.to, &request.target)?;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Collect output entries
        let output_entries = self.output.drain();
        
        use std::collections::HashMap;
        Ok(ExecutionResult {
            final_ir: Some(ir),
            serialized,
            output_entries,
            metadata: HashMap::new(),
            execution_time_ms,
        })
    }
    
    /// Load data from source
    fn load(&self, source: &Source) -> Result<RawData, OrchestratorError> {
        // Find appropriate loader by matching naming convention
        let loader_name = match source.kind {
            crate::loader::SourceKind::File => "file_loader",
            crate::loader::SourceKind::Stdin => "stdin_loader",
            crate::loader::SourceKind::Memory => "memory_loader",
            crate::loader::SourceKind::Network => "network_loader",
        };
        
        let loader = self.loaders.get(loader_name)
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
    
    /// Parse raw data to initial IR
    fn parse(&self, raw: RawData, target_format: &DataFormat) -> Result<IR, OrchestratorError> {
        // Find parser that can produce target format
        let parser = self.adaptive_parsers.all()
            .find(|p| p.output_format() == *target_format)
            .ok_or_else(|| OrchestratorError::PipelineError {
                message: format!("No adaptive parser found for format: {:?}", target_format),
            })?;
        
        parser.parse(raw)
            .map_err(|e| OrchestratorError::AdaptiveParserError {
                name: parser.metadata().name.to_string(),
                source: e,
            })
    }
    
    /// Transform IR through passes
    fn transform(
        &self,
        initial_ir: IR,
        from: &DataFormat,
        to: &DataFormat,
        options: &PassOptions,
        preferred_pass: Option<&str>,
    ) -> Result<IR, OrchestratorError> {
        // Build pass chain with cycle detection
        let pass_chain = self.build_pass_chain(from, to, preferred_pass)?;
        
        // Create pass context with output buffer
        let ctx = PassContext::with_output(
            self.config.clone(),
            self.vfs.clone(),
            self.log.clone(),
            self.output.clone(),
        ).with_options(options.clone());
        
        // Execute each pass
        let mut current_ir = initial_ir;
        for pass in pass_chain {
            let input = PassInput::new(current_ir);
            let output = pass.run(input, &ctx)
                .map_err(|e| OrchestratorError::PassError {
                    name: pass.metadata().name.to_string(),
                    source: e,
                })?;
            
            current_ir = output.data;
        }
        
        Ok(current_ir)
    }
    
    /// Build the chain of passes from `from` to `to` with cycle detection
    fn build_pass_chain<'a>(
        &'a self,
        from: &DataFormat,
        to: &DataFormat,
        preferred_pass: Option<&str>,
    ) -> Result<Vec<&'a dyn Pass>, OrchestratorError> {
        let mut chain = Vec::new();
        let mut current = from.clone();
        let mut visited = HashSet::new();
        const MAX_CHAIN_LENGTH: usize = 100;
        
        while &current != to {
            // Cycle detection: check if we've seen this format before
            if !visited.insert(current.clone()) {
                return Err(OrchestratorError::PipelineError {
                    message: format!(
                        "Cycle detected in pass chain: cannot reach '{:?}' from '{:?}'",
                        to, from
                    ),
                });
            }
            
            // Max chain length protection
            if chain.len() >= MAX_CHAIN_LENGTH {
                return Err(OrchestratorError::PipelineError {
                    message: format!(
                        "Pass chain too long (>{}), possible infinite loop",
                        MAX_CHAIN_LENGTH
                    ),
                });
            }
            
            // Find next pass that can process current format
            // First, try preferred pass if specified and matches current format
            let matching_passes: Vec<_> = self.passes.all()
                .filter(|p| p.input_format() == current)
                .collect();
            
            let pass = if matching_passes.is_empty() {
                return Err(OrchestratorError::IncompleteChain {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            } else if let Some(preferred) = preferred_pass {
                // Try to find preferred pass among matching ones
                matching_passes.into_iter()
                    .find(|p| p.metadata().name == preferred)
                    .or_else(|| self.passes.all().find(|p| p.input_format() == current))
                    .ok_or_else(|| OrchestratorError::IncompleteChain {
                        from: from.to_string(),
                        to: to.to_string(),
                    })?
            } else {
                // Use first matching pass
                matching_passes.into_iter().next().unwrap()
            };
            
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
    ) -> Result<Option<SerializedOutput>, OrchestratorError> {
        // Find emitter for this format
        let emitter = self.emitters.all()
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
    
    /// Get a reference to the loader registry
    pub fn loaders(&self) -> &LoaderRegistry {
        &self.loaders
    }
    
    /// Get a reference to the adaptive parser registry
    pub fn adaptive_parsers(&self) -> &AdaptiveParserRegistry {
        &self.adaptive_parsers
    }
    
    /// Get a reference to the pass registry
    pub fn passes(&self) -> &PassRegistry {
        &self.passes
    }
    
    /// Get a reference to the emitter registry
    pub fn emitters(&self) -> &EmitterRegistry {
        &self.emitters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let config = VmConfig::default();
        let orchestrator = Orchestrator::new(config);
        
        // Should have empty registries initially
        assert_eq!(orchestrator.loaders().len(), 0);
        assert_eq!(orchestrator.passes().len(), 0);
        assert_eq!(orchestrator.adaptive_parsers().len(), 0);
        assert_eq!(orchestrator.emitters().len(), 0);
    }
}
