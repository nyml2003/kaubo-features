//! Lightweight cancellation token based on `Arc<AtomicBool>`.
//!
//! Used by [`FetchContext`](crate::FetchContext) to allow fetchers to
//! cooperatively check for cancellation at await points, and by the
//! scheduler to cancel in-flight tasks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A lightweight, cloneable cancellation token.
///
/// Cloning a `CancellationToken` creates a new handle to the same underlying
/// flag. When **any** handle calls [`cancel`](CancellationToken::cancel),
/// **all** handles see [`is_cancelled`](CancellationToken::is_cancelled) return `true`.
///
/// # Usage
///
/// ```ignore
/// let token = CancellationToken::new();
/// let child = token.child();
///
/// // In a background task:
/// while !child.is_cancelled() {
///     do_work().await;
/// }
///
/// // From the controller:
/// token.cancel(); // all children see is_cancelled() == true
/// ```
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new, uncancelled token.
    pub fn new() -> Self {
        CancellationToken {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a child token that shares the same cancellation state.
    ///
    /// This is a cheap `Arc::clone` — no allocation beyond the initial
    /// `CancellationToken::new()`.
    pub fn child(&self) -> Self {
        CancellationToken {
            cancelled: Arc::clone(&self.cancelled),
        }
    }

    /// Signal cancellation. Idempotent — calling multiple times has no
    /// additional effect.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Check whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_token_is_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn cancel_sets_flag() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_is_idempotent() {
        let token = CancellationToken::new();
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn children_share_state() {
        let parent = CancellationToken::new();
        let child = parent.child();

        assert!(!child.is_cancelled());
        parent.cancel();
        assert!(child.is_cancelled());
    }

    #[test]
    fn child_cancel_affects_parent() {
        let parent = CancellationToken::new();
        let child = parent.child();

        child.cancel();
        assert!(parent.is_cancelled());
    }

    #[test]
    fn independent_tokens_are_independent() {
        let a = CancellationToken::new();
        let b = CancellationToken::new();

        a.cancel();
        assert!(!b.is_cancelled());
    }

    #[test]
    fn default_creates_uncancelled() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }
}
