//! WASM 平台支持
//!
//! 使用 web-sys 的 console API 输出日志。

use crate::logger::LogSink;
use crate::record::{Level, Record};
use alloc::sync::Arc;
use web_sys::console;

/// WASM 日志配置
#[derive(Clone, Debug)]
pub struct WasmConfig {
    /// 日志级别
    pub level: Level,
    /// 是否使用 console.debug
    pub use_debug: bool,
    /// 是否包含时间戳
    pub include_timestamp: bool,
}

impl WasmConfig {
    /// 创建默认配置
    pub fn new(level: Level) -> Self {
        WasmConfig {
            level,
            use_debug: true,
            include_timestamp: true,
        }
    }

    /// 开发配置
    pub fn dev() -> Self {
        Self::new(Level::Debug)
    }

    /// 生产配置
    pub fn production() -> Self {
        WasmConfig {
            level: Level::Warn,
            use_debug: false,
            include_timestamp: false,
        }
    }

    /// 初始化 WASM 日志器
    pub fn init(self) -> Arc<WasmLogger> {
        WasmLogger::new(self)
    }
}

/// WASM 专用日志器
pub struct WasmLogger {
    config: WasmConfig,
    level: core::sync::atomic::AtomicU8,
}

impl WasmLogger {
    /// 创建新的 WASM 日志器
    pub fn new(config: WasmConfig) -> Arc<Self> {
        Arc::new(WasmLogger {
            level: core::sync::atomic::AtomicU8::new(config.level as u8),
            config,
        })
    }

    /// 记录日志
    pub fn log(&self, level: Level, target: &str, message: &str) {
        if level < self.level() {
            return;
        }

        let prefix = if self.config.include_timestamp {
            alloc::format!("[{}] [{}] ", level.as_str(), target)
        } else {
            alloc::format!("[{}] ", level.as_str())
        };

        let full_message = alloc::format!("{}{}", prefix, message);

        match level {
            Level::Trace | Level::Debug => {
                if self.config.use_debug {
                    console::debug_1(&full_message.into());
                } else {
                    console::log_1(&full_message.into());
                }
            }
            Level::Info => console::info_1(&full_message.into()),
            Level::Warn => console::warn_1(&full_message.into()),
            Level::Error => console::error_1(&full_message.into()),
        }
    }

    /// 设置日志级别
    pub fn set_level(&self, level: Level) {
        self.level
            .store(level as u8, core::sync::atomic::Ordering::Relaxed);
    }

    /// 获取日志级别
    pub fn level(&self) -> Level {
        Level::from_u8(self.level.load(core::sync::atomic::Ordering::Relaxed))
            .unwrap_or(Level::Info)
    }

    /// 检查级别是否启用
    pub fn is_enabled(&self, level: Level) -> bool {
        level >= self.level()
    }
}

impl LogSink for WasmLogger {
    fn write(&self, record: &Record) {
        self.log(record.level, record.target, &record.message);
    }
}

impl LogSink for Arc<WasmLogger> {
    fn write(&self, record: &Record) {
        self.log(record.level, record.target, &record.message);
    }
}

/// WASM 专用日志宏
#[macro_export]
macro_rules! wasm_log {
    ($logger:expr, $level:expr, $($arg:tt)*) => {{
        if $logger.is_enabled($level) {
            let message = alloc::format!($($arg)*);
            $logger.log($level, module_path!(), &message);
        }
    }};
}

/// WASM 专用 trace 宏
#[macro_export]
macro_rules! wasm_trace {
    ($logger:expr, $($arg:tt)*) => {
        $crate::wasm_log!($logger, $crate::Level::Trace, $($arg)*)
    };
}

/// WASM 专用 debug 宏
#[macro_export]
macro_rules! wasm_debug {
    ($logger:expr, $($arg:tt)*) => {
        $crate::wasm_log!($logger, $crate::Level::Debug, $($arg)*)
    };
}

/// WASM 专用 info 宏
#[macro_export]
macro_rules! wasm_info {
    ($logger:expr, $($arg:tt)*) => {
        $crate::wasm_log!($logger, $crate::Level::Info, $($arg)*)
    };
}

/// WASM 专用 warn 宏
#[macro_export]
macro_rules! wasm_warn {
    ($logger:expr, $($arg:tt)*) => {
        $crate::wasm_log!($logger, $crate::Level::Warn, $($arg)*)
    };
}

/// WASM 专用 error 宏
#[macro_export]
macro_rules! wasm_error {
    ($logger:expr, $($arg:tt)*) => {
        $crate::wasm_log!($logger, $crate::Level::Error, $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_config_new() {
        let config = WasmConfig::new(Level::Debug);
        assert_eq!(config.level, Level::Debug);
        assert!(config.use_debug);
        assert!(config.include_timestamp);
    }

    #[test]
    fn test_wasm_config_dev() {
        let config = WasmConfig::dev();
        assert_eq!(config.level, Level::Debug);
        assert!(config.use_debug);
        assert!(config.include_timestamp);
    }

    #[test]
    fn test_wasm_config_production() {
        let config = WasmConfig::production();
        assert_eq!(config.level, Level::Warn);
        assert!(!config.use_debug);
        assert!(!config.include_timestamp);
    }

    #[test]
    fn test_wasm_logger_new() {
        let config = WasmConfig::new(Level::Info);
        let logger = WasmLogger::new(config);
        assert_eq!(logger.level(), Level::Info);
    }

    #[test]
    fn test_wasm_logger_level_change() {
        let config = WasmConfig::new(Level::Debug);
        let logger = WasmLogger::new(config);

        assert!(logger.is_enabled(Level::Debug));
        assert!(!logger.is_enabled(Level::Trace));

        logger.set_level(Level::Trace);
        assert!(logger.is_enabled(Level::Trace));
    }

    // 注意：以下测试需要浏览器环境，使用 wasm-pack test 运行
    // #[wasm_bindgen_test]
    // fn test_wasm_logger_log() { ... }
}
