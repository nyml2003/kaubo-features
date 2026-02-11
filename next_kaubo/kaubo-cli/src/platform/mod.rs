//! Platform - 平台适配层
//!
//! 所有 IO 副作用都在这里实现：
//! - CLI 格式化输出
//! - 文件系统操作
//! - 日志初始化（tracing subscriber）

pub mod cli;

// 重导出 CLI 功能
pub use cli::{print_error_with_source, print_source_context};
