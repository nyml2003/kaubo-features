//! 源代码位置追踪
//!
//! 支持多坐标系统，满足不同场景需求：
//! - line/column: 人类可读的错误显示（1-based）
//! - byte_offset: 文件跳转和I/O操作（0-based）
//! - utf16_column: LSP协议通信（0-based，UTF-16单元）

/// 源代码位置
///
/// 所有字段都是按需计算，不增加运行时开销
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourcePosition {
    /// 行号，1-based，用于错误显示
    pub line: usize,
    /// 列号，1-based，Unicode码点计数，用于错误显示
    pub column: usize,
    /// 字节偏移，0-based，UTF-8编码，用于文件seek
    pub byte_offset: usize,
    /// 行内UTF-16偏移，0-based，用于LSP Position.character
    pub utf16_column: usize,
}

impl SourcePosition {
    /// 创建新位置
    pub fn new(line: usize, column: usize, byte_offset: usize, utf16_column: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
            utf16_column,
        }
    }

    /// 文件起始位置
    pub fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            byte_offset: 0,
            utf16_column: 0,
        }
    }

    /// 前进一个字符
    ///
    /// # Arguments
    /// * `c` - 当前字符
    /// * `char_len` - UTF-8字节长度（1-4）
    pub fn advance(&mut self, c: char) {
        let char_len = c.len_utf8();
        let utf16_len = c.len_utf16();

        if c == '\n' {
            self.line += 1;
            self.column = 1;
            self.utf16_column = 0;
        } else {
            self.column += 1;
            self.utf16_column += utf16_len;
        }

        self.byte_offset += char_len;
    }

    /// 前进指定字节数（不更新行列号）
    ///
    /// 用于跳过已知长度的内容
    pub fn advance_bytes(&mut self, bytes: usize) {
        self.byte_offset += bytes;
    }
}

/// 源代码区间（Span）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    /// 从起始位置创建区间（结束位置相同）
    pub fn at(pos: SourcePosition) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// 合并两个位置为区间
    pub fn range(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_start() {
        let pos = SourcePosition::start();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.byte_offset, 0);
        assert_eq!(pos.utf16_column, 0);
    }

    #[test]
    fn test_position_advance_ascii() {
        let mut pos = SourcePosition::start();

        pos.advance('a'); // 1 byte, 1 UTF-16
        assert_eq!(pos.column, 2);
        assert_eq!(pos.byte_offset, 1);
        assert_eq!(pos.utf16_column, 1);

        pos.advance('b');
        assert_eq!(pos.column, 3);
        assert_eq!(pos.byte_offset, 2);
        assert_eq!(pos.utf16_column, 2);
    }

    #[test]
    fn test_position_advance_newline() {
        let mut pos = SourcePosition::start();

        pos.advance('a');
        pos.advance('\n');

        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.utf16_column, 0);
        assert_eq!(pos.byte_offset, 2);
    }

    #[test]
    fn test_position_advance_cjk() {
        let mut pos = SourcePosition::start();

        // CJK字符：3字节UTF-8，1个UTF-16单元
        pos.advance('中');
        assert_eq!(pos.column, 2);
        assert_eq!(pos.byte_offset, 3);
        assert_eq!(pos.utf16_column, 1);
    }

    #[test]
    fn test_position_advance_emoji() {
        let mut pos = SourcePosition::start();

        // Emoji：4字节UTF-8，2个UTF-16单元（代理对）
        pos.advance('🎉');
        assert_eq!(pos.column, 2);
        assert_eq!(pos.byte_offset, 4);
        assert_eq!(pos.utf16_column, 2);
    }
}
