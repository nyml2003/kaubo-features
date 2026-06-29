//! Builder trait and BuilderEvent type.
//!
//! Builders are terminal consumers — their output is NOT cached in the
//! artifact store. Each builder represents a distinct compilation goal
//! (e.g., "execute this program", "produce LSP diagnostics").

use crate::error::DagError;
use crate::fetcher::FetchContext;
use crate::types::{Artifact, ArtifactKey};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A terminal consumer that produces a final result from dependency
/// artifacts.
///
/// Unlike [`Fetcher`](crate::Fetcher), a Builder's output is NOT cached
/// in the artifact store. Builders are **entry points** — each represents
/// a distinct compilation goal.
///
/// # Type parameters
///
/// - `M` — the module identifier type.
/// - `Out` — the final output type (e.g., `CpsModule`, `RunOutcome`,
///   `LspSnapshot`).
///
/// # Example goals
///
/// | Builder | Output | Dependencies |
/// |---------|--------|-------------|
/// | `ExecuteBuilder` | `RunOutcome` | `LinkedCps` |
/// | `LspSnapshotBuilder` | `LspSnapshot` | `Semantic` for all open files |
/// | `CompileBuilder` | `CpsModule` | `LinkedCps` |
pub trait Builder<M, Out>: Send + 'static
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
    Out: Send + 'static,
{
    /// Human-readable name for logging and debugging.
    fn name(&self) -> &str;

    /// The keys this builder depends on.
    ///
    /// The scheduler resolves all declared dependencies before calling
    /// [`build`](Builder::build). Additional dependencies can be requested
    /// at runtime via [`FetchContext::request_dependency`].
    fn dependencies(&self) -> Vec<ArtifactKey<M>>;

    /// Execute the builder.
    ///
    /// `inputs` contains the resolved artifacts for each dependency key,
    /// in the same order as returned by [`dependencies`](Builder::dependencies).
    ///
    /// `ctx` provides access to the scheduler for requesting additional
    /// dependencies at runtime and for emitting progress/result events.
    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<M>>,
        ctx: &'a mut FetchContext<M>,
    ) -> Pin<Box<dyn Future<Output = Result<Out, DagError<M>>> + Send + 'a>>;
}

// ── BuilderEvent ─────────────────────────────────────────────────────

/// Events emitted during a build.
///
/// Returned as a stream from [`DagScheduler::build`](crate::DagScheduler::build).
/// Consumers (CLI, LSP) iterate the stream and handle each event.
///
/// Phase 3 will add `Degraded` variant for partial-failure scenarios.
#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
pub enum BuilderEvent<M, Out>
where
    M: fmt::Debug + fmt::Display + Clone,
    Out: Clone,
{
    /// The build completed successfully.
    Done(Out),
    /// The build failed with an error.
    Error(Arc<DagError<M>>),
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    type M = String;

    #[test]
    fn builder_event_done_format() {
        let event = BuilderEvent::<M, i64>::Done(42);
        assert!(format!("{event:?}").contains("Done"));
    }

    #[test]
    fn builder_event_error_format() {
        let err = Arc::new(DagError::<M>::Cancelled);
        let event = BuilderEvent::<M, i64>::Error(err);
        assert!(format!("{event:?}").contains("Error"));
    }
}
