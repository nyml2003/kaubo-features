mod kit;
use crate::kit::lexer::state_machine::builder::build_keyword_machine;
fn main() {
    #[derive(Debug, Clone, PartialEq)]
    enum Token {
        Null,
    }
    let mut machine = build_keyword_machine("null", Token::Null).unwrap();
    println!("Hello, world!");
    let input = "nuxnull";
    for c in input.chars() {
        let state = machine.process_event(c);
        println!("{:?}", state)
    }
}
