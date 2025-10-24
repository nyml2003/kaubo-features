use super::state_machine::types::TokenKindTrait;
pub trait CLexerTokenKindTrait: TokenKindTrait {
    // 要求枚举必须能够提供这些特定成员
    fn invalid_token() -> Self;
    fn whitespace() -> Self;
    fn tab() -> Self;
    fn newline() -> Self;
    fn comment() -> Self;

    // 检查当前实例是否为特定成员
    fn is_invalid(&self) -> bool {
        self == &Self::invalid_token()
    }

    fn is_whitespace(&self) -> bool {
        self == &Self::whitespace()
    }

    fn is_tab(&self) -> bool {
        self == &Self::tab()
    }

    fn is_newline(&self) -> bool {
        self == &Self::newline()
    }
    fn is_comment(&self) -> bool {
        self == &Self::comment()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinate {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<TokenKind>
where
    TokenKind: CLexerTokenKindTrait,
{
    pub kind: TokenKind,
    pub value: String,
    pub coordinate: Coordinate,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EatStatus {
    Continue, // 继续处理下一个字符
    Stop,     // 状态机不匹配，停止当前Token
    Eof,      // 到达输入末尾
    Wait,     // 缓冲区数据不足
}
