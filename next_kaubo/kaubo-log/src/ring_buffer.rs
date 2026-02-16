//! 日志专用环形缓冲区（no_std + alloc 兼容）

use crate::logger::LogSink;
use crate::record::Record;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

/// 环形缓冲区统计信息
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RingBufferStats {
    /// 当前记录数
    pub record_count: usize,
    /// 因缓冲区满而丢弃的记录数
    pub dropped_count: usize,
    /// 缓冲区容量
    pub capacity: usize,
}

/// 日志环形缓冲区
///
/// 当缓冲区满时，新记录会覆盖最旧的记录（FIFO）
pub struct LogRingBuffer {
    inner: spin::Mutex<VecDeque<Record>>,
    capacity: usize,
    dropped: AtomicUsize,
}

impl LogRingBuffer {
    /// 创建新的环形缓冲区
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(LogRingBuffer {
            inner: spin::Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            dropped: AtomicUsize::new(0),
        })
    }

    /// 写入记录（满了则覆盖旧数据）
    fn push(&self, record: Record) {
        let mut inner = self.inner.lock();
        if inner.len() >= self.capacity {
            inner.pop_front();
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        inner.push_back(record);
    }

    /// 获取当前所有记录（按时间顺序）
    pub fn dump_records(&self) -> Vec<Record> {
        let inner = self.inner.lock();
        inner.iter().cloned().collect()
    }

    /// 将日志转储到字符串
    #[cfg(feature = "alloc")]
    pub fn dump(&self) -> alloc::string::String {
        let records = self.dump_records();
        records
            .iter()
            .map(|r| r.format())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 清空缓冲区
    pub fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.clear();
        self.dropped.store(0, Ordering::Relaxed);
    }

    /// 获取统计信息
    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            record_count: self.inner.lock().len(),
            dropped_count: self.dropped.load(Ordering::Relaxed),
            capacity: self.capacity,
        }
    }

    /// 获取当前记录数
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 获取容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 获取已丢弃的记录数
    pub fn dropped_count(&self) -> usize {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl LogSink for LogRingBuffer {
    fn write(&self, record: &Record) {
        self.push(record.clone());
    }
}

impl LogSink for Arc<LogRingBuffer> {
    fn write(&self, record: &Record) {
        self.push(record.clone());
    }
}

// 使用 spin crate 的 Mutex 实现（no_std 兼容）
impl Clone for LogRingBuffer {
    fn clone(&self) -> Self {
        LogRingBuffer {
            inner: spin::Mutex::new(VecDeque::with_capacity(self.capacity)),
            capacity: self.capacity,
            dropped: AtomicUsize::new(0),
        }
    }
}

// 安全标记
unsafe impl Send for LogRingBuffer {}
unsafe impl Sync for LogRingBuffer {}

// 添加 spin crate 兼容层
pub(crate) mod spin {
    //! 简单的自旋锁实现（no_std 兼容）

    use core::cell::UnsafeCell;
    use core::ops::{Deref, DerefMut};
    use core::sync::atomic::{AtomicBool, Ordering};

    pub struct Mutex<T> {
        locked: AtomicBool,
        data: UnsafeCell<T>,
    }

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}

    pub struct MutexGuard<'a, T> {
        mutex: &'a Mutex<T>,
    }

    impl<T> Mutex<T> {
        pub const fn new(data: T) -> Self {
            Mutex {
                locked: AtomicBool::new(false),
                data: UnsafeCell::new(data),
            }
        }

        pub fn lock(&self) -> MutexGuard<'_, T> {
            // 简单自旋锁
            while self
                .locked
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                core::hint::spin_loop();
            }
            MutexGuard { mutex: self }
        }

        fn unlock(&self) {
            self.locked.store(false, Ordering::Release);
        }
    }

    impl<'a, T> Deref for MutexGuard<'a, T> {
        type Target = T;
        fn deref(&self) -> &T {
            unsafe { &*self.mutex.data.get() }
        }
    }

    impl<'a, T> DerefMut for MutexGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut T {
            unsafe { &mut *self.mutex.data.get() }
        }
    }

    impl<'a, T> Drop for MutexGuard<'a, T> {
        fn drop(&mut self) {
            self.mutex.unlock();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;
    use std::sync::Arc;

    #[test]
    fn test_basic_operations() {
        let buffer = LogRingBuffer::new(3);

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 3);

        buffer.push(Record::new(Level::Info, "test", "msg1"));
        assert_eq!(buffer.len(), 1);

        buffer.push(Record::new(Level::Info, "test", "msg2"));
        buffer.push(Record::new(Level::Info, "test", "msg3"));
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_push_not_full_branch() {
        // 测试缓冲区未满时的分支（if len >= capacity 的 false 分支）
        let buffer = LogRingBuffer::new(10);

        // 只添加2条，缓冲区未满
        buffer.push(Record::new(Level::Info, "test", "msg1"));
        buffer.push(Record::new(Level::Info, "test", "msg2"));

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.dropped_count(), 0); // 没有丢弃

        let records = buffer.dump_records();
        assert_eq!(records[0].message, "msg1");
        assert_eq!(records[1].message, "msg2");
    }

    #[test]
    fn test_overflow_behavior() {
        let buffer = LogRingBuffer::new(3);

        for i in 0..5 {
            buffer.push(Record::new(Level::Info, "test", alloc::format!("msg{i}")));
        }

        assert_eq!(buffer.len(), 3);

        let records = buffer.dump_records();
        assert_eq!(records[0].message, "msg2");
        assert_eq!(records[1].message, "msg3");
        assert_eq!(records[2].message, "msg4");

        assert_eq!(buffer.dropped_count(), 2);
    }

    #[test]
    fn test_log_sink_trait() {
        let buffer = LogRingBuffer::new(10);
        let record = Record::new(Level::Debug, "test::module", "test message");

        buffer.write(&record);

        let records = buffer.dump_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "test message");
    }

    #[test]
    fn test_arc_log_sink() {
        let buffer = LogRingBuffer::new(10);
        let record = Record::new(Level::Info, "test", "via arc");

        let arc_buffer: Arc<LogRingBuffer> = Arc::clone(&buffer);
        arc_buffer.write(&record);

        assert_eq!(buffer.len(), 1);
        assert_eq!(arc_buffer.len(), 1);
    }

    #[test]
    fn test_clear() {
        let buffer = LogRingBuffer::new(10);

        buffer.push(Record::new(Level::Info, "test", "msg1"));
        buffer.push(Record::new(Level::Info, "test", "msg2"));
        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.dropped_count(), 0);
    }

    #[test]
    fn test_dump_format() {
        let buffer = LogRingBuffer::new(10);

        buffer.push(Record::new(Level::Info, "test", "first line"));
        buffer.push(Record::new(Level::Warn, "test", "second line"));

        let dump = buffer.dump();
        assert!(dump.contains("first line"));
        assert!(dump.contains("second line"));
        assert!(dump.contains("INFO"));
        assert!(dump.contains("WARN"));
    }

    #[test]
    fn test_stats() {
        let buffer = LogRingBuffer::new(5);

        // 初始状态
        let stats = buffer.stats();
        assert_eq!(stats.record_count, 0);
        assert_eq!(stats.dropped_count, 0);
        assert_eq!(stats.capacity, 5);

        // 添加记录
        buffer.push(Record::new(Level::Info, "test", "msg1"));
        buffer.push(Record::new(Level::Info, "test", "msg2"));

        let stats = buffer.stats();
        assert_eq!(stats.record_count, 2);
        assert_eq!(stats.dropped_count, 0);

        // 溢出触发丢弃
        for i in 0..10 {
            buffer.push(Record::new(Level::Info, "test", format!("msg{i}")));
        }

        let stats = buffer.stats();
        assert_eq!(stats.record_count, 5); // 容量上限
        assert!(stats.dropped_count > 0); // 有丢弃记录
    }

    #[test]
    fn test_clone() {
        let buffer = LogRingBuffer::new(10);
        buffer.push(Record::new(Level::Info, "test", "original"));

        // 克隆缓冲区（需要解引用 Arc）
        let cloned: LogRingBuffer = (*buffer).clone();

        // 克隆的是独立副本，不影响原缓冲区
        assert_eq!(buffer.len(), 1);
        assert_eq!(cloned.len(), 0); // 新缓冲区为空

        // 验证克隆缓冲区的容量正确
        assert_eq!(cloned.capacity(), 10);
    }

    #[test]
    fn test_log_sink_direct_write() {
        // 直接测试 LogSink trait 的 write 方法
        let buffer = LogRingBuffer::new(10);
        let record = Record::new(Level::Debug, "test::sink", "direct write");

        // 通过 LogSink trait 写入
        LogSink::write(&buffer, &record);

        let records = buffer.dump_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "direct write");
    }

    #[test]
    fn test_concurrent_access() {
        // 多线程并发访问测试 - 触发 spin 锁的竞争分支
        use std::sync::Barrier;

        let buffer = Arc::new(LogRingBuffer::new(1000));
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for i in 0..10 {
            let buf = Arc::clone(&buffer);
            let b = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                // 同步所有线程，增加竞争概率
                b.wait();
                for j in 0..10 {
                    buf.push(Record::new(
                        Level::Info,
                        "test",
                        format!("thread {i} msg {j}"),
                    ));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // 验证所有记录都写入
        assert_eq!(buffer.len(), 100);
    }
}
