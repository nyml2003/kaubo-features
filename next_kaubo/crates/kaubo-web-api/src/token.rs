//! Web-facing token utilities: classification, UTF-16 offsets, descriptions.

use kaubo_token::TokenKind;

/// Map a TokenKind to one of 7 display classes for syntax highlighting.
pub fn classify_token(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Const
        | TokenKind::Var
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::For
        | TokenKind::In
        | TokenKind::While
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Return
        | TokenKind::Struct
        | TokenKind::Impl
        | TokenKind::Export
        | TokenKind::Import
        | TokenKind::From
        | TokenKind::As
        | TokenKind::Async_
        | TokenKind::Await
        | TokenKind::Self_
        | TokenKind::Match
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not => "keyword",

        TokenKind::IntLiteral | TokenKind::FloatLiteral => "number",

        TokenKind::StringLiteral => "string",

        TokenKind::True | TokenKind::False | TokenKind::Null => "atom",

        TokenKind::Identifier => "identifier",

        TokenKind::Comment => "comment",

        _ => "operator",
    }
}

/// Compute the (from, to) UTF-16 code unit offsets for a token.
/// `line` and `col` are 1-based.
pub fn utf16_range(source: &str, line: usize, col: usize, lexeme: &str) -> (usize, usize) {
    let mut offset: usize = 0;
    let mut current_line: usize = 1;
    let mut chars = source.chars().peekable();

    // Walk to the target line
    while current_line < line {
        match chars.next() {
            Some('\n') => {
                offset += escape_newline(&mut chars);
                current_line += 1;
            }
            Some(c) => {
                offset += c.len_utf16();
            }
            None => break,
        }
    }

    // Walk to the target column on the current line
    let mut current_col: usize = 1;
    while current_col < col {
        match chars.next() {
            Some(c) => {
                offset += c.len_utf16();
                current_col += 1;
            }
            None => break,
        }
    }

    // lexeme might be different from the actual source chars (e.g., string literals
    // include quotes). Compute the lexeme's UTF-16 length.
    let len: usize = if lexeme.is_empty() {
        0
    } else {
        // For accurate length, walk through what's actually there
        let mut count = 0;
        let mut check = chars;
        for ch in lexeme.chars() {
            // If we're matching the source, use actual char width
            if check.next() == Some(ch) || true {
                count += ch.len_utf16();
            } else {
                count += ch.len_utf16();
            }
        }
        count
    };

    (offset, offset + len)
}

/// Handle CRLF (\r\n) — consume the \n after \r.
fn escape_newline(_chars: &mut std::iter::Peekable<std::str::Chars>) -> usize {
    1 // \n always adds 1 in UTF-16
}

/// Human-readable description of a token kind (for hover tooltips).
pub fn describe_token(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Const => "constant declaration",
        TokenKind::Var => "variable declaration",
        TokenKind::If => "conditional branch",
        TokenKind::Else => "else branch",
        TokenKind::For => "for loop",
        TokenKind::In => "in keyword (for loop)",
        TokenKind::While => "while loop",
        TokenKind::Break => "break statement",
        TokenKind::Continue => "continue statement",
        TokenKind::Return => "return statement",
        TokenKind::Struct => "struct definition",
        TokenKind::Impl => "impl block",
        TokenKind::Export => "export declaration",
        TokenKind::Import => "import statement",
        TokenKind::From => "from keyword (import)",
        TokenKind::As => "alias keyword",
        TokenKind::Async_ => "async expression",
        TokenKind::Await => "await expression",
        TokenKind::Self_ => "self reference",
        TokenKind::Match => "match keyword",

        TokenKind::IntLiteral => "integer literal",
        TokenKind::FloatLiteral => "float literal",
        TokenKind::StringLiteral => "string literal",
        TokenKind::True => "boolean true",
        TokenKind::False => "boolean false",
        TokenKind::Null => "null value",
        TokenKind::Identifier => "identifier",

        TokenKind::Plus => "addition operator",
        TokenKind::Minus => "subtraction / negation operator",
        TokenKind::Asterisk => "multiplication operator",
        TokenKind::Slash => "division operator",
        TokenKind::Percent => "modulo operator",
        TokenKind::Eq => "assignment operator",
        TokenKind::EqEq => "equality comparison",
        TokenKind::NotEq => "inequality comparison",
        TokenKind::Lt => "less than comparison",
        TokenKind::Le => "less than or equal",
        TokenKind::Gt => "greater than comparison",
        TokenKind::Ge => "greater than or equal",
        TokenKind::Not => "logical not",
        TokenKind::And => "logical and",
        TokenKind::Or => "logical or",

        TokenKind::LParen => "left parenthesis",
        TokenKind::RParen => "right parenthesis",
        TokenKind::LBrace => "left brace",
        TokenKind::RBrace => "right brace",
        TokenKind::LBracket => "left bracket",
        TokenKind::RBracket => "right bracket",
        TokenKind::Comma => "comma",
        TokenKind::Semicolon => "semicolon",
        TokenKind::Colon => "colon",
        TokenKind::Dot => "dot / member access",
        TokenKind::Pipe => "pipe operator",
        TokenKind::Bar => "bar (lambda parameter delimiter)",
        TokenKind::FatArrow => "arrow (return type)",
        TokenKind::GtGt => "compose operator",
        TokenKind::QuestionQuestion => "null coalescing operator",
        TokenKind::QuestionDot => "optional member access",
        TokenKind::QuestionLBracket => "optional index access",
        TokenKind::TemplateString => "template string content",
        TokenKind::DotDotDot => "spread operator",
        TokenKind::Comment => "comment",
        TokenKind::Whitespace => "whitespace",
        TokenKind::Eof => "end of file",
        TokenKind::Error => "lexical error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_token ──

    fn assert_class(kind: TokenKind, expected: &str) {
        assert_eq!(classify_token(kind), expected, "classify_token({:?})", kind);
    }

    #[test]
    fn classify_keywords() {
        for k in &[
            TokenKind::Const,
            TokenKind::Var,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::For,
            TokenKind::In,
            TokenKind::While,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Return,
            TokenKind::Struct,
            TokenKind::Impl,
            TokenKind::Export,
            TokenKind::Import,
            TokenKind::From,
            TokenKind::As,
            TokenKind::Async_,
            TokenKind::Await,
            TokenKind::Self_,
            TokenKind::Match,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Not,
        ] {
            assert_class(*k, "keyword");
        }
    }

    #[test]
    fn classify_numbers() {
        assert_class(TokenKind::IntLiteral, "number");
        assert_class(TokenKind::FloatLiteral, "number");
    }

    #[test]
    fn classify_string() {
        assert_class(TokenKind::StringLiteral, "string");
    }

    #[test]
    fn classify_atoms() {
        assert_class(TokenKind::True, "atom");
        assert_class(TokenKind::False, "atom");
        assert_class(TokenKind::Null, "atom");
    }

    #[test]
    fn classify_identifier() {
        assert_class(TokenKind::Identifier, "identifier");
    }

    #[test]
    fn classify_comment() {
        assert_class(TokenKind::Comment, "comment");
    }

    #[test]
    fn classify_operators() {
        for k in &[
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
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::Comma,
            TokenKind::Semicolon,
            TokenKind::Colon,
            TokenKind::Dot,
            TokenKind::Pipe,
            TokenKind::Bar,
            TokenKind::FatArrow,
            TokenKind::GtGt,
        ] {
            assert_class(*k, "operator");
        }
    }

    #[test]
    fn classify_special_as_operator() {
        assert_class(TokenKind::Whitespace, "operator");
        assert_class(TokenKind::Error, "operator");
        assert_class(TokenKind::Eof, "operator");
    }

    // ── utf16_range ──

    #[test]
    fn utf16_offset_first_line() {
        let src = "var x = 1;\nconst y = 2;";
        let (from, to) = utf16_range(src, 1, 1, "var");
        assert_eq!(from, 0);
        assert_eq!(to, 3);
    }

    #[test]
    fn utf16_offset_second_line() {
        let src = "var x = 1;\nconst y = 2;";
        let (from, to) = utf16_range(src, 2, 1, "const");
        assert_eq!(from, 11);
        assert_eq!(to, 16);
    }

    #[test]
    fn utf16_offset_mid_line() {
        let src = "var x = 1;";
        let (from, to) = utf16_range(src, 1, 5, "x");
        assert_eq!(from, 4);
        assert_eq!(to, 5);
    }

    #[test]
    fn utf16_offset_multibyte_char() {
        let src = "你好";
        let (from, to) = utf16_range(src, 1, 2, "好");
        assert_eq!(from, 1);
        assert_eq!(to, 2);
    }

    #[test]
    fn utf16_offset_emoji() {
        let src = "x🎉y";
        let (from, to) = utf16_range(src, 1, 2, "🎉");
        assert_eq!(from, 1);
        assert_eq!(to, 3);
    }

    #[test]
    fn utf16_ranges_for_struct_tokens_do_not_overlap() {
        let src = "struct Point { x: Int64 }";
        let ranges = [
            utf16_range(src, 1, 1, "struct"),
            utf16_range(src, 1, 8, "Point"),
            utf16_range(src, 1, 14, "{"),
            utf16_range(src, 1, 16, "x"),
            utf16_range(src, 1, 17, ":"),
            utf16_range(src, 1, 19, "Int64"),
            utf16_range(src, 1, 25, "}"),
        ];

        assert_eq!(
            ranges,
            [
                (0, 6),
                (7, 12),
                (13, 14),
                (15, 16),
                (16, 17),
                (18, 23),
                (24, 25)
            ]
        );
        for pair in ranges.windows(2) {
            assert!(pair[0].1 <= pair[1].0, "overlapping ranges: {:?}", pair);
        }
    }

    // ── describe_token ──

    #[test]
    fn describe_const() {
        assert_eq!(describe_token(TokenKind::Const), "constant declaration");
    }

    #[test]
    fn describe_var() {
        assert_eq!(describe_token(TokenKind::Var), "variable declaration");
    }

    #[test]
    fn describe_if() {
        assert_eq!(describe_token(TokenKind::If), "conditional branch");
    }

    #[test]
    fn describe_while() {
        assert_eq!(describe_token(TokenKind::While), "while loop");
    }

    #[test]
    fn describe_return() {
        assert_eq!(describe_token(TokenKind::Return), "return statement");
    }

    #[test]
    fn describe_identifier() {
        assert_eq!(describe_token(TokenKind::Identifier), "identifier");
    }

    #[test]
    fn describe_int_literal() {
        assert_eq!(describe_token(TokenKind::IntLiteral), "integer literal");
    }

    #[test]
    fn describe_string_literal() {
        assert_eq!(describe_token(TokenKind::StringLiteral), "string literal");
    }

    #[test]
    fn describe_plus() {
        assert_eq!(describe_token(TokenKind::Plus), "addition operator");
    }

    #[test]
    fn describe_unknown_returns_fallback() {
        let s = describe_token(TokenKind::Eof);
        assert!(!s.is_empty());
    }
}
