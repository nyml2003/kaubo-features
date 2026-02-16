//! 标准库测试
//!
//! 测试 std 模块的功能

mod common;
use common::{get_float, get_int, get_string, run_code};

// ===== 基础函数测试 =====

#[test]
fn test_std_type() {
    // 整数类型
    let result = run_code(
        r#"
        import std;
        return std.type(123);
    "#,
    )
    .unwrap();
    assert_eq!(get_string(&result), Some("int".to_string()));

    // 字符串类型
    let result = run_code(
        r#"
        import std;
        return std.type("hello");
    "#,
    )
    .unwrap();
    assert_eq!(get_string(&result), Some("string".to_string()));

    // 布尔类型
    let result = run_code(
        r#"
        import std;
        return std.type(true);
    "#,
    )
    .unwrap();
    assert_eq!(get_string(&result), Some("bool".to_string()));

    // null 类型
    let result = run_code(
        r#"
        import std;
        return std.type(null);
    "#,
    )
    .unwrap();
    assert_eq!(get_string(&result), Some("null".to_string()));
}

#[test]
fn test_std_to_string() {
    let result = run_code(
        r#"
        import std;
        return std.to_string(123);
    "#,
    )
    .unwrap();
    assert_eq!(get_string(&result), Some("123".to_string()));
}

// ===== 数学函数测试 =====

#[test]
fn test_std_sqrt() {
    let result = run_code(
        r#"
        import std;
        return std.sqrt(16.0);
    "#,
    )
    .unwrap();
    assert_eq!(get_float(&result), Some(4.0));

    let result = run_code(
        r#"
        import std;
        return std.sqrt(2.0);
    "#,
    )
    .unwrap();
    let value = get_float(&result).unwrap();
    assert!((value - 1.414).abs() < 0.01);
}

#[test]
fn test_std_sin_cos() {
    // sin(0) = 0
    let result = run_code(
        r#"
        import std;
        return std.sin(0.0);
    "#,
    )
    .unwrap();
    let value = get_float(&result).unwrap();
    assert!(value.abs() < 0.0001);

    // cos(0) = 1
    let result = run_code(
        r#"
        import std;
        return std.cos(0.0);
    "#,
    )
    .unwrap();
    let value = get_float(&result).unwrap();
    assert!((value - 1.0).abs() < 0.0001);
}

#[test]
fn test_std_floor_ceil() {
    let result = run_code("import std; return std.floor(3.1);").unwrap();
    assert!(result.return_value.is_some(), "Should have return value");

    let result = run_code("import std; return std.ceil(2.9);").unwrap();
    assert!(result.return_value.is_some(), "Should have return value");
}

// ===== 数学常量测试 =====

#[test]
fn test_std_pi() {
    let result = run_code(
        r#"
        import std;
        return std.PI;
    "#,
    )
    .unwrap();
    let value = get_float(&result).unwrap();
    assert!((value - std::f64::consts::PI).abs() < 0.0001);
}

#[test]
fn test_std_e() {
    let result = run_code(
        r#"
        import std;
        return std.E;
    "#,
    )
    .unwrap();
    let value = get_float(&result).unwrap();
    assert!((value - 2.71828).abs() < 0.0001);
}

// ===== 综合测试 =====

#[test]
fn test_std_combined() {
    // 使用 std 计算圆的面积
    let result = run_code(
        r#"
        import std;
        var circle_area = |r| {
            return std.PI * r * r;
        };
        return circle_area(5.0);
    "#,
    )
    .unwrap();
    let value = get_float(&result).unwrap();
    assert!((value - 78.54).abs() < 0.01);
}

#[test]
fn test_std_pythagorean() {
    // 使用 std 计算勾股定理
    let result = run_code(
        r#"
        import std;
        var hypotenuse = |a, b| {
            return std.sqrt((a * a + b * b) as float);
        };
        return hypotenuse(3, 4);
    "#,
    )
    .unwrap();
    assert_eq!(get_float(&result), Some(5.0));
}

// ===== 断言测试 =====

#[test]
fn test_std_assert_success() {
    // 断言成功不应该报错
    let result = run_code(
        r#"
        import std;
        std.assert(true);
        return 1;
    "#,
    );
    if let Err(ref e) = result {
        eprintln!("Error: {e}");
    }
    assert!(result.is_ok());
    assert_eq!(get_int(&result.unwrap()), Some(1));
}

#[test]
fn test_std_assert_failure() {
    // 断言失败应该运行时错误
    let result = run_code(
        r#"
        import std;
        std.assert(false);
        return 1;
    "#,
    );
    assert!(result.is_err());
}
