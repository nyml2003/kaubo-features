//! Kaubo Token 类型定义

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Default)]
#[repr(u8)]
pub enum KauboTokenKind {
    // 错误/状态类型
    Utf8Error = 0,
    Comment,

    // 关键字 (11-37)
    Var,
    If,
    Else,
    Elif,
    While,
    For,
    Return,
    In,
    Yield,
    True,
    False,
    Null,
    Break,
    Continue,
    Struct,
    Interface,
    Impl,
    Import,
    As,
    From,
    Pass,
    And,
    Or,
    Not,
    Async,
    Await,
    Module,
    Operator,
    Pub,
    Json,

    // 字面量 (100-102)
    LiteralInteger = 100,
    LiteralString,
    LiteralFloat,

    // 标识符 (120)
    Identifier = 120,

    // 双字符符号 (130-134)
    DoubleEqual = 130,
    ExclamationEqual,
    GreaterThanEqual,
    LessThanEqual,
    FatArrow,

    // 单字符符号 (150-168)
    GreaterThan = 150,
    LessThan,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,
    Colon,
    Equal,
    Comma,
    Semicolon,
    LeftParenthesis,
    RightParenthesis,
    LeftCurlyBrace,
    RightCurlyBrace,
    LeftSquareBracket,
    RightSquareBracket,
    Dot,
    Pipe,

    // 空白字符 (240-242)
    Whitespace = 240,
    Tab,
    NewLine,

    // 无效token（默认值）
    #[default]
    InvalidToken = 255,
}

impl From<KauboTokenKind> for u8 {
    fn from(val: KauboTokenKind) -> Self {
        val as u8
    }
}
