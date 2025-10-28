use crate::kit::lexer::{state_machine::types::TokenKindTrait, types::CLexerTokenKindTrait};

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Default)]
#[repr(u8)]
pub enum KauboTokenKind {
    // 错误/状态类型
    Utf8Error = 0,
    Comment = 1,

    // 关键字
    Var = 11,
    If = 12,
    Else = 13,
    Elif = 14,
    While = 15,
    For = 16,
    Return = 17,
    In = 18,
    Yield = 19,
    True = 20,
    False = 21,
    Null = 22,
    Break = 23,
    Continue = 24,
    Struct = 25,
    Interface = 26,
    Import = 27,
    As = 28,
    From = 29,
    Pass = 30,
    And = 31,
    Or = 32,
    Not = 33,
    Async = 34,
    Await = 35,

    // 字面量
    LiteralInteger = 100,
    LiteralString = 101,

    // 标识符
    Identifier = 120,

    // 双字符符号
    DoubleEqual = 130,
    ExclamationEqual = 131,
    GreaterThanEqual = 132,
    LessThanEqual = 133,

    // 单字符符号
    GreaterThan = 150,
    LessThan = 151,
    Plus = 152,
    Minus = 153,
    Asterisk = 154,
    Slash = 155,
    Colon = 156,
    Equal = 157,
    Comma = 158,
    Semicolon = 159,
    LeftParenthesis = 160,
    RightParenthesis = 161,
    LeftCurlyBrace = 162,
    RightCurlyBrace = 163,
    LeftSquareBracket = 164,
    RightSquareBracket = 165,
    Dot = 166,
    Pipe = 167,

    // 空白字符
    Whitespace = 240,
    Tab = 241,
    NewLine = 242,

    // 无效token（默认值）
    #[default]
    InvalidToken = 255,
}

impl From<u8> for KauboTokenKind {
    fn from(value: u8) -> Self {
        match value {
            0 => KauboTokenKind::Utf8Error,
            1 => KauboTokenKind::Comment,
            11 => KauboTokenKind::Var,
            12 => KauboTokenKind::If,
            13 => KauboTokenKind::Else,
            14 => KauboTokenKind::Elif,
            15 => KauboTokenKind::While,
            16 => KauboTokenKind::For,
            17 => KauboTokenKind::Return,
            18 => KauboTokenKind::In,
            19 => KauboTokenKind::Yield,
            20 => KauboTokenKind::True,
            21 => KauboTokenKind::False,
            22 => KauboTokenKind::Null,
            23 => KauboTokenKind::Break,
            24 => KauboTokenKind::Continue,
            25 => KauboTokenKind::Struct,
            26 => KauboTokenKind::Interface,
            27 => KauboTokenKind::Import,
            28 => KauboTokenKind::As,
            29 => KauboTokenKind::From,
            30 => KauboTokenKind::Pass,
            31 => KauboTokenKind::And,
            32 => KauboTokenKind::Or,
            33 => KauboTokenKind::Not,
            34 => KauboTokenKind::Async,
            35 => KauboTokenKind::Await,
            100 => KauboTokenKind::LiteralInteger,
            101 => KauboTokenKind::LiteralString,
            120 => KauboTokenKind::Identifier,
            130 => KauboTokenKind::DoubleEqual,
            131 => KauboTokenKind::ExclamationEqual,
            132 => KauboTokenKind::GreaterThanEqual,
            133 => KauboTokenKind::LessThanEqual,
            150 => KauboTokenKind::GreaterThan,
            151 => KauboTokenKind::LessThan,
            152 => KauboTokenKind::Plus,
            153 => KauboTokenKind::Minus,
            154 => KauboTokenKind::Asterisk,
            155 => KauboTokenKind::Slash,
            156 => KauboTokenKind::Colon,
            157 => KauboTokenKind::Equal,
            158 => KauboTokenKind::Comma,
            159 => KauboTokenKind::Semicolon,
            160 => KauboTokenKind::LeftParenthesis,
            161 => KauboTokenKind::RightParenthesis,
            162 => KauboTokenKind::LeftCurlyBrace,
            163 => KauboTokenKind::RightCurlyBrace,
            164 => KauboTokenKind::LeftSquareBracket,
            165 => KauboTokenKind::RightSquareBracket,
            166 => KauboTokenKind::Dot,
            167 => KauboTokenKind::Pipe,
            240 => KauboTokenKind::Whitespace,
            241 => KauboTokenKind::Tab,
            242 => KauboTokenKind::NewLine,
            255 => KauboTokenKind::InvalidToken,
            _ => KauboTokenKind::InvalidToken,
        }
    }
}

impl Into<u8> for KauboTokenKind {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TokenKindTrait for KauboTokenKind {}

impl CLexerTokenKindTrait for KauboTokenKind {
    fn invalid_token() -> Self {
        KauboTokenKind::InvalidToken
    }

    fn whitespace() -> Self {
        KauboTokenKind::Whitespace
    }

    fn tab() -> Self {
        KauboTokenKind::Tab
    }

    fn newline() -> Self {
        KauboTokenKind::NewLine
    }

    fn comment() -> Self {
        KauboTokenKind::Comment
    }
}
