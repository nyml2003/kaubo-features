//! Kaubo CLI - Command line interface
//!
//! Handles argument parsing, file IO, terminal output, and logging initialization.

extern crate alloc;

use clap::Parser;
use std::process;

mod platform;

use crate::platform::print_error_with_source;
use kaubo_api::{init_config, RunConfig, Value};
use kaubo_config::{CompilerConfig, LimitConfig};
use kaubo_log::{LogConfig as LoggerConfig, Logger};

#[derive(Parser)]
#[command(
    name = "kaubo",
    about = "Kaubo programming language",
    version = "0.1.0"
)]
struct Cli {
    /// Source file path
    file: String,

    /// Log level (-v=info, -vv=debug, -vvv=trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Compile only, do not execute
    #[arg(long)]
    compile_only: bool,

    /// Dump bytecode
    #[arg(long)]
    dump_bytecode: bool,

    /// Show execution steps
    #[arg(long)]
    show_steps: bool,

    /// Show source content
    #[arg(long)]
    show_source: bool,
}

fn main() {
    let cli = Cli::parse();

    // Read source file
    let source = match std::fs::read_to_string(&cli.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Cannot read file '{}': {}", cli.file, e);
            process::exit(1);
        }
    };

    // Build run configuration with logger
    let run_config = build_run_config(&cli);

    // Initialize API config (global singleton for convenience)
    init_config(run_config.clone());

    // Show source
    if cli.show_source {
        println!("[Source]");
        for (i, line) in source.lines().enumerate() {
            println!("{:3} | {}", i + 1, line);
        }
        println!("[Execution Result]");
    }

    // Show step info
    if cli.show_steps {
        println!("[Kaubo VM - Bytecode Execution]");
        println!("======================");
        println!("File: {}", cli.file);
    }

    // Execute
    if cli.compile_only {
        handle_compile_only(&source, run_config);
    } else {
        handle_run(&source, run_config);
    }
}

/// 将字节码输出到 stdout（简洁模式，无 DEBUG 日志）
fn dump_bytecode_to_stdout(chunk: &kaubo_core::runtime::bytecode::chunk::Chunk, name: &str) {
    use kaubo_core::runtime::bytecode::OpCode;
    
    println!("== {} ==", name);
    println!("Constants:");
    for (i, constant) in chunk.constants.iter().enumerate() {
        println!("  [{:3}] {:?}", i, constant);
    }
    println!("Bytecode:");

    let mut offset = 0;
    while offset < chunk.code.len() {
        let line_info = if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
            "   | ".to_string()
        } else {
            format!("{:4} ", chunk.lines[offset])
        };

        let instruction = chunk.code[offset];
        let opcode = OpCode::from(instruction);

        match opcode {
            // 无操作数指令
            op if op.operand_size() == 0 => {
                println!("{:04} {}{}", offset, line_info, op.name());
                offset += 1;
            }
            // u8 操作数
            OpCode::LoadConst => {
                let idx = chunk.code[offset + 1];
                println!("{:04} {}LOAD_CONST {}", offset, line_info, idx);
                offset += 2;
            }
            OpCode::LoadLocal => {
                let idx = chunk.code[offset + 1];
                println!("{:04} {}LOAD_LOCAL {}", offset, line_info, idx);
                offset += 2;
            }
            OpCode::StoreLocal => {
                let idx = chunk.code[offset + 1];
                println!("{:04} {}STORE_LOCAL {}", offset, line_info, idx);
                offset += 2;
            }
            OpCode::Call => {
                let argc = chunk.code[offset + 1];
                println!("{:04} {}CALL {}", offset, line_info, argc);
                offset += 2;
            }
            OpCode::GetField | OpCode::SetField | OpCode::LoadMethod => {
                let idx = chunk.code[offset + 1];
                println!("{:04} {}{} {}", offset, line_info, opcode.name(), idx);
                offset += 2;
            }
            // u16 操作数
            OpCode::Jump => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let target = (hi << 8) | lo;
                println!("{:04} {}JUMP {}", offset, line_info, target);
                offset += 3;
            }
            OpCode::JumpIfFalse => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let target = (hi << 8) | lo;
                println!("{:04} {}JUMP_IF_FALSE {}", offset, line_info, target);
                offset += 3;
            }
            OpCode::JumpBack => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let target = (hi << 8) | lo;
                println!("{:04} {}JUMP_BACK {}", offset, line_info, target);
                offset += 3;
            }
            OpCode::ModuleGet => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let id = (hi << 8) | lo;
                println!("{:04} {}MODULE_GET {}", offset, line_info, id);
                offset += 3;
            }
            OpCode::BuildStruct => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let id = (hi << 8) | lo;
                let count = chunk.code[offset + 3];
                println!("{:04} {}BUILD_STRUCT {} {}", offset, line_info, id, count);
                offset += 4;
            }
            // 其他 u16 操作数
            OpCode::LoadConstWide => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let idx = (hi << 8) | lo;
                println!("{:04} {}LOAD_CONST_WIDE {}", offset, line_info, idx);
                offset += 3;
            }
            // 默认处理
            _ => {
                let size = opcode.operand_size();
                if size == 1 {
                    let operand = chunk.code[offset + 1];
                    println!("{:04} {}{} {}", offset, line_info, opcode.name(), operand);
                    offset += 2;
                } else if size == 2 {
                    let hi = chunk.code[offset + 1] as u16;
                    let lo = chunk.code[offset + 2] as u16;
                    let val = (hi << 8) | lo;
                    println!("{:04} {}{} {}", offset, line_info, opcode.name(), val);
                    offset += 3;
                } else {
                    println!("{:04} {}{}", offset, line_info, opcode.name());
                    offset += 1;
                }
            }
        }
    }
}

fn handle_compile_only(source: &str, config: RunConfig) {
    use kaubo_api::compile_with_config;

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
                if config.show_steps {
                    println!("[Bytecode Disassembly]");
                }
                dump_bytecode_to_stdout(&output.chunk, "main");
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

fn handle_run(source: &str, config: RunConfig) {
    use kaubo_api::run;

    if config.show_steps {
        println!("[Execution]");
    }

    match run(source, &config) {
        Ok(output) => {
            if config.show_steps {
                println!("✅ Execution successful!");
                if let Some(value) = output.value {
                    println!("Return value: {}", value);
                }
            } else if let Some(value) = output.value {
                // Non-step mode: only print return value (actual program output)
                if value != Value::NULL {
                    println!("{}", value);
                }
            }
        }
        Err(e) => {
            print_error_with_source(&e, source);
            process::exit(1);
        }
    }
}

fn build_run_config(cli: &Cli) -> RunConfig {
    // 只在 verbose 或 show_steps 时启用 dev logger
    // dump_bytecode 现在使用独立的 stdout 输出，不需要 logger
    let needs_logger = cli.verbose > 0 || cli.show_steps;
    let logger = if needs_logger {
        let (logger, _) = LoggerConfig::dev().init();
        logger
    } else {
        Logger::noop()
    };

    RunConfig {
        show_steps: cli.show_steps,
        dump_bytecode: cli.dump_bytecode,
        compiler: CompilerConfig::default(),
        limits: LimitConfig::default(),
        logger,
    }
}
