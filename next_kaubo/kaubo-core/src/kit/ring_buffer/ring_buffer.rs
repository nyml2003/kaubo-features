use std::sync::{Arc, Condvar, Mutex};
use thiserror::Error;

/// 环形缓冲区可能产生的错误类型
#[derive(Debug, Error, PartialEq)]
pub enum RingBufferError {
    /// 尝试向已关闭的缓冲区写入数据
    #[error("Cannot push to closed ring buffer")]
    BufferClosed,

    /// 尝试从已关闭且为空的缓冲区读取数据
    #[error("Cannot pop from empty and closed ring buffer")]
    BufferClosedAndEmpty,

    /// 互斥锁被污染（poisoned）
    #[error("Mutex poisoned: {0}")]
    MutexPoisoned(String),

    /// 尝试访问超出缓冲区当前大小的位置
    #[error("Index {0} out of bounds for buffer size {1}")]
    IndexOutOfBounds(usize, usize),
}

/// 线程安全的环形缓冲区，适用于生产者-消费者模型
pub struct RingBuffer {
    inner: Mutex<Inner>,
    not_full: Condvar,
    not_empty: Condvar,
}

/// 环形缓冲区的内部数据结构，受Mutex保护
struct Inner {
    buffer: Vec<u8>,
    capacity: usize,
    head: usize,  // 读取指针
    tail: usize,  // 写入指针
    size: usize,  // 当前数据量
    closed: bool, // 缓冲区关闭标记
}

impl RingBuffer {
    /// 创建新的环形缓冲区，返回Arc指针以便多线程共享
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(RingBuffer {
            inner: Mutex::new(Inner {
                buffer: vec![0; capacity],
                capacity,
                head: 0,
                tail: 0,
                size: 0,
                closed: false,
            }),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        })
    }

    /// 向缓冲区添加数据（阻塞式）：满时阻塞，关闭后禁止写入
    pub fn push(&self, item: u8) -> Result<(), RingBufferError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;

        // 等待缓冲区不满且未关闭
        while inner.size == inner.capacity && !inner.closed {
            inner = self
                .not_full
                .wait(inner)
                .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        }

        if inner.closed {
            return Err(RingBufferError::BufferClosed);
        }

        // 写入数据并更新尾指针与大小
        let tail = inner.tail;
        inner.buffer[tail] = item;
        inner.tail = (tail + 1) % inner.capacity;
        inner.size += 1;

        // 通知消费者有新数据
        self.not_empty.notify_one();

        Ok(())
    }

    /// 从缓冲区获取数据（阻塞式）：空时阻塞，关闭且空时返回错误
    pub fn pop(&self) -> Result<u8, RingBufferError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;

        // 等待缓冲区非空且未关闭
        while inner.size == 0 && !inner.closed {
            inner = self
                .not_empty
                .wait(inner)
                .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        }

        if inner.size == 0 && inner.closed {
            return Err(RingBufferError::BufferClosedAndEmpty);
        }

        // 读取数据并更新头指针与大小
        let item = inner.buffer[inner.head];
        inner.head = (inner.head + 1) % inner.capacity;
        inner.size -= 1;

        // 通知生产者有空闲空间
        self.not_full.notify_one();

        Ok(item)
    }

    /// 尝试获取数据（非阻塞式）：空时返回None
    #[allow(dead_code)]
    fn try_pop(&self) -> Option<Result<u8, RingBufferError>> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(e) => return Some(Err(RingBufferError::MutexPoisoned(e.to_string()))),
        };

        if inner.size == 0 {
            return None;
        }

        let item = inner.buffer[inner.head];
        inner.head = (inner.head + 1) % inner.capacity;
        inner.size -= 1;

        self.not_full.notify_one();
        Some(Ok(item))
    }

    /// 尝试观察缓冲区头部数据（非阻塞式）
    #[allow(dead_code)]
    fn try_peek(&self) -> Option<Result<u8, RingBufferError>> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(e) => return Some(Err(RingBufferError::MutexPoisoned(e.to_string()))),
        };

        if inner.size == 0 {
            return None;
        }

        Some(Ok(inner.buffer[inner.head]))
    }

    /// 尝试观察缓冲区指定位置数据（非阻塞式）
    pub fn try_peek_k(&self, k: usize) -> Option<Result<u8, RingBufferError>> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(e) => return Some(Err(RingBufferError::MutexPoisoned(e.to_string()))),
        };

        if k >= inner.size {
            return Some(Err(RingBufferError::IndexOutOfBounds(k, inner.size)));
        }

        let index = (inner.head + k) % inner.capacity;
        Some(Ok(inner.buffer[index]))
    }

    /// 关闭缓冲区：不再接受新数据，唤醒所有阻塞的线程
    pub fn close(&self) -> Result<(), RingBufferError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        inner.closed = true;
        self.not_full.notify_all();
        self.not_empty.notify_all();
        Ok(())
    }

    /// 检查缓冲区是否已关闭
    #[allow(dead_code)]
    fn is_closed(&self) -> Result<bool, RingBufferError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        Ok(inner.closed)
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> Result<bool, RingBufferError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        Ok(inner.size == 0)
    }

    /// 检查缓冲区是否已满
    #[allow(dead_code)]
    fn is_full(&self) -> Result<bool, RingBufferError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        Ok(inner.size == inner.capacity)
    }

    /// 获取当前缓冲区中的数据量
    pub fn get_size(&self) -> Result<usize, RingBufferError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        Ok(inner.size)
    }

    /// 检查缓冲区中的数据量是否大于等于指定值
    #[allow(dead_code)]
    fn is_size_at_least(&self, size: usize) -> Result<bool, RingBufferError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        Ok(inner.size >= size)
    }

    /// 获取缓冲区的总容量
    #[allow(dead_code)]
    fn get_capacity(&self) -> Result<usize, RingBufferError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| RingBufferError::MutexPoisoned(e.to_string()))?;
        Ok(inner.capacity)
    }
}

// 安全标记：RingBuffer可以安全地跨线程发送和共享
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_operations() {
        let rb = RingBuffer::new(3);

        // 测试初始状态
        assert_eq!(rb.get_capacity().unwrap(), 3);
        assert_eq!(rb.get_size().unwrap(), 0);
        assert!(rb.is_empty().unwrap());
        assert!(!rb.is_full().unwrap());
        assert!(!rb.is_closed().unwrap());

        // 测试push和pop
        assert!(rb.push(1).is_ok());
        assert_eq!(rb.get_size().unwrap(), 1);
        assert!(!rb.is_empty().unwrap());
        assert!(!rb.is_full().unwrap());

        assert!(rb.push(2).is_ok());
        assert_eq!(rb.get_size().unwrap(), 2);

        assert!(rb.push(3).is_ok());
        assert_eq!(rb.get_size().unwrap(), 3);
        assert!(rb.is_full().unwrap());

        // 测试peek
        assert_eq!(rb.try_peek(), Some(Ok(1)));
        assert_eq!(rb.try_peek_k(0), Some(Ok(1)));
        assert_eq!(rb.try_peek_k(1), Some(Ok(2)));
        assert_eq!(rb.try_peek_k(2), Some(Ok(3)));
        assert_eq!(
            rb.try_peek_k(3),
            Some(Err(RingBufferError::IndexOutOfBounds(3, 3)))
        );

        // 测试pop
        assert_eq!(rb.pop(), Ok(1));
        assert_eq!(rb.get_size().unwrap(), 2);

        assert_eq!(rb.pop(), Ok(2));
        assert_eq!(rb.get_size().unwrap(), 1);

        assert_eq!(rb.pop(), Ok(3));
        assert_eq!(rb.get_size().unwrap(), 0);
        assert!(rb.is_empty().unwrap());
    }

    #[test]
    fn test_try_pop() {
        let rb = RingBuffer::new(2);

        assert!(rb.try_pop().is_none());

        rb.push(1).unwrap();
        assert_eq!(rb.try_pop(), Some(Ok(1)));
        assert!(rb.try_pop().is_none());

        rb.push(2).unwrap();
        rb.push(3).unwrap();
        assert_eq!(rb.try_pop(), Some(Ok(2)));
        assert_eq!(rb.try_pop(), Some(Ok(3)));
        assert!(rb.try_pop().is_none());
    }

    #[test]
    fn test_close_behavior() {
        let rb = RingBuffer::new(2);

        rb.push(1).unwrap();
        rb.close().unwrap();

        assert!(rb.is_closed().unwrap());
        assert_eq!(rb.pop(), Ok(1));
        assert_eq!(rb.pop(), Err(RingBufferError::BufferClosedAndEmpty));
        assert_eq!(rb.push(2), Err(RingBufferError::BufferClosed));
    }

    #[test]
    fn test_producer_consumer_model() {
        let rb = RingBuffer::new(5);
        let rb2 = Arc::clone(&rb);

        // 启动生产者线程
        let producer = thread::spawn(move || {
            for i in 0..10 {
                rb.push(i as u8).unwrap();
                thread::sleep(Duration::from_millis(10));
            }
            rb.close().unwrap();
        });

        // 启动消费者线程
        let consumer = thread::spawn(move || {
            let mut received = Vec::new();
            loop {
                match rb2.pop() {
                    Ok(val) => received.push(val),
                    Err(RingBufferError::BufferClosedAndEmpty) => break,
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }
            received
        });

        producer.join().unwrap();
        let received = consumer.join().unwrap();

        assert_eq!(received, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_buffer_full_blocking() {
        let rb = RingBuffer::new(2);
        let rb2 = Arc::clone(&rb);

        // 填充缓冲区
        rb.push(1).unwrap();
        rb.push(2).unwrap();
        assert!(rb.is_full().unwrap());

        // 尝试在另一个线程中推送，应该被阻塞
        let handle = thread::spawn(move || {
            let start = std::time::Instant::now();
            let result = rb2.push(3);
            (start.elapsed() >= Duration::from_millis(50)) && result.is_ok()
        });

        // 等待一会儿，确保生产者线程已经开始并被阻塞
        thread::sleep(Duration::from_millis(100));

        // 消费一个元素，给生产者腾出空间
        assert_eq!(rb.pop(), Ok(1));

        // 等待生产者线程完成
        let result = handle.join().unwrap();
        assert!(result);

        // 验证缓冲区状态
        assert_eq!(rb.get_size().unwrap(), 2);
        assert_eq!(rb.pop(), Ok(2));
        assert_eq!(rb.pop(), Ok(3));
    }

    #[test]
    fn test_buffer_empty_blocking() {
        let rb = RingBuffer::new(2);
        let rb2 = Arc::clone(&rb);

        // 缓冲区为空
        assert!(rb.is_empty().unwrap());

        // 尝试在另一个线程中获取，应该被阻塞
        let handle = thread::spawn(move || {
            let start = std::time::Instant::now();
            let result = rb2.pop();
            (start.elapsed() >= Duration::from_millis(50)) && result.is_ok()
        });

        // 等待一会儿，确保消费者线程已经开始并被阻塞
        thread::sleep(Duration::from_millis(100));

        // 生产一个元素，给消费者提供数据
        rb.push(1).unwrap();

        // 等待消费者线程完成
        let result = handle.join().unwrap();
        assert!(result);

        // 验证缓冲区状态
        assert!(rb.is_empty().unwrap());
    }
}
