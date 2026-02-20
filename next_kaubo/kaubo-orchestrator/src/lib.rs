//! Kaubo Orchestrator
//! 
//! Pipeline execution engine and component management.

pub mod component;
pub mod loader;
pub mod converter;
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
pub use converter::{Converter};
pub use pass::{Pass, PassContext, Input as PassInput, Output as PassOutput};
pub use emitter::{Emitter, Target, TargetKind};
pub use registry::{LoaderRegistry, ConverterRegistry, PassRegistry, EmitterRegistry};
pub use pipeline::{PipelineEngine, ExecutionRequest, ExecutionResult};
pub use context::Context;
pub use error::{OrchestratorError, PassError, LoaderError, ConverterError, EmitterError};
pub use output::{OutputHandle, OutputEntry, OutputBuffer, new_output_buffer};

// Re-export component implementations
pub use adapters::{CodeGenPass, CompilePass, MultiModulePass, ParserPass};
pub use loaders::FileLoader;
pub use emitters::{FileEmitter, StdoutEmitter};
pub use passes::NoOpPass;

use std::sync::Arc;
use kaubo_config::VmConfig;
use kaubo_vfs::{VirtualFileSystem, MemoryFileSystem};
use kaubo_log::Logger;

/// The main orchestrator that manages components and executes pipelines
pub struct Orchestrator {
    config: VmConfig,
    vfs: Arc<dyn VirtualFileSystem + Send + Sync>,
    log: Arc<Logger>,
    
    // Component registries
    loaders: LoaderRegistry,
    converters: ConverterRegistry,
    passes: PassRegistry,
    emitters: EmitterRegistry,
    
    // Pipeline engine
    pipeline: PipelineEngine,
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration
    pub fn new(config: VmConfig) -> Self {
        let vfs: Arc<dyn VirtualFileSystem + Send + Sync> = Arc::new(MemoryFileSystem::new());
        let log: Arc<Logger> = Logger::new(kaubo_log::Level::Info);
        let config_arc = Arc::new(config);
        let output = new_output_buffer();
        
        Self {
            config: (*config_arc).clone(),
            vfs: vfs.clone(),
            log: log.clone(),
            loaders: LoaderRegistry::new(),
            converters: ConverterRegistry::new(),
            passes: PassRegistry::new(),
            emitters: EmitterRegistry::new(),
            pipeline: PipelineEngine::with_output(config_arc, vfs, log, output),
        }
    }
    
    /// Create with custom VFS
    pub fn with_vfs<VFS: VirtualFileSystem + Send + Sync + 'static>(
        config: VmConfig,
        vfs: Arc<VFS>,
    ) -> Self {
        let log: Arc<Logger> = Logger::new(kaubo_log::Level::Info);
        let config_arc = Arc::new(config.clone());
        let vfs_dyn: Arc<dyn VirtualFileSystem + Send + Sync> = vfs.clone();
        let output = new_output_buffer();
        
        Self {
            config,
            vfs: vfs_dyn.clone(),
            log: log.clone(),
            loaders: LoaderRegistry::new(),
            converters: ConverterRegistry::new(),
            passes: PassRegistry::new(),
            emitters: EmitterRegistry::new(),
            pipeline: PipelineEngine::with_output(config_arc, vfs_dyn, log, output),
        }
    }
    
    /// Register a loader component
    pub fn register_loader(&mut self, loader: Box<dyn Loader>) {
        self.loaders.register(loader);
    }
    
    /// Register a converter component
    pub fn register_converter(&mut self, converter: Box<dyn Converter>) {
        self.converters.register(converter);
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
        self.pipeline.execute(
            request,
            &self.loaders,
            &self.converters,
            &self.passes,
            &self.emitters,
        )
    }
    
    /// Get a reference to the loader registry
    pub fn loaders(&self) -> &LoaderRegistry {
        &self.loaders
    }
    
    /// Get a reference to the converter registry
    pub fn converters(&self) -> &ConverterRegistry {
        &self.converters
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
    }
}
