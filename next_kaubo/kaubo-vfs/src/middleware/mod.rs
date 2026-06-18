//! VFS Middleware System
//!
//! Provides a composable middleware layer for the Virtual File System.

mod builder;
mod layered;
mod middleware;
mod stage;

// Re-export core types
pub use builder::VfsBuilder;
pub use layered::LayeredVFS;
pub use middleware::{Middleware, Next};
pub use stage::Stage;

// Re-export built-in middlewares
pub mod cached;
pub mod logged;
pub mod mapped;

pub use cached::CachedLayer;
pub use logged::LoggedLayer;
pub use mapped::{MappedLayer, ModuleContext};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFileSystem;
    use crate::VirtualFileSystem;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Clone)]
    struct TraceFs {
        inner: MemoryFileSystem,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl TraceFs {
        fn new(events: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                inner: MemoryFileSystem::new(),
                events,
            }
        }

        fn with_file(events: Arc<Mutex<Vec<String>>>, path: &str, content: &[u8]) -> Self {
            Self {
                inner: MemoryFileSystem::with_files([(path, content.to_vec())]),
                events,
            }
        }

        fn push(&self, msg: impl Into<String>) {
            self.events.lock().unwrap().push(msg.into());
        }
    }

    impl crate::VirtualFileSystem for TraceFs {
        fn read_file(&self, path: &Path) -> crate::VfsResult<Vec<u8>> {
            self.push(format!("backend:read:{}", path.display()));
            self.inner.read_file(path)
        }

        fn write_file(&self, path: &Path, content: &[u8]) -> crate::VfsResult<()> {
            self.push(format!("backend:write:{}", path.display()));
            self.inner.write_file(path, content)
        }

        fn exists(&self, path: &Path) -> bool {
            self.push(format!("backend:exists:{}", path.display()));
            self.inner.exists(path)
        }

        fn is_file(&self, path: &Path) -> bool {
            self.push(format!("backend:is_file:{}", path.display()));
            self.inner.is_file(path)
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.push(format!("backend:is_dir:{}", path.display()));
            self.inner.is_dir(path)
        }
    }

    #[derive(Clone)]
    struct TraceLayer {
        name: &'static str,
        stage: Stage,
        events: Arc<Mutex<Vec<String>>>,
    }

    impl TraceLayer {
        fn new(name: &'static str, stage: Stage, events: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name,
                stage,
                events,
            }
        }

        fn push(&self, msg: impl Into<String>) {
            self.events.lock().unwrap().push(msg.into());
        }
    }

    impl Middleware for TraceLayer {
        fn stage(&self) -> Stage {
            self.stage
        }

        fn read_file(&self, path: &Path, next: &dyn Next) -> crate::VfsResult<Vec<u8>> {
            self.push(format!("{}:read:pre", self.name));
            let result = next.read_file(path);
            self.push(format!("{}:read:post", self.name));
            result
        }

        fn write_file(&self, path: &Path, content: &[u8], next: &dyn Next) -> crate::VfsResult<()> {
            self.push(format!("{}:write:pre", self.name));
            let result = next.write_file(path, content);
            self.push(format!("{}:write:post", self.name));
            result
        }

        fn exists(&self, path: &Path, next: &dyn Next) -> bool {
            self.push(format!("{}:exists:pre", self.name));
            let result = next.exists(path);
            self.push(format!("{}:exists:post", self.name));
            result
        }

        fn is_file(&self, path: &Path, next: &dyn Next) -> bool {
            self.push(format!("{}:is_file:pre", self.name));
            let result = next.is_file(path);
            self.push(format!("{}:is_file:post", self.name));
            result
        }

        fn is_dir(&self, path: &Path, next: &dyn Next) -> bool {
            self.push(format!("{}:is_dir:pre", self.name));
            let result = next.is_dir(path);
            self.push(format!("{}:is_dir:post", self.name));
            result
        }
    }

    struct NoopLayer(Stage);

    impl Middleware for NoopLayer {
        fn stage(&self) -> Stage {
            self.0
        }
    }

    #[test]
    fn stage_priorities_are_ordered() {
        assert!(Stage::Outer < Stage::PreProcess);
        assert!(Stage::Mapping < Stage::Caching);
        assert_eq!(Stage::default(), Stage::Outer);
        assert_eq!(Stage::Caching.priority(), 400);
    }

    #[test]
    fn builder_sorts_layers_by_stage() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let backend = TraceFs::new(events.clone());
        let vfs = VfsBuilder::new(backend)
            .with(TraceLayer::new("mapping", Stage::Mapping, events.clone()))
            .with(TraceLayer::new("outer", Stage::Outer, events.clone()))
            .build();

        vfs.write_file(Path::new("/a.txt"), b"hello").unwrap();

        assert_eq!(
            events.lock().unwrap().clone(),
            vec![
                "outer:write:pre",
                "mapping:write:pre",
                "backend:write:/a.txt",
                "mapping:write:post",
                "outer:write:post",
            ]
        );
    }

    #[test]
    fn layered_vfs_default_middleware_methods_delegate() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let backend = TraceFs::with_file(events.clone(), "/file.txt", b"hello");
        let vfs = LayeredVFS::new(Arc::new(backend), vec![Box::new(NoopLayer(Stage::Outer))]);

        assert_eq!(vfs.read_file(Path::new("/file.txt")).unwrap(), b"hello");
        assert!(vfs.exists(Path::new("/file.txt")));
        assert!(vfs.is_file(Path::new("/file.txt")));
        assert!(!vfs.is_dir(Path::new("/file.txt")));

        let events = events.lock().unwrap().clone();
        assert!(events.contains(&"backend:read:/file.txt".to_string()));
        assert!(events.contains(&"backend:exists:/file.txt".to_string()));
        assert!(events.contains(&"backend:is_file:/file.txt".to_string()));
        assert!(events.contains(&"backend:is_dir:/file.txt".to_string()));
    }

    #[test]
    fn logged_and_cached_layers_work_with_backend() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let backend = TraceFs::with_file(events.clone(), "/cache.txt", b"first");
        let vfs = LayeredVFS::new(
            Arc::new(backend),
            vec![
                Box::new(LoggedLayer::new()),
                Box::new(CachedLayer::with_ttl(Duration::from_millis(0))),
            ],
        );

        assert_eq!(vfs.read_file(Path::new("/cache.txt")).unwrap(), b"first");
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(vfs.read_file(Path::new("/cache.txt")).unwrap(), b"first");

        let trace = events.lock().unwrap().clone();
        assert!(trace.iter().any(|e| e == "backend:read:/cache.txt"));
    }

    #[test]
    fn cached_layer_hits_and_invalidates() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let backend = TraceFs::with_file(events.clone(), "/memo.txt", b"old");
        let vfs = LayeredVFS::new(Arc::new(backend), vec![Box::new(CachedLayer::new())]);

        assert_eq!(vfs.read_file(Path::new("/memo.txt")).unwrap(), b"old");
        assert_eq!(vfs.read_file(Path::new("/memo.txt")).unwrap(), b"old");
        vfs.write_file(Path::new("/memo.txt"), b"new").unwrap();
        assert_eq!(vfs.read_file(Path::new("/memo.txt")).unwrap(), b"new");

        let trace = events.lock().unwrap().clone();
        assert_eq!(
            trace
                .iter()
                .filter(|e| e.starts_with("backend:read"))
                .count(),
            2
        );
        assert!(trace.iter().any(|e| e == "backend:write:/memo.txt"));
    }

    #[test]
    fn mapped_layer_resolves_module_paths() {
        let temp = std::env::temp_dir().join(format!("kaubo_vfs_mapped_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("math")).unwrap();
        let physical = temp.join("math/utils.kaubo");
        std::fs::write(&physical, b"mapped").unwrap();

        let events = Arc::new(Mutex::new(Vec::new()));
        let backend = TraceFs::with_file(events.clone(), physical.to_str().unwrap(), b"mapped");
        let vfs = LayeredVFS::new(
            Arc::new(backend),
            vec![Box::new(MappedLayer::new(ModuleContext::new(vec![
                PathBuf::from(&temp),
            ])))],
        );

        assert_eq!(
            vfs.read_file(Path::new("/mod/math.utils")).unwrap(),
            b"mapped"
        );
        assert!(vfs.exists(Path::new("/mod/math.utils")));
        assert!(vfs.is_file(Path::new("/mod/math.utils")));
        assert!(!vfs.is_dir(Path::new("/mod/math.utils")));

        let resolved = MappedLayer::new(ModuleContext::new(vec![PathBuf::from(&temp)]))
            .resolve(Path::new("/mod/math.utils"));
        assert_eq!(resolved.as_deref(), Some(physical.as_path()));
        assert!(
            MappedLayer::new(ModuleContext::new(vec![PathBuf::from(&temp)]))
                .resolve(Path::new("/plain.txt"))
                .is_none()
        );

        let _ = std::fs::remove_dir_all(&temp);
    }
}
