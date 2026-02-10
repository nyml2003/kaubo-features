//! Kaubo - A modern scripting language
//!
//! Kaubo is a modern, concise scripting language designed for embedded scenarios
//! and rapid prototyping.
//!
//! # Quick Start
//!
//! ```
//! use kaubo::{init, compile_and_run, Config};
//!
//! init(Config::default());
//! let result = compile_and_run("return 1 + 2;").unwrap();
//! println!("Result: {:?}", result.value);
//! ```

// 模块声明
pub mod compiler;
pub mod kit;
pub mod runtime;

// 新增模块
pub mod api;
pub mod config;
pub mod logger;

// 重导出常用类型
pub use api::{compile, compile_and_run, execute, CompileOutput, ExecuteOutput, KauboError};
pub use config::{Config, LogConfig, LimitConfig, CompilerConfig, Phase, init as init_config, config};
pub use logger::{init_logger, init_with_file, LogFormat};
pub use runtime::Value;

/// 初始化（使用前先调用）
///
/// # Example
/// ```no_run
/// use kaubo::{init, Config};
///
/// init(Config::default());
/// ```
pub fn init(config: Config) {
    config::init(config);
    logger::init_logger();
}

/// 快速执行（使用默认配置）
///
/// # Example
/// ```no_run
/// use kaubo::quick_run;
///
/// let result = quick_run("return 42;").unwrap();
/// ```
pub fn quick_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    if !config::is_initialized() {
        init(Config::default());
    }
    compile_and_run(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init(Config::default());
        assert!(config::is_initialized());
    }

    #[test]
    fn test_quick_run() {
        let result = quick_run("return 42;").unwrap();
        // Value 比较使用 as_int()
        let value = result.value.as_ref().and_then(|v| v.as_int());
        assert_eq!(value, Some(42));
    }
}
