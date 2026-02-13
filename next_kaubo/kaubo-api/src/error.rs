//! API 错误类型
//!
//! 提供统一的错误类型和结构化错误报告。

use thiserror::Error;

/// 词法错误（结构化）
pub use kaubo_core::kit::lexer::LexerError;

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
    use kaubo_core::compiler::parser::error::ErrorLocation;
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
        use kaubo_core::compiler::parser::error::ErrorLocation;
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
        use kaubo_core::compiler::parser::error::ErrorLocation;
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
