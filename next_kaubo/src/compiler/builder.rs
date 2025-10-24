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
        (KauboTokenKind::DoubleEqual, "=="),
        (KauboTokenKind::LessThanEqual, "<="),
        (KauboTokenKind::GreaterThanEqual, ">="),
        (KauboTokenKind::ExclamationEqual, "!="),
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
