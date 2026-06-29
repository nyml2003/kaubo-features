//! ModuleGraphFetcher — discovers the module dependency graph.
//!
//! Wraps `ModuleGraph::build()` into a DAG Fetcher. After building the
//! graph, seeds all Source artifacts into the scheduler's ready cache
//! so that downstream per-module fetchers can pick them up.

use crate::module_graph::ModuleGraph;
use crate::module_loader::ModuleLoader;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Discovers the full dependency graph from an entry module.
///
/// On completion, seeds `Source/{path}` artifacts for every discovered
/// module so that downstream fetchers can declare dependencies on them.
pub struct ModuleGraphFetcher {
    pub entry: String,
    pub loader: Arc<dyn ModuleLoader>,
}

impl ModuleGraphFetcher {
    pub fn new(entry: impl Into<String>, loader: Arc<dyn ModuleLoader>) -> Self {
        ModuleGraphFetcher {
            entry: entry.into(),
            loader,
        }
    }
}

impl Fetcher<String> for ModuleGraphFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new("__graph__".to_string(), Kind::new(Kind::MODULE_GRAPH))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![]
    }

    fn fetch<'a>(
        &'a self,
        _inputs: Vec<Artifact<String>>,
        ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let entry = self.entry.clone();
        let loader = Arc::clone(&self.loader);

        Box::pin(async move {
            // 1. Build the module graph
            let graph = ModuleGraph::build(&entry, loader.as_ref()).map_err(|e| {
                DagError::fetcher_error(
                    ArtifactKey::new("__graph__".to_string(), Kind::new(Kind::MODULE_GRAPH)),
                    format!("module graph: {e}"),
                )
            })?;

            // 2. Seed Source artifacts for every discovered module
            for (path, source) in &graph.sources {
                let artifact = Artifact::new(path.clone(), Kind::new(Kind::SOURCE), source.clone());
                ctx.seed_artifact(artifact);
            }

            Ok(Artifact::new("__graph__".to_string(), Kind::new(Kind::MODULE_GRAPH), graph))
        })
    }
}
