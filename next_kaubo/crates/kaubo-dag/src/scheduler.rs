//! DagScheduler — the core DAG orchestration engine.
//!
//! The scheduler coordinates [`Fetcher`](crate::Fetcher) and
//! [`Builder`](crate::Builder) instances over a dynamic, lazily-expanded
//! dependency graph. All coordination happens through
//! `request_dependency()` calls, which trigger fetcher creation, cache
//! lookup, and wake-up chains.

use crate::builder::{Builder, BuilderEvent};
use crate::cancel::CancellationToken;
use crate::error::DagError;
use crate::fetcher::{FetchContext, ProgressEvent, ResultEvent};
use crate::registry::FetcherRegistry;
use crate::spawner::Spawner;
use crate::store::ArtifactStore;
use crate::types::{Artifact, ArtifactKey};
use futures::channel::{mpsc, oneshot};
use futures::stream::Stream;
use std::collections::HashSet;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

// ── DagScheduler ─────────────────────────────────────────────────────

/// The core DAG scheduler.
///
/// Coordinates `Fetcher` and `Builder` instances over a dynamic,
/// lazily-expanded dependency graph keyed by [`ArtifactKey<M>`].
///
/// # Type parameters
///
/// - `M` — the module identifier type. All fetchers and builders in a
///   single scheduler instance share the same `M`.
///
/// # Lifecycle
///
/// 1. Create a [`FetcherRegistry`] and register all fetcher factories.
/// 2. Create a [`Spawner`] (or use a test spawner).
/// 3. Call [`DagScheduler::new`] to create the scheduler.
/// 4. Call [`build`](DagScheduler::build) with a builder to start a build.
pub struct DagScheduler<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Artifact storage (ready cache + in-flight tracking + reverse deps).
    pub(crate) store: ArtifactStore<M>,

    /// Registry of fetcher factories.
    pub(crate) registry: Arc<FetcherRegistry<M>>,

    /// Platform spawner for background work.
    spawner: Arc<dyn Spawner>,
}

impl<M> DagScheduler<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Create a new scheduler, wrapped in `Arc` for shared ownership.
    ///
    /// The registry should already have all fetcher factories registered
    /// before any calls to [`build`](DagScheduler::build).
    pub fn new(registry: FetcherRegistry<M>, spawner: Arc<dyn Spawner>) -> Arc<Self> {
        Arc::new(DagScheduler {
            store: ArtifactStore::new(),
            registry: Arc::new(registry),
            spawner,
        })
    }

    /// Execute a Builder and return a stream of [`BuilderEvent`]s.
    ///
    /// This is the primary public API. The builder's dependencies are
    /// resolved via the DAG, and the final result (or error) is streamed
    /// back to the caller.
    ///
    /// # Cancellation
    ///
    /// Dropping the returned stream triggers cancellation of all in-flight
    /// tasks for this build.
    pub fn build<Out>(
        self: &Arc<Self>,
        builder: Box<dyn Builder<M, Out>>,
    ) -> BuildStream<M, Out>
    where
        Out: Clone + Send + 'static,
    {
        let (progress_tx, _progress_rx) = mpsc::unbounded::<ProgressEvent<M>>();
        // Phase 2: switch to bounded mpsc::channel(64) with a proper
        // streaming consumer that drains events. Using unbounded here
        // prevents deadlock when >64 fetchers complete before the
        // consumer starts polling.
        let (result_tx, _result_rx) = mpsc::unbounded::<ResultEvent<M>>();
        let (done_tx, done_rx) = oneshot::channel::<Result<Out, DagError<M>>>();
        let cancel = self.spawner.cancellation_token();

        let scheduler = Arc::clone(self);

        // Build the initial fetch context
        let mut ctx = FetchContext {
            scheduler: Arc::clone(&scheduler),
            call_stack: Vec::new(),
            call_stack_set: HashSet::new(),
            cancel: cancel.child(),
            progress_tx: progress_tx.clone(),
            result_tx: result_tx.clone(),
        };

        // Spawn the builder execution as a background task
        self.spawner.spawn(Box::pin(async move {
            // Resolve all declared dependencies first
            let mut inputs: Vec<Artifact<M>> = Vec::new();
            for dep_key in builder.dependencies() {
                match ctx.request_dependency(dep_key).await {
                    Ok(artifact) => inputs.push(artifact),
                    Err(e) => {
                        let _ = done_tx.send(Err(e));
                        return;
                    }
                }
            }

            // Execute the builder and send the result
            let result = builder.build(inputs, &mut ctx).await;
            let _ = done_tx.send(result);
        }));

        BuildStream {
            done_rx,
            _cancel: cancel,
        }
    }

    /// Internal: resolve a dependency.
    ///
    /// Called from [`FetchContext::request_dependency`]. Returns a boxed
    /// future to avoid infinite-size issues with indirect async recursion
    /// (scheduler → fetcher → context → scheduler).
    ///
    /// `parent_call_stack` and `parent_call_stack_set` carry the call stack
    /// from the caller's [`FetchContext`] so that sub-fetchers inherit it
    /// for cross-fetcher cycle detection.
    #[allow(clippy::type_complexity)]
    pub(crate) fn request_dependency(
        self: &Arc<Self>,
        key: ArtifactKey<M>,
        progress_tx: mpsc::UnboundedSender<ProgressEvent<M>>,
        result_tx: mpsc::UnboundedSender<ResultEvent<M>>,
        cancel: CancellationToken,
        parent_call_stack: Vec<ArtifactKey<M>>,
        parent_call_stack_set: HashSet<ArtifactKey<M>>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send>> {
        let scheduler = Arc::clone(self);

        Box::pin(async move {
            // 1. Check ready cache — only return if fully complete
            if let Some(artifact) = scheduler.store.get_ready(&key) {
                if artifact.is_final {
                    let _ = progress_tx.unbounded_send(ProgressEvent::DependencyReady {
                        key: key.clone(),
                    });
                    return Ok(artifact);
                }
                // is_final=false: artifact is being streamed. Fall through
                // to in-flight waiter path so we're woken when it completes.
            }

            // 2. Check in-flight — if so, register as waiter and await
            if let Some(rx) = scheduler.store.register_waiter(&key) {
                return rx.await.unwrap_or(Err(DagError::Cancelled));
            }

            // 3. Not in-flight — create fetcher and execute
            let cancel_fetcher = cancel.child();
            let registered = scheduler
                .store
                .mark_in_flight(key.clone(), cancel_fetcher.clone());

            if !registered {
                // Race: another task marked it in-flight between our check
                // and now. Register as waiter instead.
                if let Some(rx) = scheduler.store.register_waiter(&key) {
                    return rx.await.unwrap_or(Err(DagError::Cancelled));
                }
            }

            // Emit started event
            let _ = progress_tx.unbounded_send(ProgressEvent::Started {
                key: key.clone(),
            });

            // Create fetcher
            let fetcher = match scheduler.registry.create(&key) {
                Some(f) => f,
                None => {
                    let err = Err(DagError::NoFetcherForKind(key.kind.to_string()));
                    scheduler.store.complete(&key, err.clone());
                    return err;
                }
            };

            // Build FetchContext for this fetcher, inheriting the parent's
            // call stack so that cross-fetcher cycles are detected.
            let mut ctx = FetchContext {
                scheduler: Arc::clone(&scheduler),
                call_stack: parent_call_stack,
                call_stack_set: parent_call_stack_set,
                cancel: cancel_fetcher,
                progress_tx: progress_tx.clone(),
                result_tx: result_tx.clone(),
            };

            // Resolve declared dependencies
            let mut inputs = Vec::new();
            for dep_key in fetcher.dependencies() {
                match ctx.request_dependency(dep_key).await {
                    Ok(artifact) => inputs.push(artifact),
                    Err(e) => {
                        scheduler.store.complete(&key, Err(e.clone()));
                        return Err(e);
                    }
                }
            }

            // Execute the fetcher
            let result = fetcher.fetch(inputs, &mut ctx).await;

            // Store result and wake waiters — but only for final artifacts.
            // Streaming artifacts (is_final=false) are already registered
            // via ctx.spawn_streaming() and will be finalized later.
            if result.as_ref().map_or(true, |a| a.is_final) {
                scheduler.store.complete(&key, result.clone());
            }

            // Emit result event (unbounded, non-blocking in Phase 1)
            match &result {
                Ok(artifact) => {
                    let _ = result_tx.unbounded_send(ResultEvent::Done {
                        key: key.clone(),
                        artifact: artifact.clone(),
                    });
                }
                Err(e) => {
                    let _ = result_tx.unbounded_send(ResultEvent::Error {
                        key: key.clone(),
                        error: Arc::new(e.clone()),
                    });
                }
            }

            result
        })
    }

    /// Access the spawner (for use by bridge code).
    #[allow(dead_code)]
    pub fn spawner(&self) -> &Arc<dyn Spawner> {
        &self.spawner
    }

    /// Access the registry (for inspection).
    #[allow(dead_code)]
    pub fn registry(&self) -> &Arc<FetcherRegistry<M>> {
        &self.registry
    }

    /// Inject a pre-computed artifact into the ready cache.
    ///
    /// This is useful for seeding externally-produced data (e.g., source
    /// text read from disk) that downstream fetchers depend on.
    pub fn seed_artifact(&self, artifact: Artifact<M>) {
        self.store.put_ready(artifact);
    }

    /// Mark a streaming artifact as complete. Moves it from InFlight to
    /// Ready cache and wakes all tasks waiting on it.
    pub fn notify_final(
        &self,
        key: ArtifactKey<M>,
        final_data: std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) {
        self.store.finalize_streaming(&key, final_data);
    }

    /// Return the number of completed artifacts in the ready cache.
    #[allow(dead_code)]
    pub fn ready_count(&self) -> usize {
        self.store.ready_count()
    }
}

// ── BuildStream ──────────────────────────────────────────────────────

/// A stream of [`BuilderEvent`]s produced by [`DagScheduler::build`].
///
/// The stream yields exactly one event (either [`BuilderEvent::Done`] or
/// [`BuilderEvent::Error`]) then terminates.
///
/// Dropping the stream cancels all in-flight tasks for this build via
/// the stored [`CancellationToken`].
///
/// Phase 2+: The stream will also yield intermediate progress events
/// for streaming compilation feedback.
pub struct BuildStream<M, Out>
where
    M: fmt::Debug + fmt::Display + Clone,
    Out: Clone,
{
    /// Oneshot receiver for the builder's final result.
    done_rx: oneshot::Receiver<Result<Out, DagError<M>>>,
    /// Cancellation token — dropping this cancels the build.
    _cancel: CancellationToken,
}

impl<M, Out> Stream for BuildStream<M, Out>
where
    M: Eq
        + std::hash::Hash
        + Clone
        + fmt::Debug
        + fmt::Display
        + Send
        + Sync
        + 'static,
    Out: Clone + Send + 'static,
{
    type Item = BuilderEvent<M, Out>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.done_rx).poll(cx) {
            Poll::Ready(Ok(Ok(out))) => Poll::Ready(Some(BuilderEvent::Done(out))),
            Poll::Ready(Ok(Err(e))) => {
                Poll::Ready(Some(BuilderEvent::Error(Arc::new(e))))
            }
            Poll::Ready(Err(_cancelled)) => {
                // Oneshot was dropped — the sender was dropped, meaning
                // the build task was cancelled or panicked.
                Poll::Ready(Some(BuilderEvent::Error(Arc::new(DagError::Cancelled))))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<M, Out> Drop for BuildStream<M, Out>
where
    M: fmt::Debug + fmt::Display + Clone,
    Out: Clone,
{
    fn drop(&mut self) {
        self._cancel.cancel();
    }
}
