//! Kaubo - A modern scripting language
//!
//! Kaubo is a modern, concise scripting language designed for embedded scenarios
//! and rapid prototyping.
//!
//! # Architecture
//!
//! ```text
//! src/
//! ├── api/       - Public API layer (input → output)
//! ├── core/      - Pure compilation logic (no IO)
//! └── platform/  - Platform-specific adapters (IO, CLI formatting)
//! ```
//!
//! # Quick Start
//!
//! ```ignore
//! use kaubo::{init, compile_and_run, Config};
//!
//! init(Config::default());
//! let result = compile_and_run("return 1 + 2;").unwrap();
//! println!("Result: {:?}", result.value);
//! ```

// 核心层（纯逻辑，无 IO）
pub mod core;

// API 层（对外接口）
pub mod api;

// 平台适配层（IO、CLI 格式化）
pub mod platform;

// 重导出常用类型
pub use api::{compile, compile_and_run, execute, CompileOutput, ExecuteOutput, KauboError};
pub use api::{ErrorDetails, ErrorReport, LexerError, ParserError};
pub use core::{
    Config, LogConfig, CompilerConfig, LimitConfig, Phase,
    config::init as init_config, config::config,
    logger::init_logger, logger::LogFormat,
};
pub use core::runtime::Value;

/// 初始化（使用前先调用）
///
/// 只初始化配置，不初始化日志系统。
/// 在 CLI/平台层需要额外调用日志初始化。
///
/// # Example
/// ```ignore
/// use kaubo::{init, Config};
///
/// init(Config::default());
/// ```
pub fn init(config: Config) {
    core::config::init(config);
}

/// 初始化配置和日志系统
///
/// 适用于简单的使用场景，CLI 建议使用 platform 层的初始化。
///
/// # Example
/// ```ignore
/// use kaubo::{init_with_logger, Config, LogFormat};
///
/// init_with_logger(Config::default(), LogFormat::Pretty);
/// ```
pub fn init_with_logger(config: Config, format: LogFormat) {
    core::config::init(config);
    core::logger::init_with_format(format);
}

/// 快速执行（使用默认配置）
///
/// # Example
/// ```ignore
/// use kaubo::quick_run;
///
/// let result = quick_run("return 42;").unwrap();
/// ```
pub fn quick_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    if !core::config::is_initialized() {
        init(Config::default());
    }
    compile_and_run(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        // 注意：这个测试可能因为配置已初始化而失败
        // 在实际测试中使用 quick_run 代替
        if !core::config::is_initialized() {
            init(Config::default());
        }
        assert!(core::config::is_initialized());
    }

    #[test]
    fn test_quick_run() {
        let result = quick_run("return 42;").unwrap();
        let value = result.value.as_ref().and_then(|v| v.as_int());
        assert_eq!(value, Some(42));
    }
}
