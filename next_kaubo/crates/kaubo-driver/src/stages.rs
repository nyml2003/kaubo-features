//! Compilation stages — each wraps one phase of the pipeline.
//!
//! Every stage implements `Stage<I, O>` from the protocol layer.  They are
//! thin adapters over the existing functions in `kaubo-syntax`, `kaubo-infer`,
//! `kaubo-ir`, and `kaubo-vm`.

use crate::protocol::{BuildContext, BuildError, Stage};
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

// ── Pass wrapper (used by DagCoordinator, DAG fetchers) ──

/// Adapt a `kaubo_ir::pass::Pass` to the protocol `Pass` trait.
struct IrPassAdapter<T: kaubo_ir::pass::Pass> {
    inner: T,
}

impl<T: kaubo_ir::pass::Pass + Send + Sync> crate::protocol::Pass for IrPassAdapter<T> {
    fn name(&self) -> &str { self.inner.name() }
    fn run(&self, module: &mut CpsModule, _events: Option<&dyn kaubo_log::EventHandler>) {
        self.inner.run(module);
    }
}

/// Create a protocol Pass from an existing kaubo_ir pass.
pub fn adapt_pass(pass: impl kaubo_ir::pass::Pass + Send + Sync + 'static) -> std::sync::Arc<dyn crate::protocol::Pass> {
    std::sync::Arc::new(IrPassAdapter { inner: pass })
}
