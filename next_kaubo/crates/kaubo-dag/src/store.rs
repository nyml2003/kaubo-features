//! In-memory artifact storage: Ready Cache, InFlight tracking, and Reverse
//! Dependents index.
//!
//! # Data structures
//!
//! | Structure | Backing | Purpose |
//! |-----------|---------|---------|
//! | Ready Cache | `DashMap` | Completed artifacts, lock-free concurrent reads |
//! | InFlight Map | `Mutex<HashMap>` | Currently-executing computations (dedup) |
//! | Reverse Deps | `Mutex<HashMap>` | "Who depends on me?" index (invalidation) |
//!
//! # Concurrency strategy
//!
//! - **Ready Cache**: `DashMap` provides sharded locking — concurrent reads
//!   never block each other. Writes happen only at computation completion.
//! - **InFlight Map**: `Mutex<HashMap>` — writes are rare (once per key per
//!   build). The Mutex is held briefly during registration/waiter setup.
//! - **Reverse Deps**: `Mutex<HashMap>` — populated during graph expansion,
//!   read during invalidation (Phase 2).

use crate::cancel::CancellationToken;
use crate::error::DagError;
use crate::types::{Artifact, ArtifactKey};
use dashmap::DashMap;
use futures::channel::oneshot;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Mutex;

// ── InFlight Entry ───────────────────────────────────────────────────

/// Tracks a computation that is currently in progress.
struct InFlightEntry<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display,
{
    /// Oneshot senders for each waiter. When the computation completes,
    /// all senders are resolved with the result (or error).
    waiters: Vec<oneshot::Sender<Result<Artifact<M>, DagError<M>>>>,
    /// Cancellation token for this computation.
    cancel: CancellationToken,
    /// The key being computed (kept for diagnostic messages).
    #[allow(dead_code)]
    key: ArtifactKey<M>,
}

// ── Artifact Store ───────────────────────────────────────────────────

/// The central artifact store for a [`DagScheduler`](crate::DagScheduler) instance.
///
/// Manages three concerns:
///
/// 1. **Ready Cache** — completed artifacts available for immediate retrieval.
/// 2. **InFlight tracking** — computations in progress, for deduplication and
///    waiter registration.
/// 3. **Reverse Dependents** — dependency index for cache invalidation
///    propagation (Phase 2).
pub struct ArtifactStore<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display,
{
    /// Completed artifacts. `DashMap` enables lock-free concurrent reads.
    ready: DashMap<ArtifactKey<M>, Artifact<M>>,

    /// Computations in progress. `Mutex` because writes are rare.
    in_flight: Mutex<HashMap<ArtifactKey<M>, InFlightEntry<M>>>,

    /// Reverse dependency index: for each key, the set of keys that
    /// depend on it. Used for cache invalidation (Phase 2).
    reverse_deps: Mutex<HashMap<ArtifactKey<M>, HashSet<ArtifactKey<M>>>>,
}

impl<M> ArtifactStore<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Create a new, empty artifact store.
    pub fn new() -> Self {
        ArtifactStore {
            ready: DashMap::new(),
            in_flight: Mutex::new(HashMap::new()),
            reverse_deps: Mutex::new(HashMap::new()),
        }
    }

    // ── Ready Cache ──────────────────────────────────────────────

    /// Try to retrieve a completed artifact from the ready cache.
    ///
    /// Returns `None` if the artifact has not been computed yet or has
    /// been invalidated.
    pub fn get_ready(&self, key: &ArtifactKey<M>) -> Option<Artifact<M>> {
        self.ready.get(key).map(|r| r.clone())
    }

    /// Check whether a key exists in the ready cache.
    pub fn has_ready(&self, key: &ArtifactKey<M>) -> bool {
        self.ready.contains_key(key)
    }

    /// Store a completed artifact in the ready cache.
    pub fn put_ready(&self, artifact: Artifact<M>) {
        self.ready.insert(artifact.key.clone(), artifact);
    }

    /// Remove an artifact from the ready cache (for invalidation).
    #[allow(dead_code)] // Phase 2: used by invalidate()
    pub fn remove_ready(&self, key: &ArtifactKey<M>) -> Option<Artifact<M>> {
        self.ready.remove(key).map(|(_, artifact)| artifact)
    }

    // ── Reverse Dependents ───────────────────────────────────────

    /// Register a dependency edge: `dependent` depends on `dependency`.
    ///
    /// This is called during graph expansion so that invalidation can
    /// propagate downstream later.
    pub fn add_dependent(&self, dependent: ArtifactKey<M>, dependency: ArtifactKey<M>) {
        let mut deps = self.reverse_deps.lock().unwrap();
        deps.entry(dependency).or_default().insert(dependent);
    }

    /// Get all keys that directly depend on `key`.
    #[allow(dead_code)] // Phase 2: used by invalidate()
    pub fn get_dependents(&self, key: &ArtifactKey<M>) -> Vec<ArtifactKey<M>> {
        let deps = self.reverse_deps.lock().unwrap();
        deps.get(key)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    // ── InFlight ─────────────────────────────────────────────────

    /// Check whether a key is currently being computed.
    pub fn is_in_flight(&self, key: &ArtifactKey<M>) -> bool {
        let inflight = self.in_flight.lock().unwrap();
        inflight.contains_key(key)
    }

    /// Register a waiter for an in-flight computation.
    ///
    /// Returns a [`oneshot::Receiver`] that will resolve when the
    /// computation completes, or `None` if the key is **not** in-flight
    /// (caller should start the computation).
    pub fn register_waiter(
        &self,
        key: &ArtifactKey<M>,
    ) -> Option<oneshot::Receiver<Result<Artifact<M>, DagError<M>>>> {
        let mut inflight = self.in_flight.lock().unwrap();
        inflight.get_mut(key).map(|entry| {
            let (tx, rx) = oneshot::channel();
            entry.waiters.push(tx);
            rx
        })
    }

    /// Mark a key as in-flight.
    ///
    /// Returns `true` if the key was newly inserted (caller should start
    /// the computation), `false` if it was already in-flight (another task
    /// beat us to it — caller should use [`register_waiter`] instead).
    pub fn mark_in_flight(
        &self,
        key: ArtifactKey<M>,
        cancel: CancellationToken,
    ) -> bool {
        let mut inflight = self.in_flight.lock().unwrap();
        if inflight.contains_key(&key) {
            return false; // race: another task started first
        }
        inflight.insert(
            key.clone(),
            InFlightEntry {
                waiters: Vec::new(),
                cancel,
                key,
            },
        );
        true
    }

    /// Complete an in-flight computation.
    ///
    /// On success, the artifact is stored in the ready cache. All waiters
    /// are woken with the result.
    pub fn complete(&self, key: &ArtifactKey<M>, result: Result<Artifact<M>, DagError<M>>) {
        // If successful, store in ready cache
        if let Ok(ref artifact) = result {
            self.ready.insert(key.clone(), artifact.clone());
        }

        // Wake all waiters
        let mut inflight = self.in_flight.lock().unwrap();
        if let Some(entry) = inflight.remove(key) {
            for tx in entry.waiters {
                // Ignore send errors — receiver may have been dropped
                // due to cancellation or early error propagation.
                let _ = tx.send(result.clone());
            }
        }
    }

    /// Cancel an in-flight computation (without storing any result).
    ///
    /// All waiters receive `Err(DagError::Cancelled)`.
    #[allow(dead_code)] // Phase 2: used by cancel propagation
    pub fn cancel_in_flight(&self, key: &ArtifactKey<M>) {
        let mut inflight = self.in_flight.lock().unwrap();
        if let Some(entry) = inflight.remove(key) {
            entry.cancel.cancel();
            for tx in entry.waiters {
                let _ = tx.send(Err(DagError::Cancelled));
            }
        }
    }

    /// Store a streaming (non-final) artifact and register it as in-flight.
    ///
    /// Returns `true` if this is a new registration. Downstream consumers
    /// can retrieve the artifact via [`get_ready`] and start pulling data
    /// immediately.
    pub fn store_streaming(&self, artifact: Artifact<M>) -> bool {
        let key = artifact.key.clone();
        // Put in ready cache immediately (with is_final=false) so downstream
        // can access it without waiting.
        self.ready.insert(key.clone(), artifact);
        // Register as in-flight so waiters can be notified on finalize.
        let mut inflight = self.in_flight.lock().unwrap();
        if inflight.contains_key(&key) {
            return false;
        }
        inflight.insert(
            key.clone(),
            InFlightEntry {
                waiters: Vec::new(),
                cancel: CancellationToken::new(),
                key,
            },
        );
        true
    }

    /// Finalize a streaming artifact — update its data, mark is_final=true,
    /// and wake all waiters.
    pub fn finalize_streaming(
        &self,
        key: &ArtifactKey<M>,
        final_data: std::sync::Arc<dyn std::any::Any + Send + Sync>,
    ) {
        // Update the artifact in ready cache
        if let Some(mut entry) = self.ready.get_mut(key) {
            entry.is_final = true;
            entry.data = final_data;
        }
        // Wake waiters
        let mut inflight = self.in_flight.lock().unwrap();
        if let Some(entry) = inflight.remove(key) {
            let artifact = self.ready.get(key).unwrap().clone();
            for tx in entry.waiters {
                let _ = tx.send(Ok(artifact.clone()));
            }
        }
    }

    /// Get the cancellation token for an in-flight computation.
    #[allow(dead_code)] // Phase 2
    pub fn cancel_token(&self, key: &ArtifactKey<M>) -> Option<CancellationToken> {
        let inflight = self.in_flight.lock().unwrap();
        inflight.get(key).map(|entry| entry.cancel.clone())
    }

    /// Number of entries in the ready cache.
    pub fn ready_count(&self) -> usize {
        self.ready.len()
    }

    /// Number of in-flight computations.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.lock().unwrap().len()
    }
}

impl<M> Default for ArtifactStore<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Kind;

    type M = String;

    fn key(name: &str, kind_name: &str) -> ArtifactKey<M> {
        ArtifactKey::new(name.to_owned(), Kind::new(kind_name))
    }

    #[test]
    fn ready_cache_put_and_get() {
        let store = ArtifactStore::<M>::new();
        let k = key("mod", "Ast");
        let a = Artifact::new("mod".to_string(), Kind::new("Ast"), 42i64);

        store.put_ready(a);
        let retrieved = store.get_ready(&k).unwrap();
        assert_eq!(*retrieved.downcast_ref::<i64>(), 42);
    }

    #[test]
    fn ready_cache_miss_returns_none() {
        let store = ArtifactStore::<M>::new();
        assert!(store.get_ready(&key("nonexistent", "Ast")).is_none());
    }

    #[test]
    fn mark_in_flight_returns_true_for_new_key() {
        let store = ArtifactStore::<M>::new();
        assert!(store.mark_in_flight(key("a", "Source"), CancellationToken::new()));
    }

    #[test]
    fn mark_in_flight_returns_false_for_duplicate() {
        let store = ArtifactStore::<M>::new();
        let k = key("a", "Source");
        assert!(store.mark_in_flight(k.clone(), CancellationToken::new()));
        assert!(!store.mark_in_flight(k, CancellationToken::new()));
    }

    #[test]
    fn register_waiter_returns_none_when_not_in_flight() {
        let store = ArtifactStore::<M>::new();
        assert!(store.register_waiter(&key("a", "Source")).is_none());
    }

    #[test]
    fn complete_wakes_waiters_and_stores_in_cache() {
        let store = ArtifactStore::<M>::new();
        let k = key("a", "Source");

        // Mark in-flight
        store.mark_in_flight(k.clone(), CancellationToken::new());

        // Register a waiter
        let mut rx = store.register_waiter(&k).unwrap();

        // Complete
        let artifact = Artifact::new("a".to_string(), Kind::new("Source"), "hello".to_string());
        store.complete(&k, Ok(artifact));

        // Waiter should be resolved
        let opt = rx.try_recv().expect("channel should not be canceled");
        let result = opt.expect("result should be ready");
        assert!(result.is_ok());
    }

    #[test]
    fn complete_stores_in_ready_cache() {
        let store = ArtifactStore::<M>::new();
        let k = key("a", "Source");

        store.mark_in_flight(k.clone(), CancellationToken::new());
        let artifact = Artifact::new("a".to_string(), Kind::new("Source"), 99i64);
        store.complete(&k, Ok(artifact));

        let cached = store.get_ready(&k).unwrap();
        assert_eq!(*cached.downcast_ref::<i64>(), 99);
    }

    #[test]
    fn error_completion_does_not_store_in_cache() {
        let store = ArtifactStore::<M>::new();
        let k = key("a", "Bad");

        store.mark_in_flight(k.clone(), CancellationToken::new());
        store.complete(&k, Err(DagError::<M>::Cancelled));

        assert!(store.get_ready(&k).is_none());
    }

    #[test]
    fn reverse_deps_tracks_dependents() {
        let store = ArtifactStore::<M>::new();
        let dep = key("a", "Source");
        let parent = key("a", "Ast");

        store.add_dependent(parent.clone(), dep.clone());
        let deps = store.get_dependents(&dep);
        assert_eq!(deps, vec![parent]);
    }

    #[test]
    fn ready_count_tracks_entries() {
        let store = ArtifactStore::<M>::new();
        assert_eq!(store.ready_count(), 0);

        store.put_ready(Artifact::new("a".to_string(), Kind::new("Source"), 1i64));
        assert_eq!(store.ready_count(), 1);

        store.put_ready(Artifact::new("b".to_string(), Kind::new("Source"), 2i64));
        assert_eq!(store.ready_count(), 2);
    }
}
