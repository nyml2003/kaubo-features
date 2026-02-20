//! Caching middleware for VFS operations

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use crate::VfsResult;
use super::{Middleware, Next};
use super::Stage;

/// Cache entry for file content
#[derive(Debug, Clone)]
struct CacheEntry {
    content: Vec<u8>,
    created_at: Instant,
}

/// Middleware that caches file contents
pub struct CachedLayer {
    cache: RwLock<HashMap<PathBuf, CacheEntry>>,
    ttl: Duration,
}

impl CachedLayer {
    /// Create a new cached layer with default TTL (60 seconds)
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(60),
        }
    }
    
    /// Create with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl,
        }
    }
    
    /// Invalidate cache entry
    pub fn invalidate(&self, path: &Path) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(path);
        }
    }
    
    /// Clear all cache
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
    
    /// Get cached entry if valid
    fn get_cached(&self, path: &Path) -> Option<Vec<u8>> {
        let cache = self.cache.read().ok()?;
        let entry = cache.get(path)?;
        
        // Check TTL
        if entry.created_at.elapsed() > self.ttl {
            return None;
        }
        
        Some(entry.content.clone())
    }
    
    /// Insert into cache
    fn insert(&self, path: PathBuf, content: Vec<u8>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(path, CacheEntry {
                content,
                created_at: Instant::now(),
            });
        }
    }
}

impl Default for CachedLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CachedLayer {
    fn stage(&self) -> Stage {
        Stage::Caching
    }
    
    fn read_file(&self, path: &Path, next: &dyn Next) -> VfsResult<Vec<u8>> {
        // Try cache first
        if let Some(cached) = self.get_cached(path) {
            return Ok(cached);
        }
        
        // Cache miss, read from next layer
        let content = next.read_file(path)?;
        
        // Store in cache
        self.insert(path.to_path_buf(), content.clone());
        
        Ok(content)
    }
    
    fn write_file(&self, path: &Path, content: &[u8], next: &dyn Next) -> VfsResult<()> {
        // Invalidate cache on write
        self.invalidate(path);
        next.write_file(path, content)
    }
}
