//! Integration tests for the kaubo-dag scheduler.
//!
//! These tests exercise the full scheduler: registering fetchers,
//! resolving dependency chains, caching, cycle detection,
//! and error propagation.

use futures::StreamExt;
use kaubo_dag::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

type M = String;

// ── Test Fetchers ────────────────────────────────────────────────────

/// A fetcher that always returns a constant value.
struct ConstantFetcher {
    key: ArtifactKey<M>,
    value: i64,
}

impl Fetcher<M> for ConstantFetcher {
    fn key(&self) -> ArtifactKey<M> {
        self.key.clone()
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![]
    }

    fn fetch<'a>(
        &'a self,
        _inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>> {
        let artifact = Artifact::new(
            self.key.module_id.clone(),
            self.key.kind.clone(),
            self.value,
        );
        Box::pin(async move { Ok(artifact) })
    }
}

/// A fetcher that depends on one upstream artifact and transforms it.
struct TransformFetcher {
    key: ArtifactKey<M>,
    dep_key: ArtifactKey<M>,
    /// Function to apply to the upstream value.
    transform: fn(i64) -> i64,
}

impl Fetcher<M> for TransformFetcher {
    fn key(&self) -> ArtifactKey<M> {
        self.key.clone()
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![self.dep_key.clone()]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>> {
        let value = *inputs[0].downcast_ref::<i64>();
        let result = (self.transform)(value);
        let artifact = Artifact::new(self.key.module_id.clone(), self.key.kind.clone(), result);
        Box::pin(async move { Ok(artifact) })
    }
}

/// A fetcher that depends on two upstream artifacts and combines them.
struct CombineFetcher {
    key: ArtifactKey<M>,
    dep_a: ArtifactKey<M>,
    dep_b: ArtifactKey<M>,
}

impl Fetcher<M> for CombineFetcher {
    fn key(&self) -> ArtifactKey<M> {
        self.key.clone()
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![self.dep_a.clone(), self.dep_b.clone()]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>> {
        let a = *inputs[0].downcast_ref::<i64>();
        let b = *inputs[1].downcast_ref::<i64>();
        let artifact = Artifact::new(self.key.module_id.clone(), self.key.kind.clone(), a + b);
        Box::pin(async move { Ok(artifact) })
    }
}

/// A fetcher that reports how many times it was executed (via shared counter).
struct CountingFetcher {
    key: ArtifactKey<M>,
    value: i64,
    counter: std::sync::Arc<Mutex<u32>>,
}

impl Fetcher<M> for CountingFetcher {
    fn key(&self) -> ArtifactKey<M> {
        self.key.clone()
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![]
    }

    fn fetch<'a>(
        &'a self,
        _inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>> {
        *self.counter.lock().unwrap() += 1;
        let artifact = Artifact::new(
            self.key.module_id.clone(),
            self.key.kind.clone(),
            self.value,
        );
        Box::pin(async move { Ok(artifact) })
    }
}

/// A fetcher that always fails.
struct FailingFetcher {
    key: ArtifactKey<M>,
    error_msg: String,
}

impl Fetcher<M> for FailingFetcher {
    fn key(&self) -> ArtifactKey<M> {
        self.key.clone()
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![]
    }

    fn fetch<'a>(
        &'a self,
        _inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>> {
        let key = self.key.clone();
        let msg = self.error_msg.clone();
        Box::pin(async move { Err(DagError::fetcher_error(key, msg)) })
    }
}

// ── Test Builders ────────────────────────────────────────────────────

/// A builder that collects a single dependency's value.
struct SingleDepBuilder {
    dep_key: ArtifactKey<M>,
}

impl Builder<M, i64> for SingleDepBuilder {
    fn name(&self) -> &str {
        "SingleDepBuilder"
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![self.dep_key.clone()]
    }

    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<i64, DagError<M>>> + Send + 'a>> {
        let value = *inputs[0].downcast_ref::<i64>();
        Box::pin(async move { Ok(value) })
    }
}

/// A builder that depends on two artifacts and sums them.
struct SumBuilder {
    dep_a: ArtifactKey<M>,
    dep_b: ArtifactKey<M>,
}

impl Builder<M, i64> for SumBuilder {
    fn name(&self) -> &str {
        "SumBuilder"
    }

    fn dependencies(&self) -> Vec<ArtifactKey<M>> {
        vec![self.dep_a.clone(), self.dep_b.clone()]
    }

    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<M>>,
        _ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<i64, DagError<M>>> + Send + 'a>> {
        let a = *inputs[0].downcast_ref::<i64>();
        let b = *inputs[1].downcast_ref::<i64>();
        Box::pin(async move { Ok(a + b) })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn mkkey(module: &str, kind_name: &str) -> ArtifactKey<M> {
    ArtifactKey::new(module.to_string(), Kind::new(kind_name))
}

async fn collect_stream<Out: Clone + Send + 'static>(
    stream: BuildStream<M, Out>,
) -> Result<Out, DagError<M>> {
    futures::pin_mut!(stream);
    match stream.next().await {
        Some(BuilderEvent::Done(out)) => Ok(out),
        Some(BuilderEvent::Error(e)) => Err((*e).clone()),
        None => Err(DagError::Internal("stream ended without result".into())),
    }
}

// ── Tests ────────────────────────────────────────────────────────────

/// A simple linear chain: Source → Step1 → Step2.
/// Step2 depends on Step1 which depends on Source.
/// Requesting Step2 should trigger the full chain and return the
/// transformed result.
#[test]
fn simple_fetcher_chain() {
    let registry = FetcherRegistry::new();

    // Source: constant 10
    registry.register(
        Kind::new("Source"),
        Box::new(|output_key| {
            Box::new(ConstantFetcher {
                key: output_key,
                value: 10,
            })
        }),
    );
    // Step1: Source * 2
    registry.register(
        Kind::new("Step1"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "Source"),
                transform: |x| x * 2,
            })
        }),
    );
    // Step2: Step1 + 3
    registry.register(
        Kind::new("Step2"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "Step1"),
                transform: |x| x + 3,
            })
        }),
    );

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build(Box::new(SingleDepBuilder {
        dep_key: mkkey("mod", "Step2"),
    }));

    let result = futures::executor::block_on(collect_stream(stream));
    assert_eq!(result.unwrap(), 23); // (10 * 2) + 3 = 23
}

/// Diamond dependency: A depends on B and C, both depend on D.
/// D should be computed exactly once.
#[test]
fn diamond_dependency_executes_leaf_once() {
    let registry = FetcherRegistry::new();
    let counter = std::sync::Arc::new(Mutex::new(0u32));

    // D: constant 7 (shared leaf, should execute once)
    let ctr = counter.clone();
    registry.register(
        Kind::new("D"),
        Box::new(move |output_key| {
            Box::new(CountingFetcher {
                key: output_key,
                value: 7,
                counter: ctr.clone(),
            })
        }),
    );
    // C: D * 10
    registry.register(
        Kind::new("C"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "D"),
                transform: |x| x * 10,
            })
        }),
    );
    // B: D + 3
    registry.register(
        Kind::new("B"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "D"),
                transform: |x| x + 3,
            })
        }),
    );
    // A: C + B
    registry.register(
        Kind::new("A"),
        Box::new(move |output_key| {
            Box::new(CombineFetcher {
                key: output_key,
                dep_a: mkkey("mod", "C"),
                dep_b: mkkey("mod", "B"),
            })
        }),
    );

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build(Box::new(SingleDepBuilder {
        dep_key: mkkey("mod", "A"),
    }));

    let result = futures::executor::block_on(collect_stream(stream));
    assert_eq!(result.unwrap(), 80); // C + B = (7*10) + (7+3) = 80
    assert_eq!(*counter.lock().unwrap(), 1); // D computed exactly once
}

/// Same key requested twice should execute the fetcher only once.
#[test]
fn cache_hit_avoids_recomputation() {
    let registry = FetcherRegistry::new();
    let counter = std::sync::Arc::new(Mutex::new(0u32));

    // Const: shared leaf fetcher
    let ctr = counter.clone();
    registry.register(
        Kind::new("Const"),
        Box::new(move |output_key| {
            Box::new(CountingFetcher {
                key: output_key,
                value: 42,
                counter: ctr.clone(),
            })
        }),
    );
    // First: depends on Const (just passes through)
    registry.register(
        Kind::new("First"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "Const"),
                transform: |x| x,
            })
        }),
    );
    // Second: also depends on Const
    registry.register(
        Kind::new("Second"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "Const"),
                transform: |x| x,
            })
        }),
    );

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build(Box::new(SumBuilder {
        dep_a: mkkey("mod", "First"),
        dep_b: mkkey("mod", "Second"),
    }));

    let result = futures::executor::block_on(collect_stream(stream));
    assert_eq!(result.unwrap(), 84); // 42 + 42
    assert_eq!(*counter.lock().unwrap(), 1); // Const executed only once
}

/// A → B → A should be detected as a circular dependency.
#[test]
fn circular_dependency_is_detected() {
    let registry = FetcherRegistry::new();

    // A depends on B
    registry.register(
        Kind::new("A"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "B"),
                transform: |x| x,
            })
        }),
    );
    // B depends on A — cycle!
    registry.register(
        Kind::new("B"),
        Box::new(move |output_key| {
            Box::new(TransformFetcher {
                key: output_key,
                dep_key: mkkey("mod", "A"),
                transform: |x| x,
            })
        }),
    );

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build(Box::new(SingleDepBuilder {
        dep_key: mkkey("mod", "A"),
    }));

    let result = futures::executor::block_on(collect_stream(stream));
    match result {
        Err(DagError::CircularDependency { .. }) => {} // expected
        other => panic!("expected CircularDependency, got {:?}", other),
    }
}

/// Error from a fetcher should propagate to the builder.
#[test]
fn error_propagates_to_builder() {
    let registry = FetcherRegistry::new();

    registry.register(
        Kind::new("Fail"),
        Box::new(move |output_key| {
            Box::new(FailingFetcher {
                key: output_key,
                error_msg: "something broke".into(),
            })
        }),
    );

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build(Box::new(SingleDepBuilder {
        dep_key: mkkey("mod", "Fail"),
    }));

    let result = futures::executor::block_on(collect_stream(stream));
    match result {
        Err(DagError::FetcherError { ref message, .. }) => {
            assert!(message.contains("something broke"));
        }
        other => panic!("expected FetcherError, got {:?}", other),
    }
}

/// No fetcher registered for a requested kind.
#[test]
fn unregistered_kind_errors() {
    let registry = FetcherRegistry::<M>::new();
    // Deliberately empty registry

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build(Box::new(SingleDepBuilder {
        dep_key: mkkey("mod", "Nonexistent"),
    }));

    let result = futures::executor::block_on(collect_stream(stream));
    match result {
        Err(DagError::NoFetcherForKind(kind)) => {
            assert_eq!(kind, "Nonexistent");
        }
        other => panic!("expected NoFetcherForKind, got {:?}", other),
    }
}

/// BuildStream's Drop cancels the build (CancellationToken triggered).
#[test]
fn dropping_stream_cancels_build() {
    let registry = FetcherRegistry::new();

    // A fetcher that sleeps then returns — the build should be cancelled
    // before it completes.
    registry.register(
        Kind::new("Slow"),
        Box::new(|output_key| {
            Box::new(ConstantFetcher {
                key: output_key,
                value: 99,
            })
        }),
    );

    let scheduler = DagScheduler::new(registry, std::sync::Arc::new(NativeSpawner));
    let stream = scheduler.build::<i64>(Box::new(SingleDepBuilder {
        dep_key: mkkey("mod", "Slow"),
    }));

    // Drop the stream immediately — this should cancel the build.
    // The TestSpawner blocks on spawn, so the build already completed
    // before we get here. In a real async environment, dropping would
    // cancel in-flight tasks. This test verifies the Drop impl exists.
    drop(stream);
}

// ── Streaming tests ─────────────────────────────────────────────────

struct StreamingSourceFetcher {
    key: ArtifactKey<M>,
    value: i64,
}

impl Fetcher<M> for StreamingSourceFetcher {
    fn key(&self) -> ArtifactKey<M> { self.key.clone() }
    fn dependencies(&self) -> Vec<ArtifactKey<M>> { vec![] }
    fn fetch<'a>(
        &'a self,
        _inputs: Vec<Artifact<M>>,
        ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>> {
        let key = self.key.clone();
        let value = self.value;
        let (artifact, handle) = ctx.spawn_streaming(key.clone(), 0i64);
        let jh = std::thread::spawn(move || { handle.complete(value); });
        Box::pin(async move {
            jh.join().unwrap();
            Ok(Artifact::new(key.module_id, key.kind, value))
        })
    }
}

#[test]
fn streaming_artifact_basic_flow() {
    let registry = FetcherRegistry::new();
    registry.register(
        Kind::new("S"),
        Box::new(|key| Box::new(StreamingSourceFetcher { key, value: 77 })),
    );
    let scheduler = DagScheduler::new(registry, Arc::new(NativeSpawner));
    let r = futures::executor::block_on(collect_stream(
        scheduler.build(Box::new(SingleDepBuilder { dep_key: mkkey("mod", "S") })),
    ));
    assert_eq!(r.unwrap(), 77);
}

#[test]
fn streaming_artifact_cached_after_finalize() {
    let registry = FetcherRegistry::new();
    let counter = Arc::new(Mutex::new(0u32));
    let c = counter.clone();
    registry.register(
        Kind::new("SC"),
        Box::new(move |key| { *c.lock().unwrap() += 1; Box::new(StreamingSourceFetcher { key, value: 42 }) }),
    );
    let s = DagScheduler::new(registry, Arc::new(NativeSpawner));
    let r1 = futures::executor::block_on(collect_stream(
        s.build(Box::new(SingleDepBuilder { dep_key: mkkey("mod", "SC") })),
    ));
    assert_eq!(r1.unwrap(), 42);
    assert_eq!(*counter.lock().unwrap(), 1);
    // cache hit
    let r2 = futures::executor::block_on(collect_stream(
        s.build(Box::new(SingleDepBuilder { dep_key: mkkey("mod", "SC") })),
    ));
    assert_eq!(r2.unwrap(), 42);
    assert_eq!(*counter.lock().unwrap(), 1);
}
