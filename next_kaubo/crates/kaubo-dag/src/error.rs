//! Error types for the DAG scheduler.

use crate::types::ArtifactKey;
use std::fmt;
use thiserror::Error;

/// Errors that can occur during DAG scheduling and execution.
///
/// `M` is the module identifier type, matching [`ArtifactKey<M>`].
#[derive(Error, Debug, Clone)]
pub enum DagError<M: fmt::Debug + fmt::Display + Clone> {
    /// A circular dependency was detected.
    ///
    /// The `Vec` contains the cycle path starting from the repeated key.
    #[error("circular dependency detected: {path}", path = format_cycle(cycle))]
    CircularDependency {
        /// The cycle path (e.g., `[A/Ast, B/Semantic, A/Ast]`).
        cycle: Vec<ArtifactKey<M>>,
    },

    /// No [`Fetcher`](crate::Fetcher) is registered for the given [`Kind`](crate::Kind).
    #[error("no fetcher registered for kind: {0}")]
    NoFetcherForKind(String),

    /// A fetcher failed during execution.
    #[error("fetcher error for {key}: {message}")]
    FetcherError {
        /// The key that was being computed.
        key: ArtifactKey<M>,
        /// Human-readable error message.
        message: String,
    },

    /// A builder failed during execution.
    #[error("builder error: {0}")]
    BuilderError(String),

    /// The operation was cancelled via [`CancellationToken`](crate::CancellationToken).
    #[error("cancelled")]
    Cancelled,

    /// An internal invariant was violated (programmer error, not recoverable).
    #[error("internal error: {0}")]
    Internal(String),
}

fn format_cycle<M: fmt::Display>(cycle: &[ArtifactKey<M>]) -> String {
    cycle
        .iter()
        .map(|k| k.to_string())
        .collect::<Vec<_>>()
        .join(" → ")
}

impl<M: fmt::Debug + fmt::Display + Clone> DagError<M> {
    /// Convenience constructor for fetcher errors.
    pub fn fetcher_error(key: ArtifactKey<M>, message: impl Into<String>) -> Self {
        DagError::FetcherError {
            key,
            message: message.into(),
        }
    }

    /// Convenience constructor for circular dependency errors.
    pub fn circular(cycle: Vec<ArtifactKey<M>>) -> Self {
        DagError::CircularDependency { cycle }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Kind;

    #[test]
    fn circular_dependency_display() {
        let cycle = vec![
            ArtifactKey::new("a".to_string(), Kind::new(Kind::AST)),
            ArtifactKey::new("b".to_string(), Kind::new(Kind::SEMANTIC)),
            ArtifactKey::new("a".to_string(), Kind::new(Kind::AST)),
        ];
        let err = DagError::<String>::circular(cycle);
        let msg = err.to_string();
        assert!(msg.contains("circular dependency"));
        assert!(msg.contains("a/Ast → b/Semantic → a/Ast"));
    }

    #[test]
    fn no_fetcher_for_kind_display() {
        let err = DagError::<String>::NoFetcherForKind("Bytecode".into());
        assert!(err.to_string().contains("Bytecode"));
    }

    #[test]
    fn fetcher_error_display() {
        let err = DagError::fetcher_error(
            ArtifactKey::new("mod".to_string(), Kind::new(Kind::CPS)),
            "type mismatch",
        );
        let msg = err.to_string();
        assert!(msg.contains("mod/Cps"));
        assert!(msg.contains("type mismatch"));
    }
}
