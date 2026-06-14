//! Type-safe Pipeline Framework
//!
//! 每个阶段是 In → Out 的纯函数，通过类型安全的链式组合构建流水线。

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

// ============================================================
// Context
// ============================================================

#[derive(Clone)]
pub struct PipelineCtx {
    pub source_path: Option<String>,
    pub verbose: bool,
    pub metadata: std::collections::HashMap<String, String>,
}

impl PipelineCtx {
    pub fn new() -> Self {
        Self { source_path: None, verbose: false, metadata: std::collections::HashMap::new() }
    }
    pub fn with_verbose(mut self, v: bool) -> Self { self.verbose = v; self }
    pub fn with_source(mut self, p: &str) -> Self { self.source_path = Some(p.to_string()); self }
}

// ============================================================
// Capability
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    Observable,
    Measurable,
    Countable,
    Serializable,
    Dumpable,
    Executable,
}

impl Capability {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "observable" | "Observable" => Some(Capability::Observable),
            "measurable" | "Measurable" => Some(Capability::Measurable),
            "countable" | "Countable" => Some(Capability::Countable),
            "serializable" | "Serializable" => Some(Capability::Serializable),
            "dumpable" | "Dumpable" => Some(Capability::Dumpable),
            "executable" | "Executable" => Some(Capability::Executable),
            _ => None,
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityConfig {
    pub capability: Capability,
    pub action: String,
}

// ============================================================
// Stage trait
// ============================================================

pub trait Stage<In, Out>: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&self, input: In, ctx: &PipelineCtx) -> Result<Out, String>;

    fn capabilities(&self) -> Vec<Capability> { vec![] }

    fn handle_capability(&self, _cap: Capability, _output: &dyn std::any::Any)
        -> Result<String, String>
    {
        Err("unsupported capability".into())
    }
}

// ============================================================
// Pipeline
// ============================================================

pub struct Pipeline<In, Out> {
    runner: Box<dyn Fn(In, &PipelineCtx) -> Result<Out, String> + Send + Sync + 'static>,
    description: String,
    caps: Vec<Capability>,
    cap_handlers: Vec<Box<dyn Fn(&dyn std::any::Any) -> Result<String, String> + Send + Sync>>,
}

impl<A: 'static, B: 'static> Pipeline<A, B> {
    pub fn new(stage: impl Stage<A, B> + 'static) -> Self {
        let name = stage.name();
        let caps = stage.capabilities();
        let cap_handlers: Vec<Box<dyn Fn(&dyn std::any::Any) -> Result<String, String> + Send + Sync>> =
            caps.iter().map(move |c| {
                let c = *c;
                Box::new(move |output: &dyn std::any::Any| -> Result<String, String> {
                    Err(format!("capability {:?}: no handler registered", c))
                }) as Box<dyn Fn(&dyn std::any::Any) -> Result<String, String> + Send + Sync>
            }).collect();
        Self {
            runner: Box::new(move |input, ctx| stage.run(input, ctx)),
            description: name.to_string(),
            caps,
            cap_handlers,
        }
    }

    pub fn from_fn<F>(f: F) -> Self
    where
        F: Fn(A, &PipelineCtx) -> Result<B, String> + Send + Sync + 'static,
    {
        Self {
            runner: Box::new(f),
            description: "custom_fn".to_string(),
            caps: vec![],
            cap_handlers: vec![],
        }
    }

    pub fn then<C>(self, stage: impl Stage<B, C> + 'static) -> Pipeline<A, C> {
        let next_name = stage.name();
        let desc = format!("{} → {}", self.description, next_name);
        let new_caps = stage.capabilities();
        let new_handlers: Vec<Box<dyn Fn(&dyn std::any::Any) -> Result<String, String> + Send + Sync>> =
            new_caps.iter().enumerate().map(|(_, _c)| {
                Box::new(move |_output: &dyn std::any::Any| -> Result<String, String> {
                    Err("capability handler unreachable (use with_capabilities)".into())
                }) as Box<_>
            }).collect();
        Pipeline {
            runner: Box::new(move |input, ctx| {
                let mid = (self.runner)(input, ctx)?;
                stage.run(mid, ctx)
            }),
            description: desc,
            caps: new_caps,
            cap_handlers: new_handlers,
        }
    }

    pub fn then_fn<C, F>(self, f: F) -> Pipeline<A, C>
    where
        F: Fn(B, &PipelineCtx) -> Result<C, String> + Send + Sync + 'static,
    {
        let desc = format!("{} → fn", self.description);
        Pipeline {
            runner: Box::new(move |input, ctx| {
                let mid = (self.runner)(input, ctx)?;
                f(mid, ctx)
            }),
            description: desc,
            caps: vec![],
            cap_handlers: vec![],
        }
    }

    pub fn observe<F>(self, f: F) -> Pipeline<A, B>
    where
        F: Fn(&B) + Send + Sync + 'static,
    {
        let desc = self.description.clone();
        let caps = self.caps.clone();
        Pipeline {
            runner: Box::new(move |input, ctx| {
                let output = (self.runner)(input, ctx)?;
                f(&output);
                Ok(output)
            }),
            description: desc,
            caps,
            cap_handlers: vec![],
        }
    }

    pub fn adapt<C>(self, adapter: impl Adapter<B, C> + 'static) -> Pipeline<A, C> {
        let adapter_name = adapter.name();
        let desc = format!("{} → [{}]", self.description, adapter_name);
        Pipeline {
            runner: Box::new(move |input, ctx| {
                let output = (self.runner)(input, ctx)?;
                Ok(adapter.adapt(&output))
            }),
            description: desc,
            caps: vec![],
            cap_handlers: vec![],
        }
    }

    pub fn with_capabilities(mut self, config: &[CapabilityConfig]) -> Self
    where
        B: 'static,
    {
        let configs: Vec<_> = config.iter().map(|c| (c.capability, c.action.clone())).collect();
        for (cap, action) in &configs {
            if self.caps.contains(cap) {
                let Some(idx) = self.caps.iter().position(|c| *c == *cap) else {
                    continue;
                };
                let action = action.clone();
                let handler_fn: Box<dyn Fn(&dyn std::any::Any) -> Result<String, String> + Send + Sync + 'static> =
                    if action == "count" {
                        let cap = *cap;
                        Box::new(move |_output: &dyn std::any::Any| -> Result<String, String> {
                            Ok(format!("capability {:?} active", cap))
                        })
                    } else if action == "print" {
                        let cap = *cap;
                        Box::new(move |output: &dyn std::any::Any| -> Result<String, String> {
                            Ok(format!("[{}] {:?}", cap, output.type_id()))
                        })
                    } else {
                        Box::new(move |_output: &dyn std::any::Any| -> Result<String, String> {
                            Ok("ok".to_string())
                        })
                    };
                if idx < self.cap_handlers.len() {
                    self.cap_handlers[idx] = handler_fn;
                }
            }
        }
        self
    }

    pub fn caps(&self) -> &[Capability] { &self.caps }

    pub fn run(&self, input: A, ctx: &PipelineCtx) -> Result<B, String> {
        (self.runner)(input, ctx)
    }

    pub fn describe(&self) -> &str { &self.description }
}

// ============================================================
// Observer
// ============================================================

pub trait Observer<T>: Send + Sync {
    fn on_complete(&self, output: &T, stage_name: &str, timing: Duration);
    fn on_error(&self, error: &str, stage_name: &str);
}

pub fn observe_with<T>(observer: Arc<dyn Observer<T>>, stage_name: &'static str) -> impl Fn(&T) {
    let obs = observer.clone();
    move |output: &T| {
        obs.on_complete(output, stage_name, Duration::ZERO);
    }
}

pub struct CompositeObserver<T> {
    observers: Vec<Arc<dyn Observer<T>>>,
}

impl<T> CompositeObserver<T> {
    pub fn new() -> Self { Self { observers: Vec::new() } }
    pub fn add(mut self, o: Arc<dyn Observer<T>>) -> Self { self.observers.push(o); self }
}

impl<T: fmt::Debug> Observer<T> for CompositeObserver<T> {
    fn on_complete(&self, output: &T, stage_name: &str, timing: Duration) {
        for o in &self.observers {
            o.on_complete(output, stage_name, timing);
        }
    }
    fn on_error(&self, error: &str, stage_name: &str) {
        for o in &self.observers {
            o.on_error(error, stage_name);
        }
    }
}

// ============================================================
// Adapter trait + built-in adapters
// ============================================================

pub trait Adapter<From, To>: Send + Sync {
    fn name(&self) -> &'static str;
    fn adapt(&self, from: &From) -> To;
}

pub struct ToStringAdapter;

impl<T: fmt::Display> Adapter<T, String> for ToStringAdapter {
    fn name(&self) -> &'static str { "to_string" }
    fn adapt(&self, from: &T) -> String { from.to_string() }
}

pub struct ToJsonAdapter;

impl<T: serde::Serialize> Adapter<T, String> for ToJsonAdapter {
    fn name(&self) -> &'static str { "to_json" }
    fn adapt(&self, from: &T) -> String {
        serde_json::to_string_pretty(from).unwrap_or_else(|e| format!("{{error: \"{}\"}}", e))
    }
}

pub struct FileEmitter {
    path: String,
}

impl FileEmitter {
    pub fn new(path: impl Into<String>) -> Self { Self { path: path.into() } }
}

impl Adapter<Vec<u8>, String> for FileEmitter {
    fn name(&self) -> &'static str { "file_emitter" }
    fn adapt(&self, data: &Vec<u8>) -> String {
        match std::fs::write(&self.path, data) {
            Ok(_) => format!("wrote {} bytes to {}", data.len(), self.path),
            Err(e) => format!("write error: {}", e),
        }
    }
}

pub struct ConsoleEmitter;

impl<T: fmt::Display> Adapter<T, ()> for ConsoleEmitter {
    fn name(&self) -> &'static str { "console" }
    fn adapt(&self, from: &T) -> () { println!("{}", from); }
}

impl<T: fmt::Debug> Adapter<T, String> for ConsoleEmitter {
    fn name(&self) -> &'static str { "console_debug" }
    fn adapt(&self, from: &T) -> String { let s = format!("{:?}", from); println!("{}", s); s }
}
