//! CpsFetcher — Module (AST) → CpsModule (with flatten + passes).
//!
//! Wraps `kaubo_ir::cps_build::build_module` and the flatten/pass pipeline
//! into a DAG Fetcher.

use crate::protocol::Pipeline;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use kaubo_ir::flatten::flatten_module;
use std::future::Future;
use std::pin::Pin;

/// Produces an optimised `CpsModule` from a `Module`.
///
/// Internal pipeline: build_module → flatten → pass pipeline.
pub struct CpsFetcher {
    pub module_id: String,
    /// Optional optimisation passes (same as old Coordinator pipeline).
    pub pipeline: Option<Pipeline>,
}

impl CpsFetcher {
    pub fn new(module_id: impl Into<String>, pipeline: Option<Pipeline>) -> Self {
        CpsFetcher {
            module_id: module_id.into(),
            pipeline,
        }
    }
}

impl Fetcher<String> for CpsFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::CPS))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        // Depend on both AST and Semantic. Semantic catches type errors
        // (inference failures) before CPS lowering runs.
        vec![
            ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::AST)),
            ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::SEMANTIC)),
        ]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let module_id = self.module_id.clone();
        // inputs[0] = AST, inputs[1] = Semantic (used for error detection)
        let ast_artifact = inputs.into_iter().next().unwrap();
        let pipeline = self.pipeline.clone();
        Box::pin(async move {
            let module = ast_artifact.downcast_ref::<kaubo_ast::Module>();

            let mut cps =
                kaubo_ir::cps_build::build_module(module, None).map_err(|e| {
                    DagError::fetcher_error(
                        ArtifactKey::new(module_id.clone(), Kind::new(Kind::CPS)),
                        format!("build: {e}"),
                    )
                })?;

            flatten_module(&mut cps);

            if let Some(ref passes) = pipeline {
                if !passes.is_empty() {
                    passes.run(&mut cps, None);
                }
            }

            Ok(Artifact::new(module_id, Kind::new(Kind::CPS), cps))
        })
    }
}
