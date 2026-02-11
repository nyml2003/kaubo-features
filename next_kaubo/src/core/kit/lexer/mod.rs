//! Kaubo Lexer V2
//!
//! 新一代词法分析器，设计目标：
//! - 高性能：O(n)复杂度，无动态分发
//! - 可扩展：统一Scanner trait，支持多DSL
//! - 流式：原生支持增量解析
//! - IDE友好：精准位置追踪，LSP协议兼容

pub mod core;
pub mod error;
pub mod kaubo;
pub mod lexer;
pub mod scanner;
pub mod types;

pub use core::{CharStream, SourcePosition, SourceSpan, StreamError, StreamResult};
pub use error::LexerError;
pub use kaubo::{KauboScanner, KauboMode};
pub use lexer::Lexer;
pub use scanner::{Scanner, ScanResult, Token as ScannerToken, LexError, ErrorKind};

/// Lexer版本标记（用于调试和兼容性检查）
pub const LEXER_V2_VERSION: &str = "2.0.0-alpha";
