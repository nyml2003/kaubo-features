//! Core - 纯编译逻辑，无 IO
//!
//! 包含所有编译器和运行时的核心业务逻辑，
//! 只操作内存数据结构，不包含任何文件 IO 或终端输出。

pub mod compiler;
pub mod config;
pub mod kit;
pub mod logger;
pub mod runtime;

// 重导出常用类型
pub use config::{Config, LogConfig, CompilerConfig, LimitConfig, Phase};
