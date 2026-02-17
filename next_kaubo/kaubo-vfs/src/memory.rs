//! In-memory file system implementation

use crate::error::{VfsError, VfsResult};
use crate::VirtualFileSystem;
use std::collections::BTreeMap;
use std::path::Path;
use std::string::{String, ToString};
use std::sync::{Arc, RwLock};
use std::vec::Vec;

/// An in-memory file system implementation.
///
/// All files are stored in memory using a `BTreeMap`, making it suitable
/// for testing and scenarios where disk access is not desired.
///
/// # Example
/// ```
/// use kaubo_vfs::{MemoryFileSystem, VirtualFileSystem};
/// use std::path::Path;
///
/// let fs = MemoryFileSystem::new();
/// fs.write_file(Path::new("/test.txt"), b"hello").unwrap();
/// let content = fs.read_file(Path::new("/test.txt")).unwrap();
/// assert_eq!(content, b"hello");
/// ```
#[derive(Debug, Clone)]
pub struct MemoryFileSystem {
    files: Arc<RwLock<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryFileSystem {
    /// Create a new empty memory file system.
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Create a new memory file system pre-populated with files.
    ///
    /// # Arguments
    /// * `files` - Iterator of (path, content) tuples
    pub fn with_files<I, S>(files: I) -> Self
    where
        I: IntoIterator<Item = (S, Vec<u8>)>,
        S: AsRef<str>,
    {
        let fs = Self::new();
        {
            let mut map = fs.files.write().unwrap();
            for (path, content) in files {
                map.insert(path.as_ref().to_string(), content);
            }
        }
        fs
    }

    /// Normalize a path string for internal storage.
    /// Uses forward slashes consistently for cross-platform compatibility.
    fn normalize_path(&self, path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}

impl Default for MemoryFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualFileSystem for MemoryFileSystem {
    fn read_file(&self, path: &Path) -> VfsResult<Vec<u8>> {
        let normalized = self.normalize_path(path);
        let files = self.files.read().map_err(|_| VfsError::Custom {
            message: String::from("Lock poisoned"),
        })?;

        files
            .get(&normalized)
            .cloned()
            .ok_or_else(|| VfsError::NotFound {
                path: normalized.clone(),
            })
    }

    fn write_file(&self, path: &Path, content: &[u8]) -> VfsResult<()> {
        let normalized = self.normalize_path(path);
        let mut files = self.files.write().map_err(|_| VfsError::Custom {
            message: String::from("Lock poisoned"),
        })?;
        files.insert(normalized, content.to_vec());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let normalized = self.normalize_path(path);
        let files = match self.files.read() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        files.contains_key(&normalized)
    }

    fn is_file(&self, path: &Path) -> bool {
        // In memory FS, if it exists, it's a file (no directory support yet)
        self.exists(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        // TODO: Add directory support
        let _ = path;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_fs_is_empty() {
        let fs = MemoryFileSystem::new();
        assert!(!fs.exists(Path::new("/anything.txt")));
    }

    #[test]
    fn test_empty_content() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/empty.txt");

        fs.write_file(path, b"").unwrap();
        let content = fs.read_file(path).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_binary_content() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/binary.dat");

        let binary_data: Vec<u8> = (0..=255).collect();
        fs.write_file(path, &binary_data).unwrap();

        let content = fs.read_file(path).unwrap();
        assert_eq!(content, binary_data);
    }

    #[test]
    fn test_large_content() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/large.txt");

        let large_data = vec![b'x'; 1024 * 1024]; // 1MB
        fs.write_file(path, &large_data).unwrap();

        let content = fs.read_file(path).unwrap();
        assert_eq!(content.len(), large_data.len());
        assert_eq!(content, large_data);
    }

    #[test]
    fn test_special_path_characters() {
        let fs = MemoryFileSystem::new();
        // Test paths with various characters
        let paths = vec![
            "/file-with-dashes.txt",
            "/file_with_underscores.txt",
            "/dir.with.dots/file.txt",
            "/123numeric.txt",
        ];

        for path_str in &paths {
            let path = Path::new(path_str);
            fs.write_file(path, path_str.as_bytes()).unwrap();
            let content = fs.read_file(path).unwrap();
            assert_eq!(content, path_str.as_bytes());
        }
    }

    #[test]
    fn test_is_dir_always_false() {
        let fs = MemoryFileSystem::new();
        // MemoryFileSystem doesn't support directories yet
        assert!(!fs.is_dir(Path::new("/")));
        assert!(!fs.is_dir(Path::new("/some/dir")));
        
        fs.write_file(Path::new("/some/file.txt"), b"x").unwrap();
        assert!(!fs.is_dir(Path::new("/some/file.txt")));
    }

    #[test]
    fn test_clone_shares_data() {
        let fs1 = MemoryFileSystem::new();
        let path = Path::new("/shared.txt");

        fs1.write_file(path, b"shared").unwrap();

        let fs2 = fs1.clone();
        assert!(fs2.exists(path));
        assert_eq!(fs2.read_file(path).unwrap(), b"shared");

        // Write via fs2, should be visible in fs1
        fs2.write_file(path, b"modified").unwrap();
        assert_eq!(fs1.read_file(path).unwrap(), b"modified");
    }

    #[test]
    fn test_concurrent_reads() {
        let fs = MemoryFileSystem::with_files([("/test.txt", b"concurrent".to_vec())]);
        let mut handles = vec![];

        for _ in 0..10 {
            let fs_clone = fs.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let content = fs_clone.read_file(Path::new("/test.txt")).unwrap();
                    assert_eq!(content, b"concurrent");
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes() {
        let fs = MemoryFileSystem::new();
        let mut handles = vec![];

        for i in 0..10 {
            let fs_clone = fs.clone();
            let data = format!("data{}", i);
            handles.push(thread::spawn(move || {
                let path = Path::new("/concurrent.txt");
                for _ in 0..10 {
                    fs_clone.write_file(path, data.as_bytes()).unwrap();
                    let _ = fs_clone.read_file(path);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        
        // Just verify no panics occurred and file exists
        assert!(fs.exists(Path::new("/concurrent.txt")));
    }

    #[test]
    fn test_write_and_read() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/test.txt");

        fs.write_file(path, b"hello world").unwrap();

        let content = fs.read_file(path).unwrap();
        assert_eq!(content, b"hello world");
    }

    #[test]
    fn test_exists() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/exists.txt");

        assert!(!fs.exists(path));
        fs.write_file(path, b"content").unwrap();
        assert!(fs.exists(path));
    }

    #[test]
    fn test_is_file() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/file.txt");

        assert!(!fs.is_file(path));
        fs.write_file(path, b"content").unwrap();
        assert!(fs.is_file(path));
    }

    #[test]
    fn test_read_nonexistent() {
        let fs = MemoryFileSystem::new();
        let result = fs.read_file(Path::new("/nonexistent.txt"));

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VfsError::NotFound { .. }));
    }

    #[test]
    fn test_overwrite_file() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/overwrite.txt");

        fs.write_file(path, b"first").unwrap();
        fs.write_file(path, b"second").unwrap();

        let content = fs.read_file(path).unwrap();
        assert_eq!(content, b"second");
    }

    #[test]
    fn test_with_files() {
        let fs = MemoryFileSystem::with_files([
            ("/a.txt", b"content a".to_vec()),
            ("/b.txt", b"content b".to_vec()),
        ]);

        assert_eq!(fs.read_file(Path::new("/a.txt")).unwrap(), b"content a");
        assert_eq!(fs.read_file(Path::new("/b.txt")).unwrap(), b"content b");
    }

    #[test]
    fn test_multiple_files() {
        let fs = MemoryFileSystem::new();

        fs.write_file(Path::new("/dir1/a.txt"), b"a").unwrap();
        fs.write_file(Path::new("/dir1/b.txt"), b"b").unwrap();
        fs.write_file(Path::new("/dir2/c.txt"), b"c").unwrap();

        assert_eq!(fs.read_file(Path::new("/dir1/a.txt")).unwrap(), b"a");
        assert_eq!(fs.read_file(Path::new("/dir1/b.txt")).unwrap(), b"b");
        assert_eq!(fs.read_file(Path::new("/dir2/c.txt")).unwrap(), b"c");
    }
}
