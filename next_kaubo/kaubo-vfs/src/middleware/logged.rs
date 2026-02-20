//! Logging middleware for VFS operations

use std::path::Path;
use crate::VfsResult;
use super::{Middleware, Next};
use super::Stage;

/// Middleware that logs VFS operations
pub struct LoggedLayer;

impl LoggedLayer {
    /// Create a new logging layer
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggedLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggedLayer {
    fn stage(&self) -> Stage {
        Stage::Outer
    }
    
    fn read_file(&self, path: &Path, next: &dyn Next) -> VfsResult<Vec<u8>> {
        eprintln!("[VFS] read_file: {}", path.display());
        let result = next.read_file(path);
        match &result {
            Ok(_) => eprintln!("[VFS] read_file: {} OK", path.display()),
            Err(e) => eprintln!("[VFS] read_file: {} ERR: {}", path.display(), e),
        }
        result
    }
    
    fn write_file(&self, path: &Path, content: &[u8], next: &dyn Next) -> VfsResult<()> {
        eprintln!("[VFS] write_file: {} ({} bytes)", path.display(), content.len());
        next.write_file(path, content)
    }
    
    fn exists(&self, path: &Path, next: &dyn Next) -> bool {
        let result = next.exists(path);
        eprintln!("[VFS] exists: {} = {}", path.display(), result);
        result
    }
}
