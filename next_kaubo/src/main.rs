mod compiler;
mod kit;
use std::fs;

use crate::compiler::parser::parser::Parser;
fn main() {
    let mut lexer = compiler::builder::build_lexer();
    let content =
        fs::read_to_string(r"C:\Users\nyml\code\kaubo-features\next_kaubo\assets\a.txt").unwrap();
    let _ = lexer.feed(&content.as_bytes().to_vec());
    let _ = lexer.terminate();
    let mut parser = Parser::new(lexer);
    let ast = parser.parse();
    println!("{:?}", ast);
}
