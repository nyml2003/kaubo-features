//! 日志系统初始化
//!
//! 基于 `tracing` 和 `tracing-subscriber` 实现分阶段日志控制。
//!
//! # 使用示例
//! ```ignore
//! use kaubo::config::{Config, init};
//! use kaubo::logger::init_logger;
//!
//! init(Config::default());
//! init_logger();
//! ```

use std::io;
use tracing_subscriber::{
    filter::Targets, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

use crate::core::config::{self, Phase};

/// 日志输出格式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    /// 彩色格式化（开发使用）
    Pretty,
    /// 紧凑格式
    Compact,
    /// JSON 格式（工具集成）
    Json,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Pretty
    }
}

/// 初始化日志系统
///
/// 必须在 `config::init()` 之后调用。
/// 根据配置中的日志级别设置各阶段的过滤。
pub fn init_logger() {
    init_with_format(LogFormat::default());
}

/// 使用指定格式初始化日志系统
pub fn init_with_format(format: LogFormat) {
    init_with_file(format, None::<&str>);
}

/// 使用文件输出初始化日志系统
///
/// # Arguments
/// * `format` - 日志格式
/// * `file` - 日志文件路径，None 表示只输出到控制台
pub fn init_with_file<P: AsRef<std::path::Path>>(format: LogFormat, file: Option<P>) {
    if !config::is_initialized() {
        panic!("Config must be initialized before logger");
    }

    let cfg = &config::config().log;

    // 构建各阶段的目标过滤器
    let targets = Targets::new()
        .with_default(cfg.global)
        .with_target("kaubo::lexer", cfg.level_for(Phase::Lexer))
        .with_target("kaubo::parser", cfg.level_for(Phase::Parser))
        .with_target("kaubo::compiler", cfg.level_for(Phase::Compiler))
        .with_target("kaubo::vm", cfg.level_for(Phase::Vm))
        .with_target("kaubo::cli", cfg.global);

    // 如果指定了文件，创建文件输出层（同时输出到控制台和文件）
    if let Some(path) = file {
        let file_handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Failed to open log file");
        
        // 控制台层
        let stdout_layer = fmt::layer()
            .with_writer(io::stdout)
            .with_filter(targets.clone());
        
        // 文件层
        let file_layer = fmt::layer()
            .with_writer(move || file_handle.try_clone().expect("Failed to clone file handle"))
            .with_filter(targets);
        
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .init();
    } else {
        // 仅控制台
        let stdout_layer = create_format_layer(format, io::stdout).with_filter(targets);
        tracing_subscriber::registry()
            .with(stdout_layer)
            .init();
    }
}

/// 根据格式创建 formatter layer
fn create_format_layer<W, F>(format: LogFormat, make_writer: F) -> impl Layer<tracing_subscriber::Registry>
where
    W: io::Write + Send + Sync + 'static,
    F: Fn() -> W + Send + Sync + 'static,
{
    match format {
        LogFormat::Pretty => fmt::layer()
            .pretty()
            .with_target(true)
            .with_timer(fmt::time::time())
            .with_writer(make_writer)
            .boxed(),
        LogFormat::Compact => fmt::layer()
            .compact()
            .with_target(false)
            .without_time()
            .with_writer(make_writer)
            .boxed(),
        LogFormat::Json => fmt::layer()
            .json()
            .with_target(true)
            .with_timer(fmt::time::time())
            .with_writer(make_writer)
            .boxed(),
    }
}

/// 为当前测试初始化简单日志（仅打印到控制台）
#[cfg(test)]
pub fn init_test_logger() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init();
}

/// 检查指定阶段的日志是否启用（DEBUG 级别）
#[inline]
pub fn is_enabled(phase: Phase) -> bool {
    match phase {
        Phase::Lexer => tracing::enabled!(target: "kaubo::lexer", tracing::Level::DEBUG),
        Phase::Parser => tracing::enabled!(target: "kaubo::parser", tracing::Level::DEBUG),
        Phase::Compiler => tracing::enabled!(target: "kaubo::compiler", tracing::Level::DEBUG),
        Phase::Vm => tracing::enabled!(target: "kaubo::vm", tracing::Level::DEBUG),
    }
}

/// 创建指定阶段的 span
#[macro_export]
macro_rules! phase_span {
    ($phase:expr, $name:expr) => {
        match $phase {
            $crate::core::config::Phase::Lexer => tracing::span!(target: "kaubo::lexer", tracing::Level::DEBUG, $name),
            $crate::core::config::Phase::Parser => tracing::span!(target: "kaubo::parser", tracing::Level::DEBUG, $name),
            $crate::core::config::Phase::Compiler => tracing::span!(target: "kaubo::compiler", tracing::Level::DEBUG, $name),
            $crate::core::config::Phase::Vm => tracing::span!(target: "kaubo::vm", tracing::Level::DEBUG, $name),
        }
    };
    ($phase:expr, $name:expr, $($field:tt)*) => {
        match $phase {
            $crate::core::config::Phase::Lexer => tracing::span!(target: "kaubo::lexer", tracing::Level::DEBUG, $name, $($field)*),
            $crate::core::config::Phase::Parser => tracing::span!(target: "kaubo::parser", tracing::Level::DEBUG, $name, $($field)*),
            $crate::core::config::Phase::Compiler => tracing::span!(target: "kaubo::compiler", tracing::Level::DEBUG, $name, $($field)*),
            $crate::core::config::Phase::Vm => tracing::span!(target: "kaubo::vm", tracing::Level::DEBUG, $name, $($field)*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_format_default() {
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }
}
