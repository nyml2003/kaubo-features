//! Runtime-agnostic DAG scheduler for orchestrating compilation tasks.
//!
//! # Architecture
//!
//! The scheduler coordinates [`Fetcher`] and [`Builder`] instances over a dynamic,
//! lazily-expanded dependency graph keyed by [`ArtifactKey<M>`].
//!
//! # Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`ArtifactKey<M>`] | Unique coordinate `(module, kind)` in the dependency graph |
//! | [`Artifact`] | Type-erased, immutable, shareable data container |
//! | [`Fetcher<M>`] | Data producer — given inputs, produces one output artifact |
//! | [`Builder<M, Out>`] | Terminal consumer — produces a final result (not cached) |
//! | [`DagScheduler<M>`] | Core orchestration engine |
//! | [`Spawner`] | Platform abstraction for spawning async tasks |
//!
//! # Example (minimal, using `String` as module ID)
//!
//! ```ignore
//! use kaubo_dag::*;
//!
//! // 1. Register fetcher factories
//! let registry = FetcherRegistry::new();
//! registry.register(Kind::new("Source"), Box::new(|key| Box::new(MySourceFetcher { key })));
//!
//! // 2. Create scheduler
//! let scheduler = DagScheduler::new(registry, my_spawner);
//!
//! // 3. Build
//! let builder = MyBuilder::new();
//! let stream = scheduler.build(Box::new(builder));
//! ```

pub mod builder;
pub mod cancel;
pub mod error;
pub mod fetcher;
pub mod registry;
pub mod scheduler;
pub mod spawner;
pub mod store;
pub mod types;

pub use builder::{Builder, BuilderEvent};
pub use cancel::CancellationToken;
pub use error::DagError;
pub use fetcher::{FetchContext, Fetcher, ProgressEvent, ResultEvent, StreamingHandle};
pub use registry::FetcherRegistry;
pub use scheduler::{BuildStream, DagScheduler};
pub use spawner::Spawner;
pub use store::ArtifactStore;
pub use types::{Artifact, ArtifactKey, ContentHash, Kind};

#[cfg(not(target_arch = "wasm32"))]
pub use spawner::{BlockingSpawner, NativeSpawner};
#[cfg(target_arch = "wasm32")]
pub use spawner::WasmSpawner;
