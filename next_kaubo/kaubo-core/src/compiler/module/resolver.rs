//! 模块解析器
//!
//! 将 import 路径解析为文件路径，并加载模块。
//!
//! # 解析规则
//! - `import math;` → 查找 `math.kaubo`
//! - `import std.list;` → 查找 `std/list.kaubo`
//! - `from math import add;` → 查找 `math.kaubo`，提取 `add` 导出

use kaubo_vfs::{VirtualFileSystem, VfsError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::compiler::lexer::builder::build_lexer;
use crate::compiler::parser::module::Module;
use crate::compiler::parser::parser::Parser;

/// 模块解析错误
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    /// 模块未找到
    NotFound {
        /// import 路径
        import_path: String,
        /// 尝试过的文件路径
        tried: Vec<PathBuf>,
    },
    /// 文件读取错误
    ReadError {
        /// 文件路径
        path: PathBuf,
        /// 错误信息
        message: String,
    },
    /// 解析错误
    ParseError {
        /// 文件路径
        path: PathBuf,
        /// 错误信息
        message: String,
    },
    /// 循环依赖
    CircularDependency {
        /// 依赖链
        chain: Vec<String>,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound { import_path, tried } => {
                write!(f, "Module '{}' not found. Tried:", import_path)?;
                for path in tried {
                    write!(f, "\n  - {}", path.display())?;
                }
                Ok(())
            }
            ResolveError::ReadError { path, message } => {
                write!(f, "Failed to read '{}': {}", path.display(), message)
            }
            ResolveError::ParseError { path, message } => {
                write!(f, "Failed to parse '{}': {}", path.display(), message)
            }
            ResolveError::CircularDependency { chain } => {
                write!(f, "Circular dependency detected: {}", chain.join(" → "))
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// 解析后的模块
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// 模块 AST
    pub ast: Module,
    /// 绝对路径
    pub path: PathBuf,
    /// 导入路径（如 "math" 或 "std.list"）
    pub import_path: String,
    /// 源代码
    pub source: String,
}

/// 模块解析器
pub struct ModuleResolver {
    /// 虚拟文件系统
    vfs: Box<dyn VirtualFileSystem>,
    /// 项目根目录（所有模块路径基于此）
    root_dir: PathBuf,
    /// 模块缓存：import_path → ResolvedModule
    cache: HashMap<String, ResolvedModule>,
    /// 当前解析栈（用于检测循环依赖）
    resolving_stack: Vec<String>,
    /// 标准库路径（特殊处理）
    std_path: Option<PathBuf>,
}

impl ModuleResolver {
    /// 创建新的模块解析器
    ///
    /// # Arguments
    /// * `vfs` - 虚拟文件系统
    /// * `root_dir` - 项目根目录
    pub fn new(vfs: Box<dyn VirtualFileSystem>, root_dir: impl AsRef<Path>) -> Self {
        Self {
            vfs,
            root_dir: root_dir.as_ref().to_path_buf(),
            cache: HashMap::new(),
            resolving_stack: Vec::new(),
            std_path: None,
        }
    }

    /// 设置标准库路径
    pub fn with_std_path(mut self, std_path: impl AsRef<Path>) -> Self {
        self.std_path = Some(std_path.as_ref().to_path_buf());
        self
    }

    /// 解析并加载模块
    ///
    /// # Arguments
    /// * `import_path` - import 路径（如 "math" 或 "std.list"）
    ///
    /// # Returns
    /// 解析后的模块，或 ResolveError
    pub fn resolve(&mut self, import_path: &str) -> Result<&ResolvedModule, ResolveError> {
        // 检查缓存
        if self.cache.contains_key(import_path) {
            return Ok(self.cache.get(import_path).unwrap());
        }

        // 检测循环依赖
        if self.resolving_stack.contains(&import_path.to_string()) {
            let mut chain = self.resolving_stack.clone();
            chain.push(import_path.to_string());
            return Err(ResolveError::CircularDependency { chain });
        }

        // 推入解析栈
        self.resolving_stack.push(import_path.to_string());

        // 执行解析
        let result = self.resolve_uncached(import_path);

        // 弹出解析栈
        self.resolving_stack.pop();

        // 缓存结果
        if let Ok(ref module) = result {
            self.cache.insert(import_path.to_string(), module.clone());
        }

        result.map(|m| self.cache.get(import_path).unwrap())
    }

    /// 解析模块（无缓存）
    fn resolve_uncached(&self, import_path: &str) -> Result<ResolvedModule, ResolveError> {
        // 转换路径：std.list → std/list.kaubo
        let file_path = self.import_path_to_file_path(import_path);

        // 尝试多个位置
        let candidates = self.get_search_paths(&file_path, import_path);
        let mut tried_paths = Vec::new();

        for candidate in &candidates {
            tried_paths.push(candidate.clone());

            if !self.vfs.exists(candidate) {
                continue;
            }

            if !self.vfs.is_file(candidate) {
                continue;
            }

            // 读取文件
            let content = self
                .vfs
                .read_file(candidate)
                .map_err(|e| ResolveError::ReadError {
                    path: candidate.clone(),
                    message: e.to_string(),
                })?;

            let source = String::from_utf8(content).map_err(|e| ResolveError::ReadError {
                path: candidate.clone(),
                message: format!("Invalid UTF-8: {}", e),
            })?;

            // 解析 AST
            let ast = self.parse_module(&source, candidate)?;

            return Ok(ResolvedModule {
                ast,
                path: candidate.clone(),
                import_path: import_path.to_string(),
                source,
            });
        }

        Err(ResolveError::NotFound {
            import_path: import_path.to_string(),
            tried: tried_paths,
        })
    }

    /// 将 import 路径转换为文件路径（使用 '/' 作为分隔符）
    ///
    /// # Examples
    /// - "math" → "math.kaubo"
    /// - "std.list" → "std/list.kaubo"
    fn import_path_to_file_path(&self, import_path: &str) -> PathBuf {
        // 统一使用 '/' 作为分隔符，不依赖平台
        let parts: Vec<&str> = import_path.split('.').collect();
        let mut path_str = String::new();

        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                path_str.push('/');
            }
            if i == parts.len() - 1 {
                // 最后一部分加上 .kaubo 后缀
                path_str.push_str(part);
                path_str.push_str(".kaubo");
            } else {
                path_str.push_str(part);
            }
        }

        PathBuf::from(path_str)
    }

    /// 获取搜索路径列表
    fn get_search_paths(&self, file_path: &Path, import_path: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 标准库特殊处理
        if import_path == "std" || import_path.starts_with("std.") {
            if let Some(ref std_path) = self.std_path {
                // 移除 "std" 前缀
                let relative_path = if import_path == "std" {
                    PathBuf::from("std.kaubo")
                } else {
                    let without_std = &import_path[4..]; // 去掉 "std."
                    self.import_path_to_file_path(without_std)
                };
                paths.push(std_path.join(&relative_path));
                return paths;
            }
        }

        // 1. 相对于根目录
        paths.push(self.root_dir.join(file_path));

        // 2. 如果正在解析其他模块，也相对于当前模块的目录（TODO）

        paths
    }

    /// 解析模块源代码为 AST
    fn parse_module(&self, source: &str, path: &Path) -> Result<Module, ResolveError> {
        let mut lexer = build_lexer();
        lexer.feed(source.as_bytes()).map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            message: format!("Lexer error: {:?}", e),
        })?;
        lexer.terminate().map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            message: format!("Lexer error: {:?}", e),
        })?;

        let mut parser = Parser::new(lexer);
        let ast = parser.parse().map_err(|e| ResolveError::ParseError {
            path: path.to_path_buf(),
            message: format!("{}", e),
        })?;

        Ok(ast)
    }

    /// 获取缓存的模块
    pub fn get_cached(&self, import_path: &str) -> Option<&ResolvedModule> {
        self.cache.get(import_path)
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// 获取缓存大小
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// 直接从文件路径加载模块（不经过 import 路径解析）
    ///
    /// # Arguments
    /// * `path` - 文件路径
    /// * `import_path` - 用于标识的 import 路径
    ///
    /// # Returns
    /// 解析后的模块，或 ResolveError
    pub fn resolve_direct(
        &self,
        path: &Path,
        import_path: &str,
    ) -> Result<ResolvedModule, ResolveError> {
        // 读取文件
        let content = self
            .vfs
            .read_file(path)
            .map_err(|e| ResolveError::ReadError {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;

        let source = String::from_utf8(content).map_err(|e| ResolveError::ReadError {
            path: path.to_path_buf(),
            message: format!("Invalid UTF-8: {}", e),
        })?;

        // 解析 AST
        let ast = self.parse_module(&source, path)?;

        Ok(ResolvedModule {
            ast,
            path: path.to_path_buf(),
            import_path: import_path.to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_vfs::MemoryFileSystem;

    fn create_test_fs() -> MemoryFileSystem {
        MemoryFileSystem::with_files([
            ("/math.kaubo", b"var PI = 3.14;".to_vec()),
            ("/utils/string.kaubo", b"var version = \"1.0\";".to_vec()),
            (
                "/deep/nested/module.kaubo",
                b"var deep = true;".to_vec(),
            ),
        ])
    }

    #[test]
    fn test_resolve_simple_module() {
        let fs = create_test_fs();
        let mut resolver = ModuleResolver::new(Box::new(fs), "/");

        let module = resolver.resolve("math").unwrap();
        assert_eq!(module.import_path, "math");
        assert!(module.path.to_string_lossy().contains("math.kaubo"));
    }

    #[test]
    fn test_resolve_nested_module() {
        let fs = create_test_fs();
        let mut resolver = ModuleResolver::new(Box::new(fs), "/");

        let module = resolver.resolve("utils.string").unwrap();
        assert_eq!(module.import_path, "utils.string");
    }

    #[test]
    fn test_resolve_not_found() {
        let fs = create_test_fs();
        let mut resolver = ModuleResolver::new(Box::new(fs), "/");

        let result = resolver.resolve("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolveError::NotFound { .. }));
    }

    #[test]
    fn test_caching() {
        let fs = create_test_fs();
        let mut resolver = ModuleResolver::new(Box::new(fs), "/");

        // 第一次解析
        let _ = resolver.resolve("math").unwrap();
        assert_eq!(resolver.cache_size(), 1);

        // 第二次解析（从缓存）
        let _ = resolver.resolve("math").unwrap();
        assert_eq!(resolver.cache_size(), 1); // 缓存没有增加
    }

    #[test]
    fn test_import_path_to_file_path() {
        let fs = MemoryFileSystem::new();
        let resolver = ModuleResolver::new(Box::new(fs), "/");

        assert_eq!(
            resolver.import_path_to_file_path("math"),
            PathBuf::from("math.kaubo")
        );
        assert_eq!(
            resolver.import_path_to_file_path("std.list"),
            PathBuf::from("std/list.kaubo")
        );
        assert_eq!(
            resolver.import_path_to_file_path("a.b.c"),
            PathBuf::from("a/b/c.kaubo")
        );
    }
}
