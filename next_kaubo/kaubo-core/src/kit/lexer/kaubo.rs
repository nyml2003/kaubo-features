//! Kaubo 语言 Scanner 实现
//!
//! 完整的 Kaubo 词法分析器，支持：
//! - 关键字、标识符
//! - 运算符（单字符和多字符）
//! - 数字、字符串
//! - 注释

use super::core::{CharStream, SourcePosition, SourceSpan, StreamResult};
use super::scanner::{
    is_identifier_continue, is_identifier_start, LexError, ScanResult, Scanner, Token,
};

// 暂时复用现有的 TokenKind，后续可以独立定义
use crate::compiler::lexer::token_kind::KauboTokenKind;

use tracing::{trace, debug};

/// Kaubo 扫描器
pub struct KauboScanner {
    mode: KauboMode,
    /// 当前 token 的起始位置（用于构建 span）
    token_start: SourcePosition,
    /// 关键字查找表（可优化为完美哈希）
    keywords: &'static [( &'static str, KauboTokenKind)],
}

/// 扫描模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KauboMode {
    /// 默认模式
    Default,
    /// 模板字符串模式：`hello ${...}`
    TemplateString,
    /// 插值表达式模式：${...}
    Interpolation,
}

impl Scanner for KauboScanner {
    type TokenKind = KauboTokenKind;
    type Mode = KauboMode;

    fn new() -> Self {
        trace!(target: "kaubo::lexer::scanner", "Creating new KauboScanner");
        Self {
            mode: KauboMode::Default,
            token_start: SourcePosition::start(),
            keywords: &KEYWORD_TABLE,
        }
    }

    fn set_mode(&mut self, mode: Self::Mode) {
        debug!(target: "kaubo::lexer::scanner", ?mode, "Setting scanner mode");
        self.mode = mode;
    }

    fn current_mode(&self) -> Self::Mode {
        self.mode
    }

    fn next_token(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        trace!(target: "kaubo::lexer::scanner", mode = ?self.mode, "Scanning next token");
        
        let result = match self.mode {
            KauboMode::Default => self.scan_default(stream),
            KauboMode::TemplateString => self.scan_template_string(stream),
            KauboMode::Interpolation => self.scan_interpolation(stream),
        };

        trace!(target: "kaubo::lexer::scanner", 
            is_token = matches!(result, ScanResult::Token(_)),
            "Scan result"
        );
        result
    }
}

impl KauboScanner {
    /// 默认模式扫描
    fn scan_default(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        // 跳过空白符和注释
        self.skip_whitespace_and_comments(stream);

        // 记录 token 起始位置
        self.token_start = stream.position();
        trace!(target: "kaubo::lexer::scanner", 
            line = self.token_start.line,
            column = self.token_start.column,
            "Starting token scan"
        );

        // 预读第一个字符
        let c = match stream.try_peek(0) {
            StreamResult::Ok(c) => c,
            StreamResult::Incomplete => return ScanResult::Incomplete,
            StreamResult::Eof => return ScanResult::Eof,
        };

        // 根据首字符分发
        match c {
            // 单字符运算符/分隔符
            '+' => self.make_single_char(stream, KauboTokenKind::Plus),
            '-' => self.scan_minus(stream),  // - 或 ->
            '*' => self.make_single_char(stream, KauboTokenKind::Asterisk),
            '/' => self.scan_slash(stream), // 可能是注释开始
            '%' => self.make_single_char(stream, KauboTokenKind::Percent),
            '(' => self.make_single_char(stream, KauboTokenKind::LeftParenthesis),
            ')' => self.make_single_char(stream, KauboTokenKind::RightParenthesis),
            '{' => self.make_single_char(stream, KauboTokenKind::LeftCurlyBrace),
            '}' => self.make_single_char(stream, KauboTokenKind::RightCurlyBrace),
            '[' => self.make_single_char(stream, KauboTokenKind::LeftSquareBracket),
            ']' => self.make_single_char(stream, KauboTokenKind::RightSquareBracket),
            ';' => self.make_single_char(stream, KauboTokenKind::Semicolon),
            ',' => self.make_single_char(stream, KauboTokenKind::Comma),
            '.' => self.make_single_char(stream, KauboTokenKind::Dot),
            '|' => self.make_single_char(stream, KauboTokenKind::Pipe),
            ':' => self.make_single_char(stream, KauboTokenKind::Colon),

            // 多字符运算符起始
            '=' => self.scan_eq(stream),
            '!' => self.scan_bang(stream),
            '<' => self.scan_lt(stream),
            '>' => self.scan_gt(stream),

            // 字符串
            '"' | '\'' => self.scan_string(stream, c),

            // 数字
            '0'..='9' => self.scan_number(stream),

            // 标识符/关键字
            c if is_identifier_start(c) => self.scan_identifier_or_keyword(stream),

            // 非法字符
            _ => {
                let _ = stream.try_advance();
                ScanResult::Error(LexError {
                    kind: super::scanner::ErrorKind::InvalidChar(c),
                    position: self.token_start,
                    message: format!("Unexpected character '{}'", c),
                })
            }
        }
    }

    /// 模板字符串模式（预留）
    fn scan_template_string(&mut self, _stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        // TODO: 实现模板字符串扫描
        unimplemented!("Template string mode not yet implemented")
    }

    /// 插值表达式模式（预留）
    fn scan_interpolation(&mut self, _stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        // TODO: 实现插值表达式扫描
        unimplemented!("Interpolation mode not yet implemented")
    }

    /// 创建单字符 token
    fn make_single_char(
        &mut self,
        stream: &mut CharStream,
        kind: KauboTokenKind,
    ) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance();
        let end = stream.position();
        ScanResult::Token(Token::new(kind, SourceSpan::range(self.token_start, end)))
    }

    /// 扫描 '=' 系列（=, ==）
    fn scan_eq(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费 '='
        
        if stream.check('=') {
            let _ = stream.try_advance(); // 消费第二个 '='
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::DoubleEqual,
                SourceSpan::range(self.token_start, end),
            ))
        } else {
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::Equal,
                SourceSpan::range(self.token_start, end),
            ))
        }
    }

    /// 扫描 '!' 系列（!=）
    fn scan_bang(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费 '!'
        
        if stream.check('=') {
            let _ = stream.try_advance();
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::ExclamationEqual,
                SourceSpan::range(self.token_start, end),
            ))
        } else {
            // '!' 单独不是 Kaubo 的运算符，报错
            ScanResult::Error(LexError {
                kind: super::scanner::ErrorKind::InvalidChar('!'),
                position: self.token_start,
                message: "Unexpected character '!', did you mean '!='?".to_string(),
            })
        }
    }

    /// 扫描 '<' 系列（<, <=）
    fn scan_lt(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费 '<'
        
        if stream.check('=') {
            let _ = stream.try_advance();
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::LessThanEqual,
                SourceSpan::range(self.token_start, end),
            ))
        } else {
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::LessThan,
                SourceSpan::range(self.token_start, end),
            ))
        }
    }

    /// 扫描 '>' 系列（>, >=）
    fn scan_gt(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费 '>'
        
        if stream.check('=') {
            let _ = stream.try_advance();
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::GreaterThanEqual,
                SourceSpan::range(self.token_start, end),
            ))
        } else {
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::GreaterThan,
                SourceSpan::range(self.token_start, end),
            ))
        }
    }

    /// 扫描 '-' 系列（-, ->）
    fn scan_minus(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费 '-'
        
        if stream.check('>') {
            let _ = stream.try_advance();
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::FatArrow,
                SourceSpan::range(self.token_start, end),
            ))
        } else {
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::Minus,
                SourceSpan::range(self.token_start, end),
            ))
        }
    }

    /// 扫描 '/'（除法或注释）
    fn scan_slash(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费 '/'
        
        // 检查是否是注释
        if stream.check('/') {
            // 单行注释
            self.skip_line_comment(stream);
            // 注释后递归继续扫描
            self.next_token(stream)
        } else if stream.check('*') {
            // 多行注释
            self.skip_block_comment(stream);
            // 注释后递归继续扫描
            self.next_token(stream)
        } else {
            let end = stream.position();
            ScanResult::Token(Token::new(
                KauboTokenKind::Slash,
                SourceSpan::range(self.token_start, end),
            ))
        }
    }

    /// 跳过空白符和注释
    fn skip_whitespace_and_comments(&mut self, stream: &mut CharStream) {
        loop {
            match stream.try_peek(0) {
                StreamResult::Ok(c) if c.is_whitespace() => {
                    let _ = stream.try_advance();
                }
                StreamResult::Ok('/') => {
                    // 预读下一个字符判断是否是注释
                    match stream.try_peek(1) {
                        StreamResult::Ok('/') => {
                            self.skip_line_comment(stream);
                        }
                        StreamResult::Ok('*') => {
                            self.skip_block_comment(stream);
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }
    }

    /// 跳过单行注释
    fn skip_line_comment(&mut self, stream: &mut CharStream) {
        // 消费 '//'
        let _ = stream.try_advance(); // '/'
        let _ = stream.try_advance(); // '/'
        
        // 跳过到行尾
        while let StreamResult::Ok(c) = stream.try_peek(0) {
            if c == '\n' {
                break;
            }
            let _ = stream.try_advance();
        }
    }

    /// 跳过多行注释
    fn skip_block_comment(&mut self, stream: &mut CharStream) {
        // 消费 '/*'
        let _ = stream.try_advance(); // '/'
        let _ = stream.try_advance(); // '*'
        
        // 跳过到 '*/'
        while let StreamResult::Ok(c) = stream.try_peek(0) {
            if c == '*' {
                match stream.try_peek(1) {
                    StreamResult::Ok('/') => {
                        let _ = stream.try_advance(); // '*'
                        let _ = stream.try_advance(); // '/'
                        break;
                    }
                    _ => {
                        let _ = stream.try_advance();
                    }
                }
            } else {
                let _ = stream.try_advance();
            }
        }
    }

    /// 扫描字符串
    fn scan_string(
        &mut self,
        stream: &mut CharStream,
        quote: char,
    ) -> ScanResult<Token<KauboTokenKind>> {
        let _ = stream.try_advance(); // 消费开头的引号
        let mut value = String::new();

        loop {
            match stream.try_peek(0) {
                StreamResult::Ok(c) if c == quote => {
                    let _ = stream.try_advance(); // 消费结尾引号
                    let end = stream.position();
                    return ScanResult::Token(Token::with_text(
                        KauboTokenKind::LiteralString,
                        SourceSpan::range(self.token_start, end),
                        value,
                    ));
                }
                StreamResult::Ok('\\') => {
                    // 处理转义序列
                    let _ = stream.try_advance(); // 消费 '\'
                    match stream.try_peek(0) {
                        StreamResult::Ok(c) => {
                            let escaped = self.parse_escape(c);
                            value.push(escaped);
                            let _ = stream.try_advance();
                        }
                        StreamResult::Incomplete => return ScanResult::Incomplete,
                        StreamResult::Eof => {
                            return ScanResult::Error(LexError {
                                kind: super::scanner::ErrorKind::UnterminatedString,
                                position: self.token_start,
                                message: "Unterminated string literal".to_string(),
                            })
                        }
                    }
                }
                StreamResult::Ok(c) => {
                    value.push(c);
                    let _ = stream.try_advance();
                }
                StreamResult::Incomplete => return ScanResult::Incomplete,
                StreamResult::Eof => {
                    return ScanResult::Error(LexError {
                        kind: super::scanner::ErrorKind::UnterminatedString,
                        position: self.token_start,
                        message: "Unterminated string literal".to_string(),
                    })
                }
            }
        }
    }

    /// 解析转义字符
    fn parse_escape(&self, c: char) -> char {
        match c {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            _ => c, // 未知转义保留原样
        }
    }

    /// 扫描数字（整数或浮点数）
    fn scan_number(&mut self, stream: &mut CharStream) -> ScanResult<Token<KauboTokenKind>> {
        let mut value = String::new();
        let mut is_float = false;

        // 整数部分
        while let StreamResult::Ok(c) = stream.try_peek(0) {
            if c.is_ascii_digit() {
                value.push(c);
                let _ = stream.try_advance();
            } else {
                break;
            }
        }

        // 小数部分
        if let StreamResult::Ok('.') = stream.try_peek(0) {
            if let StreamResult::Ok(c) = stream.try_peek(1) {
                if c.is_ascii_digit() {
                    // 消费小数点
                    let _ = stream.try_advance();
                    value.push('.');
                    is_float = true;

                    // 小数部分数字
                    while let StreamResult::Ok(c) = stream.try_peek(0) {
                        if c.is_ascii_digit() {
                            value.push(c);
                            let _ = stream.try_advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        let end = stream.position();
        let kind = if is_float {
            KauboTokenKind::LiteralFloat
        } else {
            KauboTokenKind::LiteralInteger
        };
        ScanResult::Token(Token::with_text(
            kind,
            SourceSpan::range(self.token_start, end),
            value,
        ))
    }

    /// 扫描标识符或关键字
    fn scan_identifier_or_keyword(
        &mut self,
        stream: &mut CharStream,
    ) -> ScanResult<Token<KauboTokenKind>> {
        let mut value = String::new();

        // 首字符
        if let StreamResult::Ok(c) = stream.try_peek(0) {
            value.push(c);
            let _ = stream.try_advance();
        }

        // 后续字符
        while let StreamResult::Ok(c) = stream.try_peek(0) {
            if is_identifier_continue(c) {
                value.push(c);
                let _ = stream.try_advance();
            } else {
                break;
            }
        }

        // 查找关键字
        let kind = self.lookup_keyword(&value);
        let end = stream.position();
        
        ScanResult::Token(Token::with_text(
            kind,
            SourceSpan::range(self.token_start, end),
            value,
        ))
    }

    /// 查找关键字
    fn lookup_keyword(&self, word: &str) -> KauboTokenKind {
        for (kw, kind) in self.keywords {
            if *kw == word {
                debug!(target: "kaubo::lexer::scanner", keyword = word, "Matched keyword");
                return kind.clone();
            }
        }
        KauboTokenKind::Identifier
    }
}

/// 关键字表
static KEYWORD_TABLE: &[(&str, KauboTokenKind)] = &[
    ("var", KauboTokenKind::Var),
    ("if", KauboTokenKind::If),
    ("else", KauboTokenKind::Else),
    ("elif", KauboTokenKind::Elif),
    ("while", KauboTokenKind::While),
    ("for", KauboTokenKind::For),
    ("return", KauboTokenKind::Return),
    ("in", KauboTokenKind::In),
    ("yield", KauboTokenKind::Yield),
    ("true", KauboTokenKind::True),
    ("false", KauboTokenKind::False),
    ("null", KauboTokenKind::Null),
    ("break", KauboTokenKind::Break),
    ("continue", KauboTokenKind::Continue),
    ("struct", KauboTokenKind::Struct),
    ("interface", KauboTokenKind::Interface),
    ("import", KauboTokenKind::Import),
    ("as", KauboTokenKind::As),
    ("from", KauboTokenKind::From),
    ("pass", KauboTokenKind::Pass),
    ("and", KauboTokenKind::And),
    ("or", KauboTokenKind::Or),
    ("not", KauboTokenKind::Not),
    ("async", KauboTokenKind::Async),
    ("await", KauboTokenKind::Await),
    ("module", KauboTokenKind::Module),
    ("pub", KauboTokenKind::Pub),
    ("json", KauboTokenKind::Json),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn create_stream(input: &str) -> CharStream {
        let mut stream = CharStream::new(1024);
        stream.feed(input.as_bytes()).unwrap();
        stream.close().unwrap();
        stream
    }

    fn collect_tokens(input: &str) -> Vec<Token<KauboTokenKind>> {
        let mut stream = create_stream(input);
        let mut scanner = KauboScanner::new();
        let mut tokens = Vec::new();

        loop {
            match scanner.next_token(&mut stream) {
                ScanResult::Token(t) => tokens.push(t),
                ScanResult::Eof => break,
                ScanResult::Error(e) => {
                    panic!("Lex error: {:?}", e);
                }
                ScanResult::Incomplete => {
                    panic!("Unexpected incomplete");
                }
            }
        }

        tokens
    }

    #[test]
    fn test_single_char_operators() {
        let tokens = collect_tokens("+-*/{}[];,.|");
        assert_eq!(tokens.len(), 12);
        assert_eq!(tokens[0].kind, KauboTokenKind::Plus);
        assert_eq!(tokens[1].kind, KauboTokenKind::Minus);
        assert_eq!(tokens[2].kind, KauboTokenKind::Asterisk);
        assert_eq!(tokens[3].kind, KauboTokenKind::Slash);
        assert_eq!(tokens[4].kind, KauboTokenKind::LeftCurlyBrace);
        assert_eq!(tokens[5].kind, KauboTokenKind::RightCurlyBrace);
    }

    #[test]
    fn test_double_char_operators() {
        let tokens = collect_tokens("== != <= >= = < >");
        assert_eq!(tokens.len(), 7); // == != <= >= = < > (7个token)
        assert_eq!(tokens[0].kind, KauboTokenKind::DoubleEqual);
        assert_eq!(tokens[1].kind, KauboTokenKind::ExclamationEqual);
        assert_eq!(tokens[2].kind, KauboTokenKind::LessThanEqual);
        assert_eq!(tokens[3].kind, KauboTokenKind::GreaterThanEqual);
        assert_eq!(tokens[4].kind, KauboTokenKind::Equal);
        assert_eq!(tokens[5].kind, KauboTokenKind::LessThan);
        assert_eq!(tokens[6].kind, KauboTokenKind::GreaterThan);
    }

    #[test]
    fn test_keywords() {
        let tokens = collect_tokens("var if else while for return true false null");
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
        assert_eq!(tokens[1].kind, KauboTokenKind::If);
        assert_eq!(tokens[2].kind, KauboTokenKind::Else);
        assert_eq!(tokens[3].kind, KauboTokenKind::While);
        assert_eq!(tokens[4].kind, KauboTokenKind::For);
        assert_eq!(tokens[5].kind, KauboTokenKind::Return);
        assert_eq!(tokens[6].kind, KauboTokenKind::True);
        assert_eq!(tokens[7].kind, KauboTokenKind::False);
        assert_eq!(tokens[8].kind, KauboTokenKind::Null);
    }

    #[test]
    fn test_identifier() {
        let tokens = collect_tokens("my_var _private test123");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, KauboTokenKind::Identifier);
        assert_eq!(tokens[0].text, Some("my_var".to_string()));
    }

    #[test]
    fn test_numbers() {
        let tokens = collect_tokens("0 123 99999");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, KauboTokenKind::LiteralInteger);
        assert_eq!(tokens[0].text, Some("0".to_string()));
        assert_eq!(tokens[1].text, Some("123".to_string()));
    }

    #[test]
    fn test_float_numbers() {
        let tokens = collect_tokens("3.14 0.5 10.0");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, KauboTokenKind::LiteralFloat);
        assert_eq!(tokens[0].text, Some("3.14".to_string()));
        assert_eq!(tokens[1].kind, KauboTokenKind::LiteralFloat);
        assert_eq!(tokens[1].text, Some("0.5".to_string()));
        assert_eq!(tokens[2].kind, KauboTokenKind::LiteralFloat);
        assert_eq!(tokens[2].text, Some("10.0".to_string()));
    }

    #[test]
    fn test_float_vs_int() {
        // 3.14 是浮点数
        let tokens = collect_tokens("3.14");
        assert_eq!(tokens[0].kind, KauboTokenKind::LiteralFloat);
        
        // 3. 是整数 3 后跟点号（成员访问）
        let tokens = collect_tokens("3.;");
        assert_eq!(tokens[0].kind, KauboTokenKind::LiteralInteger);
        assert_eq!(tokens[1].kind, KauboTokenKind::Dot);
    }

    #[test]
    fn test_string_double_quote() {
        let tokens = collect_tokens(r#""hello world""#);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, KauboTokenKind::LiteralString);
        assert_eq!(tokens[0].text, Some("hello world".to_string()));
    }

    #[test]
    fn test_string_single_quote() {
        let tokens = collect_tokens("'hello world'");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, KauboTokenKind::LiteralString);
    }

    #[test]
    fn test_string_escape() {
        let tokens = collect_tokens(r#""hello\nworld""#);
        assert_eq!(tokens[0].text, Some("hello\nworld".to_string()));
    }

    #[test]
    fn test_line_comment() {
        let tokens = collect_tokens("var x; // this is a comment\nvar y;");
        assert_eq!(tokens.len(), 6); // var x ; var y ;
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
        assert_eq!(tokens[3].kind, KauboTokenKind::Var);
    }

    #[test]
    fn test_block_comment() {
        let tokens = collect_tokens("var /* comment */ x;");
        assert_eq!(tokens.len(), 3); // var x ;
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
        assert_eq!(tokens[1].text, Some("x".to_string()));
    }

    #[test]
    fn test_whitespace_skipping() {
        let tokens = collect_tokens("  var   x  =  1  ;  ");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
    }

    #[test]
    fn test_complete_statement() {
        let code = r#"var x = "hello" + 123;"#;
        let tokens = collect_tokens(code);
        
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
        assert_eq!(tokens[1].kind, KauboTokenKind::Identifier);
        assert_eq!(tokens[2].kind, KauboTokenKind::Equal);
        assert_eq!(tokens[3].kind, KauboTokenKind::LiteralString);
        assert_eq!(tokens[4].kind, KauboTokenKind::Plus);
        assert_eq!(tokens[5].kind, KauboTokenKind::LiteralInteger);
        assert_eq!(tokens[6].kind, KauboTokenKind::Semicolon);
    }

    #[test]
    fn test_json_keyword() {
        let tokens = collect_tokens("json { }");
        assert_eq!(tokens[0].kind, KauboTokenKind::Json);
        assert_eq!(tokens[1].kind, KauboTokenKind::LeftCurlyBrace);
    }

    #[test]
    fn test_fat_arrow() {
        let tokens = collect_tokens("|x| -> int");
        assert_eq!(tokens[0].kind, KauboTokenKind::Pipe);
        assert_eq!(tokens[1].kind, KauboTokenKind::Identifier);
        assert_eq!(tokens[2].kind, KauboTokenKind::Pipe);
        assert_eq!(tokens[3].kind, KauboTokenKind::FatArrow);
        assert_eq!(tokens[4].kind, KauboTokenKind::Identifier); // int
    }

    #[test]
    fn test_minus_vs_fat_arrow() {
        // 测试 - 和 -> 的区别
        let tokens = collect_tokens("x - y");
        assert_eq!(tokens[1].kind, KauboTokenKind::Minus);
        
        let tokens2 = collect_tokens("x -> y");
        assert_eq!(tokens2[1].kind, KauboTokenKind::FatArrow);
    }

    #[test]
    fn test_position_tracking() {
        let mut stream = create_stream("var x;\nvar y;");
        let mut scanner = KauboScanner::new();

        // var
        if let ScanResult::Token(t) = scanner.next_token(&mut stream) {
            assert_eq!(t.start().line, 1);
            assert_eq!(t.start().column, 1);
        }

        // x
        if let ScanResult::Token(t) = scanner.next_token(&mut stream) {
            assert_eq!(t.start().line, 1);
            assert_eq!(t.start().column, 5);
        }

        // ;
        if let ScanResult::Token(t) = scanner.next_token(&mut stream) {
            assert_eq!(t.start().line, 1);
            assert_eq!(t.start().column, 6);
        }

        // var (第二行)
        if let ScanResult::Token(t) = scanner.next_token(&mut stream) {
            assert_eq!(t.start().line, 2);
            assert_eq!(t.start().column, 1);
        }
    }
}
