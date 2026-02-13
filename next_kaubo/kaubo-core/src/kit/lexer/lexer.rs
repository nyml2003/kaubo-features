//! Lexer V2 主入口
//!
//! 使用新的 KauboScanner，直接返回 V2 Token
//!
//! # 示例
//!
//! ```rust,ignore
//! use kaubo_log::{LogConfig, Level};
//! use kaubo_core::kit::lexer::Lexer;
//!
//! let (logger, _) = LogConfig::dev().init();
//! let mut lexer = Lexer::with_logger(4096, logger);
//! ```

use crate::kit::lexer::core::StreamError;
use crate::kit::lexer::scanner::Token;
use crate::kit::lexer::{CharStream, KauboMode, KauboScanner, ScanResult, Scanner};

// 使用 kaubo-log 替代 tracing
use kaubo_log::{debug, trace, warn, Logger};
use std::sync::Arc;

// 复用现有的 TokenKind
use crate::compiler::lexer::token_kind::KauboTokenKind;

/// 新的 Lexer 实现
///
/// 使用显式 logger（遵循 kaubo-log 设计原则：结构化接口优于环境依赖）
pub struct Lexer {
    scanner: KauboScanner,
    stream: CharStream,
    eof: bool,
    logger: Arc<Logger>,
}

impl Lexer {
    /// 创建新的 Lexer（使用 noop logger，向后兼容）
    ///
    /// 如需自定义日志，请使用 [`Self::with_logger`]
    pub fn new(capacity: usize) -> Self {
        Self::with_logger(capacity, Logger::noop())
    }

    /// 创建新的 Lexer（带显式 logger）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// use kaubo_log::LogConfig;
    ///
    /// let (logger, _) = LogConfig::dev().init();
    /// let lexer = Lexer::with_logger(4096, logger);
    /// ```
    pub fn with_logger(capacity: usize, logger: Arc<Logger>) -> Self {
        trace!(logger, "Creating new Lexer with capacity {}", capacity);
        Self {
            scanner: KauboScanner::with_logger(logger.clone()),
            stream: CharStream::new(capacity),
            eof: false,
            logger,
        }
    }

    /// 向 Lexer 输入数据
    pub fn feed(&mut self, data: &[u8]) -> Result<(), StreamError> {
        trace!(self.logger, "Feeding {} bytes", data.len());
        self.stream.feed(data)
    }

    /// 标记输入结束
    pub fn terminate(&mut self) -> Result<(), StreamError> {
        trace!(self.logger, "Terminating input");
        self.eof = true;
        self.stream.close()
    }

    /// 获取下一个 Token
    pub fn next_token(&mut self) -> Option<Token<KauboTokenKind>> {
        trace!(self.logger, "Requesting next token");

        loop {
            match self.scanner.next_token(&mut self.stream) {
                ScanResult::Token(token) => {
                    debug!(
                        self.logger,
                        "Produced token: kind={:?}, text={:?}, line={}, column={}",
                        token.kind,
                        token.text,
                        token.span.start.line,
                        token.span.start.column
                    );
                    return Some(token);
                }
                ScanResult::Incomplete => {
                    if self.eof {
                        trace!(self.logger, "Incomplete at EOF, returning None");
                        return None;
                    }
                    trace!(self.logger, "Incomplete, need more input");
                    return None;
                }
                ScanResult::Eof => {
                    trace!(self.logger, "Reached EOF");
                    return None;
                }
                ScanResult::Error(e) => {
                    warn!(self.logger, "Lex error encountered: {:?}", e);
                    // 错误恢复：尝试继续
                    self.scanner.recover_error(&mut self.stream);
                    continue;
                }
            }
        }
    }

    /// 设置扫描模式（用于模板字符串等）
    pub fn set_mode(&mut self, mode: KauboMode) {
        debug!(self.logger, "Switching lexer mode to {:?}", mode);
        self.scanner.set_mode(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_log::{LogConfig, Level};

    fn lex_all(input: &str) -> Vec<Token<KauboTokenKind>> {
        // 测试时使用静默 logger
        let logger = Logger::noop();
        let mut lexer = Lexer::with_logger(1024, logger);
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

    #[test]
    fn test_lexer_with_logger() {
        use kaubo_log::LogRingBuffer;

        // 使用环形缓冲区捕获日志
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(kaubo_log::Level::Debug).with_sink(ring.clone());

        let mut lexer = Lexer::with_logger(1024, logger);
        lexer.feed(b"var x = 1;").unwrap();
        lexer.terminate().unwrap();

        // 消费一些 token
        let _ = lexer.next_token();
        let _ = lexer.next_token();

        // 验证日志被记录
        let records = ring.dump_records();
        assert!(!records.is_empty(), "Should have log records");
    }

    /// 验证日志内容的具体测试
    /// 
    /// 这个测试验证：
    /// 1. Lexer 创建时记录日志
    /// 2. feed() 时记录日志  
    /// 3. next_token() 时记录日志
    /// 4. Scanner 也正确传递了 logger
    #[test]
    fn test_lexer_logs_content() {
        use kaubo_log::LogRingBuffer;

        // 创建带 ring buffer 的 logger
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(kaubo_log::Level::Trace).with_sink(ring.clone());

        // 创建 lexer（应该记录 "Creating new Lexer"）
        let mut lexer = Lexer::with_logger(1024, logger.clone());
        let records = ring.dump_records();
        assert!(
            records.iter().any(|r| r.message.contains("Creating new Lexer")),
            "Should log Lexer creation"
        );

        // feed 数据（应该记录 "Feeding"）
        ring.clear();
        lexer.feed(b"var x;").unwrap();
        let records = ring.dump_records();
        assert!(
            records.iter().any(|r| r.message.contains("Feeding")),
            "Should log feed operation"
        );

        // terminate（应该记录 "Terminating"）
        ring.clear();
        lexer.terminate().unwrap();
        let records = ring.dump_records();
        assert!(
            records.iter().any(|r| r.message.contains("Terminating")),
            "Should log terminate operation"
        );

        // next_token（应该记录 "Requesting next token" 和 "Produced token"）
        ring.clear();
        let _ = lexer.next_token(); // var
        let records = ring.dump_records();
        assert!(
            records.iter().any(|r| r.message.contains("Requesting next token")),
            "Should log token request"
        );
        assert!(
            records.iter().any(|r| r.message.contains("Produced token")),
            "Should log produced token"
        );

        // 验证 Scanner 也收到了 logger（检查 scanner 的日志）
        ring.clear();
        let _ = lexer.next_token(); // x
        let _ = lexer.next_token(); // ;
        let records = ring.dump_records();
        // Scanner 应该记录 "Scanning next token"
        assert!(
            records.iter().any(|r| r.message.contains("Scanning next token")),
            "Scanner should log scanning operations (logger was passed to KauboScanner)"
        );
    }

    /// 验证日志级别过滤
    #[test]
    fn test_lexer_log_level_filtering() {
        use kaubo_log::LogRingBuffer;

        // 设置 Info 级别（Trace/Debug 被过滤）
        let ring = LogRingBuffer::new(100);
        let logger = Logger::new(kaubo_log::Level::Info).with_sink(ring.clone());

        let mut lexer = Lexer::with_logger(1024, logger);
        lexer.feed(b"var x;").unwrap();
        lexer.terminate().unwrap();
        
        // 消费所有 token
        while lexer.next_token().is_some() {}

        let records = ring.dump_records();
        
        // Info 级别下不应该有 Trace 日志
        assert!(
            !records.iter().any(|r| r.level == kaubo_log::Level::Trace),
            "Trace logs should be filtered at Info level"
        );
        
        // Info 级别下不应该有 Debug 日志
        assert!(
            !records.iter().any(|r| r.level == kaubo_log::Level::Debug),
            "Debug logs should be filtered at Info level"
        );
    }
}
