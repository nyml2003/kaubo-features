//! Kaubo CLI - Command line interface
//!
//! Project-based execution - all configuration from package.json

extern crate alloc;

use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

mod platform;

use crate::platform::print_error_with_source;
use kaubo_api::{compile_with_config, init_config, run, RunConfig, Value};
use kaubo_core::runtime::OpCode;

/// package.json 结构
#[derive(Debug, serde::Deserialize)]
struct PackageJson {
    /// 入口文件路径
    entry: String,
    /// 编译器配置
    compiler: Option<CompilerConfig>,
}

/// 编译器配置
#[derive(Debug, serde::Deserialize)]
struct CompilerConfig {
    /// 是否仅编译，不执行
    compile_only: Option<bool>,
    /// 是否输出字节码（JSON 格式）
    dump_bytecode: Option<bool>,
    /// 是否显示执行步骤
    show_steps: Option<bool>,
    /// 是否显示源码
    show_source: Option<bool>,
    /// 日志级别: "silent", "error", "warn", "info", "debug", "trace"
    log_level: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "kaubo",
    about = "Kaubo programming language - Project-based execution",
    version = "0.1.0"
)]
struct Cli {
    /// Configuration file path (default: ./package.json)
    #[arg(value_name = "CONFIG", default_value = "package.json")]
    config: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    // Read package.json
    let package = match read_package_json(&cli.config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Resolve entry file path (relative to package.json directory)
    let entry_path = resolve_entry_path(&cli.config, &package.entry);

    // Read source file
    let source = match std::fs::read_to_string(&entry_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Error: Cannot read entry file '{}': {}",
                entry_path.display(),
                e
            );
            process::exit(1);
        }
    };

    // Build run configuration from package.json
    let run_config = build_run_config(&package);

    // Initialize API config (global singleton for convenience)
    init_config(run_config.clone());

    // Show source
    if run_config.show_source {
        println!("[Source]");
        for (i, line) in source.lines().enumerate() {
            println!("{:3} | {}", i + 1, line);
        }
        println!("[Execution Result]");
    }

    // Show step info
    if run_config.show_steps {
        println!("[Kaubo VM - Bytecode Execution]");
        println!("======================");
        println!("Entry: {}", entry_path.display());
    }

    // Execute
    if run_config.compile_only {
        handle_compile_only(&source, run_config, &package);
    } else {
        handle_run(&source, run_config, &package);
    }
}

/// Read and parse package.json
fn read_package_json(path: &Path) -> Result<PackageJson, String> {
    if !path.exists() {
        return Err(format!(
            "未找到 '{}'\n\n当前目录不是一个 Kaubo 项目。\n提示: 创建 '{}' 文件并指定 'entry' 字段",
            path.display(),
            path.display()
        ));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取 '{}': {}", path.display(), e))?;

    let package: PackageJson = serde_json::from_str(&content)
        .map_err(|e| format!("解析 '{}' 失败: {}", path.display(), e))?;

    if package.entry.is_empty() {
        return Err(format!("'{}' 中的 'entry' 字段不能为空", path.display()));
    }

    Ok(package)
}

/// Resolve entry file path relative to package.json directory
fn resolve_entry_path(package_path: &Path, entry: &str) -> PathBuf {
    let base_dir = package_path.parent().unwrap_or(Path::new("."));
    base_dir.join(entry)
}

/// Build run configuration from package.json
fn build_run_config(package: &PackageJson) -> RunConfig {
    // Extract compiler config from package.json
    let compiler = package.compiler.as_ref();

    let show_steps = compiler.and_then(|c| c.show_steps).unwrap_or(false);
    let dump_bytecode = compiler.and_then(|c| c.dump_bytecode).unwrap_or(false);
    let show_source = compiler.and_then(|c| c.show_source).unwrap_or(false);
    let compile_only = compiler.and_then(|c| c.compile_only).unwrap_or(false);

    // Parse log level
    let log_level = compiler
        .and_then(|c| c.log_level.as_ref())
        .and_then(|s| parse_log_level(s));

    RunConfig::from_options(
        show_steps,
        dump_bytecode,
        show_source,
        compile_only,
        log_level,
    )
}

/// Parse log level string
fn parse_log_level(s: &str) -> Option<kaubo_api::kaubo_config::LogLevel> {
    use kaubo_api::kaubo_config::LogLevel;
    match s.to_lowercase().as_str() {
        "silent" => Some(LogLevel::Error), // silent = only errors
        "error" => Some(LogLevel::Error),
        "warn" => Some(LogLevel::Warn),
        "info" => Some(LogLevel::Info),
        "debug" => Some(LogLevel::Debug),
        "trace" => Some(LogLevel::Trace),
        _ => None,
    }
}

/// 将字节码输出到 stdout（JSON 格式）
fn dump_bytecode_to_stdout(chunk: &kaubo_core::Chunk, shapes: &[kaubo_core::ObjShape], name: &str) {
    dump_json_output(chunk, shapes, name);
}

/// JSON 格式输出编译结果（支持嵌套函数）
fn dump_json_output(chunk: &kaubo_core::Chunk, shapes: &[kaubo_core::ObjShape], name: &str) {
    let output = build_json_output(chunk, shapes, name);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// 递归构建 JSON 输出
fn build_json_output(chunk: &kaubo_core::Chunk, shapes: &[kaubo_core::ObjShape], name: &str) -> serde_json::Value {
    use serde_json::json;
    
    // 构建 shapes JSON
    let shapes_json: Vec<serde_json::Value> = shapes.iter().map(|s| {
        let fields: Vec<serde_json::Value> = s.field_names.iter()
            .zip(s.field_types.iter())
            .map(|(name, ty)| json!({ "name": name, "type": ty }))
            .collect();
        json!({
            "id": s.shape_id,
            "name": s.name,
            "fields": fields
        })
    }).collect();
    
    // 构建 bytecode JSON
    let bytecode_json = build_bytecode_json(chunk);
    
    // 构建嵌套函数 JSON
    let mut functions_json: Vec<serde_json::Value> = Vec::new();
    for (idx, constant) in chunk.constants.iter().enumerate() {
        // 尝试获取函数内部的 chunk
        if let Some(func_chunk) = get_function_chunk(constant) {
            let func_name = format!("{}#func_{}", name, idx);
            functions_json.push(build_json_output(func_chunk, shapes, &func_name));
        }
    }
    
    let mut result = json!({
        "name": name,
        "shapes": shapes_json,
        "bytecode": bytecode_json
    });
    
    // 如果有嵌套函数，添加到 JSON
    if !functions_json.is_empty() {
        result["functions"] = json!(functions_json);
    }
    
    result
}

/// 构建字节码指令数组（简化版，不含行号和offset）
fn build_bytecode_json(chunk: &kaubo_core::Chunk) -> Vec<serde_json::Value> {
    use serde_json::json;
    let mut bytecode_json: Vec<serde_json::Value> = Vec::new();
    let mut offset = 0;
    
    while offset < chunk.code.len() {
        let instruction = chunk.code[offset];
        let opcode = kaubo_core::runtime::OpCode::from(instruction);
        
        let size = opcode.operand_size();
        let instr_json = match opcode {
            _ if size == 0 => json!({
                "opcode": opcode.name()
            }),
            _ if size == 1 => json!({
                "opcode": opcode.name(),
                "operand": chunk.code[offset + 1]
            }),
            _ => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let val = (hi << 8) | lo;
                json!({
                    "opcode": opcode.name(),
                    "operand": val
                })
            }
        };
        bytecode_json.push(instr_json);
        offset += size as usize + 1;
    }
    
    bytecode_json
}

/// 尝试从 Value 获取函数的 chunk
fn get_function_chunk(value: &kaubo_core::Value) -> Option<&kaubo_core::Chunk> {
    // 检查是否是函数类型
    if let Some(func_ptr) = value.as_function() {
        unsafe {
            return Some(&(*func_ptr).chunk);
        }
    }
    None
}

fn handle_compile_only(source: &str, config: RunConfig, package: &PackageJson) {
    if config.show_steps {
        println!("[Compilation]");
    }

    match compile_with_config(source, &config) {
        Ok(output) => {
            if config.show_steps {
                println!("Constants: {}", output.chunk.constants.len());
                println!("Bytecode: {} bytes", output.chunk.code.len());
                println!("Locals: {}", output.local_count);
            }

            if config.dump_bytecode {
                dump_bytecode_to_stdout(&output.chunk, &output.shapes, "main");
            }

            if config.show_steps {
                println!("✅ Compilation successful");
            }
        }
        Err(e) => {
            print_error_with_source(&e, source);
            process::exit(1);
        }
    }
}

fn handle_run(source: &str, config: RunConfig, package: &PackageJson) {
    if config.show_steps {
        println!("[Execution]");
    }

    // Compile first to get chunk for bytecode dump
    if config.dump_bytecode {
        match compile_with_config(source, &config) {
            Ok(output) => {
                dump_bytecode_to_stdout(&output.chunk, &output.shapes, "main");
            }
            Err(e) => {
                print_error_with_source(&e, source);
                process::exit(1);
            }
        }
    }

    match run(source, &config) {
        Ok(output) => {
            if config.show_steps {
                println!("✅ Execution successful!");
                if let Some(value) = output.value {
                    println!("Return value: {value}");
                }
            } else if let Some(value) = output.value {
                // Non-step mode: only print return value (actual program output)
                if value != Value::NULL {
                    println!("{value}");
                }
            }
        }
        Err(e) => {
            print_error_with_source(&e, source);
            process::exit(1);
        }
    }
}
