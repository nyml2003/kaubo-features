//! 测试辅助工具
//!
//! 提供端到端测试的辅助函数

use kaubo_core::compiler::lexer::builder::build_lexer;
use kaubo_core::compiler::parser::parser::Parser;
use kaubo_core::compiler::parser::type_checker::TypeChecker;
use kaubo_core::runtime::compiler::compile_with_struct_info;
use kaubo_core::runtime::{InterpretResult, VM};
use std::collections::HashMap;

/// 执行 Kaubo 代码并返回结果（完整流程：类型检查 + 编译 + 执行）
///
/// # Example
/// ```
/// let output = run_code("var x = 5; return x;");
/// assert!(output.is_ok());
/// ```
pub fn run_code(code: &str) -> Result<ExecResult, ExecError> {
    // 词法分析
    let mut lexer = build_lexer();
    let _ = lexer.feed(&code.as_bytes().to_vec());
    let _ = lexer.terminate();

    // 语法分析
    let mut parser = Parser::new(lexer);
    let ast = parser
        .parse()
        .map_err(|e| ExecError::Parser(format!("{:?}", e)))?;

    // 类型检查（生成 shapes）
    let mut type_checker = TypeChecker::new();
    for stmt in &ast.statements {
        type_checker
            .check_statement(stmt)
            .map_err(|e| ExecError::Compiler(format!("{:?}", e)))?;
    }
    let shapes = type_checker.take_shapes();

    // 创建 struct_infos
    let struct_infos: HashMap<String, (u16, Vec<String>)> = shapes
        .iter()
        .map(|s| (s.name.clone(), (s.shape_id, s.field_names.clone())))
        .collect();

    // 编译
    let (chunk, local_count) = compile_with_struct_info(&ast, struct_infos)
        .map_err(|e| ExecError::Compiler(format!("{:?}", e)))?;

    // 执行
    let mut vm = VM::new();
    vm.init_stdlib();

    // 注册 shapes
    for shape in &shapes {
        vm.register_shape(shape as *const _);
    }

    // 根据 Chunk.method_table 初始化 Shape 的方法表
    for entry in &chunk.method_table {
        let func_value = chunk.constants[entry.const_idx as usize];
        if let Some(func_ptr) = func_value.as_function() {
            vm.register_method_to_shape(entry.shape_id, entry.method_idx, func_ptr);
        }
    }

    let result = vm.interpret_with_locals(&chunk, local_count);

    match result {
        InterpretResult::Ok => {
            let return_value = vm.stack_top();
            Ok(ExecResult {
                return_value,
                output: String::new(), // TODO: 捕获 stdout
            })
        }
        InterpretResult::RuntimeError(msg) => Err(ExecError::Runtime(msg)),
        InterpretResult::CompileError(msg) => Err(ExecError::Compiler(msg)),
    }
}

/// 执行结果
#[derive(Debug)]
pub struct ExecResult {
    /// 返回值
    pub return_value: Option<kaubo_core::Value>,
    /// 标准输出
    pub output: String,
}

/// 执行错误
#[derive(Debug)]
pub enum ExecError {
    Lexer(String),
    Parser(String),
    Compiler(String),
    Runtime(String),
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::Lexer(msg) => write!(f, "Lexer error: {}", msg),
            ExecError::Parser(msg) => write!(f, "Parser error: {}", msg),
            ExecError::Compiler(msg) => write!(f, "Compiler error: {}", msg),
            ExecError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for ExecError {}

/// 获取整数值
pub fn get_int(result: &ExecResult) -> Option<i32> {
    result.return_value.as_ref()?.as_int()
}

/// 获取浮点数值
pub fn get_float(result: &ExecResult) -> Option<f64> {
    Some(result.return_value.as_ref()?.as_float())
}

/// 获取字符串值
pub fn get_string(result: &ExecResult) -> Option<String> {
    let ptr = result.return_value.as_ref()?.as_string()?;
    unsafe { Some((&*ptr).chars.clone()) }
}
