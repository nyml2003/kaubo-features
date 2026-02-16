//! 短路求值测试

mod common;
use common::{get_int, run_code};

#[test]
fn test_and_short_circuit_false() {
    // false and x -> false，x 不会执行
    // 使用 1/0 会产生运行时错误，如果执行了的话
    let code = r#"
        return false and (1 / 0);
    "#;
    let result = run_code(code).unwrap();
    // 应该返回 false，且没有除零错误
    assert!(result.return_value.unwrap().is_false());
}

#[test]
fn test_and_short_circuit_true() {
    // true and x -> x，x 会执行
    let code = r#"
        return true and 42;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(42));
}

#[test]
fn test_or_short_circuit_true() {
    // true or x -> true，x 不会执行
    // 使用 1/0 会产生运行时错误，如果执行了的话
    let code = r#"
        return true or (1 / 0);
    "#;
    let result = run_code(code).unwrap();
    // 应该返回 true，且没有除零错误
    assert!(result.return_value.unwrap().is_true());
}

#[test]
fn test_or_short_circuit_false() {
    // false or x -> x，x 会执行
    let code = r#"
        return false or 42;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(42));
}

#[test]
fn test_and_left_false_returns_left() {
    // false and 42 -> false (假值短路)
    // 注意：在 Kaubo 中，0 是真值，只有 false/null 是假值
    let code = r#"
        return false and 42;
    "#;
    let result = run_code(code).unwrap();
    // 检查返回的是 false（Value::FALSE）
    let value = result.return_value.unwrap();
    assert!(value.is_false(), "Expected false, got {value:?}");
}

#[test]
fn test_or_left_true_returns_left() {
    // true or 42 -> true (真值短路)
    // 或者使用 1 or 42 -> 1
    let code = r#"
        return 1 or 42;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(1));
}

#[test]
fn test_complex_short_circuit() {
    // (true or x) and false -> false
    // 第一个 or 短路，返回 true
    // 然后 true and false -> false
    let code = r#"
        return (true or (1 / 0)) and false;
    "#;
    let result = run_code(code).unwrap();
    assert!(result.return_value.unwrap().is_false());
}

#[test]
fn test_chained_and() {
    // true and true and 42 -> 42
    let code = r#"
        return true and true and 42;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(42));
}

#[test]
fn test_chained_or() {
    // false or false or 42 -> 42
    let code = r#"
        return false or false or 42;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(42));
}

#[test]
fn test_mixed_short_circuit() {
    // false and (1/0) or 42 -> 42
    // false and (1/0) -> false（短路，1/0 不执行）
    // false or 42 -> 42
    let code = r#"
        return false and (1 / 0) or 42;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(42));
}
