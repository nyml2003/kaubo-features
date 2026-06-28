use kaubo_language_service::{
    completions as ls_completions, semantic_tokens as ls_semantic_tokens,
    LspCoordinator,
};
use kaubo_syntax::lexer::Lexer;
use kaubo_web_api::token::{classify_token, describe_token, utf16_range};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static COMPILED: Lazy<Mutex<Option<kaubo_driver::CpsModule>>> = Lazy::new(|| Mutex::new(None));

/// Global LSP coordinator — shared across all editor features.
static LSP: Lazy<Mutex<LspCoordinator>> = Lazy::new(|| Mutex::new(LspCoordinator::new()));

/// Global log-level setting for WASM.  `None` means logging is disabled.
/// Set via `set_log_level(level)` from JavaScript.
static LOG_LEVEL: Lazy<Mutex<Option<kaubo_log::Severity>>> = Lazy::new(|| Mutex::new(None));

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
            format!(r#"{{"kind":"{kind}","from":{from},"to":{to}}}"#)
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// Parse + type-check, return JSON error array or "[]".
#[wasm_bindgen]
pub fn diagnose(source: &str) -> String {
    kaubo_web_api::diagnose::diagnose(source)
}

/// Enable or disable structured logging from the toolchain.
///
/// `level` is a severity level: 0 = Trace, 1 = Debug, 2 = Info,
/// 3 = Warn, 4 = Error.  Pass a value outside 0-4 to disable.
///
/// When enabled, events are written to `console.error` via the
/// `ConsoleHandler` in `kaubo-log-handlers`.
#[wasm_bindgen]
pub fn set_log_level(level: u8) {
    let severity = match level {
        0 => Some(kaubo_log::Severity::Trace),
        1 => Some(kaubo_log::Severity::Debug),
        2 => Some(kaubo_log::Severity::Info),
        3 => Some(kaubo_log::Severity::Warn),
        4 => Some(kaubo_log::Severity::Error),
        _ => None,
    };
    *LOG_LEVEL.lock().unwrap() = severity;
}

/// Build a RunConfig from the current global LOG_LEVEL setting.
fn make_config() -> kaubo_driver::RunConfig {
    let events: Option<Box<dyn kaubo_log::EventHandler>> = LOG_LEVEL.lock().unwrap().map(|level| {
        Box::new(kaubo_log_handlers::make_handler(level)) as Box<dyn kaubo_log::EventHandler>
    });
    kaubo_driver::RunConfig {
        events,
        max_loop_iterations: u64::MAX,
    }
}

/// Compile source to bytecode, return instruction count.
/// Throws JsValue on parse/infer/build failure.
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<usize, JsValue> {
    let config = make_config();
    let cps = kaubo_driver::compile_source_with_config(source, &config)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

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

    let config = make_config();
    let outcome = kaubo_driver::run_module_with_config(&cps, &config)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = outcome.output.join("\n");

    // Re-store for potential re-use
    COMPILED.lock().unwrap().replace(cps);

    Ok(out)
}

/// Feed source to the LSP coordinator. Call after each text change.
#[wasm_bindgen]
pub fn lsp_on_change(source: &str) {
    let mut lsp = LSP.lock().unwrap();
    let _ = lsp.on_change(source);
}

/// Get hover information for token at UTF-16 offset.
#[wasm_bindgen]
pub fn hover(source: &str, offset: usize) -> String {
    // Try LSP coordinator first
    if let Ok(mut lsp) = LSP.lock() {
        if lsp.is_ready() {
            if let Some(info) = lsp.hover(offset) {
                return serde_json::json!({
                    "kind": info.kind,
                    "type": info.ty,
                    "description": info.description,
                })
                .to_string();
            }
        }
    }

    // Fallback: token-based hover
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

/// Go-to-definition: return JSON { line, col } or "null".
#[wasm_bindgen]
pub fn goto_def(source: &str, offset: usize) -> String {
    // Ensure LSP is up to date
    {
        let mut lsp = LSP.lock().unwrap();
        if !lsp.is_ready() {
            let _ = lsp.on_change(source);
        }
    }

    let lsp = LSP.lock().unwrap();
    if let Some(span) = lsp.goto_def(offset) {
        return serde_json::json!({
            "line": span.line,
            "col": span.col,
        })
        .to_string();
    }
    "null".to_string()
}

#[wasm_bindgen]
pub fn semantic_tokens(source: &str) -> String {
    // Update LSP state for better token classification
    if let Ok(mut lsp) = LSP.lock() {
        let _ = lsp.on_change(source);
    }
    // For now, use the existing token-based semantic tokens
    serde_json::to_string(&ls_semantic_tokens(source)).unwrap_or_else(|_| "[]".to_string())
}

#[wasm_bindgen]
pub fn complete(source: &str, offset: usize) -> String {
    // Try LSP coordinator
    {
        let mut lsp = LSP.lock().unwrap();
        if !lsp.is_ready() {
            let _ = lsp.on_change(source);
        }
    }

    let lsp = LSP.lock().unwrap();
    if lsp.is_ready() {
        let items = lsp.completions(offset);
        return serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
    }

    // Fallback
    serde_json::to_string(&ls_completions(source, offset)).unwrap_or_else(|_| "[]".to_string())
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

    #[test]
    fn semantic_tokens_include_type_method_and_function_roles() {
        let source = "struct Point { x: Int64 }\nimpl Point { dis: |self| { self.x } }\nconst p = Point { x: 1 };\np.dis();\nprint(p.x);";
        let tokens: Vec<kaubo_language_service::SemanticToken> =
            serde_json::from_str(&semantic_tokens(source)).unwrap();
        let roles: Vec<String> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(roles.contains(&"type".to_string()));
        assert!(roles.contains(&"method".to_string()));
        assert!(roles.contains(&"function".to_string()));
        assert!(roles.contains(&"field".to_string()));
    }

    #[test]
    fn completion_exposes_struct_fields_and_methods() {
        let source = "struct Point { x: Int64, y: Int64 }\nimpl Point { dis: |self| { self.x } }\nconst p = Point { x: 1, y: 2 };\np.";
        let items: Vec<kaubo_language_service::CompletionItem> =
            serde_json::from_str(&complete(source, source.len())).unwrap();
        let labels: Vec<String> = items.into_iter().map(|item| item.label).collect();
        assert!(labels.contains(&"x".to_string()));
        assert!(labels.contains(&"y".to_string()));
        assert!(labels.contains(&"dis".to_string()));
    }

    #[test]
    fn set_log_level_does_not_crash() {
        set_log_level(0); // Trace
        set_log_level(1); // Debug
        set_log_level(255); // Disable
    }

    #[test]
    fn compile_and_run_work_with_config() {
        let cps_count = compile("const x = 42;").unwrap();
        assert!(cps_count > 0);
        let output = run(&[]).unwrap();
        assert_eq!(output, ""); // no print output
    }
}
