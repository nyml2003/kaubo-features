//! Fetcher trait, FetchContext, and event types.
//!
//! This is the primary API surface that Fetcher implementors interact with.

use crate::cancel::CancellationToken;
use crate::error::DagError;
use crate::types::{Artifact, ArtifactKey};
use futures::channel::mpsc;
use std::collections::HashSet;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// Forward declaration — the actual type lives in scheduler.rs.
// FetchContext holds an Arc to it; we only need the type name here.
use crate::scheduler::DagScheduler;

// ── Event Types ──────────────────────────────────────────────────────

/// Progress events — unbounded channel, lossy delivery.
///
/// These are for UI feedback (progress bars, status updates). If the
/// consumer is slow, events may be dropped — the compilation pipeline
/// is never blocked on progress delivery.
#[derive(Debug, Clone)]
pub enum ProgressEvent<M> {
    /// A fetcher has started executing.
    Started {
        /// The key being computed.
        key: ArtifactKey<M>,
    },
    /// A progress update with a human-readable message.
    Progress {
        /// The key being computed.
        key: ArtifactKey<M>,
        /// Human-readable progress description.
        message: String,
    },
    /// A dependency has been resolved and is now available.
    DependencyReady {
        /// The key that was resolved.
        key: ArtifactKey<M>,
    },
}

/// Result events — bounded channel, reliable delivery.
///
/// These carry the actual build outcomes. The bounded channel ensures
/// backpressure: if the consumer is slow, producers will slow down.
#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
pub enum ResultEvent<M: fmt::Debug + fmt::Display + Clone> {
    /// A fetcher completed successfully.
    Done {
        /// The key that was computed.
        key: ArtifactKey<M>,
        /// The produced artifact.
        artifact: Artifact<M>,
    },
    /// A fetcher failed with an error.
    Error {
        /// The key that was being computed.
        key: ArtifactKey<M>,
        /// The error, shared for efficient propagation.
        error: Arc<DagError<M>>,
    },
}

// ── Fetcher Trait ────────────────────────────────────────────────────

/// A data producer: given a set of input artifacts, produces one output
/// artifact.
///
/// # Type parameters
///
/// - `M` — the module identifier type. All fetchers in a single scheduler
///   instance share the same `M`.
///
/// # Implementation notes
///
/// - `key()` and `dependencies()` should be pure — they must return the
///   same values every time they are called.
/// - `fetch()` is called once per artifact. If multiple tasks request the
///   same key concurrently, the scheduler deduplicates and only calls
///   `fetch()` once.
/// - Inside `fetch()`, call `ctx.request_dependency(key).await` to
///   dynamically request dependencies that were not declared in
///   `dependencies()`.
pub trait Fetcher<M>: Send + 'static
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Returns the output key this fetcher will produce.
    fn key(&self) -> ArtifactKey<M>;

    /// Returns the keys this fetcher depends on.
    ///
    /// The scheduler resolves all declared dependencies before calling
    /// [`fetch`](Fetcher::fetch). Additional dependencies can be requested
    /// at runtime via [`FetchContext::request_dependency`].
    fn dependencies(&self) -> Vec<ArtifactKey<M>>;

    /// Execute the fetcher.
    ///
    /// `inputs` contains the resolved artifacts for each dependency key,
    /// in the same order as returned by [`dependencies`](Fetcher::dependencies).
    ///
    /// `ctx` provides access to the scheduler for requesting additional
    /// dependencies at runtime and for emitting progress/result events.
    #[allow(clippy::type_complexity)]
    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<M>>,
        ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>>;
}

// ── FetchContext ─────────────────────────────────────────────────────

/// Execution context passed to every [`Fetcher::fetch`] call.
///
/// Provides:
///
/// - **Dynamic dependency resolution** via
///   [`request_dependency`](FetchContext::request_dependency).
/// - **Cycle detection** via a dual-index call stack (`Vec` for order,
///   `HashSet` for O(1) membership check).
/// - **Cancellation checking** via [`is_cancelled`](FetchContext::is_cancelled).
/// - **Event emission** via two channels:
///   - Progress channel (unbounded, lossy) for UI updates.
///   - Result channel (bounded, reliable) for build outcomes.
///
/// # Design note
///
/// The context holds an `Arc<DagScheduler<M>>` rather than a reference
/// (`&'a DagScheduler<M>`) to avoid lifetime parameter propagation.
/// This allows background `'static` tasks to hold a cloned `FetchContext`.
pub struct FetchContext<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Reference to the owning scheduler for dependency resolution.
    pub(crate) scheduler: Arc<DagScheduler<M>>,

    /// Call stack for cycle detection — `Vec` preserves order for error
    /// messages.
    pub(crate) call_stack: Vec<ArtifactKey<M>>,

    /// Call stack set — O(1) membership check.
    pub(crate) call_stack_set: HashSet<ArtifactKey<M>>,

    /// Cancellation token for cooperative cancellation.
    pub(crate) cancel: CancellationToken,

    /// Progress events channel (unbounded, lossy).
    pub(crate) progress_tx: mpsc::UnboundedSender<ProgressEvent<M>>,

    /// Result events channel (bounded, reliable).
    pub(crate) result_tx: mpsc::UnboundedSender<ResultEvent<M>>,
}

impl<M> FetchContext<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Request an artifact by key.
    ///
    /// If the artifact is already in the ready cache, it is returned
    /// immediately. If it is currently being computed, the current task
    /// is suspended until the computation completes. If it has not been
    /// started, the scheduler creates and executes the appropriate fetcher.
    ///
    /// # Cycle detection
    ///
    /// If `key` is already present in the current call stack, a
    /// [`DagError::CircularDependency`] is returned. The stack uses
    /// O(1) membership checking via `HashSet`.
    ///
    /// # Cancellation
    ///
    /// If the current task is cancelled while waiting for a dependency,
    /// the method returns [`DagError::Cancelled`].
    pub async fn request_dependency(
        &mut self,
        key: ArtifactKey<M>,
    ) -> Result<Artifact<M>, DagError<M>> {
        // 1. Cycle detection (O(1) via HashSet)
        if self.call_stack_set.contains(&key) {
            let pos = self
                .call_stack
                .iter()
                .position(|k| k == &key)
                .unwrap();
            let cycle = self.call_stack[pos..].to_vec();
            return Err(DagError::CircularDependency { cycle });
        }

        // 2. Push to call stack (dual-write)
        self.call_stack.push(key.clone());
        self.call_stack_set.insert(key.clone());

        // 3. Delegate to scheduler, passing the current call stack
        //    so that sub-fetchers can inherit it for cycle detection.
        let result = self
            .scheduler
            .request_dependency(
                key.clone(),
                self.progress_tx.clone(),
                self.result_tx.clone(),
                self.cancel.child(),
                self.call_stack.clone(),
                self.call_stack_set.clone(),
            )
            .await;

        // 4. Pop from call stack (dual-delete)
        self.call_stack.pop();
        self.call_stack_set.remove(&key);

        result
    }

    /// Check whether the current operation has been cancelled.
    ///
    /// Fetchers should call this at each await point and return
    /// `Err(DagError::Cancelled)` if `true`.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// Emit a progress event.
    ///
    /// This is a non-blocking send on an unbounded channel. If the
    /// consumer is extremely slow and memory pressure builds, events
    /// may be silently dropped to protect the compilation pipeline.
    pub fn emit_progress(&self, event: ProgressEvent<M>) {
        let _ = self.progress_tx.unbounded_send(event);
    }

    /// Seed an artifact and wake any waiters registered for this key.
    /// Use this when a fetcher produces a secondary artifact (e.g. ExportTable)
    /// that other fetchers may be waiting on via `request_dependency`.
    pub fn seed_artifact_and_wake(&self, artifact: Artifact<M>) {
        self.scheduler.store.store_and_wake(artifact);
    }

    /// Mark a key as in-flight so that downstream fetchers calling
    /// `request_dependency` will wait for it. Call `seed_artifact_and_wake`
    /// when the artifact is ready.
    pub fn mark_in_flight(&self, key: ArtifactKey<M>) {
        self.scheduler.store.mark_in_flight(key, self.cancel.child());
    }

    /// Seed a pre-computed artifact into the scheduler's ready cache.
    ///
    /// Downstream fetchers that declare a dependency on this artifact's
    /// key will find it without triggering computation.
    pub fn seed_artifact(&self, artifact: Artifact<M>) {
        self.scheduler.seed_artifact(artifact);
    }

    /// Create a streaming (non-final) artifact and return a handle for
    /// signalling completion.
    ///
    /// The artifact is immediately visible to downstream consumers (via
    /// `request_dependency`), who can start pulling data from it. When
    /// the background task finishes producing data, call
    /// [`StreamingHandle::complete`] to mark the artifact as final and
    /// move it to the ready cache.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (artifact, handle) = ctx.spawn_streaming(key, initial_data);
    /// // Start background task that produces data...
    /// std::thread::spawn(move || {
    ///     // ... produce more data ...
    ///     handle.complete(final_data); // signal completion
    /// });
    /// return Ok(artifact);
    /// ```
    pub fn spawn_streaming(
        &self,
        key: ArtifactKey<M>,
        initial_data: impl Send + Sync + 'static,
    ) -> (Artifact<M>, StreamingHandle<M>) {
        let mut artifact = Artifact::with_key(key.clone(), crate::types::ContentHash::placeholder(), initial_data);
        artifact.is_final = false;
        self.scheduler.store.store_streaming(artifact.clone());
        let handle = StreamingHandle {
            key,
            scheduler: Arc::clone(&self.scheduler),
        };
        (artifact, handle)
    }

    /// Emit a result event.
    ///
    /// Phase 1 uses an unbounded channel — the send is non-blocking.
    /// Phase 2 will switch to a bounded channel with proper backpressure
    /// once a streaming consumer drains the receiver.
    pub fn emit_result(&self, event: ResultEvent<M>) {
        let _ = self.result_tx.unbounded_send(event);
    }
}

// ── StreamingHandle ──────────────────────────────────────────────────

/// A handle for signalling completion of a streaming artifact.
///
/// Created by [`FetchContext::spawn_streaming`]. When the background task
/// finishes producing data, call [`complete`](StreamingHandle::complete)
/// to mark the artifact as final and move it to the ready cache.
///
/// Dropping the handle without calling `complete()` leaves the artifact
/// in the streaming (non-final) state. Phase 2 will add automatic GC for
/// abandoned streaming artifacts.
#[must_use = "StreamingHandle should be completed, or the artifact stays in streaming state forever"]
pub struct StreamingHandle<M>
where
    M: Eq + std::hash::Hash + Clone + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    key: ArtifactKey<M>,
    scheduler: Arc<DagScheduler<M>>,
}

impl<M> StreamingHandle<M>
where
    M: Eq + std::hash::Hash + Clone + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static,
{
    /// Complete the streaming artifact with the final data.
    ///
    /// The artifact's `is_final` flag is set to `true`, it is moved to
    /// the ready cache, and all tasks waiting on it are woken.
    pub fn complete(self, final_data: impl Send + Sync + 'static) {
        self.scheduler
            .notify_final(self.key, std::sync::Arc::new(final_data));
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    type M = String;

    #[test]
    fn progress_event_debug() {
        let event = ProgressEvent::<M>::Started {
            key: ArtifactKey::new("mod".to_string(), crate::types::Kind::new("Ast")),
        };
        let s = format!("{event:?}");
        assert!(s.contains("Started"));
        assert!(s.contains("mod"));
    }

    #[test]
    fn result_event_done_debug() {
        let artifact = Artifact::new("mod".to_string(), crate::types::Kind::new("Cps"), 42i64);
        let event = ResultEvent::Done {
            key: artifact.key.clone(),
            artifact,
        };
        let s = format!("{event:?}");
        assert!(s.contains("Done"));
        assert!(s.contains("mod"));
    }
}
