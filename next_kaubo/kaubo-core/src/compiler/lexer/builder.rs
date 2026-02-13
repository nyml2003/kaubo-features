//! Lexer 构建器
//!
//! 使用新的 Lexer V2 实现

use crate::kit::lexer::Lexer;
use kaubo_log::Logger;
use std::sync::Arc;

/// 创建新的 Lexer
pub fn build_lexer() -> Lexer {
    Lexer::new(102400)  // 100KB 缓存，支持更大文件
}

/// 创建新的 Lexer（带 logger）
pub fn build_lexer_with_logger(logger: Arc<Logger>) -> Lexer {
    Lexer::with_logger(102400, logger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::token_kind::KauboTokenKind;

    fn lex_all(input: &str) -> Vec<(KauboTokenKind, Option<String>)> {
        let mut lexer = build_lexer();
        lexer.feed(input.as_bytes()).unwrap();
        lexer.terminate().unwrap();

        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push((token.kind, token.text));
        }
        tokens
    }

    #[test]
    fn test_build_lexer() {
        let _lexer = build_lexer();
    }

    #[test]
    fn test_lexer_tokenizes_keywords() {
        let code = "var if else while for return true false null";
        let tokens = lex_all(code);

        let expected = vec![
            (KauboTokenKind::Var, "var".to_string()),
            (KauboTokenKind::If, "if".to_string()),
            (KauboTokenKind::Else, "else".to_string()),
            (KauboTokenKind::While, "while".to_string()),
            (KauboTokenKind::For, "for".to_string()),
            (KauboTokenKind::Return, "return".to_string()),
            (KauboTokenKind::True, "true".to_string()),
            (KauboTokenKind::False, "false".to_string()),
            (KauboTokenKind::Null, "null".to_string()),
        ];

        for (i, (expected_kind, expected_value)) in expected.iter().enumerate() {
            assert_eq!(tokens[i].0, *expected_kind, "Token {} kind mismatch", i);
            assert_eq!(
                tokens[i].1,
                Some(expected_value.to_string()),
                "Token {} value mismatch",
                i
            );
        }
    }

    #[test]
    fn test_lexer_tokenizes_operators() {
        let code = "+ - * / == != <= >= = < >";
        let tokens = lex_all(code);

        assert_eq!(tokens[0].0, KauboTokenKind::Plus);
        assert_eq!(tokens[1].0, KauboTokenKind::Minus);
        assert_eq!(tokens[2].0, KauboTokenKind::Asterisk);
        assert_eq!(tokens[3].0, KauboTokenKind::Slash);
        assert_eq!(tokens[4].0, KauboTokenKind::DoubleEqual);
        assert_eq!(tokens[5].0, KauboTokenKind::ExclamationEqual);
        assert_eq!(tokens[6].0, KauboTokenKind::LessThanEqual);
        assert_eq!(tokens[7].0, KauboTokenKind::GreaterThanEqual);
        assert_eq!(tokens[8].0, KauboTokenKind::Equal);
        assert_eq!(tokens[9].0, KauboTokenKind::LessThan);
        assert_eq!(tokens[10].0, KauboTokenKind::GreaterThan);
    }

    #[test]
    fn test_lexer_tokenizes_delimiters() {
        let code = "( ) { } [ ] ; , . |";
        let tokens = lex_all(code);

        assert_eq!(tokens[0].0, KauboTokenKind::LeftParenthesis);
        assert_eq!(tokens[1].0, KauboTokenKind::RightParenthesis);
        assert_eq!(tokens[2].0, KauboTokenKind::LeftCurlyBrace);
        assert_eq!(tokens[3].0, KauboTokenKind::RightCurlyBrace);
        assert_eq!(tokens[4].0, KauboTokenKind::LeftSquareBracket);
        assert_eq!(tokens[5].0, KauboTokenKind::RightSquareBracket);
        assert_eq!(tokens[6].0, KauboTokenKind::Semicolon);
        assert_eq!(tokens[7].0, KauboTokenKind::Comma);
        assert_eq!(tokens[8].0, KauboTokenKind::Dot);
        assert_eq!(tokens[9].0, KauboTokenKind::Pipe);
    }

    #[test]
    fn test_lexer_tokenizes_integer() {
        let code = "12345";
        let tokens = lex_all(code);

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, KauboTokenKind::LiteralInteger);
        assert_eq!(tokens[0].1, Some("12345".to_string()));
    }

    #[test]
    fn test_lexer_tokenizes_string() {
        let code = r#""hello world""#;
        let tokens = lex_all(code);

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, KauboTokenKind::LiteralString);
        assert_eq!(tokens[0].1, Some("hello world".to_string()));
    }

    #[test]
    fn test_lexer_tokenizes_identifier() {
        let code = "my_variable _private test123";
        let tokens = lex_all(code);

        for i in 0..3 {
            assert_eq!(tokens[i].0, KauboTokenKind::Identifier);
        }
    }

    #[test]
    fn test_lexer_tokenizes_comment() {
        let code = "// this is a comment\nvar x;";
        let tokens = lex_all(code);

        // 跳过注释和换行，应该得到 var
        assert_eq!(tokens[0].0, KauboTokenKind::Var);
        assert_eq!(tokens[1].0, KauboTokenKind::Identifier);
        assert_eq!(tokens[2].0, KauboTokenKind::Semicolon);
    }

    #[test]
    fn test_lexer_tokenizes_whitespace() {
        let code = "  \t\nvar";
        let tokens = lex_all(code);

        // 空白和制表符应该被识别并跳过
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, KauboTokenKind::Var);
    }

    #[test]
    fn test_lexer_tokenizes_all_keywords() {
        let cases = vec![
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

        for (code, expected) in cases {
            let tokens = lex_all(code);
            assert!(tokens.len() > 0, "Failed to tokenize '{}'", code);
            assert_eq!(tokens[0].0, expected, "Wrong token kind for '{}'", code);
        }
    }
}
