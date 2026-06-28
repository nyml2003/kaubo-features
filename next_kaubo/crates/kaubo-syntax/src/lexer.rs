//! Lexer — Kaubo v2

use crate::token::{Token, TokenKind};
use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let col = self.col;

        match self.bump() {
            None => Token::eof(line, col),
            Some(c) => self.scan_token(c, line, col),
        }
    }

    fn scan_token(&mut self, c: char, line: usize, col: usize) -> Token {
        let tk = |kind| Token::new(kind, c.to_string(), line, col);

        match c {
            // ── 单字符定界符 ──
            '(' => tk(TokenKind::LParen),
            ')' => tk(TokenKind::RParen),
            '{' => tk(TokenKind::LBrace),
            '}' => tk(TokenKind::RBrace),
            '[' => tk(TokenKind::LBracket),
            ']' => tk(TokenKind::RBracket),
            ',' => tk(TokenKind::Comma),
            ';' => tk(TokenKind::Semicolon),
            ':' => tk(TokenKind::Colon),
            '.' => self.scan_dot(line, col),

            // ── 运算符 (单/双字符) ──
            '+' => tk(TokenKind::Plus),
            '*' => tk(TokenKind::Asterisk),
            '%' => tk(TokenKind::Percent),
            '-' => self.scan_minus_arrow(line, col),
            '/' => self.scan_slash(line, col),
            '=' => self.scan_eq(line, col),
            '!' => self.scan_bang(line, col),
            '<' => self.scan_lt(line, col),
            '>' => self.scan_gt(line, col),
            '|' => self.scan_pipe(line, col),
            '?' => self.scan_question(line, col),
            '`' => self.scan_template_string(line, col),

            // ── 字符串 ──
            '"' | '\'' => self.scan_string(c, line, col),

            // ── 数字 ──
            '0'..='9' => self.scan_number(c, line, col),

            // ── 标识符/关键字 ──
            'a'..='z' | 'A'..='Z' | '_' => self.scan_ident(c, line, col),

            _ => Token::new(TokenKind::Error, c.to_string(), line, col),
        }
    }

    // ── 辅助 ──

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn advance(&mut self) {
        let _ = self.bump();
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn scan_dot(&mut self, line: usize, col: usize) -> Token {
        // Peek ahead for ... (DotDotDot)
        let mut peek_iter = self.chars.clone();
        if peek_iter.next() == Some('.') && peek_iter.next() == Some('.') {
            self.advance(); // second dot
            self.advance(); // third dot
            return Token::new(TokenKind::DotDotDot, "...".into(), line, col);
        }
        Token::new(TokenKind::Dot, ".".to_string(), line, col)
    }

    fn scan_minus_arrow(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('>') {
            self.advance();
            Token::new(TokenKind::FatArrow, "->".into(), line, col)
        } else {
            Token::new(TokenKind::Minus, "-".into(), line, col)
        }
    }

    fn scan_slash(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('/') {
            self.advance();
            let comment = self.collect_line_comment();
            return Token::new(TokenKind::Comment, comment, line, col);
        }
        if self.peek() == Some('*') {
            self.advance();
            let comment = self.collect_block_comment();
            return Token::new(TokenKind::Comment, comment, line, col);
        }
        Token::new(TokenKind::Slash, "/".into(), line, col)
    }

    fn collect_line_comment(&mut self) -> String {
        let mut s = String::from("//");
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            s.push(c);
            self.advance();
        }
        s
    }

    fn collect_block_comment(&mut self) -> String {
        let mut s = String::from("/*");
        let mut depth = 1;
        while let Some(c) = self.peek() {
            s.push(c);
            self.advance();
            match c {
                '/' if self.peek() == Some('*') => {
                    s.push('*');
                    self.advance();
                    depth += 1;
                }
                '*' if self.peek() == Some('/') => {
                    s.push('/');
                    self.advance();
                    depth -= 1;
                    if depth == 0 {
                        return s;
                    }
                }
                '\n' => {}
                _ => {}
            }
        }
        s
    }

    fn scan_eq(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('=') {
            self.advance();
            Token::new(TokenKind::EqEq, "==".into(), line, col)
        } else {
            Token::new(TokenKind::Eq, "=".into(), line, col)
        }
    }

    fn scan_bang(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('=') {
            self.advance();
            Token::new(TokenKind::NotEq, "!=".into(), line, col)
        } else {
            Token::new(TokenKind::Error, "!".into(), line, col)
        }
    }

    fn scan_lt(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('=') {
            self.advance();
            Token::new(TokenKind::Le, "<=".into(), line, col)
        } else {
            Token::new(TokenKind::Lt, "<".into(), line, col)
        }
    }

    fn scan_gt(&mut self, line: usize, col: usize) -> Token {
        match self.peek() {
            Some('=') => {
                self.advance();
                Token::new(TokenKind::Ge, ">=".into(), line, col)
            }
            Some('>') => {
                self.advance();
                Token::new(TokenKind::GtGt, ">>".into(), line, col)
            }
            _ => Token::new(TokenKind::Gt, ">".into(), line, col),
        }
    }

    fn scan_template_string(&mut self, line: usize, col: usize) -> Token {
        let mut s = String::from("`");
        while let Some(c) = self.bump() {
            if c == '`' {
                s.push('`');
                return Token::new(TokenKind::TemplateString, s, line, col);
            }
            if c == '\\' {
                if let Some(next) = self.bump() {
                    s.push(match next {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '`' => '`',
                        other => other,
                    });
                }
            } else {
                s.push(c);
            }
        }
        Token::new(
            TokenKind::Error,
            "unterminated template string".into(),
            line,
            col,
        )
    }

    fn scan_question(&mut self, line: usize, col: usize) -> Token {
        match self.peek() {
            Some('?') => {
                self.advance();
                Token::new(TokenKind::QuestionQuestion, "??".into(), line, col)
            }
            Some('.') => {
                self.advance();
                Token::new(TokenKind::QuestionDot, "?.".into(), line, col)
            }
            Some('[') => {
                self.advance();
                Token::new(TokenKind::QuestionLBracket, "?[".into(), line, col)
            }
            _ => Token::new(TokenKind::Error, "?".into(), line, col),
        }
    }

    fn scan_pipe(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('>') {
            self.advance();
            Token::new(TokenKind::Pipe, "|>".into(), line, col)
        } else {
            Token::new(TokenKind::Bar, "|".into(), line, col)
        }
    }

    fn scan_string(&mut self, quote: char, line: usize, col: usize) -> Token {
        let mut s = String::from(quote);
        while let Some(c) = self.bump() {
            if c == quote {
                s.push(quote);
                return Token::new(TokenKind::StringLiteral, s, line, col);
            }
            if c == '\\' {
                match self.bump() {
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some('\'') => s.push('\''),
                    Some(other) => s.push(other),
                    None => break,
                }
            } else {
                s.push(c);
            }
        }
        Token::new(
            TokenKind::Error,
            format!("unterminated string: {s}"),
            line,
            col,
        )
    }

    fn scan_number(&mut self, first: char, line: usize, col: usize) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        // Peek two chars to distinguish `42.0` (float) from `42.as_float()` (int + dot)
        if self.peek() == Some('.') {
            // save position, try to peek after dot
            let mut iter = self.chars.clone();
            iter.next(); // skip the dot
            let after_dot = iter.next();
            if after_dot.is_some_and(|c| c.is_ascii_digit()) {
                // it's a float
                s.push('.');
                self.advance();
                while let Some(&c) = self.chars.peek() {
                    if c.is_ascii_digit() || c == '_' {
                        s.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                return Token::new(TokenKind::FloatLiteral, s, line, col);
            }
            // else: dot is a separate token, return int now
        }
        Token::new(TokenKind::IntLiteral, s, line, col)
    }

    fn scan_ident(&mut self, first: char, line: usize, col: usize) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let kind = TokenKind::from_ident(&s);
        Token::new(kind, s, line, col)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    #![allow(non_snake_case)]
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        let mut lex = Lexer::new(src);
        let toks = lex.tokenize();
        toks.into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.kind)
            .collect()
    }

    fn tokens(src: &str) -> Vec<crate::token::Token> {
        Lexer::new(src)
            .tokenize()
            .into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .collect()
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            kinds("const var if else for in while break continue return"),
            vec![
                TokenKind::Const,
                TokenKind::Var,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::For,
                TokenKind::In,
                TokenKind::While,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::Return
            ]
        );
    }

    #[test]
    fn test_types_and_bool() {
        assert_eq!(
            kinds("true false null"),
            vec![TokenKind::True, TokenKind::False, TokenKind::Null]
        );
    }

    #[test]
    fn test_struct_methods() {
        let ks = kinds("struct impl export import from as async await self");
        assert_eq!(
            ks,
            vec![
                TokenKind::Struct,
                TokenKind::Impl,
                TokenKind::Export,
                TokenKind::Import,
                TokenKind::From,
                TokenKind::As,
                TokenKind::Async_,
                TokenKind::Await,
                TokenKind::Self_,
            ]
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            kinds("+ - * / % = == != < <= > >= not and or"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Asterisk,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Eq,
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::Lt,
                TokenKind::Le,
                TokenKind::Gt,
                TokenKind::Ge,
                TokenKind::Not,
                TokenKind::And,
                TokenKind::Or
            ]
        );
    }

    #[test]
    fn test_compound_operators() {
        assert_eq!(
            kinds("-> |> >>"),
            vec![TokenKind::FatArrow, TokenKind::Pipe, TokenKind::GtGt]
        );
    }

    #[test]
    fn test_delimiters() {
        assert_eq!(
            kinds("( ) { } [ ] , ; : ."),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Semicolon,
                TokenKind::Colon,
                TokenKind::Dot
            ]
        );
    }

    #[test]
    fn test_literals() {
        assert_eq!(
            kinds(r#"42 3.14 "hello" "#),
            vec![
                TokenKind::IntLiteral,
                TokenKind::FloatLiteral,
                TokenKind::StringLiteral
            ]
        );
    }

    #[test]
    fn test_string_escapes() {
        let mut lex = Lexer::new(r#""a\nb\tc\\d\"e" "#);
        let toks = lex.tokenize();
        assert_eq!(toks[0].kind, TokenKind::StringLiteral);
        assert_eq!(toks[0].lexeme, "\"a\nb\tc\\d\"e\"");
    }

    #[test]
    fn test_comments() {
        let ks = kinds("// line comment\n42 /* block */ 0");
        assert_eq!(
            ks,
            vec![
                TokenKind::Comment,
                TokenKind::IntLiteral,
                TokenKind::Comment,
                TokenKind::IntLiteral
            ]
        );
    }

    #[test]
    fn test_deep_lambda() {
        // λ syntax from design doc
        assert_eq!(
            kinds("|a,b| -> bool { a == b }"),
            vec![
                TokenKind::Bar,
                TokenKind::Identifier,
                TokenKind::Comma,
                TokenKind::Identifier,
                TokenKind::Bar,
                TokenKind::FatArrow,
                TokenKind::Identifier,
                TokenKind::LBrace,
                TokenKind::Identifier,
                TokenKind::EqEq,
                TokenKind::Identifier,
                TokenKind::RBrace
            ]
        );
    }

    #[test]
    fn test_real_variable_decl() {
        assert_eq!(
            kinds("const pi = 3.14159; var counter = 0;"),
            vec![
                TokenKind::Const,
                TokenKind::Identifier,
                TokenKind::Eq,
                TokenKind::FloatLiteral,
                TokenKind::Semicolon,
                TokenKind::Var,
                TokenKind::Identifier,
                TokenKind::Eq,
                TokenKind::IntLiteral,
                TokenKind::Semicolon
            ]
        );
    }

    #[test]
    fn token_positions_after_multichar_tokens_and_spaces() {
        let toks = tokens("struct Point { x: Int64 }");
        let positions: Vec<(TokenKind, &str, usize, usize)> = toks
            .iter()
            .map(|t| (t.kind, t.lexeme.as_str(), t.line, t.col))
            .collect();

        assert_eq!(
            positions,
            vec![
                (TokenKind::Struct, "struct", 1, 1),
                (TokenKind::Identifier, "Point", 1, 8),
                (TokenKind::LBrace, "{", 1, 14),
                (TokenKind::Identifier, "x", 1, 16),
                (TokenKind::Colon, ":", 1, 17),
                (TokenKind::Identifier, "Int64", 1, 19),
                (TokenKind::RBrace, "}", 1, 25),
            ]
        );
    }

    // ── 关键字: 逐个验证 ──

    #[test]
    fn test_keyword_const() {
        assert_eq!(kinds("const"), vec![TokenKind::Const]);
    }
    #[test]
    fn test_keyword_var() {
        assert_eq!(kinds("var"), vec![TokenKind::Var]);
    }
    #[test]
    fn test_keyword_if() {
        assert_eq!(kinds("if"), vec![TokenKind::If]);
    }
    #[test]
    fn test_keyword_else() {
        assert_eq!(kinds("else"), vec![TokenKind::Else]);
    }
    #[test]
    fn test_keyword_while() {
        assert_eq!(kinds("while"), vec![TokenKind::While]);
    }
    #[test]
    fn test_keyword_for() {
        assert_eq!(kinds("for"), vec![TokenKind::For]);
    }
    #[test]
    fn test_keyword_in() {
        assert_eq!(kinds("in"), vec![TokenKind::In]);
    }
    #[test]
    fn test_keyword_break() {
        assert_eq!(kinds("break"), vec![TokenKind::Break]);
    }
    #[test]
    fn test_keyword_continue() {
        assert_eq!(kinds("continue"), vec![TokenKind::Continue]);
    }
    #[test]
    fn test_keyword_return() {
        assert_eq!(kinds("return"), vec![TokenKind::Return]);
    }
    #[test]
    fn test_keyword_struct() {
        assert_eq!(kinds("struct"), vec![TokenKind::Struct]);
    }
    #[test]
    fn test_keyword_impl() {
        assert_eq!(kinds("impl"), vec![TokenKind::Impl]);
    }
    #[test]
    fn test_keyword_export() {
        assert_eq!(kinds("export"), vec![TokenKind::Export]);
    }
    #[test]
    fn test_keyword_import() {
        assert_eq!(kinds("import"), vec![TokenKind::Import]);
    }
    #[test]
    fn test_keyword_from() {
        assert_eq!(kinds("from"), vec![TokenKind::From]);
    }
    #[test]
    fn test_keyword_as() {
        assert_eq!(kinds("as"), vec![TokenKind::As]);
    }
    #[test]
    fn test_keyword_async() {
        assert_eq!(kinds("async"), vec![TokenKind::Async_]);
    }
    #[test]
    fn test_keyword_await() {
        assert_eq!(kinds("await"), vec![TokenKind::Await]);
    }
    #[test]
    fn test_keyword_self() {
        assert_eq!(kinds("self"), vec![TokenKind::Self_]);
    }

    // ── 关键字大小写敏感 ──

    #[test]
    fn test_keywords_are_case_sensitive() {
        // "Const" with capital C should be an identifier, not the 'const' keyword
        assert_eq!(kinds("Const"), vec![TokenKind::Identifier]);
        assert_eq!(kinds("IF"), vec![TokenKind::Identifier]);
        assert_eq!(kinds("While"), vec![TokenKind::Identifier]);
        assert_eq!(kinds("Struct"), vec![TokenKind::Identifier]);
    }

    // ── 字面量 ──

    #[test]
    fn test_int_literal_single_digit() {
        assert_eq!(kinds("0"), vec![TokenKind::IntLiteral]);
        assert_eq!(kinds("7"), vec![TokenKind::IntLiteral]);
    }

    #[test]
    fn test_int_literal_multi_digit() {
        assert_eq!(kinds("1234567890"), vec![TokenKind::IntLiteral]);
    }

    #[test]
    fn test_int_literal_underscores() {
        let toks = tokens("1_000_000");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::IntLiteral);
        assert_eq!(toks[0].lexeme, "1_000_000");
    }

    #[test]
    fn test_float_literal_simple() {
        assert_eq!(kinds("3.14"), vec![TokenKind::FloatLiteral]);
    }

    #[test]
    fn test_float_literal_leading_zero() {
        assert_eq!(kinds("0.5"), vec![TokenKind::FloatLiteral]);
    }

    #[test]
    fn test_float_literal_trailing_underscore() {
        let toks = tokens("1_000.5");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::FloatLiteral);
        assert_eq!(toks[0].lexeme, "1_000.5");
    }

    #[test]
    fn test_int_dot_ambiguity_int_then_dot() {
        // 42. followed by whitespace: "42." -> int 42 then dot
        // Actually "." alone after a number with nothing after is ambiguous;
        // we need to see what our lexer does: no digit after dot → int + dot
        let toks = tokens("42. ");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::IntLiteral);
        assert_eq!(toks[0].lexeme, "42");
        assert_eq!(toks[1].kind, TokenKind::Dot);
    }

    #[test]
    fn test_int_dot_method_call() {
        // 42.to_string() → int, dot, ident, (, )
        let ks = kinds("42.to_string()");
        assert_eq!(
            ks,
            vec![
                TokenKind::IntLiteral,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn test_float_with_decimal_then_method() {
        // 42.0.to_string() → float, dot, ident, (, )
        let ks = kinds("42.0.to_string()");
        assert_eq!(
            ks,
            vec![
                TokenKind::FloatLiteral,
                TokenKind::Dot,
                TokenKind::Identifier,
                TokenKind::LParen,
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn test_string_literal_empty() {
        let toks = tokens(r#""""#);
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::StringLiteral);
        assert_eq!(toks[0].lexeme, r#""""#);
    }

    #[test]
    fn test_string_literal_single_char() {
        let toks = tokens(r#""x""#);
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::StringLiteral);
        assert_eq!(toks[0].lexeme, r#""x""#);
    }

    #[test]
    fn test_string_literal_single_quotes() {
        // single-quoted strings are also supported
        let toks = tokens("'hello'");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::StringLiteral);
        assert_eq!(toks[0].lexeme, "'hello'");
    }

    #[test]
    fn test_string_literal_unicode() {
        let toks = tokens(r#""你好世界""#);
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::StringLiteral);
        assert_eq!(toks[0].lexeme, r#""你好世界""#);
    }

    #[test]
    fn test_string_unterminated_is_error() {
        let toks = tokens(r#""unclosed"#);
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Error);
        assert!(toks[0].lexeme.contains("unterminated"));
    }

    #[test]
    fn test_string_escape_backslash_n() {
        let toks = tokens(r#""\n""#);
        assert_eq!(toks[0].lexeme, "\"\n\"");
    }

    #[test]
    fn test_string_escape_backslash_t() {
        let toks = tokens(r#""\t""#);
        assert_eq!(toks[0].lexeme, "\"\t\"");
    }

    #[test]
    fn test_string_escape_backslash_r() {
        let toks = tokens(r#""\r""#);
        assert_eq!(toks[0].lexeme, "\"\r\"");
    }

    #[test]
    fn test_string_escape_literal_backslash() {
        let toks = tokens(r#""\\""#);
        assert_eq!(toks[0].lexeme, "\"\\\"");
    }

    #[test]
    fn test_bool_literals() {
        assert_eq!(kinds("true"), vec![TokenKind::True]);
        assert_eq!(kinds("false"), vec![TokenKind::False]);
    }

    #[test]
    fn test_null_literal() {
        assert_eq!(kinds("null"), vec![TokenKind::Null]);
    }

    // ── 注释 ──

    #[test]
    fn test_line_comment_content() {
        let toks = tokens("// this is a comment\n42");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!(toks[0].lexeme, "// this is a comment");
        assert_eq!(toks[1].kind, TokenKind::IntLiteral);
    }

    #[test]
    fn test_line_comment_at_eof() {
        let toks = tokens("// no newline at end");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!(toks[0].lexeme, "// no newline at end");
    }

    #[test]
    fn test_block_comment_content() {
        let toks = tokens("/* hello */");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!(toks[0].lexeme, "/* hello */");
    }

    #[test]
    fn test_block_comment_multiline() {
        let toks = tokens("/* line1\n   line2 */\n42");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert!(toks[0].lexeme.contains("line1"));
        assert!(toks[0].lexeme.contains("line2"));
        assert_eq!(toks[1].kind, TokenKind::IntLiteral);
    }

    #[test]
    fn test_block_comment_nested() {
        let toks = tokens("/* outer /* inner */ still outer */\n42");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert!(toks[0].lexeme.contains(" outer "));
        assert!(toks[0].lexeme.contains(" inner "));
        assert!(toks[0].lexeme.contains(" still outer "));
        assert_eq!(toks[1].kind, TokenKind::IntLiteral);
    }

    #[test]
    fn test_block_comment_unclosed() {
        // unclosed block comment consumes until EOF
        let toks = tokens("/* never ends :(");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Comment);
    }

    #[test]
    fn test_block_comment_deeply_nested() {
        let toks = tokens("/* a /* b /* c */ d */ e */\n42");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!(toks[1].kind, TokenKind::IntLiteral);
    }

    // ── 标识符 ──

    #[test]
    fn test_identifier_single_char() {
        assert_eq!(kinds("x"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn test_identifier_snake_case() {
        assert_eq!(kinds("my_var"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn test_identifier_with_numbers() {
        assert_eq!(kinds("arg2"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn test_identifier_starting_with_underscore() {
        assert_eq!(kinds("_private"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn test_identifier_uppercase() {
        // Not a keyword — should be identifier
        assert_eq!(kinds("Point"), vec![TokenKind::Identifier]);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_identifier_camelCase() {
        assert_eq!(kinds("myFunction"), vec![TokenKind::Identifier]);
    }

    #[test]
    fn test_identifier_all_caps() {
        assert_eq!(kinds("MAX_VALUE"), vec![TokenKind::Identifier]);
    }

    // ── 运算符: 完整覆盖 ──

    #[test]
    fn test_each_single_char_operator_individually() {
        assert_eq!(kinds("+"), vec![TokenKind::Plus]);
        assert_eq!(kinds("-"), vec![TokenKind::Minus]);
        assert_eq!(kinds("*"), vec![TokenKind::Asterisk]);
        assert_eq!(kinds("/"), vec![TokenKind::Slash]);
        assert_eq!(kinds("%"), vec![TokenKind::Percent]);
    }

    #[test]
    fn test_each_double_char_operator_individually() {
        assert_eq!(kinds("=="), vec![TokenKind::EqEq]);
        assert_eq!(kinds("!="), vec![TokenKind::NotEq]);
        assert_eq!(kinds("<="), vec![TokenKind::Le]);
        assert_eq!(kinds(">="), vec![TokenKind::Ge]);
        assert_eq!(kinds("->"), vec![TokenKind::FatArrow]);
        assert_eq!(kinds("|>"), vec![TokenKind::Pipe]);
        assert_eq!(kinds(">>"), vec![TokenKind::GtGt]);
    }

    #[test]
    fn test_standalone_bang_is_error() {
        // '!' alone without '=' should produce an error token
        let toks = tokens("!");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Error);
    }

    #[test]
    fn test_not_operator_is_keyword_not() {
        assert_eq!(kinds("not"), vec![TokenKind::Not]);
    }

    #[test]
    fn test_and_operator_is_keyword() {
        assert_eq!(kinds("and"), vec![TokenKind::And]);
    }

    #[test]
    fn test_or_operator_is_keyword() {
        assert_eq!(kinds("or"), vec![TokenKind::Or]);
    }

    // ── 定界符: 逐个验证 ──

    #[test]
    fn test_each_delimiter_individually() {
        assert_eq!(kinds("("), vec![TokenKind::LParen]);
        assert_eq!(kinds(")"), vec![TokenKind::RParen]);
        assert_eq!(kinds("{"), vec![TokenKind::LBrace]);
        assert_eq!(kinds("}"), vec![TokenKind::RBrace]);
        assert_eq!(kinds("["), vec![TokenKind::LBracket]);
        assert_eq!(kinds("]"), vec![TokenKind::RBracket]);
        assert_eq!(kinds(","), vec![TokenKind::Comma]);
        assert_eq!(kinds(";"), vec![TokenKind::Semicolon]);
        assert_eq!(kinds(":"), vec![TokenKind::Colon]);
        assert_eq!(kinds("."), vec![TokenKind::Dot]);
    }

    // ── 位置/span ──

    #[test]
    fn test_positions_after_whitespace() {
        let toks = tokens("   x   y");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].line, 1);
        assert_eq!(toks[0].col, 4);
        assert_eq!(toks[1].line, 1);
        assert_eq!(toks[1].col, 8);
    }

    #[test]
    fn test_positions_multiline() {
        let toks = tokens("x\ny\nz");
        assert_eq!(toks.len(), 3);
        assert_eq!((toks[0].line, toks[0].col), (1, 1));
        assert_eq!((toks[1].line, toks[1].col), (2, 1));
        assert_eq!((toks[2].line, toks[2].col), (3, 1));
    }

    #[test]
    fn test_positions_multiline_with_spaces() {
        let toks = tokens("const a = 1;\nvar b = 2;");
        // line 1: const(1,1) a(1,7) =(1,9) 1(1,11) ;(1,12)
        // line 2: var(2,1) b(2,5) =(2,7) 2(2,9) ;(2,10)
        assert_eq!(toks[0].kind, TokenKind::Const);
        assert_eq!((toks[0].line, toks[0].col), (1, 1));
        assert_eq!(toks[1].kind, TokenKind::Identifier);
        assert_eq!((toks[1].line, toks[1].col), (1, 7));
        assert_eq!(toks[5].kind, TokenKind::Var);
        assert_eq!((toks[5].line, toks[5].col), (2, 1));
    }

    #[test]
    fn test_positions_after_line_comment() {
        let toks = tokens("// comment\nx");
        assert_eq!(toks.len(), 2); // comment + identifier
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!((toks[0].line, toks[0].col), (1, 1));
        assert_eq!(toks[1].kind, TokenKind::Identifier);
        assert_eq!((toks[1].line, toks[1].col), (2, 1));
    }

    #[test]
    fn test_positions_after_block_comment() {
        let toks = tokens("/* comment */x");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!((toks[0].line, toks[0].col), (1, 1));
        assert_eq!(toks[1].kind, TokenKind::Identifier);
        assert_eq!((toks[1].line, toks[1].col), (1, 14));
    }

    #[test]
    fn test_positions_multi_char_operators() {
        // -> takes two columns
        let toks = tokens("x -> y");
        assert_eq!(toks.len(), 3);
        assert_eq!(toks[0].kind, TokenKind::Identifier);
        assert_eq!((toks[0].line, toks[0].col), (1, 1));
        assert_eq!(toks[1].kind, TokenKind::FatArrow);
        assert_eq!((toks[1].line, toks[1].col), (1, 3));
        assert_eq!(toks[2].kind, TokenKind::Identifier);
        assert_eq!((toks[2].line, toks[2].col), (1, 6));
    }

    // ── 边缘情况 ──

    #[test]
    fn test_empty_source() {
        let toks = tokens("");
        assert!(toks.is_empty());
    }

    #[test]
    fn test_only_whitespace() {
        let toks = tokens("   \t\n  \r\n  ");
        assert!(toks.is_empty());
    }

    #[test]
    fn test_only_comments() {
        let toks = tokens("// a comment\n/* another */");
        assert_eq!(toks.len(), 2);
        assert_eq!(toks[0].kind, TokenKind::Comment);
        assert_eq!(toks[1].kind, TokenKind::Comment);
    }

    #[test]
    fn test_unknown_character_is_error() {
        // Characters not recognized should produce Error tokens
        let toks = tokens("@");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Error);
    }

    #[test]
    fn test_unknown_characters_each_is_error() {
        let unknown = ['@', '#', '$', '~', '^', '\\'];
        for ch in unknown {
            let src = ch.to_string();
            let toks = tokens(&src);
            assert_eq!(toks[0].kind, TokenKind::Error, "expected Error for '{ch}'");
        }
    }

    #[test]
    fn test_lexeme_preserves_value_for_literals() {
        let toks = tokens("42 3.14 \"hello\" true");
        assert_eq!(toks[0].lexeme, "42");
        assert_eq!(toks[1].lexeme, "3.14");
        assert_eq!(toks[2].lexeme, r#""hello""#);
        assert_eq!(toks[3].lexeme, "true");
    }

    #[test]
    fn test_lexeme_preserves_operator_text() {
        let toks = tokens("<= >= == !=");
        assert_eq!(toks[0].lexeme, "<=");
        assert_eq!(toks[1].lexeme, ">=");
        assert_eq!(toks[2].lexeme, "==");
        assert_eq!(toks[3].lexeme, "!=");
    }

    #[test]
    fn test_lexeme_preserves_identifier() {
        let toks = tokens("myVariable another_one");
        assert_eq!(toks[0].lexeme, "myVariable");
        assert_eq!(toks[1].lexeme, "another_one");
    }

    // ── 复合场景 ──

    #[test]
    fn test_full_struct_declaration() {
        let ks = kinds("struct Point { x: Int64, y: Int64 }");
        assert_eq!(
            ks,
            vec![
                TokenKind::Struct,
                TokenKind::Identifier,
                TokenKind::LBrace,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::Comma,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::RBrace,
            ]
        );
    }

    #[test]
    fn test_full_function_declaration() {
        let ks = kinds("const add = |a: Int64, b: Int64| -> Int64 { return a + b; }");
        assert_eq!(ks[0], TokenKind::Const);
        assert_eq!(ks[1], TokenKind::Identifier); // add
        assert_eq!(ks[2], TokenKind::Eq);
        assert_eq!(ks[3], TokenKind::Bar);
        assert_eq!(ks[4], TokenKind::Identifier); // a
        assert_eq!(ks[5], TokenKind::Colon);
        assert_eq!(ks[6], TokenKind::Identifier); // Int64
        assert_eq!(ks[7], TokenKind::Comma);
        assert!(ks.contains(&TokenKind::FatArrow));
        assert!(ks.contains(&TokenKind::Return));
        assert!(ks.contains(&TokenKind::RBrace));
    }

    #[test]
    fn test_if_else_chain() {
        let ks = kinds("if x > 0 { return 1; } else { return 0; }");
        assert_eq!(ks[0], TokenKind::If);
        assert!(ks.contains(&TokenKind::Else));
        assert_eq!(ks[ks.len() - 1], TokenKind::RBrace);
    }

    #[test]
    fn test_while_loop() {
        let ks = kinds("while i < n { i = i + 1; }");
        assert_eq!(ks[0], TokenKind::While);
        assert_eq!(ks[1], TokenKind::Identifier);
        assert_eq!(ks[2], TokenKind::Lt);
    }

    #[test]
    fn test_for_loop() {
        let ks = kinds("for x in xs { print(x); }");
        assert_eq!(ks[0], TokenKind::For);
        assert_eq!(ks[2], TokenKind::In);
        assert_eq!(ks[4], TokenKind::LBrace);
    }
}
