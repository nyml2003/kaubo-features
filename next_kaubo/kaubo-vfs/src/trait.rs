//! VirtualFileSystem trait definition

use crate::error::VfsResult;
use std::path::Path;
use std::vec::Vec;

/// Virtual File System trait
///
/// Provides a unified interface for file operations, decoupling code from
/// specific file system implementations.
///
/// # Implementations
/// - `MemoryFileSystem`: In-memory file system
/// - `NativeFileSystem`: Native OS file system
pub trait VirtualFileSystem: Send + Sync {
    /// Read file contents
    ///
    /// # Arguments
    /// * `path` - File path
    ///
    /// # Returns
    /// File contents as bytes, or VfsError
    fn read_file(&self, path: &Path) -> VfsResult<Vec<u8>>;

    /// Write file contents
    ///
    /// Creates the file if it doesn't exist, truncates it if it does.
    ///
    /// # Arguments
    /// * `path` - File path
    /// * `content` - Content to write
    ///
    /// # Returns
    /// Ok(()) on success, or VfsError
    fn write_file(&self, path: &Path, content: &[u8]) -> VfsResult<()>;

    /// Check if path exists
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// true if the path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if path is a file
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// true if the path exists and is a file
    fn is_file(&self, path: &Path) -> bool;

    /// Check if path is a directory
    ///
    /// # Arguments
    /// * `path` - Path to check
    ///
    /// # Returns
    /// true if the path exists and is a directory
    fn is_dir(&self, path: &Path) -> bool;
}
