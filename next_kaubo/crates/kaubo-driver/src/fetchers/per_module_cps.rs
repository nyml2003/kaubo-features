//! PerModuleCpsFetcher — single module compilation (Source → CpsModule).
//!
//! Wraps the per-module compilation logic from `ModuleCompiler` into a
//! DAG Fetcher. Each module's compilation is independent of others
//! (aside from the shared ModuleGraph), enabling concurrent execution.

use crate::module_loader::ModuleLoader;
use crate::protocol::Pipeline;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use kaubo_ir::flatten::flatten_module;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Compiles a single module from Source to CpsModule.
///
/// Depends on:
/// - `Source/{path}` — the module's source text
/// - `ModuleGraph` — for import metadata and struct registration
///
/// This is intentionally a **local** compilation: cross-module symbol
/// resolution happens later in `LinkedCpsFetcher`. Imports are recorded
/// as stubs (CallExternal) that the linker resolves.
pub struct PerModuleCpsFetcher {
    pub module_path: String,
    pub pipeline: Option<Pipeline>,
    pub loader: Arc<dyn ModuleLoader>,
}

impl PerModuleCpsFetcher {
    pub fn new(module_path: impl Into<String>, pipeline: Option<Pipeline>, loader: Arc<dyn ModuleLoader>) -> Self {
        PerModuleCpsFetcher {
            module_path: module_path.into(),
            pipeline,
            loader,
        }
    }
}

impl Fetcher<String> for PerModuleCpsFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new(self.module_path.clone(), Kind::new(Kind::CPS))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![
            ArtifactKey::new(self.module_path.clone(), Kind::new(Kind::SOURCE)),
            ArtifactKey::new("__graph__".to_string(), Kind::new(Kind::MODULE_GRAPH)),
        ]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let module_path = self.module_path.clone();
        let pipeline = self.pipeline.clone();
        let loader = Arc::clone(&self.loader);
        let source_artifact = inputs.into_iter().next().unwrap(); // Source artifact is first
        // ModuleGraph artifact is second, but we get it via the dependency system

        Box::pin(async move {
            let source = source_artifact.downcast_clone::<String>();
            let path = module_path.clone();

            // Parse
            let module = kaubo_syntax::parser::Parser::new(&source)
                .parse()
                .map_err(|e| {
                    DagError::fetcher_error(
                        ArtifactKey::new(path.clone(), Kind::new(Kind::CPS)),
                        format!("parse: {e}"),
                    )
                })?;

            // Infer (local only — imports resolved at link time)
            kaubo_infer::infer_module(&module).map_err(|e| {
                DagError::fetcher_error(
                    ArtifactKey::new(path.clone(), Kind::new(Kind::CPS)),
                    format!("infer: {}", e.msg),
                )
            })?;

            // CPS build (without imports — CallExternal stubs will be
            // resolved by LinkedCpsFetcher)
            let mut cps = kaubo_ir::cps_build::build_module(&module, None).map_err(|e| {
                DagError::fetcher_error(
                    ArtifactKey::new(path.clone(), Kind::new(Kind::CPS)),
                    format!("build: {e}"),
                )
            })?;

            // Flatten + passes
            flatten_module(&mut cps);
            if let Some(ref passes) = pipeline {
                if !passes.is_empty() {
                    passes.run(&mut cps, None);
                }
            }

            Ok(Artifact::new(path, Kind::new(Kind::CPS), cps))
        })
    }
}
