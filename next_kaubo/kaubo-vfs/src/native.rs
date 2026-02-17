//! Native file system implementation

use crate::error::{VfsError, VfsResult};
use crate::VirtualFileSystem;
use std::path::Path;
use std::vec::Vec;

/// A native OS file system implementation.
///
/// This wraps `std::fs` operations and provides the `VirtualFileSystem`
/// interface for local file access.
///
/// # Example
/// ```
/// use kaubo_vfs::{NativeFileSystem, VirtualFileSystem};
/// use std::path::Path;
///
/// let fs = NativeFileSystem::new();
/// // fs.write_file(Path::new("/tmp/test.txt"), b"hello").unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct NativeFileSystem {
    // Placeholder for future configuration (e.g., base directory, sandbox)
}

impl NativeFileSystem {
    /// Create a new native file system.
    pub fn new() -> Self {
        Self {}
    }

    /// Create a new native file system with a base directory.
    ///
    /// All paths will be relative to this base directory.
    ///
    /// # Arguments
    /// * `base` - The base directory for all file operations
    pub fn with_base(base: &Path) -> Self {
        let _ = base;
        // TODO: Implement base directory support
        Self::new()
    }
}

impl Default for NativeFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualFileSystem for NativeFileSystem {
    fn read_file(&self, path: &Path) -> VfsResult<Vec<u8>> {
        std::fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VfsError::NotFound {
                    path: path.to_string_lossy().to_string(),
                }
            } else {
                e.into()
            }
        })
    }

    fn write_file(&self, path: &Path, content: &[u8]) -> VfsResult<()> {
        std::fs::write(path, content).map_err(|e| e.into())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("kaubo_vfs_{}_{}", name, std::process::id()))
    }

    #[test]
    fn test_native_exists() {
        let fs = NativeFileSystem::new();
        let temp_file = temp_file("exists");

        // Clean up if exists
        let _ = std::fs::remove_file(&temp_file);

        assert!(!fs.exists(&temp_file));

        // Create file
        {
            let mut file = std::fs::File::create(&temp_file).unwrap();
            file.write_all(b"test").unwrap();
        }

        assert!(fs.exists(&temp_file));

        // Clean up
        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_native_read_write() {
        let fs = NativeFileSystem::new();
        let temp_file = temp_file("rw");

        // Clean up if exists
        let _ = std::fs::remove_file(&temp_file);

        // Write
        fs.write_file(&temp_file, b"hello native").unwrap();

        // Read
        let content = fs.read_file(&temp_file).unwrap();
        assert_eq!(content, b"hello native");

        // Clean up
        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_native_empty_file() {
        let fs = NativeFileSystem::new();
        let temp_file = temp_file("empty");

        let _ = std::fs::remove_file(&temp_file);

        // Write empty content
        fs.write_file(&temp_file, b"").unwrap();

        // Read should succeed with empty vec
        let content = fs.read_file(&temp_file).unwrap();
        assert!(content.is_empty());

        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_native_binary_data() {
        let fs = NativeFileSystem::new();
        let temp_file = temp_file("binary");

        let _ = std::fs::remove_file(&temp_file);

        let binary_data: Vec<u8> = (0..=255).collect();
        fs.write_file(&temp_file, &binary_data).unwrap();

        let content = fs.read_file(&temp_file).unwrap();
        assert_eq!(content, binary_data);

        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_native_is_file_and_dir() {
        let fs = NativeFileSystem::new();
        let temp_file_path = temp_file("type_file");
        let temp_dir_path = temp_file("type_dir");

        // Clean up
        let _ = std::fs::remove_file(&temp_file_path);
        let _ = std::fs::remove_dir(&temp_dir_path);

        // Create file
        {
            let mut file = std::fs::File::create(&temp_file_path).unwrap();
            file.write_all(b"test").unwrap();
        }

        // Create dir
        std::fs::create_dir(&temp_dir_path).unwrap();

        assert!(fs.is_file(&temp_file_path));
        assert!(!fs.is_dir(&temp_file_path));

        assert!(!fs.is_file(&temp_dir_path));
        assert!(fs.is_dir(&temp_dir_path));

        // Clean up
        std::fs::remove_file(&temp_file_path).unwrap();
        std::fs::remove_dir(&temp_dir_path).unwrap();
    }

    #[test]
    fn test_native_nonexistent_path() {
        let fs = NativeFileSystem::new();
        let nonexistent = temp_file("nonexistent_xyz");

        // Ensure it doesn't exist
        let _ = std::fs::remove_file(&nonexistent);
        let _ = std::fs::remove_dir(&nonexistent);

        assert!(!fs.exists(&nonexistent));
        assert!(!fs.is_file(&nonexistent));
        assert!(!fs.is_dir(&nonexistent));
    }

    #[test]
    fn test_native_read_nonexistent() {
        let fs = NativeFileSystem::new();
        let temp_file = temp_file("nonexistent");

        // Ensure it doesn't exist
        let _ = std::fs::remove_file(&temp_file);

        let result = fs.read_file(&temp_file);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VfsError::NotFound { .. }));
    }

    #[test]
    fn test_native_overwrite() {
        let fs = NativeFileSystem::new();
        let temp_file = temp_file("overwrite");

        let _ = std::fs::remove_file(&temp_file);

        fs.write_file(&temp_file, b"first").unwrap();
        fs.write_file(&temp_file, b"second").unwrap();

        let content = fs.read_file(&temp_file).unwrap();
        assert_eq!(content, b"second");

        std::fs::remove_file(&temp_file).unwrap();
    }
}
