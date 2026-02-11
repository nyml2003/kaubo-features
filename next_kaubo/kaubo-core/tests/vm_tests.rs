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

// ===== 分支覆盖测试 =====

#[test]
fn test_constants_0_to_15() {
    // 测试 LoadConst0-15 内联常量路径
    for i in 0..=15 {
        let code = format!("return {};", i);
        let result = run_code(&code).unwrap();
        assert_eq!(get_int(&result), Some(i), "LoadConst{} failed", i);
    }
}

#[test]
fn test_constants_16_and_above() {
    // 测试 LoadConst (u8 索引) 路径
    let result = run_code("return 16;").unwrap();
    assert_eq!(get_int(&result), Some(16));
    
    let result = run_code("return 100;").unwrap();
    assert_eq!(get_int(&result), Some(100));
    
    let result = run_code("return 255;").unwrap();
    assert_eq!(get_int(&result), Some(255));
}

#[test]
fn test_special_values() {
    // 测试 LoadNull
    let result = run_code("return null;").unwrap();
    assert!(result.return_value.unwrap().is_null());
    
    // 测试 LoadTrue/LoadFalse
    let result = run_code("return true;").unwrap();
    assert!(result.return_value.unwrap().is_true());
    
    let result = run_code("return false;").unwrap();
    assert!(result.return_value.unwrap().is_false());
}

#[test]
fn test_local_variables_all_slots() {
    // 测试局部变量槽位 0-7 (优化路径) 和 >7 (一般路径)
    let code = r#"
        var v0 = 0; var v1 = 1; var v2 = 2; var v3 = 3;
        var v4 = 4; var v5 = 5; var v6 = 6; var v7 = 7;
        var v8 = 8; var v9 = 9; var v10 = 10;
        return v0 + v1 + v2 + v3 + v4 + v5 + v6 + v7 + v8 + v9 + v10;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(55)); // 0+1+2+...+10
}

#[test]
fn test_global_variables() {
    // 测试全局变量定义和访问
    let code = r#"
        var global_var = 42;
        return global_var;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(42));
}

#[test]
fn test_comparison_all_operators() {
    // 测试所有比较运算符的分支
    // ==
    let result = run_code("return 5 == 5;").unwrap();
    assert!(result.return_value.unwrap().is_true());
    let result = run_code("return 5 == 3;").unwrap();
    assert!(result.return_value.unwrap().is_false());
    
    // !=
    let result = run_code("return 5 != 3;").unwrap();
    assert!(result.return_value.unwrap().is_true());
    
    // >
    let result = run_code("return 5 > 3;").unwrap();
    assert!(result.return_value.unwrap().is_true());
    let result = run_code("return 3 > 5;").unwrap();
    assert!(result.return_value.unwrap().is_false());
    
    // <
    let result = run_code("return 3 < 5;").unwrap();
    assert!(result.return_value.unwrap().is_true());
    let result = run_code("return 5 < 3;").unwrap();
    assert!(result.return_value.unwrap().is_false());
    
    // Note: >= and <= opcodes are not yet implemented
    // Uncomment when implemented:
    // >=
    // let result = run_code("return 5 >= 5;").unwrap();
    // assert!(result.return_value.unwrap().is_true());
    // let result = run_code("return 5 >= 3;").unwrap();
    // assert!(result.return_value.unwrap().is_true());
    
    // <=
    // let result = run_code("return 3 <= 3;").unwrap();
    // assert!(result.return_value.unwrap().is_true());
    // let result = run_code("return 3 <= 5;").unwrap();
    // assert!(result.return_value.unwrap().is_true());
}

#[test]
fn test_if_else_both_branches() {
    // 测试 if-else 两个分支
    let code = r#"
        var x = 5;
        if (x > 3) {
            x = 10;
        } else {
            x = 20;
        }
        return x;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(10));
    
    // else 分支
    let code = r#"
        var x = 1;
        if (x > 3) {
            x = 10;
        } else {
            x = 20;
        }
        return x;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(20));
}

#[test]
fn test_nested_if() {
    // 测试嵌套 if
    let code = r#"
        var x = 5;
        var y = 10;
        if (x > 3) {
            if (y > 8) {
                return 1;
            } else {
                return 2;
            }
        } else {
            return 3;
        }
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(1));
}

#[test]
fn test_while_zero_iterations() {
    // 测试 0 次循环
    let code = r#"
        var sum = 0;
        var i = 10;
        while (i < 5) {
            sum = sum + i;
            i = i + 1;
        }
        return sum;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(0));
}

#[test]
fn test_while_multiple_iterations() {
    // 测试多次循环
    let code = r#"
        var sum = 0;
        var i = 0;
        while (i < 10) {
            sum = sum + i;
            i = i + 1;
        }
        return sum;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(45)); // 0+1+2+...+9
}

#[test]
fn test_list_operations() {
    // 测试空列表
    let result = run_code("return [];").unwrap();
    assert!(result.return_value.unwrap().is_list());
    
    // 测试索引赋值
    let code = r#"
        var list = [1, 2, 3];
        list[1] = 99;
        return list[1];
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(99));
}

#[test]
fn test_for_in_loop() {
    // 测试 for-in 循环 (语法: for var x in list)
    let code = r#"
        var sum = 0;
        for var x in [1, 2, 3, 4, 5] {
            sum = sum + x;
        }
        return sum;
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(15));
}

#[test]
fn test_function_no_params() {
    // 测试无参数函数 (使用 lambda 语法)
    let code = r#"
        var getFive = || { return 5; };
        return getFive();
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(5));
}

#[test]
fn test_function_multiple_params() {
    // 测试多参数函数 (使用 lambda 语法)
    let code = r#"
        var add = |a, b, c| { return a + b + c; };
        return add(1, 2, 3);
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(6));
}

#[test]
fn test_nested_function_calls() {
    // 测试嵌套函数调用 (使用 lambda 语法)
    let code = r#"
        var add = |a, b| { return a + b; };
        var mul = |a, b| { return a * b; };
        return add(mul(2, 3), 4);
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(10)); // 2*3 + 4 = 10
}

#[test]
fn test_truthy_values() {
    // 测试真值判断：只有 false 和 null 为假
    let code = r#"
        if (0) { return 1; } else { return 0; }
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(1)); // 0 是真值
    
    // Note: Empty string "" causes parser overflow, skip for now
    // let code = r#"
    //     if ("") { return 1; } else { return 0; }
    // "#;
    // let result = run_code(code).unwrap();
    // assert_eq!(get_int(&result), Some(1)); // 空字符串是真值
    
    let code = r#"
        if (null) { return 1; } else { return 0; }
    "#;
    let result = run_code(code).unwrap();
    assert_eq!(get_int(&result), Some(0)); // null 是假值
}

#[test]
fn test_not_operator() {
    // 测试 not 运算符的各种情况
    let code = r#"
        return not true;
    "#;
    let result = run_code(code).unwrap();
    assert!(result.return_value.unwrap().is_false());
    
    let code = r#"
        return not false;
    "#;
    let result = run_code(code).unwrap();
    assert!(result.return_value.unwrap().is_true());
    
    let code = r#"
        return not 0;
    "#;
    let result = run_code(code).unwrap();
    assert!(result.return_value.unwrap().is_false());
    
    let code = r#"
        return not null;
    "#;
    let result = run_code(code).unwrap();
    assert!(result.return_value.unwrap().is_true());
}
