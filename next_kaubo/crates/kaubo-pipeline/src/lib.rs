//! Pipeline orchestration core.
//!
//! This crate models planning, scheduling, events, cancellation, and cache
//! behavior without depending on compiler stage crates.

pub mod cache;
pub mod event;
pub mod plan;
pub mod scheduler;

pub use cache::{ArtifactCache, CacheKey};
pub use event::{EventHub, EventKind, PipelineEvent};
pub use plan::{ArtifactId, NodeId, PipelinePlan, PipelinePolicy, StageNode};
pub use scheduler::{CancellationToken, ExecutionReport, Scheduler, StageAdapter, StageError};
