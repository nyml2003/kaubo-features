//! Kaubo CLI - 命令行入口
//!
//! 使用 clap 进行参数解析，调用 api 模块执行。

use clap::Parser;
use next_kaubo::logger::{LogFormat, init_with_file};
use next_kaubo::{Config, LogConfig, Value, compile, compile_and_run};
use std::process;
use tracing::{Level, info};

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
    next_kaubo::config::init(config);

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
        info!(target: "kaubo::cli", "Kaubo VM - 字节码执行");
        info!(target: "kaubo::cli", "======================");
        info!(target: "kaubo::cli", "文件: {}", cli.file);
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
        info!(target: "kaubo::cli", "[编译]");
    }

    match compile(source) {
        Ok(output) => {
            if show_steps {
                info!(target: "kaubo::cli", "常量池: {} 个", output.chunk.constants.len());
                info!(target: "kaubo::cli", "字节码: {} bytes", output.chunk.code.len());
                info!(target: "kaubo::cli", "局部变量: {} 个", output.local_count);
            }

            if dump {
                if show_steps {
                    info!(target: "kaubo::cli", "[字节码反汇编]");
                }
                output.chunk.disassemble("main");
            }

            if show_steps {
                info!(target: "kaubo::cli", "✅ 编译成功");
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
        info!(target: "kaubo::cli", "[执行]");
    }

    match compile_and_run(source) {
        Ok(output) => {
            if show_steps {
                info!(target: "kaubo::cli", "✅ 执行成功!");
                if let Some(value) = output.value {
                    info!(target: "kaubo::cli", "返回值: {}", value);
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

/// 打印错误并显示源代码上下文
fn print_error_with_source(e: &next_kaubo::KauboError, source: &str) {
    eprintln!("❌ {}", e);

    // 获取错误位置
    let line_num = e.line();
    let column = e.column();

    if let (Some(error_line), Some(col)) = (line_num, column) {
        print_source_context(source, error_line, col);
    }
}

/// 打印源代码上下文（显示错误行前后几行）
fn print_source_context(source: &str, error_line: usize, error_col: usize) {
    const CONTEXT_LINES: usize = 5; // 错误行前后显示的上下文行数

    let lines: Vec<&str> = source.lines().collect();
    let total_lines = lines.len();

    if error_line == 0 || error_line > total_lines {
        return;
    }

    // 计算要显示的行范围
    let start_line = error_line.saturating_sub(CONTEXT_LINES).max(1);
    let end_line = (error_line + CONTEXT_LINES).min(total_lines);

    // 计算行号的最大宽度用于对齐
    let max_line_num_width = end_line.to_string().len();

    // 打印分隔线
    let separator: String = std::iter::repeat('-')
        .take(max_line_num_width + 1)
        .collect();
    eprintln!("{}|--", separator);

    // 打印上下文行
    for line_idx in start_line..=end_line {
        let line_content = lines[line_idx - 1];
        let line_str = line_idx.to_string();
        let padding_len = max_line_num_width.saturating_sub(line_str.len());
        let padding: String = std::iter::repeat(' ').take(padding_len).collect();

        if line_idx == error_line {
            // 错误行：打印行号和源代码
            eprintln!("{}{} | {}", padding, line_str, line_content);

            // 打印指向错误位置的标记
            let marker_offset = error_col.saturating_sub(1);
            let marker: String = std::iter::repeat(' ').take(marker_offset).collect();
            let separator_padding: String =
                std::iter::repeat(' ').take(max_line_num_width).collect();
            eprintln!("{} | {}^", separator_padding, marker);
        } else {
            // 普通上下文行
            eprintln!("{}{} | {}", padding, line_str, line_content);
        }
    }

    // 打印分隔线
    eprintln!("{}|--", separator);
}

fn build_config(cli: &Cli) -> Config {
    // 根据 -v 次数确定全局级别
    let global_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    // 日志格式（当前未使用，保留给未来实现）
    let _format = match cli.format {
        LogFormatArg::Pretty => LogFormat::Pretty,
        LogFormatArg::Compact => LogFormat::Compact,
        LogFormatArg::Json => LogFormat::Json,
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
