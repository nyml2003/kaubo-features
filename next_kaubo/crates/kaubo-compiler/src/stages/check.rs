//! CheckStage — 类型检查
use crate::parser::Module;
use crate::parser::type_checker::TypeChecker;

pub struct CheckStage;

impl CheckStage {
    pub fn new() -> Self { Self }
    pub fn run(&self, module: &Module) -> Result<(), String> {
        let mut checker = TypeChecker::new();
        checker.set_strict_mode(false);
        checker.check_module(module).map_err(|e| format!("type error: {}", e))
    }
}

impl kaubo_pipeline::Stage<Module, Module> for CheckStage {
    fn name(&self) -> &'static str { "TypeCheck" }
    fn run(&self, input: Module, _ctx: &kaubo_pipeline::PipelineCtx) -> Result<Module, String> {
        self.run(&input).map(|_| input)
    }
}
