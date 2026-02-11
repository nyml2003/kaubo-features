//! 字符流抽象
//!
//! 将字节流（RingBuffer）转换为Unicode字符流
//! 支持UTF-8解码、位置追踪、预读和回溯

use std::sync::Arc;

use super::position::SourcePosition;
use crate::kit::ring_buffer::ring_buffer::{RingBuffer, RingBufferError};

/// 字符流错误
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum StreamError {
    #[error("UTF-8 decode error at byte offset {0}")]
    Utf8Error(usize),
    
    #[error("Buffer error: {0}")]
    Buffer(#[from] RingBufferError),
    
    #[error("Stream closed")]
    Closed,
}

/// 流式读取结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamResult<T> {
    /// 成功读取
    Ok(T),
    /// 需要更多输入（流式场景）
    Incomplete,
    /// 流已结束
    Eof,
}

/// 字符流
///
/// 包装RingBuffer，提供字符级操作
pub struct CharStream {
    /// 底层字节缓冲区
    buffer: Arc<RingBuffer>,
    /// 当前位置
    position: SourcePosition,
    /// 缓冲区是否已关闭（EOF）
    is_closed: bool,
}

impl CharStream {
    /// 创建新的字符流
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: RingBuffer::new(capacity),
            position: SourcePosition::start(),
            is_closed: false,
        }
    }

    /// 从现有RingBuffer创建
    pub fn from_buffer(buffer: Arc<RingBuffer>) -> Self {
        Self {
            buffer,
            position: SourcePosition::start(),
            is_closed: false,
        }
    }

    /// 获取当前位置
    pub fn position(&self) -> SourcePosition {
        self.position
    }

    /// 是否已关闭（EOF）
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// 向流中写入数据（生产者接口）
    pub fn feed(&mut self, data: &[u8]) -> Result<(), StreamError> {
        if self.is_closed {
            return Err(StreamError::Closed);
        }
        for &byte in data {
            self.buffer.push(byte)?;
        }
        Ok(())
    }

    /// 关闭流（标记EOF）
    pub fn close(&mut self) -> Result<(), StreamError> {
        self.is_closed = true;
        self.buffer.close()?;
        Ok(())
    }

    /// 尝试预读第n个字符（不消费）
    ///
    /// # Returns
    /// - `StreamResult::Ok(c)` - 成功读取字符
    /// - `StreamResult::Incomplete` - 缓冲区不足（需要更多输入）
    /// - `StreamResult::Eof` - 已到达EOF
    pub fn try_peek(&self, offset: usize) -> StreamResult<char> {
        // 获取引导字节
        let lead_byte = match self.buffer.try_peek_k(offset) {
            Some(Ok(byte)) => byte,
            Some(Err(RingBufferError::IndexOutOfBounds(_, _))) => {
                if self.is_closed {
                    return StreamResult::Eof;
                } else {
                    return StreamResult::Incomplete;
                }
            }
            Some(Err(_e)) => return StreamResult::Ok(replacement_char()), // 或者返回错误？
            None => {
                if self.is_closed {
                    return StreamResult::Eof;
                } else {
                    return StreamResult::Incomplete;
                }
            }
        };

        // 计算UTF-8序列长度
        let seq_len = match utf8_sequence_length(lead_byte) {
            Some(len) => len,
            None => return StreamResult::Ok(replacement_char()), // 非法UTF-8首字节
        };

        // 检查是否有足够字节
        let required_size = offset + seq_len;
        let current_size = match self.buffer.get_size() {
            Ok(size) => size,
            Err(_) => return StreamResult::Incomplete,
        };

        if required_size > current_size {
            if self.is_closed {
                // 已关闭但字节不完整，返回替换字符
                return StreamResult::Ok(replacement_char());
            } else {
                return StreamResult::Incomplete;
            }
        }

        // 读取完整UTF-8序列
        let mut bytes = Vec::with_capacity(seq_len);
        for i in 0..seq_len {
            match self.buffer.try_peek_k(offset + i) {
                Some(Ok(byte)) => bytes.push(byte),
                _ => return StreamResult::Ok(replacement_char()),
            }
        }

        // 解码UTF-8
        match std::str::from_utf8(&bytes) {
            Ok(s) => StreamResult::Ok(s.chars().next().unwrap_or(replacement_char())),
            Err(_) => StreamResult::Ok(replacement_char()),
        }
    }

    /// 尝试读取并消费一个字符
    ///
    /// # Returns
    /// - `StreamResult::Ok(c)` - 成功读取并前进
    /// - `StreamResult::Incomplete` - 需要更多输入
    /// - `StreamResult::Eof` - EOF
    pub fn try_advance(&mut self) -> StreamResult<char> {
        match self.try_peek(0) {
            StreamResult::Ok(c) => {
                self.position.advance(c);
                // 消费字节
                let len = c.len_utf8();
                for _ in 0..len {
                    let _ = self.buffer.pop();
                }
                StreamResult::Ok(c)
            }
            StreamResult::Incomplete => StreamResult::Incomplete,
            StreamResult::Eof => StreamResult::Eof,
        }
    }

    /// 检查当前字符是否匹配（不消费）
    pub fn check(&self, expected: char) -> bool {
        matches!(self.try_peek(0), StreamResult::Ok(c) if c == expected)
    }

    /// 检查当前字符是否在集合中（不消费）
    pub fn check_in(&self, chars: &[char]) -> bool {
        matches!(self.try_peek(0), StreamResult::Ok(c) if chars.contains(&c))
    }

    /// 消费当前字符如果匹配
    ///
    /// Returns true if matched and consumed
    pub fn match_char(&mut self, expected: char) -> bool {
        if self.check(expected) {
            let _ = self.try_advance();
            true
        } else {
            false
        }
    }
}

/// 获取UTF-8序列长度
fn utf8_sequence_length(lead_byte: u8) -> Option<usize> {
    match lead_byte {
        0x00..=0x7F => Some(1),   // ASCII
        0xC0..=0xDF => Some(2),   // 2字节序列
        0xE0..=0xEF => Some(3),   // 3字节序列
        0xF0..=0xF7 => Some(4),   // 4字节序列
        _ => None,                // 非法首字节（续字节或超出范围）
    }
}

/// Unicode替换字符（用于错误恢复）
fn replacement_char() -> char {
    '\u{FFFD}'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_ascii() {
        let mut stream = CharStream::new(1024);
        stream.feed(b"abc").unwrap();
        stream.close().unwrap();

        assert!(stream.check('a'));
        assert_eq!(stream.try_advance(), StreamResult::Ok('a'));
        assert_eq!(stream.try_advance(), StreamResult::Ok('b'));
        assert_eq!(stream.try_advance(), StreamResult::Ok('c'));
        assert_eq!(stream.try_advance(), StreamResult::Eof);
    }

    #[test]
    fn test_stream_cjk() {
        let mut stream = CharStream::new(1024);
        stream.feed("中文字".as_bytes()).unwrap();
        stream.close().unwrap();

        assert_eq!(stream.try_advance(), StreamResult::Ok('中'));
        assert_eq!(stream.try_advance(), StreamResult::Ok('文'));
        assert_eq!(stream.try_advance(), StreamResult::Ok('字'));
    }

    #[test]
    fn test_stream_emoji() {
        let mut stream = CharStream::new(1024);
        stream.feed("🎉".as_bytes()).unwrap();
        stream.close().unwrap();

        assert_eq!(stream.try_advance(), StreamResult::Ok('🎉'));
    }

    #[test]
    fn test_stream_position_tracking() {
        let mut stream = CharStream::new(1024);
        stream.feed(b"a\nb").unwrap();
        stream.close().unwrap();

        let start = stream.position();
        assert_eq!(start.line, 1);
        assert_eq!(start.column, 1);

        stream.try_advance(); // 'a'
        let pos1 = stream.position();
        assert_eq!(pos1.line, 1);
        assert_eq!(pos1.column, 2);

        stream.try_advance(); // '\n'
        let pos2 = stream.position();
        assert_eq!(pos2.line, 2);
        assert_eq!(pos2.column, 1);
    }

    #[test]
    fn test_stream_incomplete() {
        let mut stream = CharStream::new(1024);
        // 只写入UTF-8多字节序列的第一部分
        stream.feed(&[0xF0]).unwrap(); // 4字节序列的首字节
        // 不关闭，模拟流式等待

        assert_eq!(stream.try_peek(0), StreamResult::Incomplete);
        assert_eq!(stream.try_advance(), StreamResult::Incomplete);

        // 继续写入剩余字节
        stream.feed(&[0x9F, 0x8E, 0x89]).unwrap();
        stream.close().unwrap();

        assert_eq!(stream.try_advance(), StreamResult::Ok('🎉'));
    }

    #[test]
    fn test_stream_match_char() {
        let mut stream = CharStream::new(1024);
        stream.feed(b"abc").unwrap();
        stream.close().unwrap();

        assert!(stream.match_char('a'));
        assert!(!stream.match_char('a')); // 已经消费了
        assert!(stream.match_char('b'));
    }
}
