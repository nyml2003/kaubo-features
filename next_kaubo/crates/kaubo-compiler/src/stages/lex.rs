//! LexStage — 词法分析
//!
//! Source text → TokenStream

use kaubo_ir::value::Value;
use crate::lexer::v2::Lexer as LexerV2;

pub struct LexStage;

/// Lex stage: source text → token stream (as formatted string)
#[derive(Debug, Clone)]
pub struct TokenStream {
    pub tokens: Vec<String>,
    pub count: usize,
}

impl LexStage {
    pub fn new() -> Self { Self }

    pub fn run(&self, source: &str) -> Result<TokenStream, String> {
        let mut lexer = LexerV2::new(4096);
        lexer.feed(source.as_bytes()).map_err(|e| format!("lexer feed: {:?}", e))?;
        lexer.terminate().map_err(|e| format!("lexer terminate: {:?}", e))?;

        let mut tokens = Vec::new();
        while let Some(t) = lexer.next_token() {
            tokens.push(format!("{:?}", t.kind));
        }
        let count = tokens.len();
        Ok(TokenStream { tokens, count })
    }
}

impl kaubo_pipeline::Stage<String, TokenStream> for LexStage {
    fn name(&self) -> &'static str { "Lexer" }
    fn run(&self, input: String, _ctx: &kaubo_pipeline::PipelineCtx) -> Result<TokenStream, String> {
        self.run(&input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple() {
        let stage = LexStage::new();
        let result = stage.run("var x = 1;").unwrap();
        assert!(result.count > 0);
        assert_eq!(result.tokens[0], "Var");
    }
}
