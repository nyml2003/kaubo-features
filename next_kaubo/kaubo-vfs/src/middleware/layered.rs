//! Layered VFS that executes middleware chain

use std::path::Path;
use std::sync::Arc;
use crate::{VirtualFileSystem, VfsResult};
use super::{Middleware, Next};

/// VFS implementation that executes a middleware chain
pub struct LayeredVFS {
    backend: Arc<dyn VirtualFileSystem + Send + Sync>,
    middlewares: Vec<Box<dyn Middleware>>,
}

impl LayeredVFS {
    /// Create a new layered VFS
    pub(crate) fn new(
        backend: Arc<dyn VirtualFileSystem + Send + Sync>,
        middlewares: Vec<Box<dyn Middleware>>,
    ) -> Self {
        Self {
            backend,
            middlewares,
        }
    }
}

/// Chain executor for a specific operation
struct ChainExecutor<'a> {
    backend: &'a dyn VirtualFileSystem,
    middlewares: &'a [Box<dyn Middleware>],
    index: usize,
}

impl<'a> ChainExecutor<'a> {
    fn new(backend: &'a dyn VirtualFileSystem, middlewares: &'a [Box<dyn Middleware>], index: usize) -> Self {
        Self { backend, middlewares, index }
    }
}

impl<'a> Next for ChainExecutor<'a> {
    fn read_file(&self, path: &Path) -> VfsResult<Vec<u8>> {
        if self.index >= self.middlewares.len() {
            return self.backend.read_file(path);
        }
        
        let middleware = &self.middlewares[self.index];
        let next = ChainExecutor::new(self.backend, self.middlewares, self.index + 1);
        middleware.read_file(path, &next)
    }
    
    fn write_file(&self, path: &Path, content: &[u8]) -> VfsResult<()> {
        if self.index >= self.middlewares.len() {
            return self.backend.write_file(path, content);
        }
        
        let middleware = &self.middlewares[self.index];
        let next = ChainExecutor::new(self.backend, self.middlewares, self.index + 1);
        middleware.write_file(path, content, &next)
    }
    
    fn exists(&self, path: &Path) -> bool {
        if self.index >= self.middlewares.len() {
            return self.backend.exists(path);
        }
        
        let middleware = &self.middlewares[self.index];
        let next = ChainExecutor::new(self.backend, self.middlewares, self.index + 1);
        middleware.exists(path, &next)
    }
    
    fn is_file(&self, path: &Path) -> bool {
        if self.index >= self.middlewares.len() {
            return self.backend.is_file(path);
        }
        
        let middleware = &self.middlewares[self.index];
        let next = ChainExecutor::new(self.backend, self.middlewares, self.index + 1);
        middleware.is_file(path, &next)
    }
    
    fn is_dir(&self, path: &Path) -> bool {
        if self.index >= self.middlewares.len() {
            return self.backend.is_dir(path);
        }
        
        let middleware = &self.middlewares[self.index];
        let next = ChainExecutor::new(self.backend, self.middlewares, self.index + 1);
        middleware.is_dir(path, &next)
    }
}

impl VirtualFileSystem for LayeredVFS {
    fn read_file(&self, path: &Path) -> VfsResult<Vec<u8>> {
        let executor = ChainExecutor::new(&*self.backend, &self.middlewares, 0);
        executor.read_file(path)
    }
    
    fn write_file(&self, path: &Path, content: &[u8]) -> VfsResult<()> {
        let executor = ChainExecutor::new(&*self.backend, &self.middlewares, 0);
        executor.write_file(path, content)
    }
    
    fn exists(&self, path: &Path) -> bool {
        let executor = ChainExecutor::new(&*self.backend, &self.middlewares, 0);
        executor.exists(path)
    }
    
    fn is_file(&self, path: &Path) -> bool {
        let executor = ChainExecutor::new(&*self.backend, &self.middlewares, 0);
        executor.is_file(path)
    }
    
    fn is_dir(&self, path: &Path) -> bool {
        let executor = ChainExecutor::new(&*self.backend, &self.middlewares, 0);
        executor.is_dir(path)
    }
}
