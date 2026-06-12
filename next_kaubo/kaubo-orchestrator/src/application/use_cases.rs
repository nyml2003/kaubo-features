//! 应用层 — 用例编排

use crate::domain::interfaces::{OutputPort, Platform, SourceRepo};
use crate::domain::models::hir::HirModule;
use crate::domain::optimization::pass::OptimizationPipeline;
use std::sync::Arc;

/// 编译用例：源码 → 优化 → Chunk
pub struct CompileUseCase {
    pub platform: Arc<dyn Platform>,
    pub source_repo: Arc<dyn SourceRepo>,
    pub optimizer: OptimizationPipeline,
}

impl CompileUseCase {
    pub fn new(
        platform: Arc<dyn Platform>,
        source_repo: Arc<dyn SourceRepo>,
    ) -> Self {
        Self {
            platform,
            source_repo,
            optimizer: OptimizationPipeline::default(),
        }
    }

    /// 执行编译
    pub fn execute(&self, entry: &str) -> Result<(), String> {
        let _source = self.source_repo.read(entry)?;
        // TODO: actual compilation pipeline
        Ok(())
    }
}

/// 运行用例：编译 + VM 执行
pub struct RunUseCase {
    pub platform: Arc<dyn Platform>,
    pub source_repo: Arc<dyn SourceRepo>,
    pub output: Arc<dyn OutputPort>,
}

impl RunUseCase {
    pub fn new(
        platform: Arc<dyn Platform>,
        source_repo: Arc<dyn SourceRepo>,
        output: Arc<dyn OutputPort>,
    ) -> Self {
        Self {
            platform,
            source_repo,
            output,
        }
    }

    /// 执行运行
    pub fn execute(&self, entry: &str) -> Result<(), String> {
        let _source = self.source_repo.read(entry)?;
        // TODO: parse → lower → optimize → compile → run
        Ok(())
    }
}

/// HIR 优化器服务
pub struct OptimizerService {
    pipeline: OptimizationPipeline,
}

impl OptimizerService {
    pub fn new(pipeline: OptimizationPipeline) -> Self {
        Self { pipeline }
    }

    pub fn optimize(&self, module: HirModule) -> Result<HirModule, String> {
        self.pipeline.run(module)
    }
}

impl Default for OptimizerService {
    fn default() -> Self {
        Self {
            pipeline: crate::domain::optimization::pass::create_default_pipeline(),
        }
    }
}
