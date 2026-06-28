//! Protocol layer — contracts for stages, passes, pipelines, and caching.
//!
//! Each trait defines *what* a computation looks like without prescribing
//! *who* calls it or *how* inputs/outputs are structured.

use kaubo_ir::cps::CpsModule;
use kaubo_log::EventHandler;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;

/// A compilation stage: input I → output O.
///
/// I and O are generic — each Stage defines its own concrete types.
/// The Coordinator wires them together.
pub trait Stage<I, O> {
    /// Human-readable name (for logging and cache key prefix).
    fn name(&self) -> &str;

    /// Execute the computation.
    fn execute(&self, input: I, ctx: &BuildContext) -> Result<O, BuildError>;
}

/// Context passed to every Stage during execution.
pub struct BuildContext<'a> {
    /// Event handler for structured logging (may be an EventRouter).
    pub events: Option<&'a dyn EventHandler>,
}

/// A pass transforms a CpsModule in-place.
pub trait Pass {
    fn name(&self) -> &str;
    fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>);
}

/// Blanket impl so `Box<dyn Pass>` can be used as a Pass.
impl Pass for Box<dyn Pass> {
    fn name(&self) -> &str {
        (**self).name()
    }
    fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>) {
        (**self).run(module, events);
    }
}

/// An ordered pipeline of passes.
#[derive(Default)]
pub struct Pipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { passes: vec![] }
    }

    pub fn add(mut self, pass: impl Pass + 'static) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    pub fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>) {
        for pass in &self.passes {
            pass.run(module, events);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }
}

/// Key-value cache for build artifacts.
///
/// Keys are opaque strings.  Type-safety is enforced via `downcast` at
/// runtime — if a Stage stores type A and another retrieves type B under
/// the same key, `get` panics with an explicit message.
pub trait ArtifactCache {
    fn get<T: Clone + 'static>(&self, key: &str) -> Option<T>;
    fn put<T: 'static + Send + Sync>(&mut self, key: String, value: T);
}

/// In-memory cache backed by a HashMap.
pub struct MemoryCache {
    store: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self { store: HashMap::new() }
    }
}

impl ArtifactCache for MemoryCache {
    fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let any = self.store.get(key)?;
        Some(
            any.downcast_ref::<T>()
                .expect("cache: type mismatch — did a Stage store a different type under this key?")
                .clone(),
        )
    }

    fn put<T: 'static + Send + Sync>(&mut self, key: String, value: T) {
        self.store.insert(key, Box::new(value));
    }
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
        }
    }
}
