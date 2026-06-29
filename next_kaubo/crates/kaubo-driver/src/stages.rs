//! Compilation stages — each wraps one phase of the pipeline.
//!
//! Every stage implements `Stage<I, O>` from the protocol layer.  They are
//! thin adapters over the existing functions in `kaubo-syntax`, `kaubo-infer`,
//! `kaubo-ir`, and `kaubo-vm`.

use crate::protocol::{BuildContext, BuildError, Stage};
use crate::RunOutcome;
use kaubo_ast::Module;
use kaubo_ir::cps::CpsModule;
use kaubo_syntax::parser::Parser;

// ── Frontend: Source → AST ──

pub struct FrontendStage;

impl Stage<&str, Module> for FrontendStage {
    fn name(&self) -> &str {
        "frontend"
    }

    fn execute(&self, source: &str, _ctx: &BuildContext) -> Result<Module, BuildError> {
        Parser::new(source).parse().map_err(|e| BuildError::Parse(e.to_string()))
    }
}

// ── Semantic: AST → SemanticArtifact ──

/// Rich output of type inference — symbols, types, references, and the
/// original type environment.  This is the primary data source for LSP.
#[derive(Debug, Clone)]
pub struct SemanticArtifact {
    pub type_env: kaubo_infer::TypeEnv,
    pub struct_fields: std::collections::HashMap<usize, Vec<(String, kaubo_infer::Type)>>,
}

pub struct SemanticStage;

impl Stage<Module, SemanticArtifact> for SemanticStage {
    fn name(&self) -> &str {
        "semantic"
    }

    fn execute(&self, module: Module, _ctx: &BuildContext) -> Result<SemanticArtifact, BuildError> {
        let (type_env, struct_fields) =
            kaubo_infer::infer_module(&module).map_err(|e| BuildError::Infer(e.msg))?;

        Ok(SemanticArtifact {
            type_env,
            struct_fields,
        })
    }
}

// ── CPS Build: AST → CpsModule ──

pub struct CpsBuildStage<'a> {
    pub events: Option<&'a dyn kaubo_log::EventHandler>,
}

impl Stage<&Module, CpsModule> for CpsBuildStage<'_> {
    fn name(&self) -> &str {
        "cps-build"
    }

    fn execute(&self, module: &Module, _ctx: &BuildContext) -> Result<CpsModule, BuildError> {
        kaubo_ir::cps_build::build_module(module, self.events).map_err(BuildError::Build)
    }
}

// ── VM Exec: CpsModule → RunOutcome ──

pub struct VmExecStage {
    pub max_loop_iterations: u64,
}

impl Stage<CpsModule, RunOutcome> for VmExecStage {
    fn name(&self) -> &str {
        "vm-exec"
    }

    fn execute(&self, cps: CpsModule, ctx: &BuildContext) -> Result<RunOutcome, BuildError> {
        if cps.functions.is_empty() {
            return Ok(RunOutcome {
                result: 0,
                output: vec![],
            });
        }

        let mut vm = kaubo_vm::VM::new();
        vm.max_loop_iterations = self.max_loop_iterations;
        vm.load(&cps).map_err(BuildError::Load)?;

        let func_idx = cps.functions.len() - 1;
        let reg_count = cps.functions[func_idx].reg_count;

        let result = vm
            .execute(func_idx, reg_count, ctx.events)
            .map_err(|e| BuildError::Runtime(format!("{e:?}")))?;

        Ok(RunOutcome {
            result,
            output: vm.output,
        })
    }
}

// ── Pass wrappers ──

/// Adapt a `kaubo_ir::pass::Pass` to the protocol `Pass` trait.
struct IrPassAdapter<T: kaubo_ir::pass::Pass> {
    inner: T,
}

impl<T: kaubo_ir::pass::Pass + Send + Sync> crate::protocol::Pass for IrPassAdapter<T> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn run(&self, module: &mut CpsModule, _events: Option<&dyn kaubo_log::EventHandler>) {
        self.inner.run(module);
    }
}

/// Create a protocol Pass from an existing kaubo_ir pass.
pub fn adapt_pass(pass: impl kaubo_ir::pass::Pass + Send + Sync + 'static) -> std::sync::Arc<dyn crate::protocol::Pass> {
    std::sync::Arc::new(IrPassAdapter { inner: pass })
}
