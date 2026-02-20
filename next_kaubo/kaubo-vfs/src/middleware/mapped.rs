//! Path mapping middleware for module resolution

use std::path::{Path, PathBuf};
use crate::VfsResult;
use super::{Middleware, Next};
use super::Stage;

/// Module context containing search paths
#[derive(Debug, Clone)]
pub struct ModuleContext {
    /// Search paths for modules (higher priority first)
    pub search_paths: Vec<PathBuf>,
}

impl ModuleContext {
    /// Create a new module context with given search paths
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }
    
    /// Create default context with standard paths
    pub fn default() -> Self {
        let mut paths = vec![PathBuf::from("./src")];
        
        // Add system standard library path
        if let Ok(std_path) = detect_std_path() {
            paths.push(std_path);
        }
        
        Self { search_paths: paths }
    }
    
    /// Create context from environment
    pub fn from_env() -> Self {
        let mut paths = vec![PathBuf::from("./src")];
        
        // KABO_PATH environment variable (like PYTHONPATH)
        if let Ok(path_str) = std::env::var("KABO_PATH") {
            for p in path_str.split(':') {
                paths.push(PathBuf::from(p));
            }
        }
        
        if let Ok(std_path) = detect_std_path() {
            paths.push(std_path);
        }
        
        Self { search_paths: paths }
    }
}

/// Detect system standard library path
fn detect_std_path() -> Result<PathBuf, std::env::VarError> {
    // Check environment variable first
    if let Ok(path) = std::env::var("KABO_STD_PATH") {
        return Ok(path.into());
    }
    
    // Platform defaults
    #[cfg(target_os = "linux")]
    return Ok(PathBuf::from("/opt/kaubo/std"));
    
    #[cfg(target_os = "macos")]
    return Ok(PathBuf::from("/usr/local/lib/kaubo/std"));
    
    #[cfg(target_os = "windows")]
    return Ok(PathBuf::from(r"C:\Program Files\Kaubo\std"));
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err(std::env::VarError::NotPresent);
}

/// Middleware that maps logical module paths to physical file paths
pub struct MappedLayer {
    ctx: ModuleContext,
}

impl MappedLayer {
    /// Create a new mapped layer with given context
    pub fn new(ctx: ModuleContext) -> Self {
        Self { ctx }
    }
    
    /// Resolve a VFS path to physical path
    /// 
    /// Module paths start with "/mod/" (e.g., "/mod/math.utils")
    /// Returns None if not a module path
    pub fn resolve(&self, path: &Path) -> Option<PathBuf> {
        let path_str = path.to_string_lossy();
        
        // Check if it's a module path
        let module_part = path_str.strip_prefix("/mod/")?;
        
        // Convert dots to path separators
        // math.utils -> math/utils.kaubo
        let relative: PathBuf = module_part.split('.').collect();
        let file_path = relative.with_extension("kaubo");
        
        // Search in paths (higher priority first)
        for search_path in &self.ctx.search_paths {
            let full_path = search_path.join(&file_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        // Not found, return first search path for error reporting
        self.ctx.search_paths.first()
            .map(|p| p.join(&file_path))
    }
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping
    }
    
    fn read_file(&self, path: &Path, next: &dyn Next) -> VfsResult<Vec<u8>> {
        match self.resolve(path) {
            Some(real_path) => next.read_file(&real_path),
            None => next.read_file(path),
        }
    }
    
    fn write_file(&self, path: &Path, content: &[u8], next: &dyn Next) -> VfsResult<()> {
        match self.resolve(path) {
            Some(real_path) => next.write_file(&real_path, content),
            None => next.write_file(path, content),
        }
    }
    
    fn exists(&self, path: &Path, next: &dyn Next) -> bool {
        match self.resolve(path) {
            Some(real_path) => next.exists(&real_path),
            None => next.exists(path),
        }
    }
    
    fn is_file(&self, path: &Path, next: &dyn Next) -> bool {
        match self.resolve(path) {
            Some(real_path) => next.is_file(&real_path),
            None => next.is_file(path),
        }
    }
}
