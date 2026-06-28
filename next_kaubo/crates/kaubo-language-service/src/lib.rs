//! Kaubo language service for editor-facing semantic features.
//!
//! This crate is a tooling/use-case layer. It provides:
//! - Token-based semantic tokens and completions (legacy)
//! - LspCoordinator: semantic-aware editor features via compiler frontend

pub mod lsp_coordinator;

pub use lsp_coordinator::{HoverInfo, InlayHint, LspCoordinator, SymbolDef, SymbolKind};

use kaubo_syntax::lexer::Lexer;
use kaubo_syntax::token::{Token, TokenKind};
use kaubo_web_api::token::{classify_token, utf16_range};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const BUILTIN_TYPES: &[&str] = &[
    "Int64",
    "Float64",
    "String",
    "Bool",
    "Null",
    "List",
    // Builtin interfaces
    "Add",
    "Subtract",
    "Multiply",
    "Divide",
    "Modulo",
    "Compare",
    "Display",
    "IntoFloat",
    "IntoInt",
    // Self type
    "Self",
];

/// Builtin methods for builtin types (used for dot-completions).
const BUILTIN_METHODS: &[(&str, &[&str])] = &[
    (
        "Int64",
        &[
            "add",
            "subtract",
            "multiply",
            "divide",
            "modulo",
            "less",
            "less_equal",
            "greater",
            "greater_equal",
            "equal",
            "not_equal",
            "to_string",
            "to_float",
        ],
    ),
    (
        "Float64",
        &[
            "add",
            "subtract",
            "multiply",
            "divide",
            "less",
            "less_equal",
            "greater",
            "greater_equal",
            "equal",
            "not_equal",
            "to_string",
            "to_int",
        ],
    ),
    (
        "String",
        &[
            "add",
            "less",
            "less_equal",
            "greater",
            "greater_equal",
            "equal",
            "not_equal",
            "to_string",
            "to_int",
        ],
    ),
    ("Bool", &["equal", "not_equal", "to_string"]),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticToken {
    pub kind: String,
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StructInfo {
    fields: Vec<String>,
    methods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SemanticModel {
    structs: BTreeMap<String, StructInfo>,
    vars: BTreeMap<String, String>,
}

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let tokens = visible_tokens(source);
    let model = build_model(&tokens);

    tokens
        .iter()
        .enumerate()
        .map(|(idx, token)| {
            let kind = semantic_kind(&tokens, idx, &model);
            let (from, to) = utf16_range(source, token.line, token.col, &token.lexeme);
            SemanticToken { kind, from, to }
        })
        .collect()
}

pub fn completions(source: &str, offset: usize) -> Vec<CompletionItem> {
    let tokens = visible_tokens(source);
    let model = build_model(&tokens);

    // Determine receiver type: simple variable/literal, or chained method call
    let type_name: String;
    if let Some(object_name) = object_before_dot(source, offset) {
        type_name = if let Some(tn) = model.vars.get(&object_name) {
            tn.clone()
        } else if let Some(tn) = literal_type_name(&object_name) {
            tn.to_string()
        } else {
            return Vec::new();
        };
    } else if let Some(tn) = chain_type_before_dot(source, offset, &model) {
        type_name = tn;
    } else {
        return Vec::new();
    }

    let mut result = Vec::new();

    // Try struct fields + methods
    if let Some(info) = model.structs.get(&type_name) {
        for field in &info.fields {
            result.push(CompletionItem {
                label: field.clone(),
                kind: "field".to_string(),
                detail: Some(type_name.clone()),
            });
        }
        for method in &info.methods {
            result.push(CompletionItem {
                label: method.clone(),
                kind: "method".to_string(),
                detail: Some(format!("{type_name} method")),
            });
        }
    }

    // Try builtin type methods
    for (bt_name, methods) in BUILTIN_METHODS {
        if type_name == *bt_name {
            for method in *methods {
                result.push(CompletionItem {
                    label: method.to_string(),
                    kind: "method".to_string(),
                    detail: Some(format!("{bt_name} method")),
                });
            }
        }
    }

    result
}

fn visible_tokens(source: &str) -> Vec<Token> {
    Lexer::new(source)
        .tokenize()
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .collect()
}

fn semantic_kind(tokens: &[Token], idx: usize, model: &SemanticModel) -> String {
    let token = &tokens[idx];
    if token.kind != TokenKind::Identifier {
        return classify_token(token.kind).to_string();
    }

    let previous = idx.checked_sub(1).and_then(|i| tokens.get(i));
    let next = tokens.get(idx + 1);
    let after_next = tokens.get(idx + 2);

    if previous.is_some_and(|t| {
        t.kind == TokenKind::Struct || t.kind == TokenKind::Impl || t.kind == TokenKind::Interface
    }) || is_type_reference(tokens, idx, model)
    {
        return "type".to_string();
    }

    if previous.is_some_and(|t| t.kind == TokenKind::Dot) {
        if next.is_some_and(|t| t.kind == TokenKind::LParen) {
            return "method".to_string();
        }
        return "field".to_string();
    }

    if is_impl_method_declaration(tokens, idx) {
        return "method".to_string();
    }

    if is_field_declaration_or_literal_key(tokens, idx) {
        return "field".to_string();
    }

    if next.is_some_and(|t| t.kind == TokenKind::LParen) {
        return "function".to_string();
    }

    "identifier".to_string()
}

fn is_type_reference(tokens: &[Token], idx: usize, model: &SemanticModel) -> bool {
    let previous = idx.checked_sub(1).and_then(|i| tokens.get(i));
    let next = tokens.get(idx + 1);

    previous.is_some_and(|t| {
        t.kind == TokenKind::FatArrow
            || (t.kind == TokenKind::Colon && colon_starts_type_annotation(tokens, idx - 1))
    }) || (next.is_some_and(|t| t.kind == TokenKind::LBrace)
        && model.structs.contains_key(&tokens[idx].lexeme))
        || is_generic_type_argument(tokens, idx, model)
}

fn colon_starts_type_annotation(tokens: &[Token], colon_idx: usize) -> bool {
    let Some(name_idx) = colon_idx.checked_sub(1) else {
        return false;
    };
    if !matches!(
        tokens[name_idx].kind,
        TokenKind::Identifier | TokenKind::Self_
    ) {
        return false;
    }

    let before_name = name_idx.checked_sub(1).and_then(|idx| tokens.get(idx));
    if before_name.is_some_and(|token| matches!(token.kind, TokenKind::Const | TokenKind::Var)) {
        return true;
    }

    is_in_struct_definition(tokens, colon_idx) || is_in_lambda_param_list(tokens, colon_idx)
}

fn is_in_struct_definition(tokens: &[Token], idx: usize) -> bool {
    let mut depth = 0usize;
    let mut cursor = idx;
    while cursor > 0 {
        cursor -= 1;
        match tokens[cursor].kind {
            TokenKind::RBrace => depth += 1,
            TokenKind::LBrace if depth > 0 => depth -= 1,
            TokenKind::LBrace => {
                return cursor >= 2
                    && tokens[cursor - 1].kind == TokenKind::Identifier
                    && tokens[cursor - 2].kind == TokenKind::Struct;
            }
            _ => {}
        }
    }
    false
}

fn is_in_lambda_param_list(tokens: &[Token], colon_idx: usize) -> bool {
    let mut saw_open_pipe = false;
    let mut cursor = colon_idx;
    while cursor > 0 {
        cursor -= 1;
        match tokens[cursor].kind {
            TokenKind::Pipe => {
                saw_open_pipe = true;
                break;
            }
            TokenKind::LBrace | TokenKind::Semicolon => return false,
            _ => {}
        }
    }
    if !saw_open_pipe {
        return false;
    }

    let mut cursor = colon_idx + 1;
    while cursor < tokens.len() {
        match tokens[cursor].kind {
            TokenKind::Pipe => return true,
            TokenKind::LBrace | TokenKind::Semicolon => return false,
            _ => cursor += 1,
        }
    }
    false
}

fn is_generic_type_argument(tokens: &[Token], idx: usize, model: &SemanticModel) -> bool {
    if !idx
        .checked_sub(1)
        .and_then(|prev| tokens.get(prev))
        .is_some_and(|token| token.kind == TokenKind::Lt)
    {
        return false;
    }

    idx >= 2
        && tokens[idx - 2].kind == TokenKind::Identifier
        && is_known_type_name(&tokens[idx - 2].lexeme, model)
}

fn is_known_type_name(name: &str, model: &SemanticModel) -> bool {
    model.structs.contains_key(name) || BUILTIN_TYPES.contains(&name)
}

fn is_impl_method_declaration(tokens: &[Token], idx: usize) -> bool {
    if !tokens
        .get(idx + 1)
        .is_some_and(|token| token.kind == TokenKind::Colon)
    {
        return false;
    }

    let mut cursor = idx;
    while cursor > 0 {
        cursor -= 1;
        match tokens[cursor].kind {
            TokenKind::Impl => return true,
            TokenKind::Struct | TokenKind::Semicolon => return false,
            _ => {}
        }
    }
    false
}

fn is_field_declaration_or_literal_key(tokens: &[Token], idx: usize) -> bool {
    if !tokens
        .get(idx + 1)
        .is_some_and(|token| token.kind == TokenKind::Colon)
    {
        return false;
    }
    if idx > 0 && matches!(tokens[idx - 1].kind, TokenKind::Const | TokenKind::Var) {
        return false;
    }
    if is_impl_method_declaration(tokens, idx) {
        return false;
    }

    let mut cursor = idx;
    while cursor > 0 {
        cursor -= 1;
        match tokens[cursor].kind {
            TokenKind::LBrace => {
                let before_brace = cursor.checked_sub(1).and_then(|i| tokens.get(i));
                let before_name = cursor.checked_sub(2).and_then(|i| tokens.get(i));
                return before_brace.is_some_and(|token| token.kind == TokenKind::Identifier)
                    || before_name.is_some_and(|token| token.kind == TokenKind::Struct);
            }
            TokenKind::Semicolon | TokenKind::RBrace => return false,
            _ => {}
        }
    }
    false
}

fn build_model(tokens: &[Token]) -> SemanticModel {
    let mut model = SemanticModel::default();
    collect_structs(tokens, &mut model);
    collect_impls(tokens, &mut model);
    collect_vars(tokens, &mut model);
    model
}

fn collect_structs(tokens: &[Token], model: &mut SemanticModel) {
    let mut idx = 0;
    while idx + 2 < tokens.len() {
        if tokens[idx].kind == TokenKind::Struct && tokens[idx + 1].kind == TokenKind::Identifier {
            let name = tokens[idx + 1].lexeme.clone();
            let mut fields = Vec::new();
            idx += 2;
            while idx + 1 < tokens.len() && tokens[idx].kind != TokenKind::RBrace {
                if tokens[idx].kind == TokenKind::Identifier
                    && tokens[idx + 1].kind == TokenKind::Colon
                {
                    fields.push(tokens[idx].lexeme.clone());
                }
                idx += 1;
            }
            model.structs.entry(name).or_insert_with(|| StructInfo {
                fields,
                methods: Vec::new(),
            });
        }
        idx += 1;
    }
}

fn collect_impls(tokens: &[Token], model: &mut SemanticModel) {
    let mut idx = 0;
    while idx + 4 < tokens.len() {
        if tokens[idx].kind == TokenKind::Impl && tokens[idx + 1].kind == TokenKind::Identifier {
            let first_name = tokens[idx + 1].lexeme.clone();
            // Check for `impl Interface for Struct { ... }` vs `impl Struct { ... }`
            let (struct_name, skip_extra) = if tokens[idx + 2].kind == TokenKind::For
                && tokens[idx + 3].kind == TokenKind::Identifier
            {
                // `impl Interface for Struct`
                (tokens[idx + 3].lexeme.clone(), 4)
            } else {
                // `impl Struct`
                (first_name, 2)
            };
            let info = model
                .structs
                .entry(struct_name)
                .or_insert_with(|| StructInfo {
                    fields: Vec::new(),
                    methods: Vec::new(),
                });
            idx += skip_extra;
            while idx + 1 < tokens.len() && tokens[idx].kind != TokenKind::RBrace {
                if tokens[idx].kind == TokenKind::Operator {
                    // `operator method:` — skip operator keyword
                    idx += 1;
                }
                // Skip lambda params: `|self: Point, other: Point|`
                // Single `|` is TokenKind::Bar, `|>` is TokenKind::Pipe
                if tokens[idx].kind == TokenKind::Bar || tokens[idx].kind == TokenKind::Pipe {
                    let closing = tokens[idx].kind;
                    idx += 1;
                    while idx < tokens.len() && tokens[idx].kind != closing {
                        idx += 1;
                    }
                    if idx < tokens.len() { idx += 1; } // skip closing |
                    continue;
                }
                if idx + 1 < tokens.len()
                    && tokens[idx].kind == TokenKind::Identifier
                    && tokens[idx + 1].kind == TokenKind::Colon
                {
                    info.methods.push(tokens[idx].lexeme.clone());
                }
                idx += 1;
            }
        }
        idx += 1;
    }
}

fn collect_vars(tokens: &[Token], model: &mut SemanticModel) {
    let struct_names: BTreeSet<String> = model.structs.keys().cloned().collect();
    let mut idx = 0;
    while idx + 3 < tokens.len() {
        if matches!(tokens[idx].kind, TokenKind::Const | TokenKind::Var)
            && tokens[idx + 1].kind == TokenKind::Identifier
        {
            let var_name = tokens[idx + 1].lexeme.clone();
            if tokens[idx + 2].kind == TokenKind::Colon
                && tokens[idx + 3].kind == TokenKind::Identifier
                && struct_names.contains(&tokens[idx + 3].lexeme)
            {
                model
                    .vars
                    .insert(var_name.clone(), tokens[idx + 3].lexeme.clone());
            }

            let mut lookahead = idx + 2;
            while lookahead + 1 < tokens.len() && tokens[lookahead].kind != TokenKind::Semicolon {
                if tokens[lookahead].kind == TokenKind::Eq
                    && tokens[lookahead + 1].kind == TokenKind::Identifier
                    && struct_names.contains(&tokens[lookahead + 1].lexeme)
                    && tokens
                        .get(lookahead + 2)
                        .is_some_and(|token| token.kind == TokenKind::LBrace)
                {
                    model
                        .vars
                        .insert(var_name.clone(), tokens[lookahead + 1].lexeme.clone());
                }
                lookahead += 1;
            }
        }
        idx += 1;
    }
}

/// Return type of a builtin method call (e.g. ("Int64", "to_float") → "Float64").
fn builtin_method_return(ty: &str, method: &str) -> Option<&'static str> {
    match (ty, method) {
        ("Int64", "to_float") => Some("Float64"),
        ("Int64", "to_string") => Some("String"),
        ("Float64", "to_int") => Some("Int64"),
        ("Float64", "to_string") => Some("String"),
        ("Bool", "to_string") => Some("String"),
        ("String", "to_int") => Some("Int64"),
        _ => None,
    }
}

/// Resolve the type of a chained call before a dot, e.g. `1.to_float().`
/// Returns the return type of the last method in the chain.
fn chain_type_before_dot(source: &str, offset: usize, model: &SemanticModel) -> Option<String> {
    let prefix = source.get(..offset)?;
    let trimmed = prefix.trim_end();
    let dot_pos = trimmed.rfind('.')?;
    let before_dot = trimmed[..dot_pos].trim_end();

    // Must end with `)` — a method call
    let inner = before_dot.strip_suffix(')')?;

    // Find the matching `(` — walk back tracking balanced parens
    let mut depth = 1i32;
    let mut call_end = inner.len();
    for (i, c) in inner.char_indices().rev() {
        if c == ')' { depth += 1; }
        if c == '(' { depth -= 1; }
        if depth == 0 {
            call_end = i;
            break;
        }
    }
    if depth != 0 { return None; } // unbalanced

    let before_call = &inner[..call_end];
    // Extract method name: find the last `.` before the call, or use the whole thing
    let (receiver_text, method_name) = if let Some(dot_pos) = before_call.rfind('.') {
        let method = &before_call[dot_pos + 1..];
        let receiver = &before_call[..dot_pos];
        (receiver, method.to_string())
    } else {
        ("", before_call.to_string())
    };

    if method_name.is_empty() { return None; }

    // Clean receiver: extract rightmost alphanumeric segment (skip outer context)
    // e.g. "print(1" → "1",  "f(x).obj" → "obj" (actually "f(x)" → not reached here)
    let clean_receiver: String = {
        let chars: Vec<char> = receiver_text.chars().collect();
        let end = chars.len();
        let start = chars
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, &c)| {
                if !c.is_alphanumeric() && c != '_' && c != '.' {
                    Some(i + 1)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        if start >= end {
            return None;
        }
        chars[start..end].iter().collect()
    };

    // Determine receiver type
    let receiver_type: String = if let Some(lt) = literal_type_name(&clean_receiver) {
        lt.to_string()
    } else if let Some(tn) = model.vars.get(&clean_receiver) {
        tn.clone()
    } else {
        return None;
    };

    builtin_method_return(&receiver_type, &method_name).map(|s| s.to_string())
}

/// Map a literal token to its builtin type name (e.g. "1" → "Int64", "true" → "Bool").
fn literal_type_name(name: &str) -> Option<&'static str> {
    if name == "true" || name == "false" {
        return Some("Bool");
    }
    if name == "null" {
        return Some("Null");
    }
    // String literal: "content"
    if name.starts_with('"') && name.ends_with('"') {
        return Some("String");
    }
    if name.chars().all(|c| c.is_ascii_digit()) {
        return Some("Int64");
    }
    // Float: digits.digits
    if name.contains('.') && name.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Some("Float64");
    }
    None
}

pub(crate) fn object_before_dot(source: &str, offset: usize) -> Option<String> {
    let prefix = source.get(..offset)?;
    let trimmed = prefix.trim_end();
    // Find the last dot — handles both `1.|` and `1.t|`
    let dot_pos = trimmed.rfind('.')?;
    let before_dot = trimmed[..dot_pos].trim_end();

    // String literal: "hello". → return the whole quoted string
    if before_dot.starts_with('"') && before_dot.ends_with('"') {
        return Some(before_dot.to_string());
    }

    let end = before_dot.len();
    let start = before_dot[..end]
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map_or(0, |idx| idx + 1);
    let object = &before_dot[start..end];
    if object.is_empty() {
        None
    } else {
        Some(object.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(source: &str, text: &str, kind: &str) -> SemanticToken {
        semantic_tokens(source)
            .into_iter()
            .find(|token| token.kind == kind && source.get(token.from..token.to) == Some(text))
            .unwrap_or_else(|| panic!("missing {kind} token {text}"))
    }

    #[test]
    fn marks_struct_names_as_type() {
        let source = "struct Point { x: Int64, y: Int64 }";
        let point = token(source, "Point", "type");
        assert_eq!((point.from, point.to), (7, 12));
    }

    #[test]
    fn marks_lowercase_struct_names_as_type() {
        let source = "struct point { x: Int64 }\nconst p: point = point { x: 1 };";
        let types: Vec<&str> = semantic_tokens(source)
            .into_iter()
            .filter(|token| token.kind == "type")
            .filter_map(|token| source.get(token.from..token.to))
            .collect();

        assert_eq!(types, vec!["point", "Int64", "point", "point"]);
    }

    #[test]
    fn does_not_classify_uppercase_identifiers_as_types_by_name() {
        let source = "const Value = 1;\nValue();";
        let value_tokens: Vec<String> = semantic_tokens(source)
            .into_iter()
            .filter_map(|token| {
                (source.get(token.from..token.to) == Some("Value")).then_some(token.kind)
            })
            .collect();

        assert_eq!(value_tokens, vec!["identifier", "function"]);
    }

    #[test]
    fn marks_impl_method_declarations_and_calls() {
        let source = "struct Point { x: Int64 }\nimpl Point { dis: |self| { self.x } }\nconst p = Point { x: 1 };\np.dis();";
        let tokens = semantic_tokens(source);
        assert!(tokens.iter().any(|token| {
            token.kind == "method" && source.get(token.from..token.to) == Some("dis")
        }));
    }

    #[test]
    fn marks_field_access() {
        let source = "struct Point { x: Int64 }\nconst p = Point { x: 1 };\np.x;";
        let field = token(source, "x", "field");
        assert_eq!(source.get(field.from..field.to), Some("x"));
    }

    #[test]
    fn marks_struct_fields_and_literal_keys() {
        let source = "struct Point { x: Int64 }\nconst p = Point { x: 1 };";
        let fields: Vec<&str> = semantic_tokens(source)
            .into_iter()
            .filter(|token| token.kind == "field")
            .filter_map(|token| source.get(token.from..token.to))
            .collect();

        assert_eq!(fields, vec!["x", "x"]);
    }

    #[test]
    fn marks_function_calls() {
        let source = "print(add(2, 3));";
        let tokens = semantic_tokens(source);
        assert!(tokens.iter().any(|token| {
            token.kind == "function" && source.get(token.from..token.to) == Some("print")
        }));
        assert!(tokens.iter().any(|token| {
            token.kind == "function" && source.get(token.from..token.to) == Some("add")
        }));
    }

    #[test]
    fn completes_fields_and_methods_for_struct_instance() {
        let source = "struct Point { x: Int64, y: Int64 }\nimpl Point { dis: |self| { self.x } }\nconst p = Point { x: 1, y: 2 };\np.";
        let items = completions(source, source.len());
        let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
        assert!(labels.contains(&"x"));
        assert!(labels.contains(&"y"));
        assert!(labels.contains(&"dis"));
    }

    #[test]
    fn semantic_tokens_empty_source() {
        let tokens = semantic_tokens("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn semantic_tokens_keywords() {
        let source = "const var if else while for in break continue return";
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert!(kinds.iter().all(|k| *k == "keyword"));
    }

    #[test]
    fn semantic_tokens_number_classification() {
        let source = "42 3.14";
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert_eq!(kinds, vec!["number", "number"]);
    }

    #[test]
    fn semantic_tokens_string_classification() {
        let source = r#""hello""#;
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert_eq!(kinds, vec!["string"]);
    }

    #[test]
    fn semantic_tokens_comment_classification() {
        let source = "// comment\n42";
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert_eq!(kinds, vec!["comment", "number"]);
    }

    #[test]
    fn semantic_tokens_block_comment() {
        let source = "/* block */ 42";
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert_eq!(kinds, vec!["comment", "number"]);
    }

    #[test]
    fn completions_no_dot_returns_empty() {
        let items = completions("print", 5);
        assert!(items.is_empty());
    }

    #[test]
    fn completions_empty_source() {
        let items = completions("", 0);
        assert!(items.is_empty());
    }

    #[test]
    fn completions_on_non_struct_object() {
        // Literals now resolve to their builtin type — 42. → Int64 methods
        let items = completions("42.", 3);
        assert!(!items.is_empty());
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"to_string"));
        assert!(labels.contains(&"to_float"));
    }

    #[test]
    fn semantic_tokens_operator_classification() {
        let source = "+ - * / % = == != < <= > >=";
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert!(kinds.iter().all(|k| *k == "operator"));
    }

    #[test]
    fn semantic_tokens_atom_classification() {
        let source = "true false null";
        let tokens = semantic_tokens(source);
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind.as_str()).collect();
        assert_eq!(kinds, vec!["atom", "atom", "atom"]);
    }
}
