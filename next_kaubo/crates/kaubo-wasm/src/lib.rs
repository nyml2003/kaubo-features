use kaubo_syntax::lexer::Lexer;
use kaubo_web_api::token::{classify_token, describe_token, utf16_range};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static COMPILED: Lazy<Mutex<Option<kaubo_driver::CpsModule>>> = Lazy::new(|| Mutex::new(None));

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Tokenize source, return JSON array of {kind, from, to}.
#[wasm_bindgen]
pub fn lex(source: &str) -> String {
    let tokens = Lexer::new(source).tokenize();
    let items: Vec<String> = tokens
        .iter()
        .filter(|t| {
            !matches!(
                t.kind,
                kaubo_syntax::TokenKind::Whitespace | kaubo_syntax::TokenKind::Eof
            )
        })
        .map(|t| {
            let kind = classify_token(t.kind);
            let (from, to) = utf16_range(source, t.line, t.col, &t.lexeme);
            format!(r#"{{"kind":"{}","from":{},"to":{}}}"#, kind, from, to)
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Parse + type-check, return JSON error array or "[]".
#[wasm_bindgen]
pub fn diagnose(source: &str) -> String {
    kaubo_web_api::diagnose::diagnose(source)
}

/// Compile source to bytecode, return instruction count.
/// Throws JsValue on parse/infer/build failure.
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<usize, JsValue> {
    let cps =
        kaubo_driver::compile_source(source).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if cps.functions.is_empty() {
        return Err(JsValue::from_str("no functions in compiled module"));
    }

    let count = kaubo_driver::instruction_count(&cps);

    *COMPILED.lock().unwrap() = Some(cps);
    Ok(count)
}

/// Run previously compiled bytecode, return print() output.
/// Throws JsValue on execution failure or if nothing was compiled.
#[wasm_bindgen]
pub fn run(_bytes: &[u8]) -> Result<String, JsValue> {
    let cps = COMPILED
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| JsValue::from_str("no compiled module"))?;

    let outcome = kaubo_driver::run_module(&cps).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = outcome.output.join("\n");

    // Re-store for potential re-use
    COMPILED.lock().unwrap().replace(cps);

    Ok(out)
}

/// Get hover information for token at UTF-16 offset.
#[wasm_bindgen]
pub fn hover(source: &str, offset: usize) -> String {
    let tokens = Lexer::new(source).tokenize();
    for t in &tokens {
        let (from, to) = utf16_range(source, t.line, t.col, &t.lexeme);
        if offset >= from && offset < to {
            return serde_json::json!({
                "kind": classify_token(t.kind),
                "from": from,
                "to": to,
                "description": describe_token(t.kind),
            })
            .to_string();
        }
    }
    "null".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_struct_tokens_have_non_overlapping_ranges() {
        let raw = lex("struct Point { x: Int64 }");
        let tokens: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap();

        assert_eq!(tokens[0]["kind"], "keyword");
        assert_eq!(tokens[0]["from"], 0);
        assert_eq!(tokens[0]["to"], 6);
        assert_eq!(tokens[1]["kind"], "identifier");
        assert_eq!(tokens[1]["from"], 7);
        assert_eq!(tokens[1]["to"], 12);
        assert_eq!(tokens[2]["kind"], "operator");
        assert_eq!(tokens[2]["from"], 13);
        assert_eq!(tokens[2]["to"], 14);

        for pair in tokens.windows(2) {
            let previous_to = pair[0]["to"].as_u64().unwrap();
            let next_from = pair[1]["from"].as_u64().unwrap();
            assert!(
                previous_to <= next_from,
                "overlapping token ranges in {raw}"
            );
        }
    }
}
