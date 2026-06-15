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

// ── Diagnose (structured errors) ──────────────────────────────────────────

use kaubo_compiler::parser::Parser;

/// Diagnose Kaubo source code — returns structured errors as JSON.
///
/// Takes source code, runs lexer + parser + type checker.
/// Returns a JSON array of diagnostic objects:
///   `[{"severity":"error","line":1,"column":3,"from":2,"to":5,"message":"..."}]`
///
/// `from`/`to` are UTF-16 code unit offsets (compatible with CodeMirror / VSCode).
/// If no errors, returns `"[]"`.
#[wasm_bindgen]
pub fn diagnose(source: &str) -> String {
    let owned = source.to_owned();
    let utf16_starts = build_utf16_line_starts(&owned);

    let mut errors: Vec<String> = Vec::new();

    // Stage 1: Lexer
    let mut lexer = Lexer::new(4096);
    if lexer.feed(owned.as_bytes()).is_err() {
        return "[]".to_string();
    }
    if lexer.terminate().is_err() {
        return "[]".to_string();
    }

    // Stage 2: Parser
    let mut parser = Parser::new(lexer);
    let module = match parser.parse() {
        Ok(m) => m,
        Err(e) => {
            let line = e.line().unwrap_or(1);
            let col = e.column().unwrap_or(1);
            let offset = line_col_to_utf16_offset(line, col, &utf16_starts, &owned);
            errors.push(format_diagnostic("error", line, col, offset, offset, &e.to_string()));
            return format!("[{}]", errors.join(","));
        }
    };

    // Stage 3: Type checker
    use kaubo_compiler::CheckStage;
    let check_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CheckStage::new().run(&module)
    }));
    match check_result {
        Ok(Err(msg)) => {
            errors.push(format_diagnostic("error", 1, 1, 0, 0, &msg));
        }
        Err(panic) => {
            let msg = panic.downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Internal type check error".to_string());
            errors.push(format_diagnostic("error", 1, 1, 0, 0, &msg));
        }
        _ => {}
    }

    if errors.is_empty() {
        return "[]".to_string();
    }
    format!("[{}]", errors.join(","))
}

fn line_col_to_utf16_offset(
    line: usize,
    column: usize,
    utf16_starts: &[usize],
    source: &str,
) -> usize {
    let line_idx = line.saturating_sub(1);
    let base = utf16_starts.get(line_idx).copied().unwrap_or(0);

    // Walk the line to compute UTF-16 offset for column
    let mut utf16_offset = 0usize;
    let mut byte_pos = 0usize;
    for ch in source.chars() {
        if byte_pos >= column.saturating_sub(1) {
            break;
        }
        byte_pos += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    base + utf16_offset
}

fn format_diagnostic(
    severity: &str,
    line: usize,
    column: usize,
    from: usize,
    to: usize,
    message: &str,
) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    write!(
        &mut s,
        r#"{{"severity":"{severity}","line":{line},"column":{column},"from":{from},"to":{to},"message":"{escaped}"}}"#
    )
    .ok();
    s
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
