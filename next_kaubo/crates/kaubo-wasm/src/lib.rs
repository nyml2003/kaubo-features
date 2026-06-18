use kaubo_ir::cps::CpsModule;
use kaubo_syntax::lexer::Lexer;
use kaubo_syntax::parser::Parser;
use kaubo_web_api::token::{classify_token, describe_token, utf16_range};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static COMPILED: Lazy<Mutex<Option<CpsModule>>> = Lazy::new(|| Mutex::new(None));

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
    let module = Parser::new(source)
        .parse()
        .map_err(|e| JsValue::from_str(&e))?;
    kaubo_infer::infer_module(&module).map_err(|e| JsValue::from_str(&e.msg))?;
    let mut cps = kaubo_ir::cps_build::build_module(&module).map_err(|e| JsValue::from_str(&e))?;
    kaubo_ir::flatten::flatten_module(&mut cps);
    kaubo_ir::pass::run_passes(&mut cps, &[&kaubo_ir::pass::fold::ConstantFold]);

    if cps.functions.is_empty() {
        return Err(JsValue::from_str("no functions in compiled module"));
    }

    let mut count = 0usize;
    for func in &cps.functions {
        for block in &func.blocks {
            count += block.instrs.len() + 1;
        }
    }

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

    if cps.functions.is_empty() {
        return Err(JsValue::from_str("compiled module has no functions"));
    }

    let mut vm = kaubo_vm::VM::new();
    vm.load(&cps).map_err(|e| JsValue::from_str(&e))?;

    let func_idx = cps.functions.len() - 1;
    let reg_count = cps.functions[func_idx].reg_count;
    let result = vm
        .execute(func_idx, reg_count)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    let out = vm.output.join("\n");

    // Re-store for potential re-use
    COMPILED.lock().unwrap().replace(cps);

    let _ = result; // result value accessible via `out` if printed
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
