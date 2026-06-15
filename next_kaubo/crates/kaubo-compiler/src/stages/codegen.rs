//! CodegenStage — AST → Chunk
use crate::parser::{Module, StmtKind};
use crate::codegen::compile_with_struct_info_and_logger;
use kaubo_ir::Chunk;
use std::collections::HashMap;

pub struct CodegenStage;

impl CodegenStage {
    pub fn new() -> Self { Self }
    pub fn run(&self, module: &Module) -> Result<Chunk, String> {
        // Pre-pass: collect struct definitions so codegen can look up
        // shape IDs when it encounters struct literals.
        let mut struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)> = HashMap::new();
        let mut next_id: u16 = 1;
        for stmt in &module.statements {
            if let StmtKind::Struct(s) = stmt.as_ref() {
                let names: Vec<_> = s.fields.iter().map(|f| f.name.clone()).collect();
                let types: Vec<_> = s.fields.iter().map(|f| f.type_annotation.to_string()).collect();
                struct_infos.insert(s.name.clone(), (next_id, names, types));
                next_id += 1;
            }
        }

        compile_with_struct_info_and_logger(
            module,
            struct_infos.clone(),
            kaubo_log::Logger::new(kaubo_log::Level::Warn),
        )
        .map(|(mut chunk, local_count)| {
            chunk.shape_table = struct_infos
                .into_iter()
                .map(|(name, (id, fields, types))| (id, name, fields, types))
                .collect();
            (chunk, local_count)
        })
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
