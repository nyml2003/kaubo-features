//! API 错误类型
//!
//! 提供统一的错误类型和结构化错误报告。

use thiserror::Error;

/// 词法错误（结构化）
pub use kaubo_core::kit::lexer::LexerError;

/// ErrorLocation 重新导出
use kaubo_core::compiler::parser::error::ErrorLocation;

/// 语法错误（结构化）
pub use kaubo_core::compiler::parser::error::ParserError;

/// 类型错误（结构化）
pub use kaubo_core::compiler::parser::type_checker::TypeError;

/// Kaubo 错误类型
#[derive(Error, Debug, Clone)]
pub enum KauboError {
    /// 词法分析错误（结构化）
    #[error("{0}")]
    Lexer(#[from] LexerError),

    /// 语法分析错误（结构化）
    #[error("{0}")]
    Parser(#[from] ParserError),

    /// 类型错误（结构化）
    #[error("Type error: {0}")]
    Type(#[from] TypeError),

    /// 编译错误
    #[error("Compiler error: {0}")]
    Compiler(String),

    /// 运行时错误
    #[error("Runtime error: {0}")]
    Runtime(String),
}

/// 辅助函数：将 ErrorLocation 转换为元组
fn location_to_tuple(
    loc: &kaubo_core::compiler::parser::error::ErrorLocation,
) -> (&'static str, Option<usize>, Option<usize>) {
    match loc {
        ErrorLocation::At(coord) => ("at", Some(coord.line), Some(coord.column)),
        ErrorLocation::After(coord) => ("after", Some(coord.line), Some(coord.column)),
        ErrorLocation::Eof => ("eof", None, None),
        ErrorLocation::Unknown => ("unknown", None, None),
    }
}

impl KauboError {
    /// 获取错误行号（如果有）
    pub fn line(&self) -> Option<usize> {
        match self {
            KauboError::Lexer(e) => Some(e.line()),
            KauboError::Parser(e) => e.line(),
            KauboError::Type(e) => match &e {
                TypeError::Mismatch { location, .. }
                | TypeError::ReturnTypeMismatch { location, .. }
                | TypeError::UndefinedVar { location, .. }
                | TypeError::UnsupportedOp { location, .. }
                | TypeError::CannotInfer { location, .. } => match location {
                    ErrorLocation::At(coord) | ErrorLocation::After(coord) => Some(coord.line),
                    _ => None,
                },
            },
            _ => None,
        }
    }

    /// 获取错误列号（如果有）
    pub fn column(&self) -> Option<usize> {
        match self {
            KauboError::Lexer(e) => Some(e.column()),
            KauboError::Parser(e) => e.column(),
            KauboError::Type(e) => match &e {
                TypeError::Mismatch { location, .. }
                | TypeError::ReturnTypeMismatch { location, .. }
                | TypeError::UndefinedVar { location, .. }
                | TypeError::UnsupportedOp { location, .. }
                | TypeError::CannotInfer { location, .. } => match location {
                    ErrorLocation::At(coord) | ErrorLocation::After(coord) => Some(coord.column),
                    _ => None,
                },
            },
            _ => None,
        }
    }

    /// 获取错误阶段名称
    pub fn phase(&self) -> &'static str {
        match self {
            KauboError::Lexer(_) => "lexer",
            KauboError::Parser(_) => "parser",
            KauboError::Type(_) => "type",
            KauboError::Compiler(_) => "compiler",
            KauboError::Runtime(_) => "runtime",
        }
    }

    /// 转换为结构化错误报告
    ///
    /// 适用于 Web API、LSP 等需要结构化数据的场景。
    /// CLI 可以直接打印，上层应用可以序列化为 JSON。
    ///
    /// # Example
    /// ```ignore
    /// match compile_and_run(source) {
    ///     Err(e) => {
    ///         let report = e.to_report();
    ///         // CLI: 直接打印
    ///         println!("{}", report);
    ///         // Web: 序列化为 JSON
    ///         let json = serde_json::json!(report);
    ///     }
    /// }
    /// ```
    pub fn to_report(&self) -> ErrorReport {
        match self {
            KauboError::Lexer(e) => ErrorReport {
                phase: "lexer",
                line: Some(e.line()),
                column: Some(e.column()),
                error_kind: format!("{:?}", e.kind),
                message: e.message.clone(),
                details: None,
            },
            KauboError::Parser(e) => {
                let (loc_type, line, col) = match &e.location {
                    kaubo_core::compiler::parser::error::ErrorLocation::At(coord) => {
                        ("at", Some(coord.line), Some(coord.column))
                    }
                    kaubo_core::compiler::parser::error::ErrorLocation::After(coord) => {
                        ("after", Some(coord.line), Some(coord.column))
                    }
                    kaubo_core::compiler::parser::error::ErrorLocation::Eof => ("eof", None, None),
                    kaubo_core::compiler::parser::error::ErrorLocation::Unknown => {
                        ("unknown", None, None)
                    }
                };
                ErrorReport {
                    phase: "parser",
                    line,
                    column: col,
                    error_kind: format!("{:?}", e.kind),
                    message: e.to_string(),
                    details: Some(ErrorDetails::Location {
                        location_type: loc_type,
                    }),
                }
            }
            KauboError::Type(e) => {
                let (loc_type, line, col, error_kind) = match &e {
                    TypeError::Mismatch { location, .. } => {
                        let (t, l, c) = location_to_tuple(location);
                        (t, l, c, "TypeMismatch")
                    }
                    TypeError::ReturnTypeMismatch { location, .. } => {
                        let (t, l, c) = location_to_tuple(location);
                        (t, l, c, "ReturnTypeMismatch")
                    }
                    TypeError::UndefinedVar { location, .. } => {
                        let (t, l, c) = location_to_tuple(location);
                        (t, l, c, "UndefinedVar")
                    }
                    TypeError::UnsupportedOp { location, .. } => {
                        let (t, l, c) = location_to_tuple(location);
                        (t, l, c, "UnsupportedOp")
                    }
                    TypeError::CannotInfer { location, .. } => {
                        let (t, l, c) = location_to_tuple(location);
                        (t, l, c, "CannotInfer")
                    }
                };
                ErrorReport {
                    phase: "type",
                    line,
                    column: col,
                    error_kind: error_kind.to_string(),
                    message: e.to_string(),
                    details: Some(ErrorDetails::Location {
                        location_type: loc_type,
                    }),
                }
            }
            KauboError::Compiler(msg) => ErrorReport {
                phase: "compiler",
                line: None,
                column: None,
                error_kind: "CompileError".to_string(),
                message: msg.clone(),
                details: None,
            },
            KauboError::Runtime(msg) => ErrorReport {
                phase: "runtime",
                line: None,
                column: None,
                error_kind: "RuntimeError".to_string(),
                message: msg.clone(),
                details: None,
            },
        }
    }
}

/// 结构化错误报告
///
/// 上层应用（CLI、Web、LSP）可以根据自己的需求格式化。
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorReport {
    /// 错误阶段: lexer, parser, compiler, runtime
    pub phase: &'static str,
    /// 错误行号（1-based，如果有）
    pub line: Option<usize>,
    /// 错误列号（1-based，如果有）
    pub column: Option<usize>,
    /// 错误类型（可用于程序化处理）
    pub error_kind: String,
    /// 人类可读的错误消息
    pub message: String,
    /// 额外详情（位置类型等）
    pub details: Option<ErrorDetails>,
}

/// 错误额外详情
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorDetails {
    /// 位置相关信息
    Location { location_type: &'static str },
}

impl std::fmt::Display for ErrorReport {
    /// 默认的 CLI 友好格式
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => {
                write!(
                    f,
                    "[{}:{}] {} error: {}",
                    line, col, self.phase, self.message
                )
            }
            _ => write!(f, "[{}] {} error: {}", self.phase, self.phase, self.message),
        }
    }
}

impl ErrorReport {
    /// 转换为 JSON 格式（Web API 使用）
    ///
    /// 不依赖 serde，手动构建 JSON 字符串。
    pub fn to_json(&self) -> String {
        let line = self
            .line
            .map(|l| l.to_string())
            .unwrap_or_else(|| "null".to_string());
        let col = self
            .column
            .map(|c| c.to_string())
            .unwrap_or_else(|| "null".to_string());

        format!(
            r#"{{"phase":"{}","line":{},"column":{},"error_kind":"{}","message":"{}"}}"#,
            self.phase,
            line,
            col,
            escape_json(&self.error_kind),
            escape_json(&self.message)
        )
    }

    /// 简洁格式（适合终端）
    pub fn to_short(&self) -> String {
        format!("{}: {}", self.phase, self.message)
    }
}

/// 简单的 JSON 字符串转义
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_core::compiler::parser::error::ErrorLocation;
    use kaubo_core::compiler::parser::{ParserError, ParserErrorKind};
    use kaubo_core::kit::lexer::types::Coordinate;
    use kaubo_core::kit::lexer::{ErrorKind, SourcePosition};

    #[test]
    fn test_lexer_error_line_column() {
        // 创建一个 lexer error
        let pos = SourcePosition::new(10, 5, 100, 5);
        let lexer_err = LexerError::at(ErrorKind::Utf8Error, pos);
        let err = KauboError::Lexer(lexer_err);

        assert_eq!(err.line(), Some(10));
        assert_eq!(err.column(), Some(5));
        assert_eq!(err.phase(), "lexer");
    }

    #[test]
    fn test_parser_error_line_column() {
        let parser_err = ParserError::at(ParserErrorKind::MissingRightParen, 3, 7);
        let err = KauboError::Parser(parser_err);

        assert_eq!(err.line(), Some(3));
        assert_eq!(err.column(), Some(7));
        assert_eq!(err.phase(), "parser");
    }

    #[test]
    fn test_type_error_line_column() {
        let type_err = TypeError::UndefinedVar {
            name: "x".to_string(),
            location: ErrorLocation::At(Coordinate {
                line: 5,
                column: 10,
            }),
        };
        let err = KauboError::Type(type_err);

        assert_eq!(err.line(), Some(5));
        assert_eq!(err.column(), Some(10));
        assert_eq!(err.phase(), "type");
    }

    #[test]
    fn test_compiler_error() {
        let err = KauboError::Compiler("unknown error".to_string());
        assert_eq!(err.line(), None);
        assert_eq!(err.column(), None);
        assert_eq!(err.phase(), "compiler");
    }

    #[test]
    fn test_runtime_error() {
        let err = KauboError::Runtime("division by zero".to_string());
        assert_eq!(err.line(), None);
        assert_eq!(err.column(), None);
        assert_eq!(err.phase(), "runtime");
    }

    #[test]
    fn test_error_report_display_with_location() {
        let report = ErrorReport {
            phase: "parser",
            line: Some(10),
            column: Some(5),
            error_kind: "UnexpectedToken".to_string(),
            message: "expected ';'".to_string(),
            details: Some(ErrorDetails::Location {
                location_type: "at",
            }),
        };

        let display = format!("{}", report);
        assert!(display.contains("[10:5]"));
        assert!(display.contains("parser"));
        assert!(display.contains("expected ';'"));
    }

    #[test]
    fn test_error_report_display_without_location() {
        let report = ErrorReport {
            phase: "compiler",
            line: None,
            column: None,
            error_kind: "CompileError".to_string(),
            message: "out of memory".to_string(),
            details: None,
        };

        let display = format!("{}", report);
        assert!(display.contains("[compiler]"));
        assert!(display.contains("compiler error"));
    }

    #[test]
    fn test_error_report_to_json() {
        let report = ErrorReport {
            phase: "lexer",
            line: Some(1),
            column: Some(2),
            error_kind: "InvalidChar".to_string(),
            message: "invalid character '@'".to_string(),
            details: None,
        };

        let json = report.to_json();
        assert!(json.contains("\"phase\":\"lexer\""));
        assert!(json.contains("\"line\":1"));
        assert!(json.contains("\"column\":2"));
        assert!(json.contains("\"error_kind\":\"InvalidChar\""));
        assert!(json.contains("\"message\":\"invalid character '@'\""));
    }

    #[test]
    fn test_error_report_to_json_null_values() {
        let report = ErrorReport {
            phase: "runtime",
            line: None,
            column: None,
            error_kind: "RuntimeError".to_string(),
            message: "panic".to_string(),
            details: None,
        };

        let json = report.to_json();
        assert!(json.contains("\"line\":null"));
        assert!(json.contains("\"column\":null"));
    }

    #[test]
    fn test_error_report_to_short() {
        let report = ErrorReport {
            phase: "type",
            line: Some(5),
            column: Some(10),
            error_kind: "TypeMismatch".to_string(),
            message: "expected int, found string".to_string(),
            details: None,
        };

        assert_eq!(report.to_short(), "type: expected int, found string");
    }

    #[test]
    fn test_lexer_error_to_report() {
        let pos = SourcePosition::new(3, 8, 50, 8);
        let lexer_err = LexerError::at(ErrorKind::Utf8Error, pos);
        let err = KauboError::Lexer(lexer_err);
        let report = err.to_report();

        assert_eq!(report.phase, "lexer");
        assert_eq!(report.line, Some(3));
        assert_eq!(report.column, Some(8));
    }

    #[test]
    fn test_compiler_error_to_report() {
        let err = KauboError::Compiler("stack overflow".to_string());
        let report = err.to_report();

        assert_eq!(report.phase, "compiler");
        assert_eq!(report.line, None);
        assert_eq!(report.column, None);
        assert_eq!(report.error_kind, "CompileError");
        assert_eq!(report.message, "stack overflow");
        assert_eq!(report.details, None);
    }

    #[test]
    fn test_runtime_error_to_report() {
        let err = KauboError::Runtime("index out of bounds".to_string());
        let report = err.to_report();

        assert_eq!(report.phase, "runtime");
        assert_eq!(report.error_kind, "RuntimeError");
        assert_eq!(report.message, "index out of bounds");
    }

    #[test]
    fn test_type_error_mismatch_to_report() {
        let type_err = TypeError::Mismatch {
            expected: "int".to_string(),
            found: "string".to_string(),
            location: ErrorLocation::At(Coordinate {
                line: 10,
                column: 5,
            }),
        };
        let err = KauboError::Type(type_err);
        let report = err.to_report();

        assert_eq!(report.phase, "type");
        assert_eq!(report.line, Some(10));
        assert_eq!(report.column, Some(5));
        assert_eq!(report.error_kind, "TypeMismatch");
    }

    #[test]
    fn test_type_error_return_type_mismatch_to_report() {
        let type_err = TypeError::ReturnTypeMismatch {
            expected: "void".to_string(),
            found: "int".to_string(),
            location: ErrorLocation::After(Coordinate {
                line: 20,
                column: 1,
            }),
        };
        let err = KauboError::Type(type_err);
        let report = err.to_report();

        assert_eq!(report.line, Some(20));
        assert_eq!(report.column, Some(1));
        match report.details {
            Some(ErrorDetails::Location { location_type }) => assert_eq!(location_type, "after"),
            _ => panic!("Expected Location details with 'after' type"),
        }
    }

    #[test]
    fn test_type_error_eof_location() {
        let type_err = TypeError::UndefinedVar {
            name: "foo".to_string(),
            location: ErrorLocation::Eof,
        };
        let err = KauboError::Type(type_err);
        let report = err.to_report();

        assert_eq!(report.line, None);
        assert_eq!(report.column, None);
        match report.details {
            Some(ErrorDetails::Location { location_type }) => assert_eq!(location_type, "eof"),
            _ => panic!("Expected Location details with 'eof' type"),
        }
    }

    #[test]
    fn test_type_error_unknown_location() {
        let type_err = TypeError::UnsupportedOp {
            op: "+".to_string(),
            location: ErrorLocation::Unknown,
        };
        let err = KauboError::Type(type_err);
        let report = err.to_report();

        assert_eq!(report.line, None);
        assert_eq!(report.column, None);
        match report.details {
            Some(ErrorDetails::Location { location_type }) => assert_eq!(location_type, "unknown"),
            _ => panic!("Expected Location details with 'unknown' type"),
        }
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("hello\\world"), "hello\\\\world");
        assert_eq!(escape_json("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_json("hello\tworld"), "hello\\tworld");
        assert_eq!(escape_json("hello\rworld"), "hello\\rworld");
    }

    #[test]
    fn test_error_report_to_json_with_special_chars() {
        let report = ErrorReport {
            phase: "parser",
            line: Some(1),
            column: Some(1),
            error_kind: "Error\"Type".to_string(),
            message: "line1\nline2\ttab".to_string(),
            details: None,
        };

        let json = report.to_json();
        assert!(json.contains("\\\"")); // 引号被转义
        assert!(json.contains("\\n")); // 换行被转义
        assert!(json.contains("\\t")); // tab被转义
    }

    #[test]
    fn test_error_details_clone() {
        let details = ErrorDetails::Location {
            location_type: "at",
        };
        let cloned = details.clone();
        match cloned {
            ErrorDetails::Location { location_type } => assert_eq!(location_type, "at"),
        }
    }

    #[test]
    fn test_error_report_clone() {
        let report = ErrorReport {
            phase: "lexer",
            line: Some(1),
            column: Some(2),
            error_kind: "Test".to_string(),
            message: "test".to_string(),
            details: Some(ErrorDetails::Location {
                location_type: "at",
            }),
        };
        let cloned = report.clone();
        assert_eq!(cloned.phase, "lexer");
        assert_eq!(cloned.line, Some(1));
        assert_eq!(cloned.column, Some(2));
    }

    #[test]
    fn test_error_report_equality() {
        let report1 = ErrorReport {
            phase: "lexer",
            line: Some(1),
            column: Some(2),
            error_kind: "Test".to_string(),
            message: "test".to_string(),
            details: None,
        };
        let report2 = ErrorReport {
            phase: "lexer",
            line: Some(1),
            column: Some(2),
            error_kind: "Test".to_string(),
            message: "test".to_string(),
            details: None,
        };
        let report3 = ErrorReport {
            phase: "parser",
            line: Some(1),
            column: Some(2),
            error_kind: "Test".to_string(),
            message: "test".to_string(),
            details: None,
        };
        assert_eq!(report1, report2);
        assert_ne!(report1, report3);
    }
}
