//! Platform abstraction for spawning asynchronous tasks.
//!
//! The scheduler core calls [`Spawner::spawn`] to run background work.
//! Native environments use a thread pool; WASM uses
//! `wasm_bindgen_futures::spawn_local`.
//!
//! Phase 1 defines only the trait. Concrete implementations live in
//! `kaubo-driver` (native) and `kaubo-wasm` (WASM, Phase 2).

use crate::cancel::CancellationToken;
use std::future::Future;
use std::pin::Pin;

/// Platform abstraction for spawning asynchronous tasks.
///
/// # Design
///
/// The trait is deliberately minimal. The scheduler only needs three
/// capabilities from the runtime:
///
/// | Method | Purpose |
/// |--------|---------|
/// | [`spawn`](Spawner::spawn) | Run a background future to completion |
/// | [`yield_now`](Spawner::yield_now) | Yield execution to let other tasks progress |
/// | [`cancellation_token`](Spawner::cancellation_token) | Create a cancellation token |
///
/// # WASM compatibility
///
/// The trait is object-safe (dyn-compatible) so it can be stored as
/// `Arc<dyn Spawner>`. This is critical for the scheduler to be platform-
/// agnostic.
pub trait Spawner: Send + Sync {
    /// Spawn a background future.
    ///
    /// The spawned future runs to completion independently. Cancellation
    /// is cooperatively checked via the [`CancellationToken`] passed
    /// through [`FetchContext`](crate::FetchContext) — the `Spawner` does
    /// not own cancellation logic.
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);

    /// Yield the current task's execution slot so that other ready tasks
    /// can make progress.
    ///
    /// **Semantic guarantee:**
    /// - After calling `yield_now().await`, the current task is
    ///   re-queued at the end of the ready queue.
    /// - Other ready tasks get a chance to run in the gap.
    /// - This is NOT "yield the CPU core" (`thread::yield_now`) — it is
    ///   "yield the coroutine scheduling slot."
    ///
    /// **Use cases:**
    /// - WASM single-threaded: prevent long computations from blocking
    ///   UI rendering and event handling.
    /// - Native: insert yield points in compute-heavy loops to prevent
    ///   a single task from monopolizing the scheduler.
    fn yield_now(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;

    /// Create a new [`CancellationToken`].
    fn cancellation_token(&self) -> CancellationToken;
}

/// Extension trait for spawners that support blocking the current thread
/// on a future (native platforms only).
///
/// This is split from [`Spawner`] because the generic `block_on<F>`
/// method prevents the trait from being dyn-compatible. The scheduler
/// never calls `block_on` — it's only used by bridge code in
/// `kaubo-driver` to wrap async APIs in synchronous functions.
///
/// WASM targets do not implement this trait, as blocking the browser
/// main thread is forbidden.
#[cfg(not(target_arch = "wasm32"))]
pub trait BlockingSpawner: Spawner {
    /// Block the current thread until the future completes.
    fn block_on<F: Future>(&self, future: F) -> F::Output;
}

// ── Native Spawner ───────────────────────────────────────────────────

/// A [`Spawner`] implementation for native (non-WASM) environments.
///
/// Uses `std::thread::spawn` to run background futures on a separate
/// thread. `yield_now` bounces through a oneshot channel on a new thread
/// to ensure other tasks get a chance to run.
///
/// # Thread safety
///
/// `NativeSpawner` is `Send + Sync` and can be safely wrapped in
/// `Arc<dyn Spawner>`.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Default)]
pub struct NativeSpawner;

#[cfg(not(target_arch = "wasm32"))]
impl Spawner for NativeSpawner {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        // Run the future to completion on a dedicated OS thread.
        std::thread::spawn(move || {
            futures::executor::block_on(future);
        });
    }

    fn yield_now(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        // Bounce through a oneshot channel on a fresh thread.
        // The thread signals immediately, but the round-trip through the
        // OS scheduler gives other tasks a chance to make progress.
        let (tx, rx) = futures::channel::oneshot::channel::<()>();
        std::thread::spawn(move || {
            let _ = tx.send(());
        });
        Box::pin(async move {
            let _ = rx.await;
        })
    }

    fn cancellation_token(&self) -> CancellationToken {
        CancellationToken::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl BlockingSpawner for NativeSpawner {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        futures::executor::block_on(future)
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_spawner_creates_token() {
        let s = NativeSpawner;
        let token = s.cancellation_token();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn native_spawner_spawn_runs_future() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = std::sync::Arc::new(AtomicBool::new(false));
        let f = flag.clone();

        let s = NativeSpawner;
        s.spawn(Box::pin(async move {
            f.store(true, Ordering::SeqCst);
        }));

        // Give the thread a moment to run
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn native_spawner_block_on_returns_value() {
        let s = NativeSpawner;
        let result = s.block_on(async { 42i64 });
        assert_eq!(result, 42);
    }

    #[test]
    fn native_spawner_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NativeSpawner>();
    }
}
