//! Protocol layer — contracts for stages, passes, and pipelines.
//!
//! Stage/Pipeline are retained for backward compat (LSP uses Stage directly).

use kaubo_ir::cps::CpsModule;
use kaubo_log::EventHandler;
use std::fmt;

/// A compilation stage: input I → output O.
pub trait Stage<I, O> {
    fn name(&self) -> &str;
    fn execute(&self, input: I, ctx: &BuildContext) -> Result<O, BuildError>;
}

/// Context passed to every Stage during execution.
pub struct BuildContext<'a> {
    pub events: Option<&'a dyn EventHandler>,
}

/// A pass transforms a CpsModule in-place.
pub trait Pass: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>);
}

/// Blanket impl so `Arc<dyn Pass>` can be used as a Pass.
impl Pass for std::sync::Arc<dyn Pass> {
    fn name(&self) -> &str { (**self).name() }
    fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>) {
        (**self).run(module, events);
    }
}

/// Ordered pipeline of passes. Uses `Arc` so the pipeline is cheap to clone.
#[derive(Default, Clone)]
pub struct Pipeline {
    passes: Vec<std::sync::Arc<dyn Pass>>,
}

impl Pipeline {
    pub fn new() -> Self { Self { passes: vec![] } }
    pub fn add(mut self, pass: impl Pass + 'static) -> Self {
        self.passes.push(std::sync::Arc::new(pass));
        self
    }
    pub fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>) {
        for pass in &self.passes { pass.run(module, events); }
    }
    pub fn is_empty(&self) -> bool { self.passes.is_empty() }
}

/// Errors that can occur during a build.
#[derive(Debug, Clone)]
pub enum BuildError {
    Parse(String),
    Infer(String),
    Build(String),
    Load(String),
    Runtime(String),
    Bug(String),
    /// 循环模块依赖。
    CircularImport {
        cycle: Vec<String>,
    },
    /// 导入的模块不存在。
    ImportNotFound {
        path: String,
        name: String,
    },
    /// 导入的符号在被导入模块中未导出。
    ExportNotFound {
        name: String,
        path: String,
    },
    /// 同名符号冲突（同一模块重复导入同名符号）。
    SymbolConflict {
        name: String,
        path1: String,
        path2: String,
    },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Parse(msg) => write!(f, "parse: {msg}"),
            BuildError::Infer(msg) => write!(f, "infer: {msg}"),
            BuildError::Build(msg) => write!(f, "build: {msg}"),
            BuildError::Load(msg) => write!(f, "load: {msg}"),
            BuildError::Runtime(msg) => write!(f, "runtime: {msg}"),
            BuildError::Bug(msg) => write!(f, "bug: {msg}"),
            BuildError::CircularImport { cycle } => {
                write!(f, "circular import: {}", cycle.join(" → "))
            }
            BuildError::ImportNotFound { path, name } => {
                write!(f, "import not found: '{name}' in {path}")
            }
            BuildError::ExportNotFound { name, path } => {
                write!(f, "export '{name}' not found in module {path}")
            }
            BuildError::SymbolConflict { name, path1, path2 } => {
                write!(
                    f,
                    "symbol conflict: '{name}' imported from both {path1} and {path2}"
                )
            }
        }
    }
}
