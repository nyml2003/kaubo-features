//! Kaubo CLI - Command line interface
//!
//! Handles argument parsing, file IO, terminal output, and logging initialization.

extern crate alloc;

use clap::Parser;
use std::process;

mod platform;

use crate::platform::print_error_with_source;
use kaubo_api::{compile, compile_and_run, init_config, RunConfig, Value};
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
        handle_compile_only(&source, cli.dump_bytecode, cli.show_steps);
    } else {
        handle_run(&source, cli.show_steps);
    }
}

fn handle_compile_only(source: &str, dump: bool, show_steps: bool) {
    if show_steps {
        println!("[Compilation]");
    }

    match compile(source) {
        Ok(output) => {
            if show_steps {
                println!("Constants: {}", output.chunk.constants.len());
                println!("Bytecode: {} bytes", output.chunk.code.len());
                println!("Locals: {}", output.local_count);
            }

            if dump {
                if show_steps {
                    println!("[Bytecode Disassembly]");
                }
                output.chunk.disassemble("main");
            }

            if show_steps {
                println!("✅ Compilation successful");
            }
        }
        Err(e) => {
            print_error_with_source(&e, source);
            process::exit(1);
        }
    }
}

fn handle_run(source: &str, show_steps: bool) {
    if show_steps {
        println!("[Execution]");
    }

    match compile_and_run(source) {
        Ok(output) => {
            if show_steps {
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
    // 根据 verbose 级别创建 logger
    let logger = match cli.verbose {
        0 => Logger::noop(),
        _ => {
            // 使用开发配置创建 logger（输出到 stdout）
            let (logger, _) = LoggerConfig::dev().init();
            logger
        }
    };

    RunConfig {
        show_steps: cli.show_steps,
        dump_bytecode: cli.dump_bytecode,
        compiler: CompilerConfig::default(),
        limits: LimitConfig::default(),
        logger,
    }
}
