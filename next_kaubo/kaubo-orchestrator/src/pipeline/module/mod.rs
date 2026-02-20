//! 模块系统
//!
//! 提供模块解析、加载和缓存功能。
//! 单文件即模块：每个 .kaubo 文件是一个独立的模块。

pub mod multi_file;
pub mod resolver;

pub use multi_file::{MultiFileCompiler, MultiFileCompileResult, CompileUnit, MultiFileError};
pub use resolver::{ModuleResolver, ResolvedModule, ResolveError};

use crate::pipeline::parser::module::Module;

/// 模块缓存项
#[derive(Debug, Clone)]
pub struct ModuleCacheEntry {
    /// 模块 AST
    pub ast: Module,
    /// 文件路径
    pub path: std::path::PathBuf,
    /// 源代码哈希（用于热重载检测变化）
    pub source_hash: u64,
}
