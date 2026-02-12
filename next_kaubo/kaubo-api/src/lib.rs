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
// compile_to_chunk 不再使用，compile_ast 直接使用 compile_with_shapes
use kaubo_core::runtime::{InterpretResult, VM};
use kaubo_core::kit::lexer::SourcePosition;

// Re-export config
pub mod config;
pub use config::{RunConfig, init as init_config, config as get_config, is_initialized};

// Re-export error and types
pub mod error;
pub mod types;
pub use error::{ErrorDetails, ErrorReport, KauboError, LexerError, ParserError, TypeError};
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
    let result = execute_with_config(&compiled.chunk, compiled.local_count, &compiled.shapes, &config.limits)?;
    
    info!("Execution completed");
    Ok(result)
}

/// Compile with explicit configuration
fn compile_with_config(
    source: &str,
    config: &CompilerConfig,
) -> Result<CompileOutput, KauboError> {
    use kaubo_core::compiler::lexer::builder::build_lexer;
    use kaubo_core::compiler::parser::parser::Parser;
    use kaubo_core::compiler::parser::TypeChecker;

    let mut lexer = build_lexer();
    lexer
        .feed(&source.as_bytes().to_vec())
        .map_err(|e| LexerError::from_stream_error(e, kaubo_core::kit::lexer::SourcePosition::start()))?;
    lexer.terminate()
        .map_err(|e| LexerError::from_stream_error(e, kaubo_core::kit::lexer::SourcePosition::start()))?;

    let mut parser = Parser::new(lexer);
    let ast = parser.parse().map_err(KauboError::Parser)?;

    // 类型检查（如果启用了 emit_debug_info 则启用严格模式）
    let mut type_checker = TypeChecker::new();
    if config.emit_debug_info {
        type_checker.set_strict_mode(true);
    }
    
    // 对模块中的每个语句进行类型检查
    for stmt in &ast.statements {
        if let Err(type_error) = type_checker.check_statement(stmt) {
            return Err(KauboError::Type(type_error));
        }
    }
    
    // 获取生成的 shapes
    let shapes = type_checker.take_shapes();

    let output = compile_ast(&ast, shapes)?;
    Ok(output)
}

/// Execute with explicit configuration
fn execute_with_config(
    chunk: &Chunk,
    local_count: usize,
    shapes: &[kaubo_core::runtime::object::ObjShape],
    _limits: &LimitConfig,
) -> Result<ExecuteOutput, KauboError> {
    let mut vm = VM::new();
    
    // 注册所有 shapes 到 VM
    for shape in shapes {
        vm.register_shape(shape as *const _);
    }
    
    // 根据 Chunk.method_table 初始化 Shape 的方法表
    // 方法函数存储在常量池中，需要在执行前注册到 Shape
    for entry in &chunk.method_table {
        let func_value = chunk.constants[entry.const_idx as usize];
        if let Some(func_ptr) = func_value.as_function() {
            vm.register_method_to_shape(entry.shape_id, entry.method_idx, func_ptr);
        }
    }
    
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
#[instrument(target = "kaubo::compiler", skip(ast, shapes))]
pub fn compile_ast(
    ast: &kaubo_core::compiler::parser::Module,
    shapes: Vec<kaubo_core::runtime::object::ObjShape>,
) -> Result<CompileOutput, KauboError> {
    use kaubo_core::runtime::compiler::compile_with_struct_info;
    use std::collections::HashMap;
    
    info!("Starting compiler");
    
    // 创建 struct name -> (shape_id, field_names) 映射
    let struct_infos: HashMap<String, (u16, Vec<String>)> = shapes.iter()
        .map(|s| (s.name.clone(), (s.shape_id, s.field_names.clone())))
        .collect();

    let (chunk, local_count) = compile_with_struct_info(ast, struct_infos)
        .map_err(|e| KauboError::Compiler(format!("{:?}", e)))?;

    debug!(
        constants = chunk.constants.len(),
        code_bytes = chunk.code.len(),
        shapes = shapes.len(),
        "compilation completed"
    );

    info!("Compiler completed");

    Ok(CompileOutput { chunk, local_count, shapes })
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
pub fn execute(chunk: &Chunk, local_count: usize, shapes: &[kaubo_core::runtime::object::ObjShape]) -> Result<ExecuteOutput, KauboError> {
    let config = get_config();
    execute_with_config(chunk, local_count, shapes, &config.limits)
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
