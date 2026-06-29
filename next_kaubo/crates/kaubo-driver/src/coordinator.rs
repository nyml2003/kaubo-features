//! Coordinator — wires stages into a build pipeline with caching and event
//! routing.  This is the "business logic" layer that knows the specific order
//! of stages, which ones to cache, and how to fan out events.

use crate::event::{EventRouter, EventSink};
use crate::module_compiler::ModuleCompiler;
use crate::module_graph::ModuleGraph;
use crate::module_loader::ModuleLoader;
use crate::protocol::{ArtifactCache, BuildError, MemoryCache, Pipeline, Stage};
use crate::stages::{CpsBuildStage, FrontendStage, SemanticArtifact, SemanticStage, VmExecStage};
use crate::RunOutcome;
use kaubo_ast::Module;
use kaubo_ir::cps::CpsModule;
use kaubo_ir::flatten::flatten_module;
use kaubo_log::EventHandler;

/// SHA-256 hash of a byte slice (hex-encoded).
fn sha256_hex(data: &[u8]) -> String {
    use std::hash::Hasher;
    // Use std's DefaultHasher for simplicity — production should use real SHA-256.
    // Phase 3b replaces this with a proper cryptographic hash.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash_slice(data, &mut hasher);
    format!("{:016x}", hasher.finish())
}

pub struct Coordinator {
    cache: MemoryCache,
    pipeline: Pipeline,
    router: EventRouter,
    max_loop_iterations: u64,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            cache: MemoryCache::new(),
            pipeline: Pipeline::new(),
            router: EventRouter::new(),
            max_loop_iterations: u64::MAX,
        }
    }

    // ── Builder API ──

    pub fn with_pipeline(mut self, pipeline: Pipeline) -> Self {
        self.pipeline = pipeline;
        self
    }

    pub fn with_sink(mut self, sink: Box<dyn EventSink>) -> Self {
        self.router.add(sink);
        self
    }

    pub fn with_max_loop_iterations(mut self, limit: u64) -> Self {
        self.max_loop_iterations = limit;
        self
    }

    // ── Cache key helpers ──
    //
    // Format: {namespace}/{stage}/{content_hash}
    // Single-module: namespace = "local"
    // Phase 3b: namespace = ModuleId

    fn cache_key(&self, namespace: &str, content_hash: &str, stage: &str) -> String {
        format!("{namespace}/{stage}/{content_hash}")
    }

    fn cache_key_source(&self, source: &str, stage: &str) -> String {
        let hash = sha256_hex(source.as_bytes());
        self.cache_key("local", &hash, stage)
    }

    // ── Public API ──

    /// Full compile + execute (单文件模式).
    pub fn run(&mut self, source: &str) -> Result<RunOutcome, BuildError> {
        let cps = self.build_cps(source)?;
        self.execute(cps)
    }

    /// Compile source to optimised CPS (with flatten + passes) (单文件模式).
    pub fn compile(&mut self, source: &str) -> Result<CpsModule, BuildError> {
        self.build_cps(source)
    }

    /// 多文件编译 + 执行。
    ///
    /// `entry` 是入口模块路径，`loader` 用于加载依赖。
    /// 内部构建模块图 → 按序编译 → 链接 → 执行。
    pub fn run_file(
        &mut self,
        entry: &str,
        loader: &dyn ModuleLoader,
    ) -> Result<RunOutcome, BuildError> {
        let cps = self.compile_file(entry, loader)?;
        self.execute(cps)
    }

    /// 多文件编译（产生链接后的 CpsModule）。
    pub fn compile_file(
        &mut self,
        entry: &str,
        loader: &dyn ModuleLoader,
    ) -> Result<CpsModule, BuildError> {
        let graph = ModuleGraph::build(entry, loader)?;
        let mut compiler = ModuleCompiler::new(loader);
        compiler.compile_all(&graph)
    }

    /// LSP: build only to Semantic (stops before CPS lowering).
    pub fn semantic_at(&mut self, module: &Module) -> Result<SemanticArtifact, BuildError> {
        let hash = sha256_hex(format!("{:?}", module).as_bytes());
        let key = self.cache_key("local", &hash, "semantic");

        if let Some(cached) = self.cache.get::<SemanticArtifact>(&key) {
            return Ok(cached);
        }

        let semantic = SemanticStage.execute(module.clone(), &self.build_ctx())?;
        self.cache.put(key, semantic.clone());
        Ok(semantic)
    }

    /// Access the event router (for adding sinks after construction).
    pub fn router_mut(&mut self) -> &mut EventRouter {
        &mut self.router
    }

    pub fn events_ref(&self) -> Option<&dyn EventHandler> {
        if self.router.is_empty() {
            None
        } else {
            Some(&self.router)
        }
    }

    // ── Internal pipeline ──

    fn build_ctx(&self) -> crate::protocol::BuildContext<'_> {
        crate::protocol::BuildContext {
            events: self.events_ref(),
        }
    }

    fn frontend(&self, source: &str) -> Result<Module, BuildError> {
        FrontendStage.execute(source, &self.build_ctx())
    }

    fn build_cps(&mut self, source: &str) -> Result<CpsModule, BuildError> {
        let key = self.cache_key_source(source, "cps");
        if let Some(cached) = self.cache.get::<CpsModule>(&key) {
            return Ok(cached);
        }

        let module = self.frontend(source)?;
        let semantic = self.semantic_at(&module)?;

        let events = self.events_ref();
        let cps_stage = CpsBuildStage { events };
        let mut cps = cps_stage.execute(&module, &self.build_ctx())?;

        flatten_module(&mut cps);
        self.pipeline.run(&mut cps, events);

        self.cache.put(key, cps.clone());
        Ok(cps)
    }

    fn execute(&mut self, cps: CpsModule) -> Result<RunOutcome, BuildError> {
        let stage = VmExecStage {
            max_loop_iterations: self.max_loop_iterations,
        };
        stage.execute(cps, &self.build_ctx())
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        self.router.close_all();
    }
}
