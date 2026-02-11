//! CLI 格式化输出
//!
//! 提供命令行友好的错误显示和源码上下文打印。

use kaubo_api::KauboError;

/// 打印错误并显示源代码上下文
pub fn print_error_with_source(e: &KauboError, source: &str) {
    eprintln!("❌ {}", e);

    // 获取错误位置
    let line_num = e.line();
    let column = e.column();

    if let (Some(error_line), Some(col)) = (line_num, column) {
        print_source_context(source, error_line, col);
    }
}

/// 打印源代码上下文（显示错误行前后几行）
pub fn print_source_context(source: &str, error_line: usize, error_col: usize) {
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
