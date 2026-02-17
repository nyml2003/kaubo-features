//! Kaubo CLI - Command line interface
//!
//! Project-based execution - all configuration from package.json

extern crate alloc;

use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

mod platform;

use crate::platform::print_error_with_source;
use kaubo_api::{compile_project_with_config, compile_with_config, init_config, run, RunConfig, Value};
use kaubo_core::binary::{
    encode_chunk_with_context, BinaryWriter, BuildMode, DecodeContext, EncodeContext, 
    FunctionPool, SectionData, SectionKind, ShapeEntry, ShapeTable, StringPool, VMExecuteBinary, WriteOptions,
};
use kaubo_core::VM;
use std::fs;

/// 执行模式
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExecutionMode {
    /// 自动选择：二进制存在且最新则执行二进制，否则解释执行
    Auto,
    /// 总是解释执行源码
    Source,
    /// 执行二进制（不存在则报错）
    Binary,
}

impl ExecutionMode {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "source" => ExecutionMode::Source,
            "binary" => ExecutionMode::Binary,
            _ => ExecutionMode::Auto, // 默认 auto
        }
    }
}

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
    /// 执行模式: "auto" | "source" | "binary"
    /// - "auto": 自动选择（如果二进制存在且最新则执行二进制，否则解释执行源码）
    /// - "source": 总是解释执行源码
    /// - "binary": 执行二进制（不存在则报错）
    mode: Option<String>,
    /// 是否生成二进制文件 (emit .kaubod)
    emit_binary: Option<bool>,
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

    // Execute based on mode
    let mode = get_execution_mode(&package);
    let emit_binary = should_emit_binary(&package);
    let binary_path = get_binary_path(&entry_path);
    
    if run_config.compile_only {
        // Compile-only mode: compile and optionally emit binary
        handle_compile_only(&source, run_config, &package, emit_binary.then_some(&binary_path));
    } else {
        // Run mode: choose execution method based on mode
        handle_run(&source, run_config, &package, &entry_path, mode, emit_binary, &binary_path);
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

/// Get execution mode from compiler config
fn get_execution_mode(package: &PackageJson) -> ExecutionMode {
    package
        .compiler
        .as_ref()
        .and_then(|c| c.mode.as_ref())
        .map(|m| ExecutionMode::from_str(m))
        .unwrap_or(ExecutionMode::Auto)
}

/// Check if should emit binary file
fn should_emit_binary(package: &PackageJson) -> bool {
    package
        .compiler
        .as_ref()
        .and_then(|c| c.emit_binary)
        .unwrap_or(false)
}

/// Get binary path from source path (e.g., main.kaubo -> main.kaubod)
fn get_binary_path(source_path: &Path) -> PathBuf {
    let mut binary_path = source_path.to_path_buf();
    if let Some(stem) = source_path.file_stem() {
        let parent = source_path.parent().unwrap_or(Path::new("."));
        binary_path = parent.join(format!("{}.kaubod", stem.to_string_lossy()));
    }
    binary_path
}

/// Check if binary exists and is up-to-date (newer than source)
fn is_binary_up_to_date(source_path: &Path, binary_path: &Path) -> bool {
    if !binary_path.exists() {
        return false;
    }
    
    let source_modified = match fs::metadata(source_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false, // Can't determine, assume out of date
    };
    
    let binary_modified = match fs::metadata(binary_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false, // Can't determine, assume out of date
    };
    
    binary_modified >= source_modified
}

/// Compile source and optionally emit binary file
fn compile_and_emit(
    source: &str,
    config: &RunConfig,
    binary_path: Option<&Path>,
) -> Result<kaubo_api::CompileOutput, String> {
    use kaubo_api::compile_with_config;
    
    let output = compile_with_config(source, config)
        .map_err(|e| format!("Compilation error: {:?}", e))?;
    
    // Emit binary if requested
    if let Some(path) = binary_path {
        // 使用新的上下文编码来支持堆对象
        let mut string_pool = StringPool::new();
        let mut function_pool = FunctionPool::new();
        let mut shape_table = ShapeTable::new();
        
        // 先添加模块元数据到 String Pool（确保索引稳定）
        let main_idx = string_pool.add("main");
        let main_kaubo_idx = string_pool.add("main.kaubo");
        
        // 注册编译时收集的 Shape 到 ShapeTable
        for shape in &output.shapes {
            let name_idx = string_pool.add(&shape.name);
            let field_name_indices: Vec<u32> = shape.field_names.iter()
                .map(|name| string_pool.add(name))
                .collect();
            let field_type_indices: Vec<u32> = shape.field_types.iter()
                .map(|ty| string_pool.add(ty))
                .collect();
            
            let entry = ShapeEntry {
                shape_id: shape.shape_id,
                name_idx,
                field_count: shape.field_names.len() as u16,
                field_name_indices,
                field_type_indices,
            };
            shape_table.add(entry);
        }
        
        let mut ctx = EncodeContext::new(&mut string_pool, &mut function_pool, &mut shape_table);
        
        let chunk_data = encode_chunk_with_context(&output.chunk, &mut ctx)
            .map_err(|e| format!("Failed to encode chunk: {:?}", e))?;
        
        // 创建完整的二进制文件
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };
        
        let mut writer = BinaryWriter::new(options);
        
        // 写入 String Pool
        writer.write_section(SectionKind::StringPool, &ctx.string_pool.serialize());
        
        // 写入 Function Pool
        writer.write_section(SectionKind::FunctionPool, &ctx.function_pool.serialize());
        
        // 写入 Shape Table（如果非空）
        if !ctx.shape_table.is_empty() {
            writer.write_section(SectionKind::ShapeTable, &ctx.shape_table.serialize());
        }
        
        // 写入 Module Table（简化版，单模块）
        use kaubo_core::binary::{ModuleEntry, ModuleTable};
        let mut module_table = ModuleTable::new();
        module_table.add(ModuleEntry {
            name_idx: main_idx,
            source_path_idx: main_kaubo_idx,
            chunk_offset: 0,
            chunk_size: chunk_data.len() as u32,
            shape_start: 0,
            shape_count: 0,
            export_start: 0,
            export_count: 0,
            import_start: 0,
            import_count: 0,
        });
        writer.write_section(SectionKind::ModuleTable, &module_table.serialize());
        
        // 写入 Chunk Data
        writer.write_section(SectionKind::ChunkData, &chunk_data);
        
        // 设置入口点
        writer.set_entry(0, 0);
        
        // 写入文件
        let binary_data = writer.finish();
        fs::write(path, binary_data)
            .map_err(|e| format!("Failed to write binary file: {}", e))?;
    }
    
    Ok(output)
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

fn handle_compile_only(
    source: &str, 
    config: RunConfig, 
    package: &PackageJson,
    binary_path: Option<&Path>,
) {
    if config.show_steps {
        println!("[Compilation]");
    }

    match compile_and_emit(source, &config, binary_path) {
        Ok(output) => {
            if config.show_steps {
                println!("Constants: {}", output.chunk.constants.len());
                println!("Bytecode: {} bytes", output.chunk.code.len());
                println!("Locals: {}", output.local_count);
            }

            if config.dump_bytecode {
                dump_bytecode_to_stdout(&output.chunk, &output.shapes, "main");
            }

            if let Some(path) = binary_path {
                if config.show_steps {
                    println!("📦 Binary emitted: {}", path.display());
                }
            }

            if config.show_steps {
                println!("✅ Compilation successful");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn handle_run(
    source: &str, 
    config: RunConfig, 
    package: &PackageJson, 
    entry_path: &Path,
    mode: ExecutionMode,
    emit_binary: bool,
    binary_path: &Path,
) {
    if config.show_steps {
        println!("[Execution Mode: {:?}]", mode);
    }

    // Determine execution strategy based on mode
    let use_binary = match mode {
        ExecutionMode::Binary => {
            // Binary mode: must use binary file
            if !binary_path.exists() {
                eprintln!("Error: Binary mode specified but binary not found: {}", 
                         binary_path.display());
                eprintln!("       Run with compile-only mode first to generate binary.");
                process::exit(1);
            }
            true
        }
        ExecutionMode::Source => {
            // Source mode: always use source
            false
        }
        ExecutionMode::Auto => {
            // Auto mode: use binary if it exists and is up-to-date
            let up_to_date = is_binary_up_to_date(entry_path, binary_path);
            if config.show_steps {
                if up_to_date {
                    println!("📦 Using cached binary: {}", binary_path.display());
                } else {
                    println!("📝 Binary out of date or missing, using source");
                }
            }
            up_to_date
        }
    };

    // Handle bytecode dump if requested
    if config.dump_bytecode && !use_binary {
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

    // Execute based on strategy
    if use_binary {
        // Execute binary file
        execute_binary_file(binary_path, &config, emit_binary);
    } else {
        // Execute from source
        execute_from_source(source, entry_path, &config, emit_binary, binary_path);
    }
}

/// Execute binary file directly
fn execute_binary_file(binary_path: &Path, config: &RunConfig, emit_binary: bool) {
    if config.show_steps {
        println!("[Binary Execution]");
        println!("  Binary: {}", binary_path.display());
    }

    // Read binary file
    let binary_data = match fs::read(binary_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error: Failed to read binary file '{}': {}", 
                     binary_path.display(), e);
            process::exit(1);
        }
    };

    // Create VM and execute binary
    let mut vm = VM::new();
    
    match vm.execute_binary(binary_data) {
        Ok(result) => {
            if config.show_steps {
                println!("✅ Execution successful!");
                println!("  Result: {:?}", result);
            }
        }
        Err(e) => {
            eprintln!("Error: Binary execution failed: {:?}", e);
            process::exit(1);
        }
    }
}

/// Execute from source (compile + interpret)
fn execute_from_source(
    source: &str,
    entry_path: &Path,
    config: &RunConfig,
    emit_binary: bool,
    binary_path: &Path,
) {
    if config.show_steps {
        println!("[Source Execution]");
    }

    // Check if source contains imports - try multi-file compilation
    let has_imports = source.contains("import ");
    
    if has_imports {
        let root_dir = entry_path.parent().unwrap_or(Path::new("."));
        match compile_project_with_config(entry_path, root_dir, config) {
            Ok(result) => {
                if config.show_steps {
                    println!("✅ Multi-file compilation successful!");
                    println!("  Compiled {} modules:", result.units.len());
                    for (i, unit) in result.units.iter().enumerate() {
                        println!("    {}. {} ({})", i + 1, unit.import_path, unit.path.display());
                    }
                }
            }
            Err(e) => {
                eprintln!("Multi-file compilation error: {}", e);
                process::exit(1);
            }
        }
    }

    // Compile and optionally emit binary
    if emit_binary {
        if config.show_steps {
            println!("📦 Emitting binary: {}", binary_path.display());
        }
        match compile_and_emit(source, config, Some(binary_path)) {
            Ok(_) => {
                if config.show_steps {
                    println!("✅ Binary generated successfully");
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to emit binary: {}", e);
                // Continue with execution even if binary emission fails
            }
        }
    }

    // Execute from source
    match run(source, config) {
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
