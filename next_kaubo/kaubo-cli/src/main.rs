//! Kaubo CLI - Command line interface for Kaubo language
//!
//! Uses Orchestrator for all operations.
//!
//! 运行方式:
//!   cargo run -p kaubo-cli -- examples/hello/package.json

use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;

use kaubo_orchestrator::{
    FileLoader, FileEmitter, StdoutEmitter, BytecodeEmitter, SourceParser,
    CompilePass, MultiModulePass,
    Orchestrator, Source, Target, OutputBuffer, OutputEntry, new_output_buffer,
    ExecutionRequest, DataFormat, IR,
};
use kaubo_config::VmConfig;
use kaubo_orchestrator::vm::core::{VM, InterpretResult};
use kaubo_orchestrator::vm::binary::{SectionData, VMExecuteBinary};

/// CLI 参数
#[derive(Parser)]
#[command(
    name = "kaubo",
    about = "Kaubo language compiler - Orchestrator-based",
    version = "0.1.0"
)]
struct Cli {
    /// 配置文件路径 (package.json)
    #[arg(value_name = "CONFIG", default_value = "package.json")]
    config: PathBuf,
    
    /// 显示编译步骤
    #[arg(short, long)]
    verbose: bool,
    
    /// 仅编译，不执行
    #[arg(short, long)]
    compile_only: bool,
    
    /// 输出字节码（JSON 格式）
    #[arg(long)]
    dump_bytecode: bool,
    
    /// 生成二进制文件 (.kaubod)
    #[arg(long)]
    emit_binary: bool,
    
    /// 执行模式: auto | source | binary
    #[arg(short, long, default_value = "auto")]
    mode: String,
}

/// package.json 结构
#[derive(Debug, serde::Deserialize)]
struct PackageJson {
    name: String,
    version: String,
    entry: String,
    #[serde(default)]
    compiler: Option<CompilerConfig>,
}

#[derive(Debug, serde::Deserialize)]
struct CompilerConfig {
    #[serde(default)]
    compile_only: Option<bool>,
    #[serde(default)]
    dump_bytecode: Option<bool>,
    #[serde(default)]
    show_steps: Option<bool>,
    #[serde(default)]
    show_source: Option<bool>,
    #[serde(default)]
    log_level: Option<String>,
    #[serde(default)]
    emit_binary: Option<bool>,
    #[serde(default)]
    mode: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    
    // 读取 package.json
    let package = match read_package_json(&cli.config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };
    
    // 解析配置选项
    let verbose = cli.verbose || package.compiler.as_ref().and_then(|c| c.show_steps).unwrap_or(false);
    let compile_only = cli.compile_only || package.compiler.as_ref().and_then(|c| c.compile_only).unwrap_or(false);
    let dump_bytecode = cli.dump_bytecode || package.compiler.as_ref().and_then(|c| c.dump_bytecode).unwrap_or(false);
    let emit_binary = cli.emit_binary || package.compiler.as_ref().and_then(|c| c.emit_binary).unwrap_or(false);
    let mode = cli.mode;
    
    if verbose {
        println!("=== Kaubo CLI ===\n");
        println!("Project: {} v{}", package.name, package.version);
        println!("Entry: {}", package.entry);
        println!("Mode: {}", mode);
        println!();
    }
    
    // 解析入口文件路径
    let entry_path = cli.config.parent()
        .unwrap_or(Path::new("."))
        .join(&package.entry);
    
    // 根据模式执行
    match mode.as_str() {
        "binary" => {
            // 执行已存在的二进制文件
            let binary_path = entry_path.with_extension("kaubod");
            execute_binary_file(&binary_path, verbose);
        }
        _ => {
            // 编译执行
            if verbose {
                println!("Starting compilation...");
            }
            
            match compile_and_execute(
                &entry_path,
                verbose,
                compile_only,
                dump_bytecode,
                emit_binary,
            ) {
                Ok(_) => {
                    if verbose {
                        println!("\n✅ Operation completed successfully!");
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Error: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}

/// 读取源代码并判断是否为多文件模式
fn detect_mode(entry_path: &Path) -> Result<(String, bool), String> {
    // 读取入口文件内容
    let content = std::fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read entry file: {}", e))?;
    
    // 检测是否有 import 语句
    let has_imports = content.contains("import ");
    
    Ok((content, has_imports))
}

/// 编译并执行 - 使用 Orchestrator::run()
fn compile_and_execute(
    entry_path: &Path,
    verbose: bool,
    compile_only: bool,
    dump_bytecode: bool,
    emit_binary: bool,
) -> Result<(), String> {
    // 1. 检测编译模式
    if verbose {
        println!("  [1/4] Detecting compilation mode...");
    }
    
    let (_source_code, has_imports) = detect_mode(entry_path)?;
    let pass_name = if has_imports { "multi_module" } else { "compile" };
    
    if verbose {
        if has_imports {
            println!("        Detected imports - using multi-module compilation");
        } else {
            println!("        Single file compilation");
        }
        println!("        Selected pass: {}", pass_name);
    }
    
    // 2. 创建并配置编排器
    if verbose {
        println!("  [2/4] Setting up orchestrator...");
    }
    
    let config = VmConfig::default();
    let mut orchestrator = Orchestrator::new(config);
    
    // 注册 Loader 和 Emitter
    orchestrator.register_loader(Box::new(FileLoader::new()));
    orchestrator.register_emitter(Box::new(FileEmitter::new()));
    orchestrator.register_emitter(Box::new(StdoutEmitter::new()));
    orchestrator.register_emitter(Box::new(BytecodeEmitter::new()));
    
    // 注册 AdaptiveParser - 将 RawData 转换为 IR::Source
    orchestrator.register_adaptive_parser(Box::new(SourceParser::new()));
    
    // 注册 Pass
    let logger: Arc<kaubo_log::Logger> = if verbose {
        kaubo_log::Logger::new(kaubo_log::Level::Debug)
    } else {
        kaubo_log::Logger::new(kaubo_log::Level::Warn)
    };
    
    orchestrator.register_pass(Box::new(CompilePass::new(logger.clone())));
    orchestrator.register_pass(Box::new(MultiModulePass::new(logger)));
    
    // 3. 执行编译
    if verbose {
        println!("  [3/4] Compiling...");
    }
    
    // 准备执行请求
    // 注意：对于多文件模式，MultiModulePass 期望输入是文件路径字符串
    // 对于单文件模式，CompilePass 期望输入是源代码字符串
    let source = Source::file(entry_path);
    
    let request = ExecutionRequest::new(source)
        .from_to(DataFormat::Source, DataFormat::Bytecode)
        .with_target(Target::memory())
        .with_preferred_pass(pass_name);
    
    let result = orchestrator.run(request)
        .map_err(|e| format!("Compilation error: {}", e))?;
    
    // 获取编译结果 (chunk)
    let chunk = match &result.final_ir {
        Some(IR::Bytecode(chunk)) => chunk.clone(),
        _ => return Err("Expected Bytecode output".to_string()),
    };
    
    if verbose {
        println!("        Generated {} bytes of bytecode", chunk.code.len());
        println!("        Constants: {}", chunk.constants.len());
    }
    
    // 4. 可选：转储字节码
    if dump_bytecode {
        if verbose {
            println!("  [4/4] Dumping bytecode...");
        }
        dump_bytecode_to_stdout(&chunk);
    }
    
    // 5. 可选：生成二进制文件
    if emit_binary {
        if verbose {
            println!("  Emitting binary...");
        }
        let binary_path = entry_path.with_extension("kaubod");
        emit_binary_file(&chunk, &binary_path, verbose)?;
    }
    
    // 6. 执行（如果不是仅编译模式）
    if !compile_only {
        if verbose {
            if dump_bytecode || emit_binary {
                println!("  [5/5] Executing...");
            } else {
                println!("  [4/4] Executing...");
            }
        }
        
        // 输出捕获的内容（克隆以避免 borrow 问题）
        let output_entries = result.output_entries.clone();
        let has_real_output = output_entries.iter().any(|e| matches!(e, OutputEntry::Print(_) | OutputEntry::Error(_)));
        
        for entry in output_entries {
            match entry {
                OutputEntry::Print(msg) => println!("{}", msg),
                OutputEntry::Source(src) => println!("{}", src),
                OutputEntry::Bytecode(bc) => println!("{}", bc),
                OutputEntry::Info(info) => println!("{}", info),
                OutputEntry::Error(err) => eprintln!("{}", err),
            }
        }
        
        // 如果没有输出条目，直接执行字节码
        if !has_real_output {
            execute_bytecode(&chunk, verbose)?;
        }
    }
    
    Ok(())
}

/// 转储字节码到 stdout
fn dump_bytecode_to_stdout(chunk: &kaubo_orchestrator::vm::core::Chunk) {
    println!("\n=== Bytecode Dump ===");
    println!("Code size: {} bytes", chunk.code.len());
    println!("Constants: {}", chunk.constants.len());
    
    // 简化的字节码输出
    for (i, byte) in chunk.code.iter().enumerate() {
        if i % 16 == 0 {
            print!("\n  {:04x}: ", i);
        }
        print!("{:02x} ", byte);
    }
    println!();
    
    println!("\nConstants:");
    for (i, constant) in chunk.constants.iter().enumerate() {
        println!("  [{}]: {:?}", i, constant);
    }
    println!("===================\n");
}

/// 生成二进制文件
fn emit_binary_file(
    chunk: &kaubo_orchestrator::vm::core::Chunk,
    path: &Path,
    verbose: bool,
) -> Result<(), String> {
    use kaubo_orchestrator::vm::binary::{
        BinaryWriter, BuildMode, EncodeContext,
        FunctionPool, ModuleTable, SectionKind, ShapeTable, StringPool,
        WriteOptions, ModuleEntry,
    };
    
    // 创建编码上下文
    let mut string_pool = StringPool::new();
    let mut function_pool = FunctionPool::new();
    let mut shape_table = ShapeTable::new();
    
    let main_idx = string_pool.add("main");
    let main_kaubo_idx = string_pool.add("main.kaubo");
    
    let mut ctx = EncodeContext::new(&mut string_pool, &mut function_pool, &mut shape_table);
    
    let chunk_data = kaubo_orchestrator::vm::binary::encode_chunk_with_context(chunk, &mut ctx)
        .map_err(|e| format!("Failed to encode chunk: {:?}", e))?;
    
    // 创建二进制写入器
    let options = WriteOptions {
        build_mode: BuildMode::Debug,
        compress: false,
        strip_debug: false,
        source_map_external: false,
    };
    
    let mut writer = BinaryWriter::new(options);
    
    // 写入各个段
    writer.write_section(SectionKind::StringPool, &ctx.string_pool.serialize());
    writer.write_section(SectionKind::FunctionPool, &ctx.function_pool.serialize());
    
    if !ctx.shape_table.is_empty() {
        writer.write_section(SectionKind::ShapeTable, &ctx.shape_table.serialize());
    }
    
    // 模块表
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
    
    // Chunk 数据
    writer.write_section(SectionKind::ChunkData, &chunk_data);
    writer.set_entry(0, 0);
    
    // 写入文件
    let binary_data = writer.finish();
    std::fs::write(path, binary_data)
        .map_err(|e| format!("Failed to write binary file: {}", e))?;
    
    if verbose {
        println!("        Binary emitted: {}", path.display());
    }
    
    Ok(())
}

/// 执行字节码
fn execute_bytecode(
    chunk: &kaubo_orchestrator::vm::core::Chunk,
    verbose: bool,
) -> Result<(), String> {
    // 创建 VM
    let mut vm = VM::new();
    
    // 执行 chunk
    match vm.interpret(&chunk) {
        InterpretResult::Ok => {
            if verbose {
                println!("        Execution completed successfully");
            }
            Ok(())
        }
        InterpretResult::CompileError(msg) => {
            Err(format!("Compile error: {}", msg))
        }
        InterpretResult::RuntimeError(msg) => {
            Err(format!("Runtime error: {}", msg))
        }
    }
}

/// 执行二进制文件
fn execute_binary_file(binary_path: &Path, verbose: bool) {
    use kaubo_orchestrator::vm::core::InterpretResult;
    
    if verbose {
        println!("  [Binary Execution]");
        println!("    Binary: {}", binary_path.display());
    }
    
    // 检查文件是否存在
    if !binary_path.exists() {
        eprintln!("Error: Binary file not found: {}", binary_path.display());
        eprintln!("       Run with --emit-binary first to generate binary.");
        process::exit(1);
    }
    
    // 读取二进制文件
    let binary_data = std::fs::read(binary_path)
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to read binary file: {}", e);
            process::exit(1);
        });
    
    // 创建 VM 并执行
    let mut vm = VM::new();
    
    match vm.execute_binary(binary_data) {
        Ok(InterpretResult::Ok) => {
            if verbose {
                println!("    ✅ Execution successful!");
            }
        }
        Ok(InterpretResult::CompileError(msg)) => {
            eprintln!("Error: Compile error in binary: {}", msg);
            process::exit(1);
        }
        Ok(InterpretResult::RuntimeError(msg)) => {
            eprintln!("Error: Runtime error: {}", msg);
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: Failed to load binary: {:?}", e);
            process::exit(1);
        }
    }
}

/// 读取并解析 package.json
fn read_package_json(path: &Path) -> Result<PackageJson, String> {
    if !path.exists() {
        return Err(format!(
            "'{}' not found\n\nThis is not a Kaubo project.
Hint: Create a '{}' file with an 'entry' field",
            path.display(),
            path.display()
        ));
    }
    
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
    
    let package: PackageJson = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))?;
    
    if package.entry.is_empty() {
        return Err(format!("'entry' field in '{}' cannot be empty", path.display()));
    }
    
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_package_json_parsing() {
        // 从 Cargo 环境变量获取项目根目录
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let test_path = Path::new(manifest_dir)
            .parent()
            .unwrap()
            .join("examples/hello/package.json");
        
        let package = read_package_json(&test_path);
        assert!(package.is_ok(), "Failed to parse: {:?}", package.err());
        
        let package = package.unwrap();
        assert_eq!(package.name, "hello");
        assert_eq!(package.entry, "main.kaubo");
    }
}
