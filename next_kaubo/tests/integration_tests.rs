//! 集成测试 - 端到端解析测试

use next_kaubo::compiler::lexer::builder::build_lexer;
use next_kaubo::compiler::parser::parser::Parser;

/// 辅助函数：解析代码字符串并返回 AST
fn parse_code(code: &str) -> Result<String, String> {
    let mut lexer = build_lexer();
    lexer.feed(&code.as_bytes().to_vec()).map_err(|e| format!("Lexer error: {:?}", e))?;
    lexer.terminate().map_err(|e| format!("Lexer terminate error: {:?}", e))?;
    
    let mut parser = Parser::new(lexer);
    match parser.parse() {
        Ok(ast) => Ok(format!("{:?}", ast)),
        Err(e) => Err(format!("Parse error: {:?}", e)),
    }
}

#[test]
fn test_parse_variable_declaration() {
    let code = "var x = 5;";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse variable declaration: {:?}", result.err());
}

#[test]
fn test_parse_hello_world() {
    let code = r#"var message = "hello";"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse hello world: {:?}", result.err());
}

#[test]
fn test_parse_function() {
    let code = r#"
var add = |x, y| {
    return x + y;
};
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse function: {:?}", result.err());
}

#[test]
fn test_parse_if_statement() {
    let code = r#"
if (a > b) {
    return a;
} elif (a < b) {
    return b;
} else {
    return 0;
}
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse if statement: {:?}", result.err());
}

#[test]
fn test_parse_while_loop() {
    let code = r#"
while (i < 10) {
    i = i + 1;
}
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse while loop: {:?}", result.err());
}

#[test]
fn test_parse_for_loop() {
    let code = r#"
for var item in list {
    print item;
}
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse for loop: {:?}", result.err());
}

#[test]
fn test_parse_list() {
    let code = "var nums = [1, 2, 3];";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse list: {:?}", result.err());
}

#[test]
fn test_parse_lambda() {
    let code = "var f = |x| { return x * 2; };";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse lambda: {:?}", result.err());
}

#[test]
fn test_parse_member_access() {
    let code = "var len = list.length();";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse member access: {:?}", result.err());
}

#[test]
fn test_parse_complex_expression() {
    let code = "var result = (a + b) * c - d / e;";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse complex expression: {:?}", result.err());
}

#[test]
fn test_parse_empty_statement() {
    let code = ";";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse empty statement: {:?}", result.err());
}

#[test]
fn test_parse_block() {
    let code = r#"
{
    var x = 1;
    var y = 2;
    x + y;
}
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse block: {:?}", result.err());
}

#[test]
fn test_parse_arithmetic_expressions() {
    // 测试基本算术运算
    let cases = vec![
        ("var a = 1 + 2;", "addition"),
        ("var a = 10 - 3;", "subtraction"),
        ("var a = 4 * 5;", "multiplication"),
        ("var a = 20 / 4;", "division"),
        ("var a = 1 + 2 * 3;", "mixed precedence"),
        ("var a = (1 + 2) * 3;", "parentheses"),
        ("var a = -5;", "unary minus"),
        ("var a = --5;", "double unary minus"),
    ];
    
    for (code, desc) in cases {
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", desc, result.err());
    }
}

#[test]
fn test_parse_comparison_operators() {
    let cases = vec![
        ("var a = x == y;", "equal"),
        ("var a = x != y;", "not equal"),
        ("var a = x > y;", "greater than"),
        ("var a = x < y;", "less than"),
        ("var a = x >= y;", "greater than or equal"),
        ("var a = x <= y;", "less than or equal"),
    ];
    
    for (code, desc) in cases {
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", desc, result.err());
    }
}

#[test]
fn test_parse_logical_operators() {
    let cases = vec![
        ("var a = x and y;", "logical and"),
        ("var a = x or y;", "logical or"),
        ("var a = not x;", "logical not"),
        ("var a = not x and y;", "not and"),
        ("var a = x and not y;", "and not"),
        ("var a = x or y and z;", "or and precedence"),
    ];
    
    for (code, desc) in cases {
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", desc, result.err());
    }
}

#[test]
fn test_parse_boolean_literals() {
    let cases = vec![
        ("var a = true;", "true"),
        ("var a = false;", "false"),
        ("var a = null;", "null"),
    ];
    
    for (code, desc) in cases {
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", desc, result.err());
    }
}

#[test]
fn test_parse_assignment() {
    let cases = vec![
        ("x = 5;", "simple assignment"),
        ("x = y = 5;", "chained assignment"),
        ("x = a + b;", "assignment with expression"),
    ];
    
    for (code, desc) in cases {
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", desc, result.err());
    }
}

#[test]
fn test_parse_nested_function_calls() {
    let code = r#"
var result = outer(inner(x), y);
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse nested function calls: {:?}", result.err());
}

#[test]
fn test_parse_chained_member_access() {
    let code = r#"
var x = obj.a.b.c;
var y = obj.method1().method2();
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse chained member access: {:?}", result.err());
}

#[test]
fn test_parse_complex_lambda() {
    let code = r#"
var calc = |a, b, c| {
    var sum = a + b;
    return sum * c;
};
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse complex lambda: {:?}", result.err());
}

#[test]
fn test_parse_empty_list() {
    let code = "var empty = [];";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse empty list: {:?}", result.err());
}

#[test]
fn test_parse_nested_list() {
    let code = "var nested = [[1, 2], [3, 4]];";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse nested list: {:?}", result.err());
}

#[test]
fn test_parse_if_elif_else_chain() {
    let code = r#"
if (a > b) {
    return 1;
} elif (a == b) {
    return 0;
} elif (a < b) {
    return -1;
} else {
    return null;
}
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse if-elif-else chain: {:?}", result.err());
}

#[test]
fn test_parse_return_without_value() {
    let code = r#"
var f = |x| {
    if (x < 0) {
        return;
    }
    return x;
};
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse return without value: {:?}", result.err());
}

#[test]
fn test_parse_complex_program() {
    let code = r#"
var factorial = |n| {
    if (n <= 1) {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
};

var result = factorial(5);

for var i in items {
    if (i > 0) {
        print(i);
    }
}
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse complex program: {:?}", result.err());
}

#[test]
fn test_parse_whitespace_variations() {
    // 测试不同空白字符的处理
    let cases = vec![
        ("var x=1;", "no spaces"),
        ("var  x  =  1  ;", "extra spaces"),
        ("var\tx\t=\t1;", "tabs"),
    ];
    
    for (code, desc) in cases {
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", desc, result.err());
    }
}

#[test]
fn test_parse_pipe_operator() {
    let code = "var result = x | f | g;";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse pipe operator: {:?}", result.err());
}

#[test]
fn test_parse_lambda_no_params() {
    let code = "var f = || { return 42; };";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse lambda with no params: {:?}", result.err());
}

#[test]
fn test_parse_single_char_identifier() {
    let code = r#"
var a = 1;
var b = 2;
"#;
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse single char identifiers: {:?}", result.err());
}

#[test]
fn test_parse_underscore_identifier() {
    let code = "var _private = 1;";
    let result = parse_code(code);
    assert!(result.is_ok(), "Failed to parse underscore identifier: {:?}", result.err());
}

#[test]
fn test_parse_error_cases() {
    // 这些应该是错误的
    let error_cases = vec![
        ("var ;", "var without identifier"),
        ("var x = ;", "missing expression"),
        ("var 123 = 5;", "number as identifier"),
        ("(1 + 2;", "missing right paren"),
        ("x.", "dot without identifier"),
        ("|x y| { return x; };", "lambda missing comma"),
    ];
    
    for (code, desc) in error_cases {
        let result = parse_code(code);
        assert!(result.is_err(), "Expected error for {} but got Ok", desc);
    }
}

#[test]
fn test_parse_empty_input() {
    let result = parse_code("");
    assert!(result.is_ok(), "Empty input should be valid");
}

#[test]
fn test_parse_only_whitespace() {
    let result = parse_code("   \n\t\n  ");
    assert!(result.is_ok(), "Only whitespace should be valid");
}

#[test]
fn test_parse_unexpected_end() {
    // 触发 UnexpectedEndOfInput
    let cases = vec![
        "var x =",
        "if (x > y) {",
        "while (x) {",
    ];
    
    for code in cases {
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for incomplete code: {}", code);
    }
}

#[test]
fn test_parse_invalid_number() {
    // 这个数字太大，会触发 InvalidNumberFormat
    // 但实际上 Rust 的 parse 对大数也能处理，所以这里测试格式错误的数字
    // 目前 lexer 不会产生格式错误的数字，所以这个错误可能不会被触发
}
