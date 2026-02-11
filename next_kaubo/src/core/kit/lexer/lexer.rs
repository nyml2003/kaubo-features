//! Lexer V2 主入口
//!
//! 使用新的 KauboScanner，直接返回 V2 Token

use crate::core::kit::lexer::{
    CharStream, KauboScanner, KauboMode, Scanner, ScanResult,
};
use crate::core::kit::lexer::scanner::Token;

use tracing::{debug, trace, warn};

// 复用现有的 TokenKind
use crate::core::compiler::lexer::token_kind::KauboTokenKind;

/// 新的 Lexer 实现
pub struct Lexer {
    scanner: KauboScanner,
    stream: CharStream,
    eof: bool,
}

impl Lexer {
    /// 创建新的 Lexer
    pub fn new(capacity: usize) -> Self {
        trace!(target: "kaubo::lexer", "Creating new Lexer with capacity {}", capacity);
        Self {
            scanner: KauboScanner::new(),
            stream: CharStream::new(capacity),
            eof: false,
        }
    }

    /// 向 Lexer 输入数据
    pub fn feed(&mut self, data: &[u8]) -> Result<(), String> {
        trace!(target: "kaubo::lexer", "Feeding {} bytes", data.len());
        self.stream.feed(data).map_err(|e| e.to_string())
    }

    /// 标记输入结束
    pub fn terminate(&mut self) -> Result<(), String> {
        trace!(target: "kaubo::lexer", "Terminating input");
        self.eof = true;
        self.stream.close().map_err(|e| e.to_string())
    }

    /// 获取下一个 Token
    pub fn next_token(&mut self) -> Option<Token<KauboTokenKind>> {
        trace!(target: "kaubo::lexer", "Requesting next token");

        loop {
            match self.scanner.next_token(&mut self.stream) {
                ScanResult::Token(token) => {
                    debug!(target: "kaubo::lexer", 
                        kind = ?token.kind, 
                        text = ?token.text,
                        line = token.span.start.line,
                        column = token.span.start.column,
                        "Produced token"
                    );
                    return Some(token);
                }
                ScanResult::Incomplete => {
                    if self.eof {
                        trace!(target: "kaubo::lexer", "Incomplete at EOF, returning None");
                        return None;
                    }
                    trace!(target: "kaubo::lexer", "Incomplete, need more input");
                    return None;
                }
                ScanResult::Eof => {
                    trace!(target: "kaubo::lexer", "Reached EOF");
                    return None;
                }
                ScanResult::Error(e) => {
                    warn!(target: "kaubo::lexer", error = ?e, "Lex error encountered");
                    // 错误恢复：尝试继续
                    self.scanner.recover_error(&mut self.stream);
                    continue;
                }
            }
        }
    }

    /// 设置扫描模式（用于模板字符串等）
    pub fn set_mode(&mut self, mode: KauboMode) {
        debug!(target: "kaubo::lexer", ?mode, "Switching lexer mode");
        self.scanner.set_mode(mode);
    }
}

impl Default for Lexer {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_all(input: &str) -> Vec<Token<KauboTokenKind>> {
        let mut lexer = Lexer::new(1024);
        lexer.feed(input.as_bytes()).unwrap();
        lexer.terminate().unwrap();

        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_basic_tokens() {
        let tokens = lex_all("var x = 1;");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
        assert_eq!(tokens[1].kind, KauboTokenKind::Identifier);
        assert_eq!(tokens[2].kind, KauboTokenKind::Equal);
        assert_eq!(tokens[3].kind, KauboTokenKind::LiteralInteger);
        assert_eq!(tokens[4].kind, KauboTokenKind::Semicolon);
    }

    #[test]
    fn test_position_tracking() {
        let tokens = lex_all("var x;\nvar y;");
        assert_eq!(tokens[0].span.start.line, 1);
        assert_eq!(tokens[3].span.start.line, 2);
    }
}
