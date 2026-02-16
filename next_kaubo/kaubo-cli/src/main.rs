//! Kaubo CLI - Command line interface
//!
//! Handles argument parsing, file IO, terminal output, and logging initialization.

extern crate alloc;

use clap::Parser;
use std::path::PathBuf;
use std::process;

mod platform;

use crate::platform::print_error_with_source;
use kaubo_api::{compile_with_config, init_config, run, RunConfig, Value};
use kaubo_api::kaubo_config::{KauboConfig, Profile};
use kaubo_core::runtime::bytecode::OpCode;

#[derive(Parser)]
#[command(
    name = "kaubo",
    about = "Kaubo programming language",
    version = "0.1.0"
)]
struct Cli {
    /// Source file path
    file: String,

    /// Configuration file path
    #[arg(short, long, value_name = "PATH", conflicts_with = "profile")]
    config: Option<PathBuf>,

    /// Use built-in profile (silent, default, dev, debug, trace)
    #[arg(short, long, value_name = "PROFILE", conflicts_with = "config")]
    profile: Option<String>,

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

    // Build run configuration
    let run_config = match build_run_config(&cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error: Failed to load configuration: {e}");
            process::exit(1);
        }
    };

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
    if run_config.show_steps {
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

/// Build run configuration from CLI arguments and config file/profile
fn build_run_config(cli: &Cli) -> Result<RunConfig, Box<dyn std::error::Error>> {
    // Load config from file, profile, or auto-detect
    let kaubo_config = if let Some(config_path) = &cli.config {
        // Explicit config file
        KauboConfig::from_file(config_path)?
    } else if let Some(profile_str) = &cli.profile {
        // Built-in profile
        let profile = parse_profile(profile_str)?;
        KauboConfig::from_profile(profile)
    } else {
        // Auto-detect config file or use default profile
        KauboConfig::find_and_load().unwrap_or_else(|| KauboConfig::from_profile(Profile::Default))
    };

    // Build RunConfig with CLI overrides
    Ok(RunConfig::from_config(
        &kaubo_config,
        cli.show_steps,
        cli.dump_bytecode,
        cli.verbose,
    ))
}

/// Parse profile string to Profile enum
fn parse_profile(s: &str) -> Result<Profile, String> {
    match s.to_lowercase().as_str() {
        "silent" => Ok(Profile::Silent),
        "default" => Ok(Profile::Default),
        "dev" => Ok(Profile::Dev),
        "debug" => Ok(Profile::Debug),
        "trace" => Ok(Profile::Trace),
        _ => Err(format!(
            "Unknown profile: {s}. Available: silent, default, dev, debug, trace"
        )),
    }
}

/// 将字节码输出到 stdout（简洁模式，无 DEBUG 日志）
fn dump_bytecode_to_stdout(chunk: &kaubo_core::Chunk, name: &str) {
    println!("== {name} ==");
    println!("Constants:");
    for (i, constant) in chunk.constants.iter().enumerate() {
        println!("  [{i:3}] {constant:?}");
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
                println!("{offset:04} {line_info}LOAD_CONST {idx}");
                offset += 2;
            }
            OpCode::LoadLocal => {
                let idx = chunk.code[offset + 1];
                println!("{offset:04} {line_info}LOAD_LOCAL {idx}");
                offset += 2;
            }
            OpCode::StoreLocal => {
                let idx = chunk.code[offset + 1];
                println!("{offset:04} {line_info}STORE_LOCAL {idx}");
                offset += 2;
            }
            OpCode::Call => {
                let argc = chunk.code[offset + 1];
                println!("{offset:04} {line_info}CALL {argc}");
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
                println!("{offset:04} {line_info}JUMP {target}");
                offset += 3;
            }
            OpCode::JumpIfFalse => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let target = (hi << 8) | lo;
                println!("{offset:04} {line_info}JUMP_IF_FALSE {target}");
                offset += 3;
            }
            OpCode::JumpBack => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let target = (hi << 8) | lo;
                println!("{offset:04} {line_info}JUMP_BACK {target}");
                offset += 3;
            }
            OpCode::ModuleGet => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let id = (hi << 8) | lo;
                println!("{offset:04} {line_info}MODULE_GET {id}");
                offset += 3;
            }
            OpCode::BuildStruct => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let id = (hi << 8) | lo;
                let count = chunk.code[offset + 3];
                println!("{offset:04} {line_info}BUILD_STRUCT {id} {count}");
                offset += 4;
            }
            // 其他 u16 操作数
            OpCode::LoadConstWide => {
                let hi = chunk.code[offset + 1] as u16;
                let lo = chunk.code[offset + 2] as u16;
                let idx = (hi << 8) | lo;
                println!("{offset:04} {line_info}LOAD_CONST_WIDE {idx}");
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
    if config.show_steps {
        println!("[Execution]");
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
