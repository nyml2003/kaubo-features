//! 日志宏实现（no_std 兼容）

/// 记录 Trace 级别日志
#[macro_export]
macro_rules! trace {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::Level::Trace, $($arg)*)
    };
}

/// 记录 Debug 级别日志
#[macro_export]
macro_rules! debug {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::Level::Debug, $($arg)*)
    };
}

/// 记录 Info 级别日志
#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::Level::Info, $($arg)*)
    };
}

/// 记录 Warn 级别日志
#[macro_export]
macro_rules! warn {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::Level::Warn, $($arg)*)
    };
}

/// 记录 Error 级别日志
#[macro_export]
macro_rules! error {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::Level::Error, $($arg)*)
    };
}

/// 内部使用的通用日志宏
#[macro_export]
macro_rules! log {
    ($logger:expr, $level:expr, $($arg:tt)*) => {{
        // 惰性求值：先检查级别，只有启用时才格式化消息
        if $logger.is_enabled($level) {
            let message = alloc::format!($($arg)*);
            $logger.log($level, module_path!(), message);
        }
    }};
}

#[cfg(test)]
mod tests {
    use crate::{Level, LogRingBuffer, Logger};

    #[test]
    fn test_trace_macro() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Trace).with_sink(ring.clone());

        trace!(logger, "test trace");
        trace!(logger, "formatted {}", "value");

        let records = ring.dump_records();
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.level == Level::Trace));
    }

    #[test]
    fn test_debug_macro() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        debug!(logger, "test debug");
        debug!(logger, "value = {}", 42);

        let records = ring.dump_records();
        assert_eq!(records.len(), 2);
        assert!(records[1].message.contains("42"));
    }

    #[test]
    fn test_level_filtering_in_macros() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Warn).with_sink(ring.clone());

        // 这些应该被过滤掉
        trace!(logger, "trace msg");
        debug!(logger, "debug msg");
        info!(logger, "info msg");

        // 这些应该被记录
        warn!(logger, "warn msg");
        error!(logger, "error msg");

        let records = ring.dump_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].level, Level::Warn);
        assert_eq!(records[1].level, Level::Error);
    }

    #[test]
    fn test_formatting() {
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(Level::Debug).with_sink(ring.clone());

        let name = "test";
        let count = 42;
        debug!(logger, "processing {}: count = {}", name, count);

        let records = ring.dump_records();
        assert!(records[0].message.contains("processing test: count = 42"));
    }
}
