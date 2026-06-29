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
use kaubo_dag::{Artifact, ArtifactKey, BuilderEvent, DagError, DagScheduler, FetcherRegistry, Kind};
use kaubo_ir::cps::CpsModule;
use kaubo_dag::Spawner;
use kaubo_ir::pass::{empty_block::EmptyBlockElim, fold::ConstantFold, move_fold::MoveFold};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use kaubo_dag::{BlockingSpawner, NativeSpawner};
#[cfg(target_arch = "wasm32")]
use kaubo_dag::WasmSpawner;

/// Platform-appropriate spawner.
#[cfg(not(target_arch = "wasm32"))]
fn default_spawner() -> Arc<dyn kaubo_dag::Spawner> {
    Arc::new(NativeSpawner)
}
#[cfg(target_arch = "wasm32")]
fn default_spawner() -> Arc<dyn kaubo_dag::Spawner> {
    Arc::new(WasmSpawner)
}

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
        let spawner = default_spawner();

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

    /// Create with a custom spawner (e.g. SyncSpawner for WASM sync API).
    pub fn new_with_spawner(spawner: Arc<dyn Spawner>) -> Self {
        let pipeline = Pipeline::new()
            .add(adapt_pass(EmptyBlockElim))
            .add(adapt_pass(MoveFold))
            .add(adapt_pass(ConstantFold));
        let registry = FetcherRegistry::<String>::new();
        let pipeline_for_cps = pipeline;
        registry.register(Kind::new(Kind::CPS), Box::new(move |key| {
            Box::new(fetchers::cps::CpsFetcher::new(key.module_id.clone(), Some(pipeline_for_cps.clone())))
        }));
        registry.register(Kind::new(Kind::SEMANTIC), Box::new(|key| {
            Box::new(fetchers::semantic::SemanticFetcher::new(key.module_id.clone()))
        }));
        DagCoordinator { scheduler: DagScheduler::new(registry, spawner) }
    }

    /// Create a DagCoordinator for multi-file compilation.
    ///
    /// Registers:
    /// - ModuleGraphFetcher (discovers graph + seeds Sources)
    /// - PerModuleCpsFetcher (concurrent per-module compilation via dynamic deps)
    /// - LinkedCpsFetcher (collects all Cps + links)
    pub fn new_multifile(
        entry: impl Into<String>,
        loader: Arc<dyn ModuleLoader>,
        pipeline: Option<Pipeline>,
    ) -> Self {
        let registry = FetcherRegistry::<String>::new();
        let spawner = default_spawner();

        let entry_str: String = entry.into();
        let l1 = Arc::clone(&loader);
        let l2 = Arc::clone(&loader);
        let p = pipeline.clone();

        registry.register(Kind::new(Kind::MODULE_GRAPH), Box::new(move |_key| {
            Box::new(fetchers::module_graph::ModuleGraphFetcher::new(entry_str.clone(), Arc::clone(&l1)))
        }));
        // Placeholder for ExportTable — PerModuleCpsFetcher seeds these
        // before downstream modules request them, so this factory is a
        // safety net that panics if an ExportTable is requested without
        // first being seeded.
        registry.register(Kind::new("ExportTable"), Box::new(|key| {
            panic!("ExportTable/{module_id} was requested but not seeded by PerModuleCpsFetcher", module_id = key.module_id)
        }));
        // Concurrent per-module compilation with full import resolution
        registry.register(Kind::new(Kind::CPS), Box::new(move |key| {
            Box::new(fetchers::per_module_cps::PerModuleCpsFetcher::new(key.module_id.clone(), p.clone(), Arc::clone(&l2)))
        }));
        registry.register(Kind::new(Kind::LINKED_CPS), Box::new(|_key| {
            Box::new(fetchers::linked_cps::LinkedCpsFetcher::new())
        }));

        let scheduler = DagScheduler::new(registry, spawner);
        DagCoordinator { scheduler }
    }

    // ── Async helpers ──────────────────────────────────────────────

    async fn collect_build<Out: Clone + Send + 'static>(
        stream: kaubo_dag::BuildStream<String, Out>,
    ) -> Result<Out, DagError<String>> {
        futures::pin_mut!(stream);
        match futures::StreamExt::next(&mut stream).await {
            Some(BuilderEvent::Done(out)) => Ok(out),
            Some(BuilderEvent::Error(e)) => Err((*e).clone()),
            None => Err(DagError::Internal("stream ended without result".into())),
        }
    }

    /// Async: compile source to CpsModule.
    pub async fn compile_source_async(&self, source: &str, max_loop_iterations: u64) -> Result<CpsModule, DagError<String>> {
        let module_id = "mod".to_string();
        let module = kaubo_syntax::parser::Parser::new(source).parse().map_err(|e| {
            DagError::fetcher_error(ArtifactKey::new(module_id.clone(), Kind::new(Kind::AST)), format!("parse: {e}"))
        })?;
        self.scheduler.seed_artifact(Artifact::new(module_id.clone(), Kind::new(Kind::AST), module));
        Self::collect_build(self.scheduler.build(Box::new(CpsBuilder { module_id, max_loop_iterations }))).await
    }

    /// Async: compile and execute.
    pub async fn run_source_async(&self, source: &str, max_loop_iterations: u64) -> Result<RunOutcome, DagError<String>> {
        let cps = self.compile_source_async(source, max_loop_iterations).await?;
        self.scheduler.seed_artifact(Artifact::new("mod".to_string(), Kind::new(Kind::CPS), cps));
        Self::collect_build(self.scheduler.build(Box::new(ExecuteBuilder::new("mod").with_max_loop_iterations(max_loop_iterations)))).await
    }

    /// Async: multi-file compile.
    pub async fn compile_file_async(&self, entry: &str, _loader: Arc<dyn ModuleLoader>) -> Result<CpsModule, DagError<String>> {
        let builder = Box::new(LinkedCpsBuilder { entry: entry.to_string() });
        Self::collect_build(self.scheduler.build(builder)).await
    }

    /// Async: multi-file compile + execute.
    pub async fn run_file_async(&self, entry: &str, loader: Arc<dyn ModuleLoader>) -> Result<RunOutcome, DagError<String>> {
        let cps = self.compile_file_async(entry, loader).await?;
        self.scheduler.seed_artifact(Artifact::new("__linked__".to_string(), Kind::new(Kind::CPS), cps));
        Self::collect_build(self.scheduler.build(Box::new(ExecuteBuilder::new("__linked__")))).await
    }

    // ── Sync wrappers (native only) ─────────────────────────────────

    /// Compile source text (native only — blocks current thread).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn compile_source(&self, source: &str) -> Result<CpsModule, DagError<String>> {
        self.compile_source_with_config(source, u64::MAX)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn compile_source_with_config(&self, source: &str, max_loop_iterations: u64) -> Result<CpsModule, DagError<String>> {
        let s = NativeSpawner;
        s.block_on(self.compile_source_async(source, max_loop_iterations))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_source(&self, source: &str) -> Result<RunOutcome, DagError<String>> {
        self.run_source_with_config(source, u64::MAX)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_source_with_config(&self, source: &str, max_loop_iterations: u64) -> Result<RunOutcome, DagError<String>> {
        let s = NativeSpawner;
        s.block_on(self.run_source_async(source, max_loop_iterations))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn compile_file(&self, entry: &str, loader: Arc<dyn ModuleLoader>) -> Result<CpsModule, DagError<String>> {
        let s = NativeSpawner;
        s.block_on(self.compile_file_async(entry, loader))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_file(&self, entry: &str, loader: Arc<dyn ModuleLoader>) -> Result<RunOutcome, DagError<String>> {
        let s = NativeSpawner;
        s.block_on(self.run_file_async(entry, loader))
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
        let cps = inputs.into_iter().next()
            .and_then(|a| a.try_downcast_clone::<CpsModule>())
            .ok_or_else(|| DagError::<String>::Internal("CpsBuilder: expected CpsModule".into()));
        Box::pin(async move { cps })
    }
}

/// Builder for multi-file: requests LinkedCps and returns it.
struct LinkedCpsBuilder { #[allow(dead_code)] entry: String }

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
        let cps = inputs.into_iter().next()
            .and_then(|a| a.try_downcast_clone::<CpsModule>())
            .ok_or_else(|| DagError::<String>::Internal("LinkedCpsBuilder: expected CpsModule".into()));
        Box::pin(async move { cps })
    }
}
