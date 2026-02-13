//! kaubo-log - 结构化日志系统
//!
//! 为 Kaubo 编译器和运行时设计的结构化日志系统，特点：
//! - **无平台耦合**：支持 `no_std` + `alloc`，通过 feature flag 选择平台
//! - **显式传递**：无全局logger，配置通过代码传入
//! - **非阻塞**：日志不卡主线程，满了覆盖旧数据
//! - **崩溃恢复**：环形缓冲区保留最后N条日志
//!
//! # 平台支持
//!
//! | Feature | 说明 | 适用场景 |
//! |---------|------|----------|
//! | `std` (默认) | 完整标准库支持 | 桌面/服务器 |
//! | `alloc` | 仅分配器，无std | 嵌入式 |
//! | `wasm` | Web 平台支持 | 浏览器 |
//!
//! # 快速开始
//!
//! ## 标准平台
//!
//! ```toml
//! [dependencies]
//! kaubo-log = { version = "0.1", features = ["stdout", "stderr", "file"] }
//! ```
//!
//! ```ignore
//! use kaubo_log::{LogConfig, debug};
//!
//! let (logger, ring) = LogConfig::dev().init();
//! debug!(logger, "应用启动成功");
//! ```
//!
//! ## WASM 平台
//!
//! ```toml
//! [dependencies]
//! kaubo-log = { version = "0.1", default-features = false, features = ["wasm"] }
//! ```
//!
//! ```ignore
//! use kaubo_log::{LogConfig, debug};
//!
//! let (logger, ring) = LogConfig::wasm().init();
//! debug!(logger, "WASM 启动成功");
//! ```
//!
//! ## no_std + alloc 平台
//!
//! ```toml
//! [dependencies]
//! kaubo-log = { version = "0.1", default-features = false, features = ["alloc"] }
//! ```
//!
//! ```ignore
//! use kaubo_log::{Logger, Level, LogRingBuffer, debug};
//!
//! // 仅支持环形缓冲区（无stdout/stderr）
//! let ring = LogRingBuffer::new(1000);
//! let logger = Logger::new(Level::Debug).with_sink(ring);
//! debug!(logger, "嵌入式日志");
//! ```
//!
//! # 命名规范
//!
//! ⚠️ **重要**：代码中禁止使用 `_` 开头的变量名。
//!
//! ```rust
//! // ❌ 错误
//! let _temp = 42;
//! let _unused = "hello";
//!
//! // ✅ 正确
//! let temp = 42;
//! let unused = "hello";
//! ```
//!
//! 原因：`_` 前缀在 Rust 中通常表示"故意不使用"，但 AI 生成代码时容易产生歧义，
//! 且不利于代码审查。如需忽略未使用变量，请显式使用 `drop()` 或添加 `#[allow(unused)]`。

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

// 核心模块（所有平台通用）
mod macros;
mod record;
mod span;

// 条件编译模块
#[cfg(feature = "alloc")]
mod ring_buffer;

#[cfg(any(feature = "std", feature = "alloc"))]
mod logger;

// 平台特定配置
#[cfg(feature = "std")]
mod config;

#[cfg(feature = "wasm")]
mod wasm;

// 核心导出（所有平台）
pub use record::{Level, Record};

// 宏通过 #[macro_export] 自动导出到 crate 根：
// trace!, debug!, info!, warn!, error!, log!

// 条件导出
#[cfg(feature = "alloc")]
pub use ring_buffer::{LogRingBuffer, RingBufferStats};

#[cfg(any(feature = "std", feature = "alloc"))]
pub use logger::{LogSink, Logger};

#[cfg(feature = "std")]
pub use logger::{FileSink, StderrSink, StdoutSink};

#[cfg(feature = "std")]
pub use config::{LogConfig, OutputConfig};

#[cfg(feature = "wasm")]
pub use wasm::{WasmConfig, WasmLogger};

pub use span::{Span, SpanId};

/// 日志结果类型
#[cfg(feature = "std")]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(not(feature = "std"))]
pub type Result<T> = core::result::Result<T, Error>;

/// 日志系统错误类型
#[derive(Debug)]
pub enum Error {
    /// 环形缓冲区已满（非覆盖模式下）
    BufferFull,
    /// IO错误（仅std平台）
    #[cfg(feature = "std")]
    Io(std::io::Error),
    /// 序列化错误
    Serialize(&'static str),
    /// 不支持的操作
    Unsupported,
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::BufferFull => write!(f, "Ring buffer full"),
            #[cfg(feature = "std")]
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::Serialize(msg) => write!(f, "Serialize error: {msg}"),
            Error::Unsupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

// 条件编译测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Error > Level::Warn);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(format!("{}", Error::BufferFull), "Ring buffer full");
        assert_eq!(
            format!("{}", Error::Serialize("test error")),
            "Serialize error: test error"
        );
        assert_eq!(
            format!("{}", Error::Unsupported),
            "Operation not supported on this platform"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        match err {
            Error::Io(_) => (),
            _ => panic!("Expected Io error"),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err = Error::Io(io_err);
        let msg = format!("{}", err);
        assert!(msg.contains("IO error"));
    }
}
