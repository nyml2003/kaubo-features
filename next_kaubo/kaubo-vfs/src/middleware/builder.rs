//! VFS Builder for constructing middleware chains

use std::sync::Arc;
use crate::VirtualFileSystem;
use super::{Middleware, LayeredVFS};

/// Builder for constructing a VFS with middleware chain
///
/// # Example
/// ```rust,ignore
/// use kaubo_vfs::{VfsBuilder, NativeFileSystem};
/// use kaubo_vfs::middleware::LoggedLayer;
///
/// let vfs = VfsBuilder::new(NativeFileSystem::new())
///     .with(LoggedLayer::new())
///     .build();
/// ```
pub struct VfsBuilder {
    backend: Arc<dyn VirtualFileSystem + Send + Sync>,
    middlewares: Vec<Box<dyn Middleware>>,
}

impl VfsBuilder {
    /// Create a new VFS builder with the given backend
    pub fn new(backend: impl VirtualFileSystem + Send + Sync + 'static) -> Self {
        Self {
            backend: Arc::new(backend),
            middlewares: Vec::new(),
        }
    }
    
    /// Add a middleware to the chain
    ///
    /// Middlewares are automatically sorted by stage when built.
    pub fn with(mut self, middleware: impl Middleware + 'static) -> Self {
        self.middlewares.push(Box::new(middleware));
        self
    }
    
    /// Build the final VFS with middleware chain
    ///
    /// Middlewares are sorted by stage (lower priority first).
    pub fn build(self) -> LayeredVFS {
        let mut middlewares = self.middlewares;
        
        // Sort by stage priority
        middlewares.sort_by_key(|m| m.stage().priority());
        
        LayeredVFS::new(self.backend, middlewares)
    }
}
