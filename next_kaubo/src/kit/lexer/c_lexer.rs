use std::sync::Arc;

use super::error::LexerError;
use super::state_machine::machine::Machine;
use super::state_machine::manager::Manager;
use super::types::{Coordinate, EatStatus, Token};
use crate::kit::lexer::types::CLexerTokenKindTrait;
use crate::kit::ring_buffer::ring_buffer::RingBuffer;

pub struct Lexer<TokenKind>
where
    TokenKind: CLexerTokenKindTrait,
{
    /// 当前Token的起始坐标
    current_token_start: Coordinate,
    /// 光标当前坐标
    cursor_coordinate: Coordinate,
    /// 线程安全环形缓冲区（存储u8，后续转换为char）
    ring_buffer: Arc<RingBuffer>,
    /// 当前Token的字符数
    current_token_char_count: usize,
    /// 当前Token的字节数（用于从缓冲区读取）
    current_token_byte_count: usize,
    /// 状态机管理器
    manager: Manager<TokenKind>,
    /// 是否已标记为EOF
    eof: bool,
    /// 临时存储char，用于构建Token
    buffer: Vec<char>,
}

impl<TokenKind> Lexer<TokenKind>
where
    TokenKind: CLexerTokenKindTrait,
{
    /// 创建新的BaseLexer
    pub fn new(capacity: usize) -> Self {
        Self {
            current_token_start: Coordinate::default(),
            cursor_coordinate: Coordinate::default(),
            ring_buffer: RingBuffer::new(capacity),
            current_token_char_count: 0,
            current_token_byte_count: 0,
            manager: Manager::new(),
            eof: false,
            buffer: Vec::new(),
        }
    }

    /// 注册状态机
    pub fn register_machine(&mut self, machine: Machine<TokenKind>) {
        self.manager.add_machine(machine);
    }

    /// 向缓冲区写入字节流
    pub fn feed(&mut self, data: &Vec<u8>) -> Result<(), LexerError> {
        if data.is_empty() {
            return Ok(());
        }
        if self.eof {
            return Err(LexerError::EofAfterFeed);
        }

        // 将字符串转换为字节并推入缓冲区
        for &byte in data {
            self.ring_buffer.push(byte)?;
        }
        Ok(())
    }

    /// 标记输入结束
    pub fn terminate(&mut self) -> Result<(), LexerError> {
        self.feed(&vec![b'\n'])?;
        self.ring_buffer.close()?;
        self.eof = true;
        Ok(())
    }

    /// 获取下一个Token
    pub fn next_token(&mut self) -> Option<Token<TokenKind>> {
        // 检查缓冲区状态：空且已关闭 → 结算最后一个Token
        if self.ring_buffer.is_empty().unwrap() && self.eof {
            return self.finalize_last_token();
        }

        // 循环处理字符，直到生成Token或需要等待输入
        loop {
            match self.eat().unwrap() {
                EatStatus::Continue => continue,
                EatStatus::Stop => return self.build_token(),
                EatStatus::Eof => return self.finalize_last_token(),
                EatStatus::Wait => return None,
            }
        }
    }

    // ------------------------------
    // 内部辅助方法
    // ------------------------------
    fn utf8_char_len_from_lead(&mut self, lead_byte: u8) -> Option<usize> {
        match lead_byte {
            // 1字节：首字节最高位为0
            0x00..=0x7F => Some(1),
            // 2字节：首字节最高两位为11，第三位为0
            0xC0..=0xDF => Some(2),
            // 3字节：首字节最高三位为111，第四位为0
            0xE0..=0xEF => Some(3),
            // 4字节：首字节最高四位为1111，第五位为0
            0xF0..=0xF7 => Some(4),
            // 无效首字节（如续字节0x80~0xBF，或超出范围的0xF8~0xFF）
            _ => None,
        }
    }
    /// 读取一个char并驱动状态机
    fn eat(&mut self) -> Result<EatStatus, LexerError> {
        // 尝试获取当前位置的引导字节
        let leading_byte = match self.ring_buffer.try_peek_k(self.current_token_byte_count) {
            Some(Ok(byte)) => byte,
            Some(Err(_)) | None => {
                // 无法获取引导字节，返回Wait
                return Ok(EatStatus::Wait);
            }
        };

        // 获取UTF-8编码长度
        let code_point_len = match self.utf8_char_len_from_lead(leading_byte) {
            Some(len) => len,
            None => {
                // 非法UTF-8字节，返回Stop
                return Ok(EatStatus::Stop);
            }
        };

        // 检查缓冲区是否有足够的字节
        let required_length = self.current_token_byte_count + code_point_len;
        let buffer_size = self.ring_buffer.get_size()?;
        if required_length > buffer_size {
            // 缓冲区不足，检查是否已到EOF
            return Ok(if self.eof {
                EatStatus::Eof
            } else {
                EatStatus::Wait
            });
        }

        // 读取完整的UTF-8字节序列
        let mut bytes = Vec::with_capacity(code_point_len);
        for i in 0..code_point_len {
            let byte = match self
                .ring_buffer
                .try_peek_k(self.current_token_byte_count + i)
            {
                Some(Ok(byte)) => byte,
                Some(Err(_)) | None => {
                    // 缓冲区不足，返回Wait
                    return Ok(EatStatus::Wait);
                }
            };
            bytes.push(byte);
        }

        // 验证UTF-8有效性并获取字符
        let c = std::str::from_utf8(&bytes)
            .map_err(|_| LexerError::Utf8Error)?
            .chars()
            .next()
            .ok_or(LexerError::Utf8Error)?;

        if !self.manager.process_event(c) {
            // 处理失败，停止当前Token
            return Ok(EatStatus::Stop);
        }

        // 所有字节处理成功，更新计数
        self.current_token_char_count += 1;
        self.current_token_byte_count += code_point_len;
        self.buffer.push(c);

        Ok(EatStatus::Continue)
    }

    /// 从缓冲区弹出指定字节数的数据
    fn pop_token_bytes(&mut self, len: usize) -> Result<Vec<u8>, LexerError> {
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push(self.ring_buffer.pop()?);
        }
        Ok(bytes)
    }

    /// 重置当前Token的状态
    fn reset_token_state(&mut self) {
        self.current_token_char_count = 0;
        self.current_token_byte_count = 0;
        self.buffer.clear();
        self.manager.reset();
        self.current_token_start = self.cursor_coordinate;
    }

    /// 更新光标位置
    fn update_cursor(&mut self) {
        // 处理换行符
        let newline_count = self.buffer.iter().rev().filter(|&&c| c == '\n').count();
        if newline_count > 0 {
            self.cursor_coordinate.line += newline_count;
            // 找到最后一个换行符后的字符位置
            let last_newline_pos = self
                .buffer
                .iter()
                .rposition(|&c| c == '\n')
                .unwrap_or(self.buffer.len());
            self.cursor_coordinate.column = self.buffer.len() - last_newline_pos;
        } else {
            // 普通字符，列号增加
            self.cursor_coordinate.column += self.buffer.len();
        }
    }

    /// EOF时结算最后一个Token
    fn finalize_last_token(&mut self) -> Option<Token<TokenKind>> {
        None
        // if self.current_token_char_count == 0 {
        //     return None;
        // }

        // // 弹出当前Token的所有字节
        // let token_bytes = match self.pop_token_bytes(self.current_token_byte_count) {
        //     Ok(bytes) => bytes,
        //     Err(_) => return None,
        // };

        // // 转换为字符串
        // let token_str = String::from_utf8_lossy(&token_bytes).to_string();

        // // 获取最佳匹配的状态机
        // let (best_machine_id, _) = self.manager.select_best_match();
        // let token_kind = best_machine_id
        //     .and_then(|id| self.manager.get_machine(id))
        //     .map(|m| m.get_token_type())
        //     .unwrap_or_else(TokenKind::invalid_token);

        // // 忽略空白、换行、制表符
        // if token_kind.is_whitespace() || token_kind.is_newline() || token_kind.is_tab() {
        //     self.update_cursor();
        //     self.reset_token_state();
        //     return None;
        // }

        // // 构建最终Token
        // let token = Token {
        //     kind: token_kind,
        //     value: token_str,
        //     coordinate: self.current_token_start,
        // };

        // self.update_cursor();
        // self.reset_token_state();
        // Some(token)
    }

    /// 正常构建Token
    fn build_token(&mut self) -> Option<Token<TokenKind>> {
        // 弹出当前Token的字节数据
        let token_bytes = match self.pop_token_bytes(self.current_token_byte_count) {
            Ok(bytes) => bytes,
            Err(_) => return None,
        };

        // 转换为字符串
        let token_str = String::from_utf8_lossy(&token_bytes).to_string();

        // 获取最佳匹配状态机
        let (best_machine_id, _) = self.manager.select_best_match();
        let token_kind =
            match best_machine_id.and_then(|id| self.manager.get_machine_token_kind_by_index(id)) {
                Some(token_kind) => token_kind,
                None => {
                    // 无匹配状态机，构建无效Token
                    let token = Token {
                        kind: TokenKind::invalid_token(),
                        value: token_str,
                        coordinate: self.current_token_start,
                    };
                    self.update_cursor();
                    self.reset_token_state();
                    return Some(token);
                }
            };

        // 处理特殊Token类型
        match () {
            _ if token_kind.is_whitespace() => {
                self.update_cursor();
                self.reset_token_state();
                self.next_token()
            }
            _ if token_kind.is_newline() => {
                self.update_cursor();
                self.reset_token_state();
                self.next_token()
            }
            _ if token_kind.is_tab() => {
                // 制表符按4个空格计算列号
                self.cursor_coordinate.column += 4;
                self.reset_token_state();
                self.next_token()
            }
            _ if token_kind.is_comment() => {
                self.update_cursor();
                self.reset_token_state();
                self.next_token()
            }
            _ => {
                // 构建正常Token
                let token = Token {
                    kind: token_kind,
                    value: token_str,
                    coordinate: self.current_token_start,
                };
                self.update_cursor();
                self.reset_token_state();
                Some(token)
            }
        }
    }
}

// 为BaseLexer实现Default
impl<TokenKind> Default for Lexer<TokenKind>
where
    TokenKind: CLexerTokenKindTrait,
{
    fn default() -> Self {
        Self::new(4096) // 默认4KB缓冲区
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kit::lexer::state_machine::builder::{
        build_integer_machine, build_multi_char_machine, build_newline_machine,
        build_single_char_machine, build_whitespace_machine,
    };
    use crate::kit::lexer::state_machine::types::TokenKindTrait;
    use crate::kit::lexer::types::CLexerTokenKindTrait;

    // 测试用Token枚举
    #[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Default)]
    #[repr(u8)]
    enum TestToken {
        #[default]
        InvalidToken = 255,

        Integer = 10,

        DoubleEqual = 100,
        Equal = 200,

        Whitespace = 240,
        Tab = 241,
        NewLine = 242,

        Comment = 1,
    }

    impl From<u8> for TestToken {
        fn from(value: u8) -> Self {
            match value {
                100 => TestToken::DoubleEqual,
                200 => TestToken::Equal,
                240 => TestToken::Whitespace,
                241 => TestToken::Tab,
                242 => TestToken::NewLine,
                1 => TestToken::Comment,
                10 => TestToken::Integer,
                _ => TestToken::InvalidToken,
            }
        }
    }

    impl Into<u8> for TestToken {
        fn into(self) -> u8 {
            match self {
                TestToken::InvalidToken => 255,
                TestToken::DoubleEqual => 100,
                TestToken::Equal => 200,
                TestToken::Whitespace => 240,
                TestToken::Tab => 241,
                TestToken::NewLine => 242,
                TestToken::Integer => 10,
                TestToken::Comment => 1,
            }
        }
    }

    impl TokenKindTrait for TestToken {}

    impl CLexerTokenKindTrait for TestToken {
        fn invalid_token() -> Self {
            TestToken::InvalidToken
        }

        fn whitespace() -> Self {
            TestToken::Whitespace
        }

        fn tab() -> Self {
            TestToken::Tab
        }

        fn newline() -> Self {
            TestToken::NewLine
        }

        fn comment() -> Self {
            TestToken::Comment
        }
    }

    // 测试基础Token匹配
    #[test]
    fn test_base_lexer_basic_match() -> Result<(), LexerError> {
        let mut lexer: Lexer<TestToken> = Lexer::new(1024);

        // 注册状态机
        let equal_machine = build_single_char_machine(TestToken::Equal, '=').unwrap();
        let double_equal_machine =
            build_multi_char_machine(TestToken::DoubleEqual, vec!['=', '=']).unwrap();
        lexer.register_machine(build_integer_machine(TestToken::Integer).unwrap());
        lexer.register_machine(equal_machine);
        lexer.register_machine(double_equal_machine);
        lexer.register_machine(build_newline_machine(TestToken::NewLine).unwrap());
        lexer.register_machine(build_whitespace_machine(TestToken::Whitespace).unwrap());

        // 输入数据并终止
        lexer.feed(&"== = 123".as_bytes().to_vec())?;
        lexer.terminate()?;

        let token1 = lexer.next_token();
        if let Some(token) = token1 {
            assert_eq!(token.kind, TestToken::DoubleEqual);
            assert_eq!(token.value, "==".to_string());
        } else {
            panic!("Expected token1 to be Some");
        }

        let token2 = lexer.next_token(); // 空格被忽略
        if let Some(token) = token2 {
            assert_eq!(token.kind, TestToken::Equal);
            assert_eq!(token.value, "=".to_string());
        } else {
            panic!("Expected token2 to be Some");
        }

        let token3 = lexer.next_token();
        if let Some(token) = token3 {
            assert_eq!(token.kind, TestToken::Integer);
            assert_eq!(token.value, "123".to_string());
        } else {
            panic!("Expected token3 to be Some");
        }
        
        let token4 = lexer.next_token();
        assert!(token4.is_none());

        Ok(())
    }
}
