//! Shared token contract types.

/// Token 种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenKind {
    // ── 语法关键字 (19) ──
    Const,    // const
    Var,      // var
    If,       // if
    Else,     // else
    For,      // for
    In,       // in
    While,    // while
    Break,    // break
    Continue, // continue
    Return,   // return
    Struct,   // struct
    Impl,     // impl
    Export,   // export
    Import,   // import
    From,     // from
    As,       // as (import 别名)
    Async_,   // async
    Await,    // await
    Self_,    // self

    // ── 字面量 ──
    Identifier,    // 标识符
    IntLiteral,    // 整数字面量
    FloatLiteral,  // 浮点字面量
    StringLiteral, // 字符串字面量
    True,          // true
    False,         // false
    Null,          // null

    // ── 单/双字符运算符 ──
    Plus,     // +
    Minus,    // -
    Asterisk, // *
    Slash,    // /
    Percent,  // %
    Eq,       // = 赋值
    EqEq,     // == 相等
    NotEq,    // != 不等
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
    Not,      // not
    And,      // and
    Or,       // or

    // ── 定界符 ──
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :
    Dot,       // .

    // ── 复合运算符 ──
    Pipe,     // |>
    Bar,      // | (lambda 分隔符)
    FatArrow, // ->
    GtGt,     // >>

    // ── 特殊 ──
    Eof,        // 文件结尾
    Comment,    // 注释
    Whitespace, // 空白
    Error,      // 词法错误
}

impl TokenKind {
    /// 从标识符查找关键字
    pub fn from_ident(s: &str) -> Self {
        match s {
            "const" => Self::Const,
            "var" => Self::Var,
            "if" => Self::If,
            "else" => Self::Else,
            "for" => Self::For,
            "in" => Self::In,
            "while" => Self::While,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "return" => Self::Return,
            "struct" => Self::Struct,
            "impl" => Self::Impl,
            "export" => Self::Export,
            "import" => Self::Import,
            "from" => Self::From,
            "as" => Self::As,
            "async" => Self::Async_,
            "await" => Self::Await,
            "self" => Self::Self_,
            "true" => Self::True,
            "false" => Self::False,
            "null" => Self::Null,
            "not" => Self::Not,
            "and" => Self::And,
            "or" => Self::Or,
            _ => Self::Identifier,
        }
    }
}

/// 完整 Token
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, line: usize, col: usize) -> Self {
        Self {
            kind,
            lexeme,
            line,
            col,
        }
    }

    pub fn eof(line: usize, col: usize) -> Self {
        Self {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            line,
            col,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_are_classified_from_identifiers() {
        assert_eq!(TokenKind::from_ident("const"), TokenKind::Const);
        assert_eq!(TokenKind::from_ident("while"), TokenKind::While);
        assert_eq!(TokenKind::from_ident("self"), TokenKind::Self_);
        assert_eq!(TokenKind::from_ident("and"), TokenKind::And);
    }

    #[test]
    fn unknown_identifier_stays_identifier() {
        assert_eq!(TokenKind::from_ident("value"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("async_value"), TokenKind::Identifier);
    }

    #[test]
    fn eof_token_preserves_position() {
        let token = Token::eof(3, 8);

        assert_eq!(token.kind, TokenKind::Eof);
        assert_eq!(token.lexeme, "");
        assert_eq!(token.line, 3);
        assert_eq!(token.col, 8);
    }
}
