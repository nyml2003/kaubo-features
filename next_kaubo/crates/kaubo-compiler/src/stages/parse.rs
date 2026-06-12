//! ParseStage — 语法分析
//!
//! Source text → AST Module

use crate::parser::{Module, Parser};
use crate::lexer::v2::Lexer as LexerV2;

pub struct ParseStage;

impl ParseStage {
    pub fn new() -> Self { Self }

    pub fn run(&self, source: &str) -> Result<Module, String> {
        let mut lexer = LexerV2::new(4096);
        lexer.feed(source.as_bytes()).map_err(|e| format!("lex: {:?}", e))?;
        lexer.terminate().map_err(|e| format!("lex: {:?}", e))?;
        let mut parser = Parser::new(lexer);
        parser.parse().map_err(|e| format!("parse: {:?}", e))
    }
}

impl kaubo_pipeline::Stage<String, Module> for ParseStage {
    fn name(&self) -> &'static str { "Parser" }
    fn run(&self, input: String, _ctx: &kaubo_pipeline::PipelineCtx) -> Result<Module, String> {
        self.run(&input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let stage = ParseStage::new();
        let m = stage.run("var x = 1;").unwrap();
        assert!(m.statements.len() > 0);
    }
}
