//! API 层 - 对外接口
//!
//! 提供编译和执行的核心 API，输入 → 输出，不关心如何呈现。

use tracing::{Level, debug, error, info, instrument, span};

use crate::core::runtime::bytecode::chunk::Chunk;
use crate::core::runtime::compiler::compile as compile_to_chunk;
use crate::core::runtime::value::Value;
use crate::core::runtime::{InterpretResult, VM};

// 重导出错误和类型
pub use error::{ErrorDetails, ErrorReport, KauboError, LexerError, ParserError};
pub use types::{CompileOutput, ExecuteOutput};

mod error;
mod types;

// ==================== 独立阶段 API ====================

/// 词法分析
///
/// 将源代码转换为 Token 序列。
///
/// # Errors
/// 遇到非法字符或格式错误时返回 `KauboError::Lexer`
///
/// # 日志查看
/// 查看词法分析阶段的详细日志：
/// ```bash
/// # 方法1: 使用 -vv 启用 debug 级别日志
/// cargo run --release -- file.kaubo -vv
///
/// # 方法2: 单独设置 lexer 日志级别为 trace
/// cargo run --release -- file.kaubo --log-lexer trace
///
/// # 方法3: 结合全局和特定级别
/// cargo run --release -- file.kaubo -v --log-lexer trace
/// ```
#[instrument(target = "kaubo::lexer", skip(source), fields(len = source.len()))]
pub fn lex(
    source: &str,
) -> Result<
    Vec<crate::core::kit::lexer::scanner::Token<crate::core::compiler::lexer::token_kind::KauboTokenKind>>,
    KauboError,
> {
    use crate::core::compiler::lexer::builder::build_lexer;

    info!("Starting lexer");

    let mut lexer = build_lexer();

    lexer
        .feed(&source.as_bytes().to_vec())
        .map_err(LexerError::from_feed_error)?;

    lexer.terminate().map_err(LexerError::from_feed_error)?;

    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        debug!(
            kind = ?token.kind,
            text = ?token.text,
            "produced token"
        );
        tokens.push(token);
    }

    info!("Lexer completed: {} tokens", tokens.len());
    Ok(tokens)
}

/// 语法分析（暂未实现）
///
/// **注意**: 此函数目前未实现，直接调用会返回错误。
/// 如需解析，请使用 `compile()` 函数。
///
/// 未来版本将支持从 Token 序列直接解析 AST。
///
/// # Errors
/// 始终返回 `KauboError::Parser` 错误
#[instrument(target = "kaubo::parser", skip(_tokens), fields(count = _tokens.len()))]
#[deprecated(since = "0.1.0", note = "暂未实现，请使用 compile() 代替")]
pub fn parse(
    _tokens: Vec<
        crate::core::kit::lexer::scanner::Token<crate::core::compiler::lexer::token_kind::KauboTokenKind>,
    >,
) -> Result<crate::core::compiler::parser::Module, KauboError> {
    info!("Starting parser");

    Err(KauboError::Parser(ParserError::at_eof(
        crate::core::compiler::parser::error::ParserErrorKind::Custom(
            "parse() is not yet implemented. Use compile() instead.".to_string(),
        ),
    )))
}

/// 编译 AST
///
/// 将 AST 转换为字节码。
///
/// # Errors
/// 编译错误时返回 `KauboError::Compiler`
#[instrument(target = "kaubo::compiler", skip(ast))]
pub fn compile_ast(ast: &crate::core::compiler::parser::Module) -> Result<CompileOutput, KauboError> {
    info!("Starting compiler");

    let (chunk, local_count) = compile_to_chunk(ast).map_err(|e| KauboError::Compiler(format!("{:?}", e)))?;

    debug!(
        constants = chunk.constants.len(),
        code_bytes = chunk.code.len(),
        "compilation completed"
    );

    info!("Compiler completed");

    Ok(CompileOutput { chunk, local_count })
}

/// 执行字节码
///
/// 在 VM 中执行字节码。
///
/// # Errors
/// 运行时错误时返回 `KauboError::Runtime`
#[instrument(target = "kaubo::vm", skip(chunk), fields(constants = chunk.constants.len()))]
pub fn execute(chunk: &Chunk, local_count: usize) -> Result<ExecuteOutput, KauboError> {
    info!("Starting VM execution");

    let mut vm = VM::new();
    let result = vm.interpret_with_locals(chunk, local_count);

    match result {
        InterpretResult::Ok => {
            let value = vm.stack_top();
            debug!(return_value = ?value, "execution completed");
            info!("VM execution completed successfully");

            Ok(ExecuteOutput {
                value,
                stdout: String::new(), // TODO: 捕获 stdout
            })
        }
        InterpretResult::RuntimeError(msg) => {
            error!(error = %msg, "runtime error");
            Err(KauboError::Runtime(msg))
        }
        InterpretResult::CompileError(msg) => {
            error!(error = %msg, "compile error");
            Err(KauboError::Compiler(msg))
        }
    }
}

// ==================== 组合 API ====================

/// 编译源代码（不执行）
///
/// 完整编译流程：源代码 -> Lexer -> Parser -> Compiler -> 字节码
///
/// # Errors
/// 任何阶段出错都会返回对应的 `KauboError`
pub fn compile(source: &str) -> Result<CompileOutput, KauboError> {
    let _span = span!(Level::INFO, "compile").entered();

    info!("Starting full compilation");

    // 词法分析 + 语法分析
    use crate::core::compiler::lexer::builder::build_lexer;
    use crate::core::compiler::parser::parser::Parser;

    let mut lexer = build_lexer();
    lexer
        .feed(&source.as_bytes().to_vec())
        .map_err(LexerError::from_feed_error)?;
    lexer.terminate().map_err(LexerError::from_feed_error)?;

    let mut parser = Parser::new(lexer);
    let ast = parser.parse().map_err(KauboError::Parser)?;

    let output = compile_ast(&ast)?;

    info!("Full compilation completed");
    Ok(output)
}

/// 编译并执行（完整流程）
///
/// 源代码 -> 字节码 -> 执行 -> 结果
///
/// # Errors
/// 任何阶段出错都会返回对应的 `KauboError`
pub fn compile_and_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    let _span = span!(Level::INFO, "compile_and_run").entered();

    info!("Starting compile and run");

    let compiled = compile(source)?;
    let result = execute(&compiled.chunk, compiled.local_count)?;

    info!("Compile and run completed");
    Ok(result)
}

/// 快速编译并执行（使用默认配置）
///
/// 自动初始化默认配置并执行。
/// 仅适用于简单场景，生产环境建议手动配置。
pub fn quick_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    if !crate::core::config::is_initialized() {
        crate::core::config::init(crate::core::config::Config::default());
    }
    compile_and_run(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::kit::lexer::{ErrorKind, SourcePosition};

    #[test]
    fn test_lexer_error_conversion() {
        let pos = SourcePosition::new(10, 5, 100, 5);
        let lexer_err = LexerError::at(ErrorKind::InvalidChar('@'), pos);
        let err = KauboError::Lexer(lexer_err);

        assert_eq!(err.line(), Some(10));
        assert_eq!(err.column(), Some(5));
        assert_eq!(err.phase(), "lexer");
        assert!(err.to_string().contains("Invalid character"));
    }

    #[test]
    fn test_parser_error_conversion() {
        let parser_err = ParserError::at(
            crate::core::compiler::parser::error::ParserErrorKind::MissingRightParen,
            5,
            10,
        );
        let err = KauboError::Parser(parser_err);

        assert_eq!(err.line(), Some(5));
        assert_eq!(err.column(), Some(10));
        assert_eq!(err.phase(), "parser");
    }

    #[test]
    fn test_compiler_error() {
        let err = KauboError::Compiler("test error".to_string());
        assert_eq!(err.line(), None);
        assert_eq!(err.column(), None);
        assert_eq!(err.phase(), "compiler");
    }

    #[test]
    fn test_runtime_error() {
        let err = KauboError::Runtime("runtime error".to_string());
        assert_eq!(err.line(), None);
        assert_eq!(err.column(), None);
        assert_eq!(err.phase(), "runtime");
    }

    #[test]
    fn test_error_report() {
        let pos = SourcePosition::new(3, 7, 50, 7);
        let lexer_err = LexerError::at(ErrorKind::UnterminatedString, pos);
        let err = KauboError::Lexer(lexer_err);

        let report = err.to_report();
        assert_eq!(report.phase, "lexer");
        assert_eq!(report.line, Some(3));
        assert_eq!(report.column, Some(7));
        assert!(report.error_kind.contains("UnterminatedString"));
        assert!(report.message.contains("Unterminated"));
    }

    #[test]
    fn test_error_report_display() {
        let report = ErrorReport {
            phase: "parser",
            line: Some(10),
            column: Some(5),
            error_kind: "MissingRightParen".to_string(),
            message: "Missing right parenthesis".to_string(),
            details: None,
        };

        let display = format!("{}", report);
        assert!(display.contains("[10:5]"));
        assert!(display.contains("parser"));
        assert!(display.contains("Missing right parenthesis"));
    }

    #[test]
    fn test_error_report_json() {
        let report = ErrorReport {
            phase: "lexer",
            line: Some(5),
            column: None,
            error_kind: "InvalidChar".to_string(),
            message: "Invalid character '@'".to_string(),
            details: None,
        };

        let json = report.to_json();
        assert!(json.contains("phase"));
        assert!(json.contains("\"line\":5"));
        assert!(json.contains("\"column\":null"));
        assert!(json.contains("error_kind"));
    }

    #[test]
    fn test_error_report_short() {
        let report = ErrorReport {
            phase: "runtime",
            line: None,
            column: None,
            error_kind: "RuntimeError".to_string(),
            message: "Division by zero".to_string(),
            details: None,
        };

        assert_eq!(report.to_short(), "runtime: Division by zero");
    }
}
