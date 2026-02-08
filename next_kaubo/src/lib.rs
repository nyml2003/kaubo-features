//! Kaubo 编译器
//!
//! 包含编译器前端 (Lexer/Parser) 和运行时 (VM)。

pub mod compiler;
pub mod kit;
pub mod runtime;

// 运行时核心类型重导出
pub use runtime::Value;

use compiler::lexer::{builder::build_lexer, token_kind::KauboTokenKind};
use kit::lexer::types::Token;

use crate::kit::lexer::types::CLexerTokenKindTrait;

/// 词法分析结果
pub type LexResult = Vec<Token<KauboTokenKind>>;

/// 对输入字符串进行词法分析
///
/// # Example
/// ```
/// use next_kaubo::tokenize;
///
/// let tokens = tokenize("var x = 5;");
/// assert_eq!(tokens.len(), 5); // var, x, =, 5, ;
/// ```
pub fn tokenize(input: &str) -> LexResult {
    let mut lexer = build_lexer();
    let _ = lexer.feed(&input.as_bytes().to_vec());
    let _ = lexer.terminate();

    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        if token.kind.is_invalid_token() {
            eprintln!("Invalid token: {:?}", token);
            eprintln!("current tokens: {:?}", tokens);
            break;
        }
        tokens.push(token);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("var x = 5;");
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0].kind, KauboTokenKind::Var);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_whitespace() {
        let tokens = tokenize("   \n\t  ");
        assert!(tokens.is_empty());
    }
}
