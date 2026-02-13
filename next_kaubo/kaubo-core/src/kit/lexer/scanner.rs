//! Scanner trait 定义
//!
//! 所有 DSL 的词法分析器都需实现此 trait

use super::core::{SourcePosition, SourceSpan, StreamResult};
use kaubo_log::Logger;
use std::sync::Arc;

/// 词法扫描器 trait
///
/// 为不同 DSL（Kaubo、JSON、Vue 模板等）提供统一接口
pub trait Scanner {
    /// Token 类型
    type TokenKind: Clone + PartialEq + std::fmt::Debug;
    /// 扫描模式（用于上下文敏感的语言，如 Vue 模板）
    type Mode: Copy + PartialEq + std::fmt::Debug;

    /// 创建新扫描器
    fn new() -> Self;

    /// 创建带 logger 的扫描器（默认实现调用 new）
    fn with_logger(_logger: Arc<Logger>) -> Self
    where
        Self: Sized,
    {
        Self::new()
    }

    /// 设置扫描模式
    fn set_mode(&mut self, mode: Self::Mode);

    /// 获取当前模式
    fn current_mode(&self) -> Self::Mode;

    /// 扫描下一个 token
    ///
    /// 这是核心方法，驱动字符流并生成 token
    fn next_token(&mut self, stream: &mut super::CharStream) -> ScanResult<Token<Self::TokenKind>>;

    /// 错误恢复：跳过到下一个安全位置
    ///
    /// 默认实现跳过到下一个空白符或已知分隔符
    fn recover_error(&mut self, stream: &mut super::CharStream) -> RecoveryAction {
        // 默认实现：跳过非法字符
        while let StreamResult::Ok(c) = stream.try_peek(0) {
            if c.is_whitespace() || is_recover_point(c) {
                break;
            }
            let _ = stream.try_advance();
        }
        RecoveryAction::Continue
    }
}

/// Token 结构
#[derive(Debug, Clone, PartialEq)]
pub struct Token<K> {
    pub kind: K,
    pub span: SourceSpan,
    /// 原始文本（可选，节省内存时可省略）
    pub text: Option<String>,
}

impl<K> Token<K> {
    /// 创建新 token（不保存文本）
    pub fn new(kind: K, span: SourceSpan) -> Self {
        Self {
            kind,
            span,
            text: None,
        }
    }

    /// 创建新 token（保存文本）
    pub fn with_text(kind: K, span: SourceSpan, text: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            text: Some(text.into()),
        }
    }

    /// 获取 token 的起始位置
    pub fn start(&self) -> SourcePosition {
        self.span.start
    }

    /// 获取 token 的结束位置
    pub fn end(&self) -> SourcePosition {
        self.span.end
    }
}

/// 扫描结果
#[derive(Debug, Clone, PartialEq)]
pub enum ScanResult<T> {
    /// 成功扫描到 token
    Token(T),
    /// 需要更多输入（流式场景）
    Incomplete,
    /// 流已结束
    Eof,
    /// 扫描错误
    Error(LexError),
}

impl<T> ScanResult<T> {
    /// 将 token 类型映射为另一种类型
    pub fn map_kind<U, F>(self, f: F) -> ScanResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ScanResult::Token(t) => ScanResult::Token(f(t)),
            ScanResult::Incomplete => ScanResult::Incomplete,
            ScanResult::Eof => ScanResult::Eof,
            ScanResult::Error(e) => ScanResult::Error(e),
        }
    }
}

/// 词法错误
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub kind: ErrorKind,
    pub position: SourcePosition,
    pub message: String,
}

/// 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// 非法字符
    InvalidChar(char),
    /// 未终止的字符串
    UnterminatedString,
    /// 非法转义序列
    InvalidEscape(String),
    /// 数字格式错误
    InvalidNumber(String),
    /// UTF-8 解码错误
    Utf8Error,
    /// 其他错误
    Custom(String),
}

/// 恢复动作
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    /// 继续扫描
    Continue,
    /// 切换到指定模式
    SwitchMode,
    /// 停止扫描
    Abort,
}

/// 判断是否为错误恢复点（安全跳过位置）
fn is_recover_point(c: char) -> bool {
    matches!(
        c,
        ';' | '{'
            | '}'
            | '('
            | ')'
            | '['
            | ']'
            | ','
            | '.'
            | '+'
            | '-'
            | '*'
            | '/'
            | '='
            | '<'
            | '>'
            | '!'
    )
}

/// 辅助函数：检查字符是否为标识符起始字符
pub fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// 辅助函数：检查字符是否为标识符延续字符
pub fn is_identifier_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Token 构建器
pub struct TokenBuilder<K> {
    kind: K,
    start: SourcePosition,
}

impl<K> TokenBuilder<K> {
    pub fn new(kind: K, start: SourcePosition) -> Self {
        Self { kind, start }
    }

    pub fn build(self, end: SourcePosition) -> Token<K> {
        Token::new(self.kind, SourceSpan::range(self.start, end))
    }

    pub fn with_text(self, end: SourcePosition, text: impl Into<String>) -> Token<K> {
        Token::with_text(self.kind, SourceSpan::range(self.start, end), text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestToken {
        Plus,
        Number(i64),
    }

    #[test]
    fn test_token_new() {
        let pos = SourcePosition::start();
        let token = Token::new(TestToken::Plus, SourceSpan::at(pos));
        assert_eq!(token.kind, TestToken::Plus);
        assert!(token.text.is_none());
    }

    #[test]
    fn test_token_with_text() {
        let pos = SourcePosition::start();
        let token = Token::with_text(TestToken::Number(42), SourceSpan::at(pos), "42");
        assert_eq!(token.text, Some("42".to_string()));
    }

    #[test]
    fn test_scan_result_map() {
        let result: ScanResult<i32> = ScanResult::Token(42);
        let mapped = result.map_kind(|n| n.to_string());
        assert!(matches!(mapped, ScanResult::Token(s) if s == "42"));
    }

    #[test]
    fn test_is_identifier_start() {
        assert!(is_identifier_start('a'));
        assert!(is_identifier_start('_'));
        assert!(!is_identifier_start('1'));
        assert!(!is_identifier_start('+'));
    }

    #[test]
    fn test_is_identifier_continue() {
        assert!(is_identifier_continue('a'));
        assert!(is_identifier_continue('1'));
        assert!(is_identifier_continue('_'));
        assert!(!is_identifier_continue('+'));
    }
}
