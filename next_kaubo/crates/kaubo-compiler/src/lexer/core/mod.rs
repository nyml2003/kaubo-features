//! Lexer V2 核心基础设施
//!
//! 提供位置追踪和字符流抽象

pub mod position;
pub mod stream;

pub use position::{SourcePosition, SourceSpan};
pub use stream::{CharStream, StreamError, StreamResult};
