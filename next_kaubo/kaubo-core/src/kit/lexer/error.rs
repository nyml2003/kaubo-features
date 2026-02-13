//! Lexer 错误类型
//!
//! 提供结构化的词法错误信息，包含错误类型、位置和详细消息。

use super::core::{SourcePosition, StreamError};
use super::scanner::ErrorKind;

/// 词法错误，包含结构化信息
#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    /// 错误类型
    pub kind: ErrorKind,
    /// 错误发生的位置
    pub position: SourcePosition,
    /// 详细错误消息
    pub message: String,
}

impl LexerError {
    /// 在指定位置创建错误
    pub fn at(kind: ErrorKind, position: SourcePosition) -> Self {
        let message = Self::format_message(&kind, position);
        Self {
            kind,
            position,
            message,
        }
    }

    /// 从 StreamError 转换
    ///
    /// 用于将底层流错误转换为 LexerError。
    /// 注意：由于 StreamError 不包含 SourcePosition，位置信息会使用 start()。
    /// 建议在实际发生错误的位置调用此方法。
    pub fn from_stream_error(error: StreamError, position: SourcePosition) -> Self {
        let kind = match &error {
            StreamError::Utf8Error(_) => ErrorKind::Utf8Error,
            StreamError::Closed => ErrorKind::Custom("Stream closed".to_string()),
            StreamError::Buffer(_) => ErrorKind::Custom(error.to_string()),
        };

        Self {
            kind,
            position,
            message: error.to_string(),
        }
    }

    /// 获取行号（1-based）
    pub fn line(&self) -> usize {
        self.position.line
    }

    /// 获取列号（1-based）
    pub fn column(&self) -> usize {
        self.position.column
    }

    /// 格式化错误消息
    fn format_message(kind: &ErrorKind, position: SourcePosition) -> String {
        match kind {
            ErrorKind::InvalidChar(ch) => {
                format!(
                    "Invalid character '{}' at {}:{}",
                    ch, position.line, position.column
                )
            }
            ErrorKind::UnterminatedString => {
                format!(
                    "Unterminated string literal starting at {}:{}",
                    position.line, position.column
                )
            }
            ErrorKind::InvalidEscape(seq) => {
                format!(
                    "Invalid escape sequence '{}' at {}:{}",
                    seq, position.line, position.column
                )
            }
            ErrorKind::InvalidNumber(num) => {
                format!(
                    "Invalid number format '{}' at {}:{}",
                    num, position.line, position.column
                )
            }
            ErrorKind::Utf8Error => {
                format!(
                    "UTF-8 decoding error at {}:{}",
                    position.line, position.column
                )
            }
            ErrorKind::Custom(msg) => msg.clone(),
        }
    }
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] ", self.position.line, self.position.column)?;

        match &self.kind {
            ErrorKind::InvalidChar(ch) => write!(f, "Invalid character '{}'", ch),
            ErrorKind::UnterminatedString => write!(f, "Unterminated string literal"),
            ErrorKind::InvalidEscape(seq) => write!(f, "Invalid escape sequence '{}'", seq),
            ErrorKind::InvalidNumber(num) => write!(f, "Invalid number format '{}'", num),
            ErrorKind::Utf8Error => write!(f, "UTF-8 decoding error"),
            ErrorKind::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for LexerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_error_at_position() {
        let pos = SourcePosition::new(10, 5, 100, 5);
        let err = LexerError::at(ErrorKind::InvalidChar('@'), pos);

        assert_eq!(err.line(), 10);
        assert_eq!(err.column(), 5);
        assert!(matches!(err.kind, ErrorKind::InvalidChar('@')));
        assert!(err.message.contains("Invalid character"));
    }

    #[test]
    fn test_lexer_error_display() {
        let pos = SourcePosition::new(3, 7, 50, 7);
        let err = LexerError::at(ErrorKind::UnterminatedString, pos);

        let display = format!("{}", err);
        assert!(display.contains("3:7"));
        assert!(display.contains("Unterminated"));
    }

    #[test]
    fn test_lexer_error_invalid_char() {
        let pos = SourcePosition::new(1, 1, 0, 0);
        let err = LexerError::at(ErrorKind::InvalidChar('#'), pos);

        assert!(err.to_string().contains("Invalid character '#'"));
    }

    #[test]
    fn test_lexer_error_invalid_escape() {
        let pos = SourcePosition::new(5, 10, 100, 10);
        let err = LexerError::at(ErrorKind::InvalidEscape("\\q".to_string()), pos);

        assert!(err.message.contains("Invalid escape"));
        assert!(err.message.contains("\\q"));
    }

    #[test]
    fn test_lexer_error_clone() {
        let pos = SourcePosition::new(1, 1, 0, 0);
        let err = LexerError::at(ErrorKind::InvalidNumber("0xGG".to_string()), pos);
        let cloned = err.clone();

        assert_eq!(err.kind, cloned.kind);
        assert_eq!(err.position, cloned.position);
    }
}
