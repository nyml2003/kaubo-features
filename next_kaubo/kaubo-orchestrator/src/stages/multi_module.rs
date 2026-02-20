//! Multi-Module Pass - multi-file module compilation
//!
//! Compiles entry file and recursively compiles all imported modules.

use std::path::Path;
use std::sync::Arc;

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::adaptive_parser::{DataFormat, IR};
use crate::error::PassError;
use crate::pass::{Input, Output, Pass, PassContext};
use crate::pipeline::module::{CompileContext, ModuleId};
use crate::pipeline::codegen::compile_with_struct_info_and_logger;
use std::collections::HashMap;

/// Multi-module compilation pass
///
/// Input: Source (entry file path)
/// Output: Bytecode (merged bytecode from all modules)
pub struct MultiModulePass {
    logger: Arc<kaubo_log::Logger>,
}

impl MultiModulePass {
    /// Create a new multi-module compilation pass
    pub fn new(logger: Arc<kaubo_log::Logger>) -> Self {
        Self { logger }
    }
}

impl Component for MultiModulePass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "multi_module",
            "0.1.0",
            ComponentKind::Pass,
            Some("Multi-file module compilation (supports import)"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![DataFormat::Source], vec![DataFormat::Bytecode])
    }
}

impl Pass for MultiModulePass {
    fn input_format(&self) -> DataFormat {
        DataFormat::Source
    }

    fn output_format(&self) -> DataFormat {
        DataFormat::Bytecode
    }

    fn run(&self, _input: Input, ctx: &PassContext) -> Result<Output, PassError> {
        // Get entry file path from PassContext
        let entry_path = ctx.source_path.as_ref().ok_or_else(|| PassError::InvalidInput {
            message: "MultiModulePass requires source_path in context".to_string(),
        })?;
        
        // Get entry file's directory as root
        let root_dir = entry_path.parent().unwrap_or(Path::new("."));
        
        // Get entry file stem as module name (e.g., "main.kaubo" -> "main")
        let entry_name = entry_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PassError::InvalidInput {
                message: format!("Invalid entry path: {}", entry_path.display()),
            })?;
        
        // Create compile context with VFS from PassContext
        let mut compile_ctx = CompileContext::new(
            &*ctx.vfs,
            root_dir,
        );
        
        // Parse entry module ID
        let entry_id = ModuleId::parse(entry_name)
            .map_err(|e| PassError::InvalidInput {
                message: format!("Invalid module name '{}': {}", entry_name, e),
            })?;
        
        // Compile entry and all dependencies
        compile_ctx.get_or_compile(&entry_id)
            .map_err(|e| PassError::TransformFailed(format!("Compilation failed: {}", e)))?;
        
        // For now, only compile the entry module (simplified version)
        // Full implementation should merge bytecode from all modules
        let sorted_units = compile_ctx.get_sorted_units();
        let entry_unit = sorted_units
            .last()
            .ok_or_else(|| PassError::TransformFailed("No compiled units".to_string()))?;
        
        // Compile entry module to bytecode
        let (chunk, _) = compile_with_struct_info_and_logger(
            &entry_unit.ast,
            HashMap::new(),
            self.logger.clone(),
        )
        .map_err(|e| PassError::TransformFailed(format!("Code generation failed: {:?}", e)))?;

        Ok(Output::new(IR::Bytecode(chunk)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_vfs::MemoryFileSystem;
    use kaubo_config::VmConfig;
    use kaubo_log::Logger;
    
    fn create_test_context() -> PassContext {
        PassContext::new(
            Arc::new(VmConfig::default()),
            Arc::new(MemoryFileSystem::new()),
            Logger::new(kaubo_log::Level::Info),
        )
    }
    
    #[test]
    fn test_multi_module_pass_metadata() {
        let logger: Arc<Logger> = Logger::new(kaubo_log::Level::Info);
        let pass = MultiModulePass::new(logger);

        assert_eq!(pass.metadata().name, "multi_module");
        assert_eq!(pass.input_format(), DataFormat::Source);
        assert_eq!(pass.output_format(), DataFormat::Bytecode);
    }

    #[test]
    fn test_multi_module_capabilities() {
        let logger: Arc<Logger> = Logger::new(kaubo_log::Level::Info);
        let pass = MultiModulePass::new(logger);
        let caps = pass.capabilities();

        assert!(caps.can_accept(&DataFormat::Source));
        assert!(caps.can_produce(&DataFormat::Bytecode));
    }
}
