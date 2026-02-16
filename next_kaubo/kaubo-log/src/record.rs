//! 日志记录定义（no_std 兼容）

use core::fmt;

/// 日志级别
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    /// 最详细的跟踪信息
    Trace = 0,
    /// 调试信息
    Debug = 1,
    /// 一般信息
    Info = 2,
    /// 警告
    Warn = 3,
    /// 错误
    Error = 4,
}

impl Level {
    /// 将级别转换为字符串
    pub const fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// 从u8解析级别
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Level::Trace),
            1 => Some(Level::Debug),
            2 => Some(Level::Info),
            3 => Some(Level::Warn),
            4 => Some(Level::Error),
            _ => None,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 单条日志记录（no_std 兼容，使用 Vec 需要 alloc）
#[cfg(feature = "alloc")]
pub struct Record {
    /// Unix时间戳（毫秒）- 从某个固定起点开始
    pub timestamp_ms: u64,
    /// 日志级别
    pub level: Level,
    /// 模块路径（编译期确定）
    pub target: &'static str,
    /// 格式化后的消息
    pub message: alloc::string::String,
    /// 可选的调用链ID
    pub span_id: Option<u64>,
}

#[cfg(not(feature = "alloc"))]
pub struct Record {
    /// Unix时间戳（毫秒）
    pub timestamp_ms: u64,
    /// 日志级别
    pub level: Level,
    /// 模块路径（编译期确定）
    pub target: &'static str,
    /// 消息（固定缓冲区，no_alloc 模式）
    pub message_buffer: [u8; 256],
    /// 消息长度
    pub message_len: usize,
    /// 可选的调用链ID
    pub span_id: Option<u64>,
}

#[cfg(feature = "alloc")]
impl Record {
    /// 创建新记录
    pub fn new(
        level: Level,
        target: &'static str,
        message: impl Into<alloc::string::String>,
    ) -> Self {
        Self {
            timestamp_ms: current_timestamp_ms(),
            level,
            target,
            message: message.into(),
            span_id: None,
        }
    }

    /// 创建带span ID的记录
    pub fn with_span(mut self, span_id: u64) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// 格式化记录为字符串
    pub fn format(&self) -> alloc::string::String {
        let span_info = match self.span_id {
            Some(id) => alloc::format!(" [span={id}]"),
            None => alloc::string::String::new(),
        };

        alloc::format!(
            "[{}] {} {}{}: {}",
            format_timestamp(self.timestamp_ms),
            self.level,
            self.target,
            span_info,
            self.message
        )
    }
}

#[cfg(feature = "alloc")]
impl Clone for Record {
    fn clone(&self) -> Self {
        Self {
            timestamp_ms: self.timestamp_ms,
            level: self.level,
            target: self.target,
            message: self.message.clone(),
            span_id: self.span_id,
        }
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for Record {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Record")
            .field("timestamp_ms", &self.timestamp_ms)
            .field("level", &self.level)
            .field("target", &self.target)
            .field("message", &self.message)
            .field("span_id", &self.span_id)
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp_ms == other.timestamp_ms
            && self.level == other.level
            && self.target == other.target
            && self.message == other.message
            && self.span_id == other.span_id
    }
}

/// 获取当前时间戳（毫秒）
///
/// 在 std 平台使用系统时间，在 no_std 平台使用单调计数器
#[cfg(feature = "std")]
fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(not(feature = "std"))]
static mut MONOTONIC_COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

#[cfg(not(feature = "std"))]
fn current_timestamp_ms() -> u64 {
    // no_std 环境下使用单调递增计数器作为时间戳
    // 实际项目应该接入硬件时钟
    unsafe { MONOTONIC_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed) }
}

/// 格式化时间戳为可读字符串
fn format_timestamp(timestamp_ms: u64) -> alloc::string::String {
    let secs = timestamp_ms / 1000;
    let millis = timestamp_ms % 1000;

    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;

    alloc::format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn test_level_from_u8() {
        assert_eq!(Level::from_u8(0), Some(Level::Trace));
        assert_eq!(Level::from_u8(4), Some(Level::Error));
        assert_eq!(Level::from_u8(5), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_record_creation() {
        let record = Record::new(Level::Info, "test::module", "test message");
        assert_eq!(record.level, Level::Info);
        assert_eq!(record.target, "test::module");
        assert_eq!(record.message, "test message");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_record_with_span() {
        let record = Record::new(Level::Debug, "test", "msg").with_span(42);
        assert_eq!(record.span_id, Some(42));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_record_debug() {
        let record = Record::new(Level::Info, "test::module", "debug test");
        let debug_str = format!("{record:?}");

        // 验证 Debug 输出包含关键字段
        assert!(debug_str.contains("Record"));
        assert!(debug_str.contains("timestamp_ms"));
        assert!(debug_str.contains("level"));
        assert!(debug_str.contains("Info"));
        assert!(debug_str.contains("target"));
        assert!(debug_str.contains("test::module"));
        assert!(debug_str.contains("message"));
        assert!(debug_str.contains("debug test"));
        assert!(debug_str.contains("span_id"));
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_record_partial_eq() {
        let record1 = Record {
            timestamp_ms: 1000,
            level: Level::Info,
            target: "test",
            message: alloc::string::String::from("test message"),
            span_id: Some(1),
        };

        // 完全相同的记录
        let record2 = Record {
            timestamp_ms: 1000,
            level: Level::Info,
            target: "test",
            message: alloc::string::String::from("test message"),
            span_id: Some(1),
        };

        assert_eq!(record1, record2);

        // 不同的 timestamp
        let record3 = Record {
            timestamp_ms: 2000,
            level: Level::Info,
            target: "test",
            message: alloc::string::String::from("test message"),
            span_id: Some(1),
        };
        assert_ne!(record1, record3);

        // 不同的 level
        let record4 = Record {
            timestamp_ms: 1000,
            level: Level::Debug,
            target: "test",
            message: alloc::string::String::from("test message"),
            span_id: Some(1),
        };
        assert_ne!(record1, record4);

        // 不同的 target
        let record5 = Record {
            timestamp_ms: 1000,
            level: Level::Info,
            target: "other",
            message: alloc::string::String::from("test message"),
            span_id: Some(1),
        };
        assert_ne!(record1, record5);

        // 不同的 message
        let record6 = Record {
            timestamp_ms: 1000,
            level: Level::Info,
            target: "test",
            message: alloc::string::String::from("different message"),
            span_id: Some(1),
        };
        assert_ne!(record1, record6);

        // 不同的 span_id
        let record7 = Record {
            timestamp_ms: 1000,
            level: Level::Info,
            target: "test",
            message: alloc::string::String::from("test message"),
            span_id: None,
        };
        assert_ne!(record1, record7);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_record_format() {
        let record = Record {
            timestamp_ms: 3600000 + 120000 + 3000 + 456,
            level: Level::Info,
            target: "kaubo::lexer",
            message: alloc::string::String::from("token found"),
            span_id: Some(7),
        };

        let formatted = record.format();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("kaubo::lexer"));
        assert!(formatted.contains("token found"));
        assert!(formatted.contains("span=7"));
    }
}
