//! DagCoordinator — wires the DAG scheduler into a convenient synchronous API.
//!
//! This is the bridge between the async `kaubo-dag` crate and the existing
//! synchronous `kaubo-driver` API surface.

use crate::builders::execute::ExecuteBuilder;
use crate::fetchers;
use crate::module_loader::ModuleLoader;
use crate::protocol::Pipeline;
use crate::stages::adapt_pass;
use crate::RunOutcome;
use kaubo_dag::{Artifact, ArtifactKey, BlockingSpawner, BuilderEvent, DagError, DagScheduler, FetcherRegistry, Kind, NativeSpawner};
use kaubo_ir::cps::CpsModule;
use kaubo_ir::pass::{empty_block::EmptyBlockElim, fold::ConstantFold, move_fold::MoveFold};
use std::sync::Arc;

/// A coordinator that uses the DAG scheduler for compilation.
///
/// Holds a pre-configured scheduler with all standard fetcher factories
/// registered. Call [`compile_source`](DagCoordinator::compile_source) or
/// [`run_source`](DagCoordinator::run_source) to use it.
///
/// # Comparison with old `Coordinator`
///
/// | Feature | Old Coordinator | DagCoordinator |
/// |---------|----------------|---------------|
/// | Execution model | Synchronous `fn` calls | Async DAG expansion |
/// | Parallelism | Serial only | Parallel (Semantic ∥ Cps) |
/// | Cancellation | Not supported | `drop(stream)` cancels all tasks |
/// | Caching | String-keyed HashMap | ArtifactStore with dependency tracking |
/// | Progress | EventHandler side-channel | First-class ProgressEvent stream |
pub struct DagCoordinator {
    scheduler: Arc<DagScheduler<String>>,
}

impl DagCoordinator {
    /// Create a new DagCoordinator for single-file compilation with the
    /// standard optimisation pipeline (EmptyBlockElim + MoveFold + ConstantFold).
    pub fn new() -> Self {
        let pipeline = Pipeline::new()
            .add(adapt_pass(EmptyBlockElim))
            .add(adapt_pass(MoveFold))
            .add(adapt_pass(ConstantFold));
        Self::with_pipeline(pipeline)
    }

    /// Create a DagCoordinator for single-file compilation with a
    /// specific optimisation pipeline.
    pub fn with_pipeline(pipeline: Pipeline) -> Self {
        let registry = FetcherRegistry::<String>::new();
        let spawner = Arc::new(NativeSpawner);

        let pipeline_for_cps = pipeline;
        registry.register(
            Kind::new(Kind::CPS),
            Box::new(move |key| {
                Box::new(fetchers::cps::CpsFetcher::new(
                    key.module_id.clone(),
                    Some(pipeline_for_cps.clone()),
                ))
            }),
        );

        registry.register(
            Kind::new(Kind::SEMANTIC),
            Box::new(|key| {
                Box::new(fetchers::semantic::SemanticFetcher::new(
                    key.module_id.clone(),
                ))
            }),
        );

        let scheduler = DagScheduler::new(registry, spawner);
        DagCoordinator { scheduler }
    }

    /// Create a DagCoordinator for multi-file compilation.
    ///
    /// Registers:
    /// - ModuleGraphFetcher (discovers dependency graph + seeds Source artifacts)
    /// - LinkedCpsFetcher (delegates to ModuleCompiler for correct per-module
    ///   compilation with import resolution + linking)
    ///
    /// Per-module compilation is currently serial (ModuleCompiler's design).
    /// Phase 2 will add concurrent per-module Fetchers when inference supports
    /// import-free compilation.
    pub fn new_multifile(
        entry: impl Into<String>,
        loader: Arc<dyn ModuleLoader>,
        pipeline: Option<Pipeline>,
    ) -> Self {
        let registry = FetcherRegistry::<String>::new();
        let spawner = Arc::new(NativeSpawner);

        let entry_str: String = entry.into();
        let loader_for_graph = Arc::clone(&loader);
        let loader_for_link = Arc::clone(&loader);

        // ModuleGraphFetcher — entry point: discovers graph + seeds Sources
        registry.register(
            Kind::new(Kind::MODULE_GRAPH),
            Box::new(move |_key| {
                Box::new(fetchers::module_graph::ModuleGraphFetcher::new(
                    entry_str.clone(),
                    Arc::clone(&loader_for_graph),
                ))
            }),
        );

        // LinkedCpsFetcher — delegates to ModuleCompiler for correct
        // per-module compilation + linking
        registry.register(
            Kind::new(Kind::LINKED_CPS),
            Box::new(move |_key| {
                Box::new(fetchers::linked_cps::LinkedCpsFetcher::new(
                    Arc::clone(&loader_for_link),
                ))
            }),
        );

        let _pipeline = pipeline; // Phase 2: pass to per-module fetchers

        let scheduler = DagScheduler::new(registry, spawner);
        DagCoordinator { scheduler }
    }

    /// Compile source text using the single-file DAG pipeline.
    pub fn compile_source(&self, source: &str) -> Result<CpsModule, DagError<String>> {
        self.compile_source_with_config(source, u64::MAX)
    }

    /// Compile source text with a max loop iteration limit.
    pub fn compile_source_with_config(
        &self,
        source: &str,
        max_loop_iterations: u64,
    ) -> Result<CpsModule, DagError<String>> {
        let module_id = "mod".to_string();

        let module = kaubo_syntax::parser::Parser::new(source)
            .parse()
            .map_err(|e| {
                DagError::fetcher_error(
                    ArtifactKey::new(module_id.clone(), Kind::new(Kind::AST)),
                    format!("parse: {e}"),
                )
            })?;

        let ast_artifact = Artifact::new(module_id.clone(), Kind::new(Kind::AST), module);
        self.scheduler.seed_artifact(ast_artifact);

        let builder = Box::new(CpsBuilder { module_id, max_loop_iterations });
        let stream = self.scheduler.build(builder);
        let spawner = NativeSpawner;
        spawner.block_on(async {
            futures::pin_mut!(stream);
            match futures::StreamExt::next(&mut stream).await {
                Some(BuilderEvent::Done(cps)) => Ok(cps),
                Some(BuilderEvent::Error(e)) => Err((*e).clone()),
                None => Err(DagError::Internal("compile stream ended without result".into())),
            }
        })
    }

    /// Compile and execute source text (single-file).
    pub fn run_source(&self, source: &str) -> Result<RunOutcome, DagError<String>> {
        self.run_source_with_config(source, u64::MAX)
    }

    /// Compile and execute source text with a max loop iteration limit.
    pub fn run_source_with_config(
        &self,
        source: &str,
        max_loop_iterations: u64,
    ) -> Result<RunOutcome, DagError<String>> {
        let cps = self.compile_source_with_config(source, max_loop_iterations)?;
        let cps_artifact = Artifact::new("mod".to_string(), Kind::new(Kind::CPS), cps);
        self.scheduler.seed_artifact(cps_artifact);

        let builder = Box::new(ExecuteBuilder::new("mod").with_max_loop_iterations(max_loop_iterations));
        let stream = self.scheduler.build(builder);
        let spawner = NativeSpawner;
        spawner.block_on(async {
            futures::pin_mut!(stream);
            match futures::StreamExt::next(&mut stream).await {
                Some(BuilderEvent::Done(outcome)) => Ok(outcome),
                Some(BuilderEvent::Error(e)) => Err((*e).clone()),
                None => Err(DagError::Internal("run stream ended without result".into())),
            }
        })
    }

    /// Compile an entry module and its transitive dependencies using the
    /// DAG scheduler. Modules are compiled concurrently.
    pub fn compile_file(
        &self,
        entry: &str,
        loader: Arc<dyn ModuleLoader>,
    ) -> Result<CpsModule, DagError<String>> {
        let entry_owned = entry.to_string();
        let builder = Box::new(LinkedCpsBuilder {
            entry: entry_owned,
            loader: Arc::clone(&loader),
        });

        let stream = self.scheduler.build(builder);
        let spawner = NativeSpawner;
        spawner.block_on(async {
            futures::pin_mut!(stream);
            match futures::StreamExt::next(&mut stream).await {
                Some(BuilderEvent::Done(cps)) => Ok(cps),
                Some(BuilderEvent::Error(e)) => Err((*e).clone()),
                None => Err(DagError::Internal("compile file stream ended without result".into())),
            }
        })
    }

    /// Compile and execute a multi-file program.
    pub fn run_file(
        &self,
        entry: &str,
        loader: Arc<dyn ModuleLoader>,
    ) -> Result<RunOutcome, DagError<String>> {
        let cps = self.compile_file(entry, loader)?;
        // Seed as Kind::CPS so ExecuteBuilder (which depends on Cps/{module_id}) finds it
        let cps_artifact = Artifact::new("__linked__".to_string(), Kind::new(Kind::CPS), cps);
        self.scheduler.seed_artifact(cps_artifact);

        let builder = Box::new(ExecuteBuilder::new("__linked__"));
        let stream = self.scheduler.build(builder);
        let spawner = NativeSpawner;
        spawner.block_on(async {
            futures::pin_mut!(stream);
            match futures::StreamExt::next(&mut stream).await {
                Some(BuilderEvent::Done(outcome)) => Ok(outcome),
                Some(BuilderEvent::Error(e)) => Err((*e).clone()),
                None => Err(DagError::Internal("run file stream ended without result".into())),
            }
        })
    }

    /// Access the underlying scheduler (for advanced use).
    pub fn scheduler(&self) -> &Arc<DagScheduler<String>> {
        &self.scheduler
    }
}

impl Default for DagCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal Builders ────────────────────────────────────────────────

/// Builder that depends on a Cps artifact and returns it (single-file).
struct CpsBuilder {
    module_id: String,
    #[allow(dead_code)]
    max_loop_iterations: u64,
}

impl kaubo_dag::Builder<String, CpsModule> for CpsBuilder {
    fn name(&self) -> &str { "CpsBuild" }
    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::CPS))]
    }
    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut kaubo_dag::FetchContext<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<CpsModule, DagError<String>>> + Send + 'a>> {
        let cps = inputs.into_iter().next().unwrap().downcast_clone::<CpsModule>();
        Box::pin(async move { Ok(cps) })
    }
}

/// Builder for multi-file: requests LinkedCps and returns it.
struct LinkedCpsBuilder {
    #[allow(dead_code)]
    entry: String,
    #[allow(dead_code)]
    loader: Arc<dyn ModuleLoader>,
}

impl kaubo_dag::Builder<String, CpsModule> for LinkedCpsBuilder {
    fn name(&self) -> &str { "LinkedCpsBuild" }
    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS))]
    }
    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut kaubo_dag::FetchContext<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<CpsModule, DagError<String>>> + Send + 'a>> {
        let cps = inputs.into_iter().next().unwrap().downcast_clone::<CpsModule>();
        Box::pin(async move { Ok(cps) })
    }
}
