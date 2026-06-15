//! Kaubo WASM bindings — compile, run, and tokenize Kaubo code in the browser

use wasm_bindgen::prelude::*;
use std::sync::Mutex;

/// Shared chunk storage — compile deposits, run withdraws
static COMPILED: Mutex<Option<kaubo_ir::Chunk>> = Mutex::new(None);

/// Initialize panic hook so errors show in browser console instead of `unreachable`
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// ── Lexer (syntax highlighting) ────────────────────────────────────────────

use kaubo_compiler::lexer::v2::Lexer;
use kaubo_compiler::lexer::token_kind::KauboTokenKind;

/// Tokenize Kaubo source and return a JSON array of tokens.
///
/// Each token: `{"kind":"keyword","from":0,"to":3}`
/// Positions are UTF-16 code unit offsets (compatible with JavaScript / CodeMirror).
#[wasm_bindgen]
pub fn lex(source: &str) -> String {
    let owned = source.to_owned();
    let utf16_starts = build_utf16_line_starts(&owned);

    let mut lexer = Lexer::new(4096);
    let _ = lexer.feed(owned.as_bytes());
    let _ = lexer.terminate();

    let mut tokens = String::from("[");
    let mut first = true;

    while let Some(tok) = lexer.next_token() {
        let kind_str = token_kind_to_tag(tok.kind);
        if kind_str.is_empty() {
            continue;
        }

        let line = tok.start().line.saturating_sub(1); // 0-based
        let base = utf16_starts.get(line).copied().unwrap_or(0);
        let from = base + tok.start().utf16_column;
        let to = base + tok.end().utf16_column;

        if !first {
            tokens.push(',');
        }
        first = false;
        use std::fmt::Write;
        write!(&mut tokens, r#"{{"kind":"{kind_str}","from":{from},"to":{to}}}"#).ok();
    }

    tokens.push(']');
    tokens
}

/// Build a table of UTF-16 code unit offsets at each line start.
/// line_starts[i] = total UTF-16 code units before line i (0-based).
fn build_utf16_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    let mut total: usize = 0;
    for ch in source.chars() {
        total += ch.len_utf16();
        if ch == '\n' {
            starts.push(total);
        }
    }
    starts
}

fn token_kind_to_tag(kind: KauboTokenKind) -> &'static str {
    match kind {
        KauboTokenKind::Var
        | KauboTokenKind::If
        | KauboTokenKind::Else
        | KauboTokenKind::Elif
        | KauboTokenKind::While
        | KauboTokenKind::For
        | KauboTokenKind::Return
        | KauboTokenKind::In
        | KauboTokenKind::Break
        | KauboTokenKind::Continue
        | KauboTokenKind::Struct
        | KauboTokenKind::Impl
        | KauboTokenKind::Import
        | KauboTokenKind::As
        | KauboTokenKind::From
        | KauboTokenKind::Pass
        | KauboTokenKind::And
        | KauboTokenKind::Or
        | KauboTokenKind::Not
        | KauboTokenKind::Module
        | KauboTokenKind::Operator
        | KauboTokenKind::Pub
        | KauboTokenKind::Print
        | KauboTokenKind::Json => "keyword",

        KauboTokenKind::LiteralInteger | KauboTokenKind::LiteralFloat => "number",
        KauboTokenKind::LiteralString => "string",

        KauboTokenKind::True | KauboTokenKind::False | KauboTokenKind::Null => "atom",

        KauboTokenKind::Identifier => "identifier",

        KauboTokenKind::Comment => "comment",

        KauboTokenKind::DoubleEqual
        | KauboTokenKind::ExclamationEqual
        | KauboTokenKind::GreaterThanEqual
        | KauboTokenKind::LessThanEqual
        | KauboTokenKind::FatArrow
        | KauboTokenKind::GreaterThan
        | KauboTokenKind::LessThan
        | KauboTokenKind::Plus
        | KauboTokenKind::Asterisk
        | KauboTokenKind::Slash
        | KauboTokenKind::Percent
        | KauboTokenKind::Colon
        | KauboTokenKind::Equal
        | KauboTokenKind::Comma
        | KauboTokenKind::Semicolon
        | KauboTokenKind::LeftParenthesis
        | KauboTokenKind::RightParenthesis
        | KauboTokenKind::LeftCurlyBrace
        | KauboTokenKind::RightCurlyBrace
        | KauboTokenKind::LeftSquareBracket
        | KauboTokenKind::RightSquareBracket
        | KauboTokenKind::Dot
        | KauboTokenKind::Pipe
        | KauboTokenKind::Yield
        | KauboTokenKind::Minus => "operator",

        _ => "",
    }
}

// ── Compile & Run ──────────────────────────────────────────────────────────

/// Compile Kaubo source code, store chunk in memory.
/// Returns number of bytecode instructions (for display).
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<u32, JsValue> {
    let owned_source = source.to_owned();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let src = &owned_source;
        let module = kaubo_compiler::ParseStage::new()
            .run(src)
            .map_err(|e| JsValue::from_str(&e))?;

        kaubo_compiler::CheckStage::new()
            .run(&module)
            .map_err(|e| JsValue::from_str(&e))?;

        let chunk = kaubo_compiler::CodegenStage::new()
            .run(&module)
            .map_err(|e| JsValue::from_str(&e))?;

        let len = chunk.code.len() as u32;
        *COMPILED.lock().unwrap() = Some(chunk);
        Ok(len)
    }));

    match result {
        Ok(r) => r,
        Err(panic) => {
            let msg = panic.downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Internal compiler error".to_string());
            Err(JsValue::from_str(&format!("Panic: {msg}")))
        }
    }
}

/// Run the most recently compiled chunk, returns stdout output
#[wasm_bindgen]
pub fn run(_bytes: &[u8]) -> Result<String, JsValue> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        use kaubo_runtime::vm::VmRuntime;
        use std::sync::{Arc, Mutex as StdMutex};

        let chunk = COMPILED.lock()
            .map_err(|_| JsValue::from_str("Lock poisoned"))?
            .clone()
            .ok_or_else(|| JsValue::from_str("No compiled chunk — run compile() first"))?;

        let mut vm = kaubo_ir::VM::new();
        vm.init_stdlib();

        let output = Arc::new(StdMutex::new(String::new()));
        let out = output.clone();
        vm.set_output_callback(move |s: &str| {
            out.lock().unwrap().push_str(s);
        });

        vm.interpret(&chunk);

        let s = output.lock().unwrap().clone();
        Ok(s)
    }));

    match result {
        Ok(r) => r,
        Err(panic) => {
            let msg = panic.downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Internal runtime error".to_string());
            Err(JsValue::from_str(&format!("Panic: {msg}")))
        }
    }
}
