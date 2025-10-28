use thiserror::Error;

use crate::kit::ring_buffer::ring_buffer::RingBufferError;
#[derive(Error, Debug, PartialEq)]
pub enum LexerError {
    /// 从RingBuffer操作产生的错误
    #[error("Ring buffer error: {0}")]
    RingBufferError(#[from] RingBufferError),

    /// EOF后仍尝试写入数据
    #[error("Cannot feed data after EOF")]
    EofAfterFeed,

    /// UTF-8编码错误
    #[error("UTF-8 decoding error")]
    Utf8Error,
}
