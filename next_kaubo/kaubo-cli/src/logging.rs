//! CLI 日志系统初始化
//!
//! 基于 `tracing-subscriber` 实现分阶段日志控制。

use std::io;
use tracing_subscriber::{
    filter::Targets, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};
use crate::config::LogConfig;

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

/// 使用指定格式和日志配置初始化日志系统
pub fn init_with_file<P: AsRef<std::path::Path>>(
    log_config: &LogConfig,
    format: LogFormat,
    file: Option<P>,
) {
    // Build filter targets
    let targets = Targets::new()
        .with_default(log_config.global)
        .with_target("kaubo::lexer", log_config.level_for("kaubo::lexer"))
        .with_target("kaubo::parser", log_config.level_for("kaubo::parser"))
        .with_target("kaubo::compiler", log_config.level_for("kaubo::compiler"))
        .with_target("kaubo::vm", log_config.level_for("kaubo::vm"))
        .with_target("kaubo::cli", log_config.global);

    // If file specified, output to both console and file
    if let Some(path) = file {
        let file_handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("Failed to open log file");
        
        let stdout_layer = fmt::layer()
            .with_writer(io::stdout)
            .with_filter(targets.clone());
        
        let file_layer = fmt::layer()
            .with_writer(move || file_handle.try_clone().expect("Failed to clone file handle"))
            .with_filter(targets);
        
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .init();
    } else {
        // Console only
        let stdout_layer = create_format_layer(format, io::stdout).with_filter(targets);
        tracing_subscriber::registry()
            .with(stdout_layer)
            .init();
    }
}

/// Create formatter layer based on format
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
