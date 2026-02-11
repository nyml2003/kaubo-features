//! 高层 API - 供 CLI 和库用户使用
//!
//! 提供各阶段的独立调用和组合调用，统一错误处理。
//!
//! # 使用示例
//! ```ignore
//! use kaubo::{compile_and_run, Config, init};
//!
//! init(Config::default());
//! let result = compile_and_run("return 1 + 2;").unwrap();
//! ```

use thiserror::Error;
use tracing::{Level, debug, error, info, instrument, span};

use crate::runtime::bytecode::chunk::Chunk;
use crate::runtime::compiler::compile as compile_to_chunk;
use crate::runtime::value::Value;
use crate::runtime::{InterpretResult, VM};

/// Kaubo 错误类型
#[derive(Error, Debug, Clone)]
pub enum KauboError {
    /// 词法分析错误
    #[error("Lexer error: {message}")]
    Lexer { message: String },

    /// 语法分析错误（带位置信息）
    #[error("Parser error: {message}")]
    Parser {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    /// 编译错误
    #[error("Compiler error: {0}")]
    Compiler(String),

    /// 运行时错误
    #[error("Runtime error: {0}")]
    Runtime(String),
}

impl KauboError {
    /// 从 Lexer 错误转换
    fn from_lexer_error<E: std::fmt::Display>(e: E) -> Self {
        KauboError::Lexer {
            message: e.to_string(),
        }
    }

    /// 从 Parser 错误转换
    fn from_parser_error(e: crate::compiler::parser::error::ParserError) -> Self {
        KauboError::Parser {
            message: e.to_string(),
            line: e.line(),
            column: e.column(),
        }
    }

    /// 从 Compiler 错误转换
    fn from_compiler_error(e: crate::runtime::CompileError) -> Self {
        KauboError::Compiler(format!("{:?}", e))
    }

    /// 获取错误行号（如果有）
    pub fn line(&self) -> Option<usize> {
        match self {
            KauboError::Parser { line, .. } => *line,
            _ => None,
        }
    }

    /// 获取错误列号（如果有）
    pub fn column(&self) -> Option<usize> {
        match self {
            KauboError::Parser { column, .. } => *column,
            _ => None,
        }
    }
}

/// 编译输出
#[derive(Debug)]
pub struct CompileOutput {
    /// 字节码块
    pub chunk: Chunk,
    /// 局部变量数量
    pub local_count: usize,
}

/// 执行输出
#[derive(Debug)]
pub struct ExecuteOutput {
    /// 返回值
    pub value: Option<Value>,
    /// 标准输出捕获
    pub stdout: String,
}

// ==================== 独立阶段 API ====================

/// 词法分析
///
/// 将源代码转换为 Token 序列。
///
/// # Errors
/// 遇到非法字符或格式错误时返回 `KauboError::Lexer`
#[instrument(target = "kaubo::lexer", skip(source), fields(len = source.len()))]
pub fn lex(
    source: &str,
) -> Result<
    Vec<crate::kit::lexer::scanner::Token<crate::compiler::lexer::token_kind::KauboTokenKind>>,
    KauboError,
> {
    use crate::compiler::lexer::builder::build_lexer;

    info!("Starting lexer");

    let mut lexer = build_lexer();

    lexer
        .feed(&source.as_bytes().to_vec())
        .map_err(KauboError::from_lexer_error)?;

    lexer.terminate().map_err(KauboError::from_lexer_error)?;

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
        crate::kit::lexer::scanner::Token<crate::compiler::lexer::token_kind::KauboTokenKind>,
    >,
) -> Result<crate::compiler::parser::Module, KauboError> {
    info!("Starting parser");

    Err(KauboError::Parser {
        message: "parse() is not yet implemented. Use compile() instead.".to_string(),
        line: None,
        column: None,
    })
}

/// 编译 AST
///
/// 将 AST 转换为字节码。
///
/// # Errors
/// 编译错误时返回 `KauboError::Compiler`
#[instrument(target = "kaubo::compiler", skip(ast))]
pub fn compile_ast(ast: &crate::compiler::parser::Module) -> Result<CompileOutput, KauboError> {
    info!("Starting compiler");

    let (chunk, local_count) = compile_to_chunk(ast).map_err(KauboError::from_compiler_error)?;

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
            let value = vm.stack_top(); // 已经返回 Option<Value>
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
    use crate::compiler::lexer::builder::build_lexer;
    use crate::compiler::parser::parser::Parser;

    let mut lexer = build_lexer();
    lexer
        .feed(&source.as_bytes().to_vec())
        .map_err(KauboError::from_lexer_error)?;
    lexer.terminate().map_err(KauboError::from_lexer_error)?;

    let mut parser = Parser::new(lexer);
    let ast = parser.parse().map_err(KauboError::from_parser_error)?;

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

// ==================== 便捷函数 ====================

/// 快速编译并执行（使用默认配置）
///
/// 自动初始化默认配置并执行。
/// 仅适用于简单场景，生产环境建议手动配置。
pub fn quick_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    if !crate::config::is_initialized() {
        crate::config::init(crate::config::Config::default());
    }
    compile_and_run(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = KauboError::Lexer {
            message: "test error".to_string(),
        };
        assert!(err.to_string().contains("Lexer error"));
    }

    #[test]
    fn test_parser_error_with_location() {
        let err = KauboError::Parser {
            message: "unexpected token".to_string(),
            line: Some(10),
            column: Some(5),
        };
        assert_eq!(err.line(), Some(10));
        assert_eq!(err.column(), Some(5));
    }
}
