//! SemanticFetcher — Module (AST) → SemanticArtifact.
//!
//! Wraps `kaubo_infer::infer_module` into a DAG Fetcher.
//! Used by LSP for type information, diagnostics, and completions.

use crate::stages::SemanticArtifact;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use std::future::Future;
use std::pin::Pin;

/// Produces a `SemanticArtifact` from a `Module`.
///
/// In the DAG, this runs in **parallel** with [`CpsFetcher`](crate::fetchers::cps::CpsFetcher),
/// since both depend only on the AST.
pub struct SemanticFetcher {
    pub module_id: String,
}

impl SemanticFetcher {
    pub fn new(module_id: impl Into<String>) -> Self {
        SemanticFetcher {
            module_id: module_id.into(),
        }
    }
}

impl Fetcher<String> for SemanticFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::SEMANTIC))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::AST))]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let module_id = self.module_id.clone();
        let ast_artifact = inputs.into_iter().next().unwrap();
        Box::pin(async move {
            let Some(module) = ast_artifact.try_downcast_ref::<kaubo_ast::Module>() else {
                return Err(DagError::Internal("SemanticFetcher: expected Module artifact".into()));
            };

            let (type_env, struct_fields) =
                kaubo_infer::infer_module(module).map_err(|e| {
                    DagError::fetcher_error(
                        ArtifactKey::new(module_id.clone(), Kind::new(Kind::SEMANTIC)),
                        format!("infer: {}", e.msg),
                    )
                })?;

            let artifact_data = SemanticArtifact {
                type_env,
                struct_fields,
            };

            Ok(Artifact::new(module_id, Kind::new(Kind::SEMANTIC), artifact_data))
        })
    }
}
