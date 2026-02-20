//! Middleware trait definition

use std::path::Path;
use crate::VfsResult;
use super::Stage;

/// Middleware trait for VFS operations
///
/// Middleware can intercept and transform VFS operations.
/// Each middleware declares its stage for automatic ordering.
pub trait Middleware: Send + Sync {
    /// Get the execution stage for this middleware
    fn stage(&self) -> Stage;
    
    /// Intercept read_file operation
    fn read_file(&self, path: &Path, next: &dyn Next) -> VfsResult<Vec<u8>> {
        next.read_file(path)
    }
    
    /// Intercept write_file operation
    fn write_file(&self, path: &Path, content: &[u8], next: &dyn Next) -> VfsResult<()> {
        next.write_file(path, content)
    }
    
    /// Intercept exists operation
    fn exists(&self, path: &Path, next: &dyn Next) -> bool {
        next.exists(path)
    }
    
    /// Intercept is_file operation
    fn is_file(&self, path: &Path, next: &dyn Next) -> bool {
        next.is_file(path)
    }
    
    /// Intercept is_dir operation
    fn is_dir(&self, path: &Path, next: &dyn Next) -> bool {
        next.is_dir(path)
    }
}

/// Handle to the next middleware in chain
pub trait Next {
    /// Call next middleware for read_file
    fn read_file(&self, path: &Path) -> VfsResult<Vec<u8>>;
    
    /// Call next middleware for write_file
    fn write_file(&self, path: &Path, content: &[u8]) -> VfsResult<()>;
    
    /// Call next middleware for exists
    fn exists(&self, path: &Path) -> bool;
    
    /// Call next middleware for is_file
    fn is_file(&self, path: &Path) -> bool;
    
    /// Call next middleware for is_dir
    fn is_dir(&self, path: &Path) -> bool;
}
