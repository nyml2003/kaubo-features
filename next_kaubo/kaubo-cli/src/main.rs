//! Kaubo CLI - Command line interface
//!
//! Handles argument parsing, file IO, terminal output, and logging initialization.

use clap::Parser;
use std::process;
use tracing::Level;

mod config;
mod logging;
mod platform;

use kaubo_api::{
    compile, compile_and_run, init_config, RunConfig, Value,
};
use kaubo_config::{CompilerConfig, LimitConfig};
use crate::config::LogConfig;
use crate::logging::{LogFormat, init_with_file};
use crate::platform::print_error_with_source;

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

    /// Lexer log level
    #[arg(long, value_enum)]
    log_lexer: Option<LogLevelArg>,

    /// Parser log level
    #[arg(long, value_enum)]
    log_parser: Option<LogLevelArg>,

    /// Compiler log level
    #[arg(long, value_enum)]
    log_compiler: Option<LogLevelArg>,

    /// VM log level
    #[arg(long, value_enum)]
    log_vm: Option<LogLevelArg>,

    /// Compile only, do not execute
    #[arg(long)]
    compile_only: bool,

    /// Dump bytecode
    #[arg(long)]
    dump_bytecode: bool,

    /// Log output format
    #[arg(long, value_enum, default_value = "pretty")]
    format: LogFormatArg,

    /// Show execution steps
    #[arg(long)]
    show_steps: bool,

    /// Show source content
    #[arg(long)]
    show_source: bool,

    /// Log output to file
    #[arg(long, value_name = "FILE")]
    log_file: Option<String>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum LogLevelArg {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum LogFormatArg {
    Pretty,
    Compact,
    Json,
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

    // Build configurations
    let log_config = build_log_config(&cli);
    let run_config = build_run_config(&cli);

    // Initialize API config (global singleton for convenience)
    init_config(run_config.clone());

    // Initialize logging
    let format = match cli.format {
        LogFormatArg::Pretty => LogFormat::Pretty,
        LogFormatArg::Compact => LogFormat::Compact,
        LogFormatArg::Json => LogFormat::Json,
    };

    if let Some(log_file) = cli.log_file {
        init_with_file(&log_config, format, Some(&log_file));
    } else {
        init_with_file(&log_config, format, None::<&str>);
    }

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
        tracing::info!(target: "kaubo::cli", "Kaubo VM - Bytecode Execution");
        tracing::info!(target: "kaubo::cli", "======================");
        tracing::info!(target: "kaubo::cli", "File: {}", cli.file);
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
        tracing::info!(target: "kaubo::cli", "[Compilation]");
    }

    match compile(source) {
        Ok(output) => {
            if show_steps {
                tracing::info!(target: "kaubo::cli", "Constants: {}", output.chunk.constants.len());
                tracing::info!(target: "kaubo::cli", "Bytecode: {} bytes", output.chunk.code.len());
                tracing::info!(target: "kaubo::cli", "Locals: {}", output.local_count);
            }

            if dump {
                if show_steps {
                    tracing::info!(target: "kaubo::cli", "[Bytecode Disassembly]");
                }
                output.chunk.disassemble("main");
            }

            if show_steps {
                tracing::info!(target: "kaubo::cli", "✅ Compilation successful");
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
        tracing::info!(target: "kaubo::cli", "[Execution]");
    }

    match compile_and_run(source) {
        Ok(output) => {
            if show_steps {
                tracing::info!(target: "kaubo::cli", "✅ Execution successful!");
                if let Some(value) = output.value {
                    tracing::info!(target: "kaubo::cli", "Return value: {}", value);
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

fn build_log_config(cli: &Cli) -> LogConfig {
    let global_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    LogConfig {
        global: global_level,
        lexer: cli.log_lexer.map(to_tracing_level),
        parser: cli.log_parser.map(to_tracing_level),
        compiler: cli.log_compiler.map(to_tracing_level),
        vm: cli.log_vm.map(to_tracing_level),
    }
}

fn build_run_config(cli: &Cli) -> RunConfig {
    RunConfig {
        show_steps: cli.show_steps,
        dump_bytecode: cli.dump_bytecode,
        compiler: CompilerConfig::default(),
        limits: LimitConfig::default(),
    }
}

fn to_tracing_level(level: LogLevelArg) -> Level {
    match level {
        LogLevelArg::Off => unreachable!(), // Handled before calling
        LogLevelArg::Error => Level::ERROR,
        LogLevelArg::Warn => Level::WARN,
        LogLevelArg::Info => Level::INFO,
        LogLevelArg::Debug => Level::DEBUG,
        LogLevelArg::Trace => Level::TRACE,
    }
}
