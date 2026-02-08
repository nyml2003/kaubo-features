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
for (item) in (list) {
    print(item);
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
