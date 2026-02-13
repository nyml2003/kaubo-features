//! 日志器实现（no_std + alloc 兼容）

use crate::record::{Level, Record};
use crate::span::{Span, SpanId};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

// 使用 spin::Mutex 替代 std::sync::Mutex
use crate::ring_buffer::spin::Mutex;

/// 日志输出目标trait
pub trait LogSink: Send + Sync {
    /// 写入日志记录
    fn write(&self, record: &Record);
}

/// 日志器配置和状态
pub struct Logger {
    /// 当前日志级别（原子存储）
    level: AtomicU8,
    /// 输出目标列表
    sinks: Mutex<Vec<Box<dyn LogSink>>>,
    /// Span栈（用于跟踪嵌套调用）
    span_stack: Mutex<Vec<Span>>,
    /// 下一个Span ID
    next_span_id: AtomicU64,
}

impl Logger {
    /// 创建新的日志器
    pub fn new(level: Level) -> Arc<Self> {
        Arc::new(Logger {
            level: AtomicU8::new(level as u8),
            sinks: Mutex::new(Vec::new()),
            span_stack: Mutex::new(Vec::new()),
            next_span_id: AtomicU64::new(1),
        })
    }

    /// 添加输出目标
    pub fn with_sink<S: LogSink + 'static>(self: Arc<Self>, sink: S) -> Arc<Self> {
        {
            let mut sinks = self.sinks.lock();
            sinks.push(Box::new(sink));
        }
        self
    }

    /// 动态设置日志级别
    pub fn set_level(&self, level: Level) {
        self.level.store(level as u8, Ordering::Relaxed);
    }

    /// 获取当前日志级别
    pub fn level(&self) -> Level {
        Level::from_u8(self.level.load(Ordering::Relaxed)).unwrap_or(Level::Info)
    }

    /// 检查指定级别是否启用
    pub fn is_enabled(&self, level: Level) -> bool {
        level >= self.level()
    }

    /// 记录日志（内部方法）
    #[inline(never)]
    pub fn log(
        &self,
        level: Level,
        target: &'static str,
        message: impl Into<alloc::string::String>,
    ) {
        if !self.is_enabled(level) {
            return;
        }

        let mut record = Record::new(level, target, message);

        // 附加当前span ID（如果有）
        let stack = self.span_stack.lock();
        if let Some(span) = stack.last() {
            record = record.with_span(span.id.0);
        }

        // 写入所有sink
        let sinks = self.sinks.lock();
        for sink in sinks.iter() {
            sink.write(&record);
        }
    }

    /// 进入一个新的span，返回守卫对象
    pub fn enter_span(self: &Arc<Self>, name: &'static str) -> SpanGuard {
        let id = SpanId(self.next_span_id.fetch_add(1, Ordering::Relaxed));
        let span = Span::new(id, name);

        let mut stack = self.span_stack.lock();
        stack.push(span);

        SpanGuard {
            logger: Arc::clone(self),
        }
    }

    /// 获取当前span栈深度
    pub fn span_depth(&self) -> usize {
        self.span_stack.lock().len()
    }

    /// 创建禁用日志的no-op日志器（用于测试或禁用场景）
    pub fn noop() -> Arc<Self> {
        Self::new(Level::Error) // Error级别，且没有任何sink
    }

    /// 添加 sink（内部方法，用于 config）
    pub fn add_sink<S: LogSink + 'static>(&self, sink: S) {
        let mut sinks = self.sinks.lock();
        sinks.push(Box::new(sink));
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        // 克隆时创建新的独立实例，复制配置但不共享状态
        Logger {
            level: AtomicU8::new(self.level.load(Ordering::Relaxed)),
            sinks: Mutex::new(Vec::new()),
            span_stack: Mutex::new(Vec::new()),
            next_span_id: AtomicU64::new(1),
        }
    }
}

/// Span守卫，退出时自动弹出span栈
pub struct SpanGuard {
    logger: Arc<Logger>,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        let mut stack = self.logger.span_stack.lock();
        stack.pop();
    }
}

// 为Arc<Logger>实现LogSink，支持链式日志器
impl LogSink for Arc<Logger> {
    fn write(&self, record: &Record) {
        self.log(record.level, record.target, record.message.clone());
    }
}

#[cfg(feature = "std")]
/// 标准输出sink
pub struct StdoutSink;

#[cfg(feature = "std")]
impl LogSink for StdoutSink {
    fn write(&self, record: &Record) {
        println!("{}", record.format());
    }
}

#[cfg(feature = "std")]
/// 标准错误sink
pub struct StderrSink;

#[cfg(feature = "std")]
impl LogSink for StderrSink {
    fn write(&self, record: &Record) {
        eprintln!("{}", record.format());
    }
}

#[cfg(feature = "std")]
/// 文件sink
pub struct FileSink {
    file: std::sync::Mutex<std::fs::File>,
}

#[cfg(feature = "std")]
impl FileSink {
    /// 创建文件sink（追加模式）
    pub fn new(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(FileSink {
            file: std::sync::Mutex::new(file),
        })
    }
}

#[cfg(feature = "std")]
impl LogSink for FileSink {
    #[inline(never)]
    fn write(&self, record: &Record) {
        use std::io::Write;
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{}", record.format());
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use super::*;
    use crate::LogRingBuffer;

    fn log(logger: &Arc<Logger>, level: Level, target: &'static str, args: fmt::Arguments<'_>) {
        logger.log(level, target, alloc::format!("{}", args));
    }

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new(Level::Debug);
        assert_eq!(logger.level(), Level::Debug);
        assert!(logger.is_enabled(Level::Debug));
        assert!(!logger.is_enabled(Level::Trace));
    }

    #[test]
    fn test_level_change() {
        let logger = Logger::new(Level::Info);
        assert!(!logger.is_enabled(Level::Debug));

        logger.set_level(Level::Debug);
        assert!(logger.is_enabled(Level::Debug));
    }

    #[test]
    fn test_span_guard() {
        let logger = Logger::new(Level::Debug);
        assert_eq!(logger.span_depth(), 0);

        {
            let guard = logger.enter_span("test_span");
            assert_eq!(logger.span_depth(), 1);

            {
                let guard2 = logger.enter_span("nested");
                assert_eq!(logger.span_depth(), 2);
                drop(guard2);
            }

            assert_eq!(logger.span_depth(), 1);
            drop(guard);
        }

        assert_eq!(logger.span_depth(), 0);
    }

    #[test]
    fn test_log_with_ring_buffer() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        logger.log(Level::Info, "test", "hello world");

        let records = ring.dump_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "hello world");
    }

    #[test]
    fn test_log_disabled_level() {
        // 测试禁用的日志级别分支
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Warn).with_sink(ring.clone());

        // Debug 级别被禁用，不应该写入
        logger.log(Level::Debug, "test", "should not appear");
        assert_eq!(ring.len(), 0);

        // Warn 级别启用，应该写入
        logger.log(Level::Warn, "test", "should appear");
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn test_log_without_span() {
        // 测试没有 span 的情况（if let Some 的 None 分支）
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        // 不进入任何 span，直接记录
        logger.log(Level::Info, "test", "no span message");

        let records = ring.dump_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "no span message");
        assert_eq!(records[0].span_id, None); // 确认没有 span_id
    }

    #[test]
    fn test_log_with_span_attached() {
        // 测试有 span 的情况（if let Some 的 Some 分支）
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        {
            let guard = logger.enter_span("test_span");
            logger.log(Level::Info, "test", "with span message");
            drop(guard);
        }

        let records = ring.dump_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "with span message");
        assert!(records[0].span_id.is_some()); // 确认有 span_id
    }

    #[test]
    fn test_logger_clone() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        // 克隆 logger，应该是独立实例
        let cloned = (*logger).clone();
        assert_eq!(cloned.level(), Level::Debug);

        // 克隆的 logger 没有 sink，写入不会影响到原 ring
        cloned.log(Level::Info, "test", "from clone");
        assert_eq!(ring.len(), 0); // 原 ring 没有收到
    }

    #[test]
    fn test_log_sink_for_arc_logger() {
        let ring = LogRingBuffer::new(100);
        let logger1 = Logger::new(Level::Debug).with_sink(ring.clone());

        // 创建一个链式 logger
        let logger2 = Logger::new(Level::Debug);
        logger2.add_sink(logger1.clone());

        // 写入 logger2，应该通过 logger1 最终写入 ring
        logger2.log(Level::Info, "chain", "chained log");

        // 注意：链式 logger 会多一层转发
        let records = ring.dump_records();
        assert!(!records.is_empty());
    }

    #[test]
    fn test_log_function() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        log(
            &logger,
            Level::Info,
            "test::func",
            format_args!("formatted {}", 42),
        );

        let records = ring.dump_records();
        assert_eq!(records.len(), 1);
        assert!(records[0].message.contains("formatted 42"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_stdout_sink() {
        let sink = StdoutSink;
        let record = Record::new(Level::Info, "test", "stdout test");
        // 只测试不 panic，不验证输出
        sink.write(&record);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_stderr_sink() {
        let sink = StderrSink;
        let record = Record::new(Level::Warn, "test", "stderr test");
        // 只测试不 panic，不验证输出
        sink.write(&record);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_file_sink() {
        use std::io::Read;

        // 创建临时文件
        let temp_path = "test_log_file.tmp";

        // 创建 sink 并写入
        {
            let sink = FileSink::new(temp_path).unwrap();
            let record = Record::new(Level::Error, "test", "file test message");
            sink.write(&record);
        }

        // 读取文件验证
        let mut content = String::new();
        std::fs::File::open(temp_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert!(content.contains("file test message"));
        assert!(content.contains("ERROR"));

        // 清理
        std::fs::remove_file(temp_path).ok();
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_file_sink_append() {
        let temp_path = "test_log_append.tmp";

        // 写入第一条
        {
            let sink = FileSink::new(temp_path).unwrap();
            let record = Record::new(Level::Info, "test", "first line");
            sink.write(&record);
        }

        // 写入第二条（追加）
        {
            let sink = FileSink::new(temp_path).unwrap();
            let record = Record::new(Level::Info, "test", "second line");
            sink.write(&record);
        }

        // 验证追加
        let content = std::fs::read_to_string(temp_path).unwrap();
        assert!(content.contains("first line"));
        assert!(content.contains("second line"));

        std::fs::remove_file(temp_path).ok();
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_file_sink_write_lock_poisoned() {
        use std::sync::Arc;
        use std::thread;

        let temp_path = "test_log_poison3.tmp";

        // 创建文件 sink 并包装在 Arc 中以便跨线程共享
        let sink = Arc::new(FileSink::new(temp_path).unwrap());
        let sink_clone = Arc::clone(&sink);

        // 在一个线程中 panic，制造 poison 状态
        let handle = thread::spawn(move || {
            // 获取锁
            let _file = sink_clone.file.lock().unwrap();
            // 直接 panic，导致锁被 poison
            panic!("intentional panic to poison mutex");
        });

        // 等待线程结束（会 panic），确保 poison 完成
        assert!(handle.join().is_err());

        // 现在尝试写入，应该触发 Err 分支（锁被 poison）
        // 注意：poison 后 lock() 返回 Err，但数据仍然可以访问
        let record = Record::new(Level::Info, "test", "poisoned write");
        sink.write(&record); // 这行应该执行 if let Err 分支，即不写入

        // 验证：由于 poison 后写入被跳过，文件应该为空或只有之前的内容
        // 注意：因为 panic 的线程可能还没写入任何内容，文件可能是空的

        // 清理
        std::fs::remove_file(temp_path).ok();
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_file_sink_new_error() {
        // 测试 FileSink::new 失败的情况（Err 分支）
        // 使用一个无效的路径（在 Windows 上是非法文件名）
        let result = FileSink::new("<invalid>:path");
        assert!(result.is_err());
    }

    #[test]
    fn test_noop_logger() {
        let logger = Logger::noop();
        // noop 是 Error 级别且无 sink，任何日志都不应该被记录
        logger.log(Level::Error, "test", "should not appear");
        // 通过不 panic 来验证
    }
}
