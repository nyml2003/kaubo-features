//! Kaubo CLI - 命令行入口
//!
//! 纯前端，无业务逻辑。只负责参数解析和调用 API。

use clap::Parser;
use std::process;
use tracing::Level;

use next_kaubo::{
    compile, compile_and_run,
    platform::print_error_with_source,
    Config, LogConfig, LogFormat, Value,
    init_config,
    core::logger::init_with_file,
};

#[derive(Parser)]
#[command(
    name = "kaubo",
    about = "Kaubo programming language",
    version = "0.1.0"
)]
struct Cli {
    /// 源文件路径
    file: String,

    /// 日志级别 (-v=info, -vv=debug, -vvv=trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Lexer 日志级别
    #[arg(long, value_enum)]
    log_lexer: Option<LogLevelArg>,

    /// Parser 日志级别
    #[arg(long, value_enum)]
    log_parser: Option<LogLevelArg>,

    /// Compiler 日志级别
    #[arg(long, value_enum)]
    log_compiler: Option<LogLevelArg>,

    /// VM 日志级别
    #[arg(long, value_enum)]
    log_vm: Option<LogLevelArg>,

    /// 仅编译，不执行
    #[arg(long)]
    compile_only: bool,

    /// 打印字节码
    #[arg(long)]
    dump_bytecode: bool,

    /// 日志输出格式
    #[arg(long, value_enum, default_value = "pretty")]
    format: LogFormatArg,

    /// 显示执行步骤
    #[arg(long)]
    show_steps: bool,

    /// 显示源码内容
    #[arg(long)]
    show_source: bool,

    /// 日志输出到文件
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

    // 读取源文件
    let source = match std::fs::read_to_string(&cli.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Cannot read file '{}': {}", cli.file, e);
            process::exit(1);
        }
    };

    // 初始化配置和日志
    let config = build_config(&cli);
    init_config(config);

    // 初始化日志（支持文件输出）
    let format = match cli.format {
        LogFormatArg::Pretty => LogFormat::Pretty,
        LogFormatArg::Compact => LogFormat::Compact,
        LogFormatArg::Json => LogFormat::Json,
    };

    if let Some(log_file) = cli.log_file {
        init_with_file(format, Some(&log_file));
    } else {
        init_with_file(format, None::<&str>);
    }

    // 显示源码
    if cli.show_source {
        println!("[源码]");
        for (i, line) in source.lines().enumerate() {
            println!("{:3} | {}", i + 1, line);
        }
        println!("[执行结果]");
    }

    // 显示步骤信息
    if cli.show_steps {
        tracing::info!(target: "kaubo::cli", "Kaubo VM - 字节码执行");
        tracing::info!(target: "kaubo::cli", "======================");
        tracing::info!(target: "kaubo::cli", "文件: {}", cli.file);
    }

    // 执行
    if cli.compile_only {
        handle_compile_only(&source, cli.dump_bytecode, cli.show_steps);
    } else {
        handle_run(&source, cli.show_steps);
    }
}

fn handle_compile_only(source: &str, dump: bool, show_steps: bool) {
    if show_steps {
        tracing::info!(target: "kaubo::cli", "[编译]");
    }

    match compile(source) {
        Ok(output) => {
            if show_steps {
                tracing::info!(target: "kaubo::cli", "常量池: {} 个", output.chunk.constants.len());
                tracing::info!(target: "kaubo::cli", "字节码: {} bytes", output.chunk.code.len());
                tracing::info!(target: "kaubo::cli", "局部变量: {} 个", output.local_count);
            }

            if dump {
                if show_steps {
                    tracing::info!(target: "kaubo::cli", "[字节码反汇编]");
                }
                output.chunk.disassemble("main");
            }

            if show_steps {
                tracing::info!(target: "kaubo::cli", "✅ 编译成功");
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
        tracing::info!(target: "kaubo::cli", "[执行]");
    }

    match compile_and_run(source) {
        Ok(output) => {
            if show_steps {
                tracing::info!(target: "kaubo::cli", "✅ 执行成功!");
                if let Some(value) = output.value {
                    tracing::info!(target: "kaubo::cli", "返回值: {}", value);
                }
            } else if let Some(value) = output.value {
                // 非步骤模式下只打印返回值（这是程序的实际输出）
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

fn build_config(cli: &Cli) -> Config {
    // 根据 -v 次数确定全局级别
    let global_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    Config {
        log: LogConfig {
            global: global_level,
            lexer: cli.log_lexer.map(to_tracing_level),
            parser: cli.log_parser.map(to_tracing_level),
            compiler: cli.log_compiler.map(to_tracing_level),
            vm: cli.log_vm.map(to_tracing_level),
        },
        ..Default::default()
    }
}

fn to_tracing_level(level: LogLevelArg) -> Level {
    match level {
        LogLevelArg::Off => unreachable!(), // 在调用前处理
        LogLevelArg::Error => Level::ERROR,
        LogLevelArg::Warn => Level::WARN,
        LogLevelArg::Info => Level::INFO,
        LogLevelArg::Debug => Level::DEBUG,
        LogLevelArg::Trace => Level::TRACE,
    }
}
