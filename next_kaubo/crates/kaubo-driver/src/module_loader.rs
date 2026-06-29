//! 模块加载器——文件系统抽象。
//!
//! `ModuleLoader` trait 定义了模块系统如何读取和解析源文件。
//! 内置两个实现：`FileLoader`（文件系统）和 `MemLoader`（内存，用于测试/WASM）。

use crate::protocol::BuildError;
use kaubo_vfs::{VfsError, VirtualFileSystem};
use std::collections::HashMap;
use std::path::Path;

/// 模块加载器——解析路径并读取源文件。
///
/// Phase 3b 的 `ModuleGraph` 通过此 trait 发现和加载依赖。
pub trait ModuleLoader: Send + Sync {
    /// 读取 `path` 处的源文件内容。
    fn read(&self, path: &str) -> Result<String, BuildError>;

    /// 解析 import 路径。
    ///
    /// `from` 是包含 import 语句的模块路径，
    /// `import_path` 是 import 语句中的字符串（如 `"./math.kb"`）。
    ///
    /// 返回 `(resolved_path, canonical)`：
    /// - `resolved_path` 是规范化后的路径（如 `"math.kb"`）
    /// - `canonical` 是用于缓存的唯一键（同 `resolved_path`）
    fn resolve(&self, from: &str, import_path: &str) -> Result<(String, String), BuildError>;
}

// ── FileLoader：文件系统后端 ──

/// 基于 `kaubo_vfs::VirtualFileSystem` 的文件加载器。
pub struct FileLoader {
    vfs: Box<dyn VirtualFileSystem>,
}

impl FileLoader {
    /// 创建新的 FileLoader，使用给定的 VFS 实现。
    pub fn new(vfs: Box<dyn VirtualFileSystem>) -> Self {
        Self { vfs }
    }
}

impl ModuleLoader for FileLoader {
    fn read(&self, path: &str) -> Result<String, BuildError> {
        self.vfs.read(path).map_err(|e| match e {
            VfsError::NotFound { path } => BuildError::Build(format!("module not found: {path}")),
            VfsError::IoError { path, reason } => {
                BuildError::Build(format!("io error reading {path}: {reason}"))
            }
        })
    }

    fn resolve(&self, from: &str, import_path: &str) -> Result<(String, String), BuildError> {
        let resolved = normalize_path(from, import_path);
        // canonical 目前与 resolved 相同
        Ok((resolved.clone(), resolved))
    }
}

// ── MemLoader：内存后端（测试 / WASM） ──

/// 基于内存 `HashMap` 的模块加载器。
pub struct MemLoader {
    files: HashMap<String, String>,
}

impl MemLoader {
    /// 创建空的 MemLoader。
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// 向加载器中添加模块源码。
    pub fn insert(&mut self, path: &str, source: &str) {
        self.files.insert(path.to_string(), source.to_string());
    }
}

impl Default for MemLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleLoader for MemLoader {
    fn read(&self, path: &str) -> Result<String, BuildError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| BuildError::Build(format!("module not found in memory: {path}")))
    }

    fn resolve(&self, _from: &str, import_path: &str) -> Result<(String, String), BuildError> {
        // MemLoader 直接使用 import_path 作为 key
        let resolved = normalize_path("", import_path);
        Ok((resolved.clone(), resolved))
    }
}

// ── 路径规范化 ──

/// 规范化 import 路径。
///
/// 以 `base` 所在目录为基准，拼接 `import_path`，并处理 `./` 和 `../`。
pub fn normalize_path(base: &str, import_path: &str) -> String {
    if import_path.is_empty() {
        return String::new();
    }

    let parent = Path::new(base).parent().unwrap_or_else(|| Path::new(""));

    let joined = parent.join(import_path);

    // 标准化：去掉 ./ 和 ../
    let mut components: Vec<String> = Vec::new();
    for c in joined.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                components.pop();
            }
            other => {
                if let Some(s) = other.as_os_str().to_str() {
                    components.push(s.to_string());
                }
            }
        }
    }
    components.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_same_dir() {
        assert_eq!(normalize_path("main.kb", "./math.kb"), "math.kb");
    }

    #[test]
    fn normalize_parent_dir() {
        assert_eq!(normalize_path("sub/main.kb", "../lib.kb"), "lib.kb");
    }

    #[test]
    fn normalize_nested() {
        assert_eq!(
            normalize_path("src/a/b.kb", "../../lib/math.kb"),
            "lib/math.kb"
        );
    }

    #[test]
    fn normalize_absolute() {
        // 保留相对基准的绝对路径
        let result = normalize_path("main.kb", "std/math.kb");
        assert!(result.ends_with("std/math.kb"));
    }

    #[test]
    fn mem_loader_insert_and_read() {
        let mut loader = MemLoader::new();
        loader.insert("math.kb", "export const PI = 3.14;");
        assert_eq!(loader.read("math.kb").unwrap(), "export const PI = 3.14;");
    }

    #[test]
    fn mem_loader_missing() {
        let loader = MemLoader::new();
        assert!(loader.read("nope.kb").is_err());
    }

    #[test]
    fn mem_loader_resolve() {
        let loader = MemLoader::new();
        let (resolved, canonical) = loader.resolve("main.kb", "./math.kb").unwrap();
        assert_eq!(resolved, "math.kb");
        assert_eq!(canonical, "math.kb");
    }
}
