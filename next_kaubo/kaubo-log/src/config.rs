//! 日志配置（std 平台专用）
//!
//! 提供便捷的日志初始化配置。

#![cfg(feature = "alloc")]
use crate::logger::LogSink;
use crate::record::Record;
use crate::{Level, LogRingBuffer, Logger};
use alloc::sync::Arc;
#[cfg(feature = "file")]
use std::io::Write;

/// 文件sink
#[cfg(feature = "file")]
struct FileSink {
    file: std::sync::Mutex<std::fs::File>,
}

#[cfg(feature = "file")]
impl FileSink {
    /// 创建文件sink（追加模式）
    fn new(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(FileSink {
            file: std::sync::Mutex::new(file),
        })
    }
}

#[cfg(feature = "file")]
impl LogSink for FileSink {
    fn write(&self, record: &Record) {
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{}", record.format());
        }
    }
}

/// 日志输出目标配置
#[derive(Clone, Debug, PartialEq)]
pub enum OutputConfig {
    /// 输出到标准输出
    #[cfg(feature = "stdout")]
    Stdout,
    /// 输出到标准错误
    #[cfg(feature = "stderr")]
    Stderr,
    /// 输出到文件（路径）
    #[cfg(feature = "file")]
    File(alloc::string::String),
    /// 输出到环形缓冲区（容量）
    RingBuffer(usize),
}

/// 日志配置
///
/// 用于一键初始化日志系统
///
/// # 示例
///
/// ```
/// use kaubo_log::{LogConfig, Level};
///
/// let config = LogConfig::new(Level::Debug)
///     .with_ring_buffer(10000);
///
/// let (logger, ring) = config.init();
/// ```
#[derive(Clone, Debug)]
pub struct LogConfig {
    /// 日志级别
    pub level: Level,
    /// 输出目标列表
    pub outputs: alloc::vec::Vec<OutputConfig>,
    /// 是否启用 span 跟踪
    pub enable_span: bool,
}

impl LogConfig {
    /// 创建默认配置（Info级别，无输出）
    pub fn new(level: Level) -> Self {
        LogConfig {
            level,
            outputs: alloc::vec::Vec::new(),
            enable_span: true,
        }
    }

    /// 开发环境推荐配置
    ///
    /// - Debug 级别
    /// - 输出到 stdout
    /// - 环形缓冲区 10000 条（用于崩溃转储）
    #[cfg(all(feature = "stdout", feature = "alloc"))]
    pub fn dev() -> Self {
        LogConfig {
            level: Level::Debug,
            outputs: alloc::vec![OutputConfig::Stdout, OutputConfig::RingBuffer(10000),],
            enable_span: true,
        }
    }

    /// 生产环境推荐配置
    ///
    /// - Warn 级别
    /// - 输出到 stderr
    /// - 环形缓冲区 1000 条
    #[cfg(all(feature = "stderr", feature = "alloc"))]
    pub fn production() -> Self {
        LogConfig {
            level: Level::Warn,
            outputs: alloc::vec![OutputConfig::Stderr, OutputConfig::RingBuffer(1000),],
            enable_span: false,
        }
    }

    /// 测试环境配置（静默）
    ///
    /// - Error 级别
    /// - 无输出（noop）
    pub fn test() -> Self {
        LogConfig {
            level: Level::Error,
            outputs: alloc::vec::Vec::new(),
            enable_span: false,
        }
    }

    /// 添加 stdout 输出
    #[cfg(feature = "stdout")]
    pub fn with_stdout(mut self) -> Self {
        if !self.outputs.contains(&OutputConfig::Stdout) {
            self.outputs.push(OutputConfig::Stdout);
        }
        self
    }

    /// 添加 stderr 输出
    #[cfg(feature = "stderr")]
    pub fn with_stderr(mut self) -> Self {
        if !self.outputs.contains(&OutputConfig::Stderr) {
            self.outputs.push(OutputConfig::Stderr);
        }
        self
    }

    /// 添加文件输出
    #[cfg(feature = "file")]
    pub fn with_file(mut self, path: impl Into<alloc::string::String>) -> Self {
        self.outputs.push(OutputConfig::File(path.into()));
        self
    }

    /// 添加环形缓冲区输出
    pub fn with_ring_buffer(mut self, capacity: usize) -> Self {
        self.outputs.push(OutputConfig::RingBuffer(capacity));
        self
    }

    /// 禁用 span 跟踪
    pub fn without_span(mut self) -> Self {
        self.enable_span = false;
        self
    }

    /// 初始化日志系统
    ///
    /// 返回 (logger, Option<ring_buffer>)
    /// 如果配置了环形缓冲区，会返回它（用于崩溃转储）
    pub fn init(self) -> (Arc<Logger>, Option<Arc<LogRingBuffer>>) {
        let logger = Logger::new(self.level);
        let mut ring_buffer: Option<Arc<LogRingBuffer>> = None;

        for output in self.outputs {
            match output {
                #[cfg(feature = "stdout")]
                OutputConfig::Stdout => {
                    logger.add_sink(StdoutSinkAdapter);
                }
                #[cfg(feature = "stderr")]
                OutputConfig::Stderr => {
                    logger.add_sink(StderrSinkAdapter);
                }
                #[cfg(feature = "file")]
                OutputConfig::File(path) => {
                    if let Ok(sink) = FileSink::new(&path) {
                        logger.add_sink(sink);
                    }
                }
                OutputConfig::RingBuffer(capacity) => {
                    let ring = LogRingBuffer::new(capacity);
                    ring_buffer = Some(Arc::clone(&ring));
                    logger.add_sink(ring);
                }
            }
        }

        (logger, ring_buffer)
    }
}

// 内部适配器类型，用于简化 API

#[cfg(feature = "stdout")]
struct StdoutSinkAdapter;

#[cfg(feature = "stdout")]
impl LogSink for StdoutSinkAdapter {
    fn write(&self, record: &Record) {
        println!("{}", record.format());
    }
}

#[cfg(feature = "stderr")]
struct StderrSinkAdapter;

#[cfg(feature = "stderr")]
impl LogSink for StderrSinkAdapter {
    fn write(&self, record: &Record) {
        eprintln!("{}", record.format());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = LogConfig::new(Level::Debug);
        assert_eq!(config.level, Level::Debug);
        assert!(config.outputs.is_empty());
    }

    #[cfg(all(feature = "stdout", feature = "alloc"))]
    #[test]
    fn test_config_dev() {
        let config = LogConfig::dev();
        assert_eq!(config.level, Level::Debug);
        assert!(config.outputs.contains(&OutputConfig::Stdout));
        assert!(config
            .outputs
            .iter()
            .any(|o| matches!(o, OutputConfig::RingBuffer(10000))));
    }

    #[cfg(all(feature = "stderr", feature = "alloc"))]
    #[test]
    fn test_config_production() {
        let config = LogConfig::production();
        assert_eq!(config.level, Level::Warn);
        assert!(config.outputs.contains(&OutputConfig::Stderr));
    }

    #[test]
    fn test_config_test() {
        let config = LogConfig::test();
        assert_eq!(config.level, Level::Error);
        assert!(config.outputs.is_empty());
    }

    #[cfg(feature = "stdout")]
    #[test]
    fn test_config_builder() {
        let config = LogConfig::new(Level::Info)
            .with_stdout()
            .with_ring_buffer(5000);

        assert!(config.outputs.contains(&OutputConfig::Stdout));
        assert!(config
            .outputs
            .iter()
            .any(|o| matches!(o, OutputConfig::RingBuffer(5000))));
    }

    #[test]
    fn test_config_init() {
        let config = LogConfig::new(Level::Debug).with_ring_buffer(100);

        let (logger, ring) = config.init();

        assert_eq!(logger.level(), Level::Debug);
        assert!(ring.is_some());

        // 测试日志能写入
        crate::debug!(logger, "test message");
        let records = ring.unwrap().dump_records();
        assert_eq!(records.len(), 1);
    }

    #[cfg(feature = "stdout")]
    #[test]
    fn test_with_stdout() {
        let config = LogConfig::new(Level::Info).with_stdout();
        assert!(config.outputs.contains(&OutputConfig::Stdout));

        // 重复添加应该只保留一个
        let config2 = config.clone().with_stdout();
        let stdout_count = config2
            .outputs
            .iter()
            .filter(|o| matches!(o, OutputConfig::Stdout))
            .count();
        assert_eq!(stdout_count, 1);
    }

    #[cfg(feature = "stderr")]
    #[test]
    fn test_with_stderr() {
        let config = LogConfig::new(Level::Warn).with_stderr();
        assert!(config.outputs.contains(&OutputConfig::Stderr));

        // 重复添加应该只保留一个
        let config2 = config.clone().with_stderr();
        let stderr_count = config2
            .outputs
            .iter()
            .filter(|o| matches!(o, OutputConfig::Stderr))
            .count();
        assert_eq!(stderr_count, 1);
    }

    #[cfg(feature = "file")]
    #[test]
    fn test_with_file() {
        let config = LogConfig::new(Level::Debug).with_file("/tmp/test.log");
        assert!(config
            .outputs
            .iter()
            .any(|o| matches!(o, OutputConfig::File(_))));
    }

    #[test]
    fn test_with_ring_buffer() {
        let config = LogConfig::new(Level::Debug)
            .with_ring_buffer(1000)
            .with_ring_buffer(2000);

        let ring_count = config
            .outputs
            .iter()
            .filter(|o| matches!(o, OutputConfig::RingBuffer(_)))
            .count();
        assert_eq!(ring_count, 2); // 允许多个 ring buffer
    }

    #[test]
    fn test_without_span() {
        let config = LogConfig::new(Level::Debug).without_span();
        assert!(!config.enable_span);
    }

    #[cfg(all(feature = "stdout", feature = "stderr", feature = "file"))]
    #[test]
    fn test_config_init_with_all_outputs() {
        let temp_path = "test_config_init.log";
        let config = LogConfig::new(Level::Debug)
            .with_stdout()
            .with_stderr()
            .with_file(temp_path)
            .with_ring_buffer(100);

        let (logger, ring) = config.init();

        assert_eq!(logger.level(), Level::Debug);
        assert!(ring.is_some());

        // 测试日志能写入
        crate::info!(logger, "test all outputs");

        // 验证 ring buffer 收到
        let records = ring.unwrap().dump_records();
        assert_eq!(records.len(), 1);
        assert!(records[0].message.contains("test all outputs"));

        // 清理
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_config_init_no_ring() {
        let config = LogConfig::new(Level::Debug);
        let (logger, ring) = config.init();
        assert!(ring.is_none());
        // 无 sink 的 logger 也能正常工作
        crate::debug!(logger, "no sink");
    }

    #[test]
    fn test_output_config_clone() {
        // 使用 RingBuffer 配置测试 Clone（不依赖特定 feature）
        let config1 = OutputConfig::RingBuffer(100);
        let config2 = config1.clone();
        assert_eq!(config1, config2);

        let config3 = OutputConfig::RingBuffer(200);
        let config4 = config3.clone();
        assert_eq!(config3, config4);

        // 验证克隆后的值独立
        assert_ne!(config1, config3);
    }

    #[cfg(feature = "stdout")]
    #[test]
    fn test_output_config_clone_stdout() {
        // 测试 Stdout 配置的 Clone
        let config1 = OutputConfig::Stdout;
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[cfg(feature = "file")]
    #[test]
    fn test_config_init_file_error() {
        // 使用无效路径应该静默失败（不 panic）- 覆盖 if let Ok(sink) 的 Err 分支
        // 尝试多种无效路径确保触发 Err 分支

        // 创建一个无法创建文件的目录路径（使用已存在的文件作为目录）
        let temp_file = "test_config_temp_file.tmp";

        // 先创建一个临时文件
        std::fs::write(temp_file, "temp").unwrap();

        // 尝试在该文件"内部"创建文件（这会失败，因为 temp_file 是文件不是目录）
        let invalid_path = format!("{}/inner.log", temp_file);
        let config = LogConfig::new(Level::Debug).with_file(&invalid_path);
        let (logger, _ring) = config.init();
        crate::debug!(logger, "test with file as directory");

        // 清理
        std::fs::remove_file(temp_file).ok();

        // 同时测试其他明显无效的路径
        let other_invalid_paths = [
            "<>:\"/\\|?*",                         // Windows 非法字符
            "/dev/null/nonexistent/path/file.log", // Unix 不存在的目录
        ];

        for path in &other_invalid_paths {
            let config = LogConfig::new(Level::Debug).with_file(*path);
            let (logger, _ring) = config.init();
            crate::debug!(logger, "test with invalid path");
        }
    }

    #[cfg(feature = "stdout")]
    #[test]
    fn test_with_stdout_first_time() {
        // 测试首次添加 stdout（覆盖 if !contains 的 true 分支）
        let config = LogConfig::new(Level::Debug);
        assert!(!config.outputs.contains(&OutputConfig::Stdout));

        let config = config.with_stdout();
        assert!(config.outputs.contains(&OutputConfig::Stdout));
        assert_eq!(config.outputs.len(), 1);
    }

    #[cfg(feature = "stderr")]
    #[test]
    fn test_with_stderr_first_time() {
        // 测试首次添加 stderr（覆盖 if !contains 的 true 分支）
        let config = LogConfig::new(Level::Debug);
        assert!(!config.outputs.contains(&OutputConfig::Stderr));

        let config = config.with_stderr();
        assert!(config.outputs.contains(&OutputConfig::Stderr));
        assert_eq!(config.outputs.len(), 1);
    }
}
