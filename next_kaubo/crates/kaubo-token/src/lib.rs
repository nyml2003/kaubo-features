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
    Match,    // match

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
    Pipe,              // |>
    Bar,               // | (lambda 分隔符)
    FatArrow,          // ->
    GtGt,              // >>
    QuestionQuestion,  // ??
    QuestionDot,       // ?.
    QuestionLBracket,  // ?[
    TemplateString,    // `...` template string content
    DotDotDot,         // ...

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
            "match" => Self::Match,
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

    // ── from_ident: 所有关键字 ──

    #[test]
    fn from_ident_keywords() {
        assert_eq!(TokenKind::from_ident("const"), TokenKind::Const);
        assert_eq!(TokenKind::from_ident("var"), TokenKind::Var);
        assert_eq!(TokenKind::from_ident("if"), TokenKind::If);
        assert_eq!(TokenKind::from_ident("else"), TokenKind::Else);
        assert_eq!(TokenKind::from_ident("for"), TokenKind::For);
        assert_eq!(TokenKind::from_ident("in"), TokenKind::In);
        assert_eq!(TokenKind::from_ident("while"), TokenKind::While);
        assert_eq!(TokenKind::from_ident("break"), TokenKind::Break);
        assert_eq!(TokenKind::from_ident("continue"), TokenKind::Continue);
        assert_eq!(TokenKind::from_ident("return"), TokenKind::Return);
        assert_eq!(TokenKind::from_ident("struct"), TokenKind::Struct);
        assert_eq!(TokenKind::from_ident("impl"), TokenKind::Impl);
        assert_eq!(TokenKind::from_ident("export"), TokenKind::Export);
        assert_eq!(TokenKind::from_ident("import"), TokenKind::Import);
        assert_eq!(TokenKind::from_ident("from"), TokenKind::From);
        assert_eq!(TokenKind::from_ident("as"), TokenKind::As);
        assert_eq!(TokenKind::from_ident("async"), TokenKind::Async_);
        assert_eq!(TokenKind::from_ident("await"), TokenKind::Await);
    }

    #[test]
    fn from_ident_literals() {
        assert_eq!(TokenKind::from_ident("true"), TokenKind::True);
        assert_eq!(TokenKind::from_ident("false"), TokenKind::False);
        assert_eq!(TokenKind::from_ident("null"), TokenKind::Null);
    }

    #[test]
    fn from_ident_logical_operators() {
        assert_eq!(TokenKind::from_ident("not"), TokenKind::Not);
        assert_eq!(TokenKind::from_ident("and"), TokenKind::And);
        assert_eq!(TokenKind::from_ident("or"), TokenKind::Or);
    }

    #[test]
    fn from_ident_case_sensitive() {
        // Keywords are case-sensitive
        assert_eq!(TokenKind::from_ident("Const"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("VAR"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("True"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("NULL"), TokenKind::Identifier);
    }

    #[test]
    fn from_ident_identifiers() {
        assert_eq!(TokenKind::from_ident("x"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("myVar"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("Point"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("_hidden"), TokenKind::Identifier);
        assert_eq!(TokenKind::from_ident("test2"), TokenKind::Identifier);
    }

    // ── Token 构造 ──

    #[test]
    fn token_new_preserves_all_fields() {
        let t = Token::new(TokenKind::Identifier, "hello".into(), 5, 10);
        assert_eq!(t.kind, TokenKind::Identifier);
        assert_eq!(t.lexeme, "hello");
        assert_eq!(t.line, 5);
        assert_eq!(t.col, 10);
    }

    #[test]
    fn token_eof_is_constructible() {
        let t = Token::eof(2, 4);
        assert_eq!(t.kind, TokenKind::Eof);
        assert_eq!(t.lexeme, "");
        assert_eq!(t.line, 2);
        assert_eq!(t.col, 4);
    }

    #[test]
    fn token_debug_format_includes_fields() {
        let t = Token::new(TokenKind::Plus, "+".into(), 1, 1);
        let s = format!("{t:?}");
        assert!(s.contains("Plus"));
        assert!(s.contains("+"));
    }

    // ── TokenKind count and representation ──

    #[test]
    fn token_kind_repr_fits_u8() {
        // If repr(u8), all discriminants must fit in u8
        use TokenKind::*;
        let kinds = [
            Const, Var, If, Else, For, In, While, Break, Continue, Return,
            Struct, Impl, Export, Import, From, As, Async_, Await, Self_, Match,
            Identifier, IntLiteral, FloatLiteral, StringLiteral, True, False, Null,
            Plus, Minus, Asterisk, Slash, Percent,
            Eq, EqEq, NotEq, Lt, Le, Gt, Ge, Not, And, Or,
            LParen, RParen, LBrace, RBrace, LBracket, RBracket,
            Comma, Semicolon, Colon, Dot,
            Pipe, Bar, FatArrow, GtGt, QuestionQuestion, QuestionDot, QuestionLBracket, TemplateString, DotDotDot,
            Eof, Comment, Whitespace, Error,
        ];
        assert_eq!(kinds.len(), 65);
    }
}
