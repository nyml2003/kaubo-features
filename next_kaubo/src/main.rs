mod compiler;
mod kit;
use std::{fs, thread::sleep, time::Duration};

// Parser is not used currently, but will be used in future
// use crate::compiler::parser::parser::Parser;
fn main() {
    let mut lexer = compiler::lexer::builder::build_lexer();
    let content =
        fs::read_to_string(r"C:\Users\nyml\code\kaubo-features\next_kaubo\assets\a.txt").unwrap();
    let _ = lexer.feed(&content.as_bytes().to_vec());
    let _ = lexer.terminate();
    let mut token = lexer.next_token();
    while token != None {
        println!("{:?}", token);
        sleep(Duration::from_secs(1));
        token = lexer.next_token();
    }
    // let mut parser = Parser::new(lexer);
    // let ast = parser.parse();
    // println!("{:?}", ast);
}
