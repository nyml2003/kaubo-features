use crate::kit::lexer::types::Coordinate;

/// 语法错误，包含位置信息
#[derive(Debug, Clone, PartialEq)]
pub struct ParserError {
    /// 错误类型
    pub kind: ParserErrorKind,
    /// 错误发生的位置
    pub location: ErrorLocation,
}

/// 错误位置信息
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorLocation {
    /// 特定位置
    At(Coordinate),
    /// 在某个token之后
    After(Coordinate),
    /// 文件末尾
    Eof,
    /// 未知位置
    Unknown,
}

/// 语法错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ParserErrorKind {
    /// 意外的token
    UnexpectedToken {
        found: String,
        expected: Vec<String>,
    },
    /// 无效的数字格式
    InvalidNumberFormat(String),
    /// 缺少右括号
    MissingRightParen,
    /// 缺少右方括号
    MissingRightBracket,
    /// 缺少右花括号
    MissingRightCurly,
    /// 意外的输入结束
    UnexpectedEndOfInput,
    /// Lambda参数中需要逗号或管道符
    ExpectedCommaOrPipeInLambda,
    /// 点号后需要标识符
    ExpectedIdentifierAfterDot,
    /// 期望标识符
    ExpectedIdentifier { found: String },
    /// 自定义错误消息
    Custom(String),
}

impl ParserError {
    /// 在指定位置创建错误
    pub fn at(kind: ParserErrorKind, line: usize, column: usize) -> Self {
        Self {
            kind,
            location: ErrorLocation::At(Coordinate { line, column }),
        }
    }

    /// 在当前位置创建错误（从token获取位置）
    pub fn here(kind: ParserErrorKind, coordinate: Coordinate) -> Self {
        Self {
            kind,
            location: ErrorLocation::At(coordinate),
        }
    }

    /// 在文件末尾创建错误
    pub fn at_eof(kind: ParserErrorKind) -> Self {
        Self {
            kind,
            location: ErrorLocation::Eof,
        }
    }

    /// 获取行号（如果可用）
    pub fn line(&self) -> Option<usize> {
        match &self.location {
            ErrorLocation::At(coord) | ErrorLocation::After(coord) => Some(coord.line),
            ErrorLocation::Eof | ErrorLocation::Unknown => None,
        }
    }

    /// 获取列号（如果可用）
    pub fn column(&self) -> Option<usize> {
        match &self.location {
            ErrorLocation::At(coord) | ErrorLocation::After(coord) => Some(coord.column),
            ErrorLocation::Eof | ErrorLocation::Unknown => None,
        }
    }
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 位置前缀
        let location_prefix = match &self.location {
            ErrorLocation::At(coord) => format!("{}:{}", coord.line, coord.column),
            ErrorLocation::After(coord) => format!("{}:{}(after)", coord.line, coord.column),
            ErrorLocation::Eof => "EOF".to_string(),
            ErrorLocation::Unknown => "?:?".to_string(),
        };

        // 错误消息
        let message = match &self.kind {
            ParserErrorKind::UnexpectedToken { found, expected } => {
                if expected.is_empty() {
                    format!("Unexpected token '{found}'")
                } else {
                    format!(
                        "Unexpected token '{}', expected: {}",
                        found,
                        expected.join(", ")
                    )
                }
            }
            ParserErrorKind::InvalidNumberFormat(s) => {
                format!("Invalid number format: '{s}'")
            }
            ParserErrorKind::MissingRightParen => "Missing right parenthesis ')'".to_string(),
            ParserErrorKind::MissingRightBracket => "Missing right bracket ']'".to_string(),
            ParserErrorKind::MissingRightCurly => "Missing right curly brace '}}'".to_string(),
            ParserErrorKind::UnexpectedEndOfInput => "Unexpected end of input".to_string(),
            ParserErrorKind::ExpectedCommaOrPipeInLambda => {
                "Expected ',' or '|' in lambda parameters".to_string()
            }
            ParserErrorKind::ExpectedIdentifierAfterDot => {
                "Expected identifier after '.'".to_string()
            }
            ParserErrorKind::ExpectedIdentifier { found } => {
                format!("Expected identifier, found: '{found}'")
            }
            ParserErrorKind::Custom(msg) => msg.clone(),
        };

        write!(f, "[{location_prefix}] {message}")
    }
}

impl std::error::Error for ParserError {}

/// 解析结果类型
pub type ParseResult<T> = Result<T, ParserError>;

/// 辅助函数：创建意外token错误
pub fn unexpected_token(
    found: impl Into<String>,
    expected: Vec<impl Into<String>>,
) -> ParserErrorKind {
    ParserErrorKind::UnexpectedToken {
        found: found.into(),
        expected: expected.into_iter().map(Into::into).collect(),
    }
}

/// 辅助函数：创建期望标识符错误
pub fn expected_identifier(found: impl Into<String>) -> ParserErrorKind {
    ParserErrorKind::ExpectedIdentifier {
        found: found.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_at_location() {
        let err = ParserError::at(ParserErrorKind::MissingRightParen, 10, 5);
        assert_eq!(err.line(), Some(10));
        assert_eq!(err.column(), Some(5));
        assert!(matches!(err.kind, ParserErrorKind::MissingRightParen));
    }

    #[test]
    fn test_error_at_eof() {
        let err = ParserError::at_eof(ParserErrorKind::UnexpectedEndOfInput);
        assert_eq!(err.line(), None);
        assert_eq!(err.column(), None);
        assert!(matches!(err.location, ErrorLocation::Eof));
    }

    #[test]
    fn test_error_display_with_location() {
        let err = ParserError::at(
            ParserErrorKind::UnexpectedToken {
                found: ";".to_string(),
                expected: vec!["identifier".to_string()],
            },
            5,
            10,
        );
        let display = format!("{err}");
        assert!(display.contains("5:10"));
        assert!(display.contains("Unexpected token"));
    }

    #[test]
    fn test_error_display_eof() {
        let err = ParserError::at_eof(ParserErrorKind::UnexpectedEndOfInput);
        let display = format!("{err}");
        assert!(display.contains("EOF"));
    }

    #[test]
    fn test_unexpected_token_helper() {
        let kind = unexpected_token("+", vec!["identifier", "number"]);
        assert!(matches!(kind, ParserErrorKind::UnexpectedToken { .. }));
    }

    #[test]
    fn test_error_clone() {
        let err = ParserError::at(ParserErrorKind::MissingRightParen, 1, 1);
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_error_equality() {
        let err1 = ParserError::at(ParserErrorKind::MissingRightParen, 1, 1);
        let err2 = ParserError::at(ParserErrorKind::MissingRightParen, 1, 1);
        let err3 = ParserError::at(ParserErrorKind::UnexpectedEndOfInput, 1, 1);
        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}
