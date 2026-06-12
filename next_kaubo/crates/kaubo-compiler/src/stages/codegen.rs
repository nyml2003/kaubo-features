//! CodegenStage — AST → Chunk
use crate::parser::Module;
use crate::codegen::compile_with_struct_info_and_logger;
use kaubo_ir::Chunk;
use kaubo_ir::object::ObjString;
use kaubo_ir::value::Value;
use std::collections::HashMap;

pub struct CodegenStage;

impl CodegenStage {
    pub fn new() -> Self { Self }
    pub fn run(&self, module: &Module) -> Result<Chunk, String> {
        compile_with_struct_info_and_logger(module, HashMap::new(), kaubo_log::Logger::new(kaubo_log::Level::Warn))
            .map(|(chunk, _)| chunk)
            .map_err(|e| format!("codegen: {}", e))
    }
}

impl kaubo_pipeline::Stage<Module, Chunk> for CodegenStage {
    fn name(&self) -> &'static str { "Codegen" }
    fn run(&self, input: Module, _ctx: &kaubo_pipeline::PipelineCtx) -> Result<Chunk, String> {
        self.run(&input)
    }
}
