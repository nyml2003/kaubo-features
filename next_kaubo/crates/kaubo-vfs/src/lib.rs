//! kaubo-vfs — 虚拟文件系统抽象
//!
//! 为模块系统提供平台无关的文件 IO：
//! - CLI 用 `FsVfs`（封装 `std::fs`）
//! - WASM / 测试用 `MemVfs`（内存 HashMap）
//!
//! 只读不写，不列目录，不缓存，不引入异步。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 虚拟文件系统操作错误。
#[derive(Debug, Clone)]
pub enum VfsError {
    /// 文件不存在。
    NotFound { path: String },
    /// IO 错误。
    IoError { path: String, reason: String },
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsError::NotFound { path } => write!(f, "file not found: {path}"),
            VfsError::IoError { path, reason } => write!(f, "io error reading {path}: {reason}"),
        }
    }
}

/// 虚拟文件系统。
///
/// 读取源码文件，不涉及写操作、目录遍历、缓存。
pub trait VirtualFileSystem: Send + Sync {
    /// 读取文件内容。
    ///
    /// `path` 是调用方提供的路径（可能包含相对路径）。
    /// 返回文件内容，或错误（文件不存在 / 权限等）。
    fn read(&self, path: &str) -> Result<String, VfsError>;
}

// ── FsVfs：文件系统后端（CLI） ──

/// 基于本地文件系统的 VFS 实现。
pub struct FsVfs {
    root: PathBuf,
}

impl FsVfs {
    /// 创建以 `root` 为根目录的 FsVfs。
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }
}

impl VirtualFileSystem for FsVfs {
    fn read(&self, path: &str) -> Result<String, VfsError> {
        let full = self
            .root
            .join(path)
            .canonicalize()
            .map_err(|_| VfsError::NotFound {
                path: path.to_string(),
            })?;
        // 安全检查：确保解析后的路径仍在 root 下
        if !full.starts_with(&self.root) {
            return Err(VfsError::NotFound {
                path: path.to_string(),
            });
        }
        std::fs::read_to_string(&full).map_err(|e| VfsError::IoError {
            path: path.to_string(),
            reason: e.to_string(),
        })
    }
}

// ── MemVfs：内存后端（WASM / 测试） ──

/// 基于内存 HashMap 的 VFS 实现。
#[derive(Default)]
pub struct MemVfs {
    files: HashMap<String, String>,
}

impl MemVfs {
    /// 创建空的 MemVfs。
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// 向虚拟文件系统中插入文件。
    pub fn insert(&mut self, path: &str, source: &str) {
        self.files.insert(path.to_string(), source.to_string());
    }
}

impl VirtualFileSystem for MemVfs {
    fn read(&self, path: &str) -> Result<String, VfsError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| VfsError::NotFound {
                path: path.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mem_vfs_read_existing() {
        let mut vfs = MemVfs::new();
        vfs.insert("main.kb", "const x = 42;");
        assert_eq!(vfs.read("main.kb").unwrap(), "const x = 42;");
    }

    #[test]
    fn mem_vfs_read_missing() {
        let vfs = MemVfs::new();
        assert!(matches!(
            vfs.read("nope.kb"),
            Err(VfsError::NotFound { .. })
        ));
    }

    #[test]
    fn mem_vfs_insert_overwrite() {
        let mut vfs = MemVfs::new();
        vfs.insert("a.kb", "v1");
        vfs.insert("a.kb", "v2");
        assert_eq!(vfs.read("a.kb").unwrap(), "v2");
    }

    #[test]
    fn mem_vfs_multiple_files() {
        let mut vfs = MemVfs::new();
        vfs.insert("a.kb", "A");
        vfs.insert("b.kb", "B");
        assert_eq!(vfs.read("a.kb").unwrap(), "A");
        assert_eq!(vfs.read("b.kb").unwrap(), "B");
    }
}
