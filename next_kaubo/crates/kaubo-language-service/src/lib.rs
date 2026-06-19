//! Kaubo language service for editor-facing semantic features.
//!
//! This crate is a tooling/use-case layer. It consumes syntax-stage output and
//! produces semantic tokens and completions for adapters.

use kaubo_syntax::lexer::Lexer;
use kaubo_syntax::token::{Token, TokenKind};
use kaubo_web_api::token::{classify_token, utf16_range};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const BUILTIN_TYPES: &[&str] = &["Int64", "Float64", "String", "Bool", "Null", "List"];

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
    let Some(object_name) = object_before_dot(source, offset) else {
        return Vec::new();
    };
    let Some(type_name) = model.vars.get(&object_name) else {
        return Vec::new();
    };
    let Some(info) = model.structs.get(type_name) else {
        return Vec::new();
    };

    let mut result = Vec::new();
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

    if previous.is_some_and(|t| t.kind == TokenKind::Struct || t.kind == TokenKind::Impl)
        || is_type_reference(tokens, idx, model)
    {
        return "type".to_string();
    }

    if previous.is_some_and(|t| t.kind == TokenKind::Dot) {
        if after_next.is_some_and(|t| t.kind == TokenKind::LParen) {
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
            let struct_name = tokens[idx + 1].lexeme.clone();
            let info = model
                .structs
                .entry(struct_name)
                .or_insert_with(|| StructInfo {
                    fields: Vec::new(),
                    methods: Vec::new(),
                });
            idx += 2;
            while idx + 1 < tokens.len() && tokens[idx].kind != TokenKind::RBrace {
                if tokens[idx].kind == TokenKind::Identifier
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

fn object_before_dot(source: &str, offset: usize) -> Option<String> {
    let prefix = source.get(..offset)?;
    let trimmed = prefix.trim_end();
    let before_dot = trimmed.strip_suffix('.')?.trim_end();
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
}
