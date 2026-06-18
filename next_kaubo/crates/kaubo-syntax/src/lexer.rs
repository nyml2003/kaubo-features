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
        let mut s = String::new();
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
        let mut s = String::new();
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

    fn scan_pipe(&mut self, line: usize, col: usize) -> Token {
        if self.peek() == Some('>') {
            self.advance();
            Token::new(TokenKind::Pipe, "|>".into(), line, col)
        } else {
            Token::new(TokenKind::Bar, "|".into(), line, col)
        }
    }

    fn scan_string(&mut self, quote: char, line: usize, col: usize) -> Token {
        let mut s = String::new();
        while let Some(c) = self.bump() {
            if c == quote {
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
            format!("unterminated string: {}", s),
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
            if after_dot.map_or(false, |c| c.is_ascii_digit()) {
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
        assert_eq!(toks[0].lexeme, "a\nb\tc\\d\"e");
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
}
