//! LinkedCpsFetcher — cross-module compilation + linking.
//!
//! This is the convergence point of the multi-file DAG. After receiving
//! the ModuleGraph, it delegates to `ModuleCompiler::compile_all()` for
//! per-module compilation (with import resolution) and linking.
//!
//! Per-module compilation is serial (ModuleCompiler's current design),
//! but the DAG still provides value: ModuleGraph discovery and Source
//! loading are concurrent and cacheable. Per-module concurrency will be
//! added in Phase 2 when inference supports import-free compilation.

use crate::module_compiler::ModuleCompiler;
use crate::module_graph::ModuleGraph;
use crate::module_loader::ModuleLoader;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Compiles all modules and links them into a global CpsModule.
///
/// Internally reuses `ModuleCompiler` for guaranteed correctness of
/// import resolution, type inference with imports, and export table
/// construction. The DAG handles ModuleGraph discovery concurrently.
pub struct LinkedCpsFetcher {
    pub loader: Arc<dyn ModuleLoader>,
}

impl LinkedCpsFetcher {
    pub fn new(loader: Arc<dyn ModuleLoader>) -> Self {
        LinkedCpsFetcher { loader }
    }
}

impl Fetcher<String> for LinkedCpsFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new(
            "__graph__".to_string(),
            Kind::new(Kind::MODULE_GRAPH),
        )]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let graph_artifact = inputs.into_iter().next().unwrap();
        let loader = Arc::clone(&self.loader);

        Box::pin(async move {
            let graph = graph_artifact.downcast_ref::<ModuleGraph>();

            // Delegate to ModuleCompiler for correct per-module compilation
            // (parse + infer with imports + CPS build + export table + link).
            let mut compiler = ModuleCompiler::new(loader.as_ref());
            let linked = compiler.compile_all(&graph).map_err(|e| {
                DagError::fetcher_error(
                    ArtifactKey::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS)),
                    format!("compile: {e}"),
                )
            })?;

            Ok(Artifact::new(
                "__linked__".to_string(),
                Kind::new(Kind::LINKED_CPS),
                linked,
            ))
        })
    }
}
