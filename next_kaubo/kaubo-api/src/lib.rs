//! Kaubo API - Execution orchestration layer
//!
//! Provides unified execution interface, including:
//! - Execution flow orchestration
//! - Configuration abstraction (RunConfig)
//! - Unified error handling (KauboError)
//!
//! For CLI convenience, this crate provides a global singleton API.
//! For library use, prefer the explicit `run(source, &config)` API.

extern crate alloc;

use kaubo_log::{debug, info, Logger};
use std::collections::HashMap;
use std::sync::Arc;

use kaubo_core::lexer::{build_lexer_with_config, LexerConfig as CoreLexerConfig};
use kaubo_core::parser::{Parser, TypeChecker};
use kaubo_core::{Chunk, InterpretResult, VMConfig, VM};

// 编译器内部函数（临时，待移到 kaubo-core 公共 API）
use kaubo_core::kit::lexer::LexerError;
use kaubo_core::runtime::compiler::compile_with_struct_info_and_logger;

// Re-export config
pub mod config;
pub use config::{config as get_config, init as init_config, is_initialized, RunConfig};

// 配置统一导出
pub use kaubo_config;

// 核心类型导出
pub use kaubo_core::Value;

// 错误类型（统一封装，不暴露底层细节）
pub mod error;
pub mod types;
pub use error::{ErrorReport, KauboError};
pub use types::{CompileOutput, ExecuteOutput};

/// Execute with explicit configuration
///
/// This is the recommended API for library users.
pub fn run(source: &str, config: &RunConfig) -> Result<ExecuteOutput, KauboError> {
    info!(config.logger, "Starting execution");

    // Compile
    let compiled = compile_with_config(source, config)?;

    // Optional: dump bytecode
    if config.dump_bytecode {
        // compiled.chunk.disassemble("main");  // TODO: 需要重新实现
    }

    // Execute
    let result = execute_with_config(
        &compiled.chunk,
        compiled.local_count,
        &compiled.shapes,
        config,
    )?;

    info!(config.logger, "Execution completed");
    Ok(result)
}

/// Compile with explicit configuration
pub fn compile_with_config(source: &str, config: &RunConfig) -> Result<CompileOutput, KauboError> {
    let core_lexer_config = CoreLexerConfig {
        buffer_size: config.lexer.buffer_size,
    };
    let mut lexer = build_lexer_with_config(&core_lexer_config, config.logger.clone());
    lexer.feed(source.as_bytes()).map_err(|e| {
        LexerError::from_stream_error(e, kaubo_core::utils::SourcePosition::start())
    })?;
    lexer.terminate().map_err(|e| {
        LexerError::from_stream_error(e, kaubo_core::utils::SourcePosition::start())
    })?;

    let mut parser = Parser::with_logger(lexer, config.logger.clone());
    let ast = parser.parse().map_err(KauboError::Parser)?;

    // 类型检查（如果启用了 emit_debug_info 则启用严格模式）
    let mut type_checker = TypeChecker::with_logger(config.logger.clone());
    if config.compiler.emit_debug_info {
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

    let output = compile_ast(&ast, shapes, config.logger.clone())?;
    Ok(output)
}

/// Execute with explicit configuration
fn execute_with_config(
    chunk: &Chunk,
    local_count: usize,
    shapes: &[kaubo_core::ObjShape],
    config: &RunConfig,
) -> Result<ExecuteOutput, KauboError> {
    // 使用配置创建 VM 并初始化标准库
    let vm_config = VMConfig {
        initial_stack_size: config.vm.initial_stack_size,
        initial_frames_capacity: config.vm.initial_frames_capacity,
        inline_cache_capacity: config.vm.inline_cache_capacity,
    };
    
    // 根据配置决定是否传递 logger 用于执行追踪
    let mut vm = if config.vm.trace_execution {
        VM::with_config_and_logger(vm_config, config.logger.clone())
    } else {
        VM::with_config(vm_config)
    };
    vm.init_stdlib();

    // 注册所有 shapes 到 VM
    for shape in shapes {
        unsafe {
            vm.register_shape(shape as *const _);
        }
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
pub fn compile_ast(
    ast: &kaubo_core::parser::Module,
    shapes: Vec<kaubo_core::ObjShape>,
    logger: Arc<Logger>,
) -> Result<CompileOutput, KauboError> {
    info!(logger, "Starting compiler");

    // 创建 struct name -> (shape_id, field_names, field_types) 映射
    let struct_infos: HashMap<String, (u16, Vec<String>, Vec<String>)> = shapes
        .iter()
        .map(|s| (s.name.clone(), (s.shape_id, s.field_names.clone(), s.field_types.clone())))
        .collect();

    let (chunk, local_count) =
        compile_with_struct_info_and_logger(ast, struct_infos, logger.clone())
            .map_err(|e| KauboError::Compiler(format!("{e:?}")))?;

    debug!(
        logger,
        "compilation completed: constants={}, code_bytes={}, shapes={}",
        chunk.constants.len(),
        chunk.code.len(),
        shapes.len(),
    );

    info!(logger, "Compiler completed");

    Ok(CompileOutput {
        chunk,
        local_count,
        shapes,
    })
}

// ==================== Legacy API (using global config) ====================

/// Compile source code (uses global config)
///
/// # Panics
/// If global config is not initialized
pub fn compile(source: &str) -> Result<CompileOutput, KauboError> {
    let config = get_config();
    compile_with_config(source, config)
}

/// Execute bytecode (uses global config)
///
/// # Panics
/// If global config is not initialized
pub fn execute(
    chunk: &Chunk,
    local_count: usize,
    shapes: &[kaubo_core::ObjShape],
) -> Result<ExecuteOutput, KauboError> {
    let config = get_config();
    execute_with_config(chunk, local_count, shapes, config)
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
