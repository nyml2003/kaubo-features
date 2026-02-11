//! Kaubo API - Execution orchestration layer
//!
//! Provides unified execution interface, including:
//! - Execution flow orchestration
//! - Configuration abstraction (RunConfig)
//! - Unified error handling (KauboError)
//!
//! For CLI convenience, this crate provides a global singleton API.
//! For library use, prefer the explicit `run(source, &config)` API.

use tracing::{Level, debug, error, info, instrument, span};

use kaubo_core::runtime::bytecode::chunk::Chunk;
use kaubo_core::runtime::compiler::compile as compile_to_chunk;
use kaubo_core::runtime::{InterpretResult, VM};

// Re-export config
pub mod config;
pub use config::{RunConfig, init as init_config, config as get_config, is_initialized};

// Re-export error and types
pub mod error;
pub mod types;
pub use error::{ErrorDetails, ErrorReport, KauboError, LexerError, ParserError};
pub use types::{CompileOutput, ExecuteOutput};

// Re-export core types
pub use kaubo_core::Value;
pub use kaubo_core::{CompilerConfig, LimitConfig, Phase};
pub use kaubo_config;

/// Execute with explicit configuration
///
/// This is the recommended API for library users.
pub fn run(source: &str, config: &RunConfig) -> Result<ExecuteOutput, KauboError> {
    let _span = span!(Level::INFO, "run").entered();
    
    info!("Starting execution");
    
    // Compile
    let compiled = compile_with_config(source, &config.compiler)?;
    
    // Optional: dump bytecode
    if config.dump_bytecode {
        compiled.chunk.disassemble("main");
    }
    
    // Execute
    let result = execute_with_config(&compiled.chunk, compiled.local_count, &config.limits)?;
    
    info!("Execution completed");
    Ok(result)
}

/// Compile with explicit configuration
fn compile_with_config(
    source: &str,
    _config: &CompilerConfig,
) -> Result<CompileOutput, KauboError> {
    use kaubo_core::compiler::lexer::builder::build_lexer;
    use kaubo_core::compiler::parser::parser::Parser;

    let mut lexer = build_lexer();
    lexer
        .feed(&source.as_bytes().to_vec())
        .map_err(LexerError::from_feed_error)?;
    lexer.terminate().map_err(LexerError::from_feed_error)?;

    let mut parser = Parser::new(lexer);
    let ast = parser.parse().map_err(KauboError::Parser)?;

    let output = compile_ast(&ast)?;
    Ok(output)
}

/// Execute with explicit configuration
fn execute_with_config(
    chunk: &Chunk,
    local_count: usize,
    _limits: &LimitConfig,
) -> Result<ExecuteOutput, KauboError> {
    let mut vm = VM::new();
    let result = vm.interpret_with_locals(chunk, local_count);

    match result {
        InterpretResult::Ok => {
            let value = vm.stack_top();
            Ok(ExecuteOutput {
                value,
                stdout: String::new(),
            })
        }
        InterpretResult::RuntimeError(msg) => Err(KauboError::Runtime(msg)),
        InterpretResult::CompileError(msg) => Err(KauboError::Compiler(msg)),
    }
}

/// Compile AST to bytecode
#[instrument(target = "kaubo::compiler", skip(ast))]
pub fn compile_ast(ast: &kaubo_core::compiler::parser::Module) -> Result<CompileOutput, KauboError> {
    info!("Starting compiler");

    let (chunk, local_count) = compile_to_chunk(ast)
        .map_err(|e| KauboError::Compiler(format!("{:?}", e)))?;

    debug!(
        constants = chunk.constants.len(),
        code_bytes = chunk.code.len(),
        "compilation completed"
    );

    info!("Compiler completed");

    Ok(CompileOutput { chunk, local_count })
}

// ==================== Legacy API (using global config) ====================

/// Compile source code (uses global config)
///
/// # Panics
/// If global config is not initialized
pub fn compile(source: &str) -> Result<CompileOutput, KauboError> {
    let config = get_config();
    compile_with_config(source, &config.compiler)
}

/// Execute bytecode (uses global config)
///
/// # Panics
/// If global config is not initialized
pub fn execute(chunk: &Chunk, local_count: usize) -> Result<ExecuteOutput, KauboError> {
    let config = get_config();
    execute_with_config(chunk, local_count, &config.limits)
}

/// Compile and run (uses global config)
///
/// # Panics
/// If global config is not initialized
pub fn compile_and_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    let config = get_config();
    run(source, config)
}

/// Quick run with default config (auto-initializes if needed)
pub fn quick_run(source: &str) -> Result<ExecuteOutput, KauboError> {
    if !is_initialized() {
        init_config(RunConfig::default());
    }
    compile_and_run(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_with_explicit_config() {
        let config = RunConfig::default();
        let result = run("return 42;", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quick_run() {
        let result = quick_run("return 42;");
        assert!(result.is_ok());
    }
}
