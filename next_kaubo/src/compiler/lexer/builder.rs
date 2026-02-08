use super::token_kind::KauboTokenKind;
use crate::kit::lexer::{
    c_lexer::Lexer,
    state_machine::builder::{
        build_comment_machine, build_identifier_machine, build_integer_machine,
        build_keyword_machine, build_multi_char_machine, build_newline_machine,
        build_single_char_machine, build_string_machine, build_tab_machine,
        build_whitespace_machine,
    },
};
pub fn build_lexer() -> Lexer<KauboTokenKind> {
    let mut lexer: Lexer<KauboTokenKind> = Lexer::new(1024);

    for (kind, operator) in vec![
        (KauboTokenKind::ExclamationEqual, "!="),
        (KauboTokenKind::DoubleEqual, "=="),
        (KauboTokenKind::LessThanEqual, "<="),
        (KauboTokenKind::GreaterThanEqual, ">="),
    ] {
        lexer.register_machine(build_multi_char_machine(kind, operator.chars().collect()).unwrap());
    }

    for (kind, operator) in vec![
        (KauboTokenKind::GreaterThan, ">"),
        (KauboTokenKind::LessThan, "<"),
        (KauboTokenKind::Plus, "+"),
        (KauboTokenKind::Minus, "-"),
        (KauboTokenKind::Asterisk, "*"),
        (KauboTokenKind::Slash, "/"),
        (KauboTokenKind::Colon, ":"),
        (KauboTokenKind::Comma, ","),
        (KauboTokenKind::Semicolon, ";"),
        (KauboTokenKind::LeftParenthesis, "("),
        (KauboTokenKind::RightParenthesis, ")"),
        (KauboTokenKind::LeftCurlyBrace, "{"),
        (KauboTokenKind::RightCurlyBrace, "}"),
        (KauboTokenKind::LeftSquareBracket, "["),
        (KauboTokenKind::RightSquareBracket, "]"),
        (KauboTokenKind::Dot, "."),
        (KauboTokenKind::Pipe, "|"),
        (KauboTokenKind::Equal, "="),
    ] {
        lexer.register_machine(
            build_single_char_machine(kind, operator.chars().next().unwrap()).unwrap(),
        );
    }

    for (kind, keyword) in vec![
        (KauboTokenKind::Var, "var"),
        (KauboTokenKind::If, "if"),
        (KauboTokenKind::Else, "else"),
        (KauboTokenKind::Elif, "elif"),
        (KauboTokenKind::While, "while"),
        (KauboTokenKind::For, "for"),
        (KauboTokenKind::Return, "return"),
        (KauboTokenKind::In, "in"),
        (KauboTokenKind::Yield, "yield"),
        (KauboTokenKind::True, "true"),
        (KauboTokenKind::False, "false"),
        (KauboTokenKind::Null, "null"),
        (KauboTokenKind::Break, "break"),
        (KauboTokenKind::Continue, "continue"),
        (KauboTokenKind::Struct, "struct"),
        (KauboTokenKind::Interface, "interface"),
        (KauboTokenKind::Import, "import"),
        (KauboTokenKind::As, "as"),
        (KauboTokenKind::From, "from"),
        (KauboTokenKind::Pass, "pass"),
        (KauboTokenKind::And, "and"),
        (KauboTokenKind::Or, "or"),
        (KauboTokenKind::Not, "not"),
        (KauboTokenKind::Async, "async"),
        (KauboTokenKind::Await, "await"),
    ] {
        lexer.register_machine(build_keyword_machine(keyword, kind).unwrap());
    }

    lexer.register_machine(build_integer_machine(KauboTokenKind::LiteralInteger).unwrap());
    lexer.register_machine(build_string_machine(KauboTokenKind::LiteralString).unwrap());
    lexer.register_machine(build_identifier_machine(KauboTokenKind::Identifier).unwrap());

    lexer.register_machine(build_newline_machine(KauboTokenKind::NewLine).unwrap());
    lexer.register_machine(build_whitespace_machine(KauboTokenKind::Whitespace).unwrap());
    lexer.register_machine(build_tab_machine(KauboTokenKind::Tab).unwrap());
    lexer.register_machine(build_comment_machine(KauboTokenKind::Comment).unwrap());

    return lexer;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_lexer() {
        // 确保 lexer 被正确创建（能成功创建即表示成功）
        let _lexer = build_lexer();
    }

    #[test]
    fn test_lexer_tokenizes_keywords() {
        let mut lexer = build_lexer();
        let code = "var if else while for return true false null";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let expected = vec![
            KauboTokenKind::Var,
            KauboTokenKind::If,
            KauboTokenKind::Else,
            KauboTokenKind::While,
            KauboTokenKind::For,
            KauboTokenKind::Return,
            KauboTokenKind::True,
            KauboTokenKind::False,
            KauboTokenKind::Null,
        ];

        for expected_kind in expected {
            let token = lexer.next_token();
            assert!(token.is_some(), "Expected token {:?}", expected_kind);
            assert_eq!(token.unwrap().kind, expected_kind);
        }
    }

    #[test]
    fn test_lexer_tokenizes_operators() {
        let mut lexer = build_lexer();
        let code = "+ - * / == != <= >= = < >";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let expected = vec![
            KauboTokenKind::Plus,
            KauboTokenKind::Minus,
            KauboTokenKind::Asterisk,
            KauboTokenKind::Slash,
            KauboTokenKind::DoubleEqual,
            KauboTokenKind::ExclamationEqual,
            KauboTokenKind::LessThanEqual,
            KauboTokenKind::GreaterThanEqual,
            KauboTokenKind::Equal,
            KauboTokenKind::LessThan,
            KauboTokenKind::GreaterThan,
        ];

        for expected_kind in expected {
            let token = lexer.next_token();
            assert!(token.is_some(), "Expected token {:?}", expected_kind);
            assert_eq!(token.unwrap().kind, expected_kind);
        }
    }

    #[test]
    fn test_lexer_tokenizes_delimiters() {
        let mut lexer = build_lexer();
        let code = "( ) { } [ ] ; , . |";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let expected = vec![
            KauboTokenKind::LeftParenthesis,
            KauboTokenKind::RightParenthesis,
            KauboTokenKind::LeftCurlyBrace,
            KauboTokenKind::RightCurlyBrace,
            KauboTokenKind::LeftSquareBracket,
            KauboTokenKind::RightSquareBracket,
            KauboTokenKind::Semicolon,
            KauboTokenKind::Comma,
            KauboTokenKind::Dot,
            KauboTokenKind::Pipe,
        ];

        for expected_kind in expected {
            let token = lexer.next_token();
            assert!(token.is_some(), "Expected token {:?}", expected_kind);
            assert_eq!(token.unwrap().kind, expected_kind);
        }
    }

    #[test]
    fn test_lexer_tokenizes_integer() {
        let mut lexer = build_lexer();
        let code = "12345";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let token = lexer.next_token();
        assert!(token.is_some());
        assert_eq!(token.unwrap().kind, KauboTokenKind::LiteralInteger);
    }

    #[test]
    fn test_lexer_tokenizes_string() {
        let mut lexer = build_lexer();
        let code = r#""hello world""#;
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let token = lexer.next_token();
        assert!(token.is_some());
        assert_eq!(token.unwrap().kind, KauboTokenKind::LiteralString);
    }

    #[test]
    fn test_lexer_tokenizes_identifier() {
        let mut lexer = build_lexer();
        let code = "my_variable _private test123";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        for _ in 0..3 {
            let token = lexer.next_token();
            assert!(token.is_some());
            assert_eq!(token.unwrap().kind, KauboTokenKind::Identifier);
        }
    }

    #[test]
    fn test_lexer_tokenizes_comment() {
        let mut lexer = build_lexer();
        let code = "// this is a comment\nvar x;";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        // 跳过注释和换行，应该得到 var
        let token = lexer.next_token();
        assert!(token.is_some());
        assert_eq!(token.unwrap().kind, KauboTokenKind::Var);
    }

    #[test]
    fn test_lexer_tokenizes_whitespace() {
        let mut lexer = build_lexer();
        let code = "  \t\nvar";
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        // 空白和制表符应该被识别
        let token = lexer.next_token();
        assert!(token.is_some());
        // 跳过空白后应该得到 var
        assert_eq!(token.unwrap().kind, KauboTokenKind::Var);
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
        ];

        for (code, expected) in cases {
            let mut lexer = build_lexer();
            let _ = lexer.feed(&code.as_bytes().to_vec());
            let _ = lexer.terminate();

            let token = lexer.next_token();
            assert!(token.is_some(), "Failed to tokenize '{}'", code);
            assert_eq!(token.unwrap().kind, expected, "Wrong token kind for '{}'", code);
        }
    }
}
