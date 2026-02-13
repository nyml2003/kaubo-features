//! 字符串转义序列测试

mod common;
use common::{get_string, run_code};

#[test]
fn test_string_escape_newline() {
    let code = r#"
        return "hello\nworld";
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("hello\nworld".to_string()));
}

#[test]
fn test_string_escape_tab() {
    let code = r#"
        return "hello\tworld";
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("hello\tworld".to_string()));
}

#[test]
fn test_string_escape_quote() {
    let code = r#"
        return "say \"hello\"";
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("say \"hello\"".to_string()));
}

#[test]
fn test_string_escape_backslash() {
    let code = r#"
        return "path\\to\\file";
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("path\\to\\file".to_string()));
}

#[test]
fn test_string_escape_carriage_return() {
    let code = r#"
        return "hello\rworld";
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("hello\rworld".to_string()));
}

#[test]
fn test_string_escape_multiple() {
    let code = r#"
        return "line1\nline2\nline3";
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("line1\nline2\nline3".to_string()));
}

#[test]
fn test_string_escape_single_quote() {
    let code = r#"
        return 'it\'s working';
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_string(&result), Some("it's working".to_string()));
}
