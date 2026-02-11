//! VM 执行测试
//!
//! 端到端测试：编译并执行 Kaubo 代码

mod common;
use common::{get_float, get_int, run_code};

// ===== 基础运算测试 =====

#[test]
fn test_basic_arithmetic() {
    // 加法
    let result = run_code("return 1 + 2;").unwrap();
    assert_eq!(get_int(&result), Some(3i32));

    // 减法
    let result = run_code("return 10 - 3;").unwrap();
    assert_eq!(get_int(&result), Some(7));

    // 乘法
    let result = run_code("return 4 * 5;").unwrap();
    assert_eq!(get_int(&result), Some(20));

    // 除法 - 结果为浮点数
    let result = run_code("return 20 / 4;").unwrap();
    assert_eq!(get_float(&result), Some(5.0));
}

#[test]
fn test_operator_precedence() {
    // 先乘除后加减
    let result = run_code("return 2 + 3 * 4;").unwrap();
    assert_eq!(get_int(&result), Some(14));

    // 括号改变优先级
    let result = run_code("return (2 + 3) * 4;").unwrap();
    assert_eq!(get_int(&result), Some(20));
}

#[test]
fn test_unary_operators() {
    // 负号
    let result = run_code("return -5;").unwrap();
    assert_eq!(get_int(&result), Some(-5));

    // 双重负号
    let result = run_code("return --5;").unwrap();
    assert_eq!(get_int(&result), Some(5));

    // 非运算
    let result = run_code("return not true;").unwrap();
    assert!(result.return_value.unwrap().is_false());
}

// ===== 变量测试 =====

#[test]
fn test_variable_declaration() {
    let result = run_code(r#"
        var x = 42;
        return x;
    "#).unwrap();
    assert_eq!(get_int(&result), Some(42));
}

#[test]
fn test_variable_assignment() {
    let result = run_code(r#"
        var x = 10;
        x = 20;
        return x;
    "#).unwrap();
    assert_eq!(get_int(&result), Some(20));
}

// ===== 函数测试 =====

#[test]
fn test_lambda_call() {
    let result = run_code(r#"
        var add = |a, b| {
            return a + b;
        };
        return add(3, 4);
    "#).unwrap();
    assert_eq!(get_int(&result), Some(7));
}

#[test]
fn test_closure() {
    let result = run_code(r#"
        var make_counter = || {
            var count = 0;
            return || {
                count = count + 1;
                return count;
            };
        };
        var counter = make_counter();
        counter();
        counter();
        return counter();
    "#).unwrap();
    assert_eq!(get_int(&result), Some(3i32));
}

// ===== 条件测试 =====

#[test]
fn test_if_statement() {
    let result = run_code(r#"
        var x = 5;
        if (x > 3) {
            return 1;
        }
        return 0;
    "#).unwrap();
    assert_eq!(get_int(&result), Some(1));
}

#[test]
fn test_if_else() {
    let result = run_code(r#"
        var x = 2;
        if (x > 3) {
            return 1;
        } else {
            return 0;
        }
    "#).unwrap();
    assert_eq!(get_int(&result), Some(0));
}

// ===== 循环测试 =====

#[test]
fn test_while_loop() {
    let result = run_code(r#"
        var sum = 0;
        var i = 1;
        while (i <= 5) {
            sum = sum + i;
            i = i + 1;
        }
        return sum;
    "#).unwrap();
    assert_eq!(get_int(&result), Some(15)); // 1+2+3+4+5
}

// ===== 列表测试 =====

#[test]
fn test_list_creation() {
    let result = run_code(r#"
        var list = [1, 2, 3];
        return list[0];
    "#).unwrap();
    assert_eq!(get_int(&result), Some(1));
}

#[test]
fn test_list_index() {
    let result = run_code(r#"
        var list = [10, 20, 30];
        return list[1];
    "#).unwrap();
    assert_eq!(get_int(&result), Some(20));
}

// ===== 递归测试 =====

// TODO: 递归需要函数能引用自身（通过闭包或命名函数）
// #[test]
// fn test_factorial() {
//     let result = run_code(r#"
//         var factorial = |n| {
//             if (n <= 1) { return 1; }
//             return n * factorial(n - 1);
//         };
//         return factorial(5);
//     "#).unwrap();
//     assert_eq!(get_int(&result), Some(120));
// }

// ===== 比较运算测试 =====

#[test]
fn test_comparison_operators() {
    // 等于
    let result = run_code("return 5 == 5;").unwrap();
    assert!(result.return_value.unwrap().is_true());

    // 不等于
    let result = run_code("return 5 != 3;").unwrap();
    assert!(result.return_value.unwrap().is_true());

    // 大于
    let result = run_code("return 5 > 3;").unwrap();
    assert!(result.return_value.unwrap().is_true());

    // 小于
    let result = run_code("return 3 < 5;").unwrap();
    assert!(result.return_value.unwrap().is_true());
}

// ===== 逻辑运算测试 =====

// TODO: 逻辑操作需要编译器支持短路求值
// #[test]
// fn test_logical_and() {
//     let result = run_code("return true and true;").unwrap();
//     assert!(result.return_value.unwrap().is_true());
// }

// #[test]
// fn test_logical_or() {
//     let result = run_code("return false or true;").unwrap();
//     assert!(result.return_value.unwrap().is_true());
// }
