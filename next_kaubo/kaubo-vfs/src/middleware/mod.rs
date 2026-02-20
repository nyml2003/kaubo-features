//! VFS Middleware System
//!
//! Provides a composable middleware layer for the Virtual File System.

mod stage;
mod middleware;
mod builder;
mod layered;

// Re-export core types
pub use stage::Stage;
pub use middleware::{Middleware, Next};
pub use builder::VfsBuilder;
pub use layered::LayeredVFS;

// Re-export built-in middlewares
pub mod logged;
pub mod mapped;
pub mod cached;

pub use logged::LoggedLayer;
pub use mapped::{MappedLayer, ModuleContext};
pub use cached::CachedLayer;
