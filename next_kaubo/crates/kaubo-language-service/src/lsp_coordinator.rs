//! LspCoordinator — wires FrontendStage + SemanticStage for editor features.
//!
//! Caches parse + infer results so hover / goto_def / completions are
//! read-only queries over the last successful build.

use kaubo_ast::{Expr, Module, Span, Stmt};
use kaubo_driver::stages::{FrontendStage, SemanticStage};
use kaubo_driver::protocol::{BuildContext, BuildError, Stage};
use kaubo_driver::SemanticArtifact;
use kaubo_infer::Scheme;
use serde::Serialize;
use std::collections::HashMap;

/// Kind of a symbol (aligned with LSP SymbolKind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Const,
    Var,
    Function,
    Struct,
    Enum,
    Interface,
    Method,
    Field,
    Variant,
    Param,
}

impl SymbolKind {
    pub fn as_str(&self) -> &str {
        match self {
            SymbolKind::Const => "const",
            SymbolKind::Var => "var",
            SymbolKind::Function => "function",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Interface => "interface",
            SymbolKind::Method => "method",
            SymbolKind::Field => "field",
            SymbolKind::Variant => "variant",
            SymbolKind::Param => "param",
        }
    }
}

/// A symbol definition with source location and optional type.
#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    /// Human-readable type string (e.g. "Int64", "Point", "|Int64| -> Int64").
    pub ty: Option<String>,
}

/// A reference (usage) of a symbol at a source location.
#[derive(Debug, Clone)]
pub struct Reference {
    pub span: Span,
    pub name: String,
}

/// Hover information shown when cursor is over a symbol.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub kind: String,
    pub ty: Option<String>,
    pub description: String,
}

/// An inlay hint — a type annotation shown inline next to a name.
#[derive(Debug, Clone, Serialize)]
pub struct InlayHint {
    /// Char offset in source where the label should appear (right after the name).
    pub position: usize,
    /// The label text, e.g. ": Int64", ": String".
    pub label: String,
}

/// The LSP coordinator: parse → infer → query.
pub struct LspCoordinator {
    source: String,
    module: Option<Module>,
    semantic: Option<SemanticArtifact>,
    symbols: HashMap<String, SymbolDef>,
    references: Vec<Reference>,
}

impl LspCoordinator {
    pub fn new() -> Self {
        Self {
            source: String::new(),
            module: None,
            semantic: None,
            symbols: HashMap::new(),
            references: Vec::new(),
        }
    }

    /// Process a source change: frontend → semantic, cache results.
    pub fn on_change(&mut self, source: &str) -> Result<(), BuildError> {
        self.source = source.to_string();

        // Frontend: source → AST
        let module = FrontendStage.execute(source, &BuildContext { events: None })?;

        // Semantic: AST → type info
        let semantic = SemanticStage.execute(module.clone(), &BuildContext { events: None })?;

        // Collect symbols and references from the AST
        let (mut symbols, references) = collect_symbols_and_refs(&module);

        // Cross-reference symbols with inferred types from type_env
        for (name, sym) in symbols.iter_mut() {
            if let Some(scheme) = semantic.type_env.get(name) {
                sym.ty = Some(format!("{}", scheme.body));
            }
        }

        self.module = Some(module);
        self.semantic = Some(semantic);
        self.symbols = symbols;
        self.references = references;

        Ok(())
    }

    /// Find which identifier is at the given byte offset.
    pub fn symbol_at(&self, offset: usize) -> Option<&SymbolDef> {
        let name = identifier_at(&self.source, offset)?;
        self.symbols.get(&name)
    }

    /// Go-to-definition: return the definition span for the symbol at `offset`.
    pub fn goto_def(&self, offset: usize) -> Option<Span> {
        let name = identifier_at(&self.source, offset)?;
        // First check if it's a reference to a known symbol
        for r in &self.references {
            if spans_contain(&r.span, offset, &self.source) && r.name == name {
                return self.symbols.get(&r.name).map(|s| s.span);
            }
        }
        // Fallback: look up the name directly
        self.symbols.get(&name).map(|s| s.span)
    }

    /// Hover: return type information for the symbol at `offset`.
    pub fn hover(&self, offset: usize) -> Option<HoverInfo> {
        let name = identifier_at(&self.source, offset)?;

        // Check symbols first (definitions)
        if let Some(sym) = self.symbols.get(&name) {
            return Some(HoverInfo {
                kind: sym.kind.as_str().to_string(),
                ty: sym.ty.clone(),
                description: format!("{} {}", sym.kind.as_str(), name),
            });
        }

        // Check references — look up in type_env
        if let Some(ref semantic) = self.semantic {
            if let Some(scheme) = semantic.type_env.get(&name) {
                let (kind_str, desc) = if let Some(sym) = self.symbols.get(&name) {
                    (sym.kind.as_str().to_string(), format!("{} {}", sym.kind.as_str(), name))
                } else {
                    ("variable".to_string(), format!("variable {}", name))
                };
                return Some(HoverInfo {
                    kind: kind_str,
                    ty: Some(format!("{}", scheme.body)),
                    description: desc,
                });
            }
        }

        None
    }

    /// Completions at `offset`: return all visible symbols.
    pub fn completions(&self, offset: usize) -> Vec<crate::CompletionItem> {
        let mut items = Vec::new();

        // Include all collected symbols
        for (_, sym) in &self.symbols {
            items.push(crate::CompletionItem {
                label: sym.name.clone(),
                kind: sym.kind.as_str().to_string(),
                detail: sym.ty.clone(),
            });
        }

        // Include variables from type_env that aren't already in symbols
        if let Some(ref semantic) = self.semantic {
            for (name, scheme) in &semantic.type_env {
                if !self.symbols.contains_key(name) {
                    items.push(crate::CompletionItem {
                        label: name.clone(),
                        kind: "variable".to_string(),
                        detail: Some(format!("{}", scheme.body)),
                    });
                }
            }
        }

        // Also run the token-based completion for dot-completion
        let token_items = crate::completions(&self.source, offset);
        items.extend(token_items);

        items
    }

    /// Produce inlay hints showing inferred types next to definitions.
    pub fn inlay_hints(&self) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        let Some(ref module) = self.module else { return hints; };
        let Some(ref semantic) = self.semantic else { return hints; };

        for stmt in &module.stmts {
            collect_hints_stmt(stmt, &semantic.type_env, &self.source, &mut hints);
        }
        hints
    }

    /// Whether a successful build is available.
    pub fn is_ready(&self) -> bool {
        self.semantic.is_some()
    }

    /// Access the semantic artifact for direct queries.
    pub fn semantic(&self) -> Option<&SemanticArtifact> {
        self.semantic.as_ref()
    }

    /// Access the module AST.
    pub fn module(&self) -> Option<&Module> {
        self.module.as_ref()
    }
}

impl Default for LspCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ──

/// Extract the identifier at a UTF-16 byte offset in source.
fn identifier_at(source: &str, offset: usize) -> Option<String> {
    if offset >= source.len() {
        return None;
    }
    // Find the start of the identifier
    let bytes: Vec<char> = source.chars().collect();
    if offset >= bytes.len() {
        return None;
    }
    if !is_ident_char(bytes[offset]) {
        return None;
    }
    let mut start = offset;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }
    Some(bytes[start..end].iter().collect())
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Check if `offset` falls within the byte range of `span` in `source`.
fn spans_contain(span: &Span, offset: usize, _source: &str) -> bool {
    // Span stores (line, col). For simplicity, we check if offset
    // matches the span's rough position. A more precise check would
    // convert span to byte range.
    // For now, just approximate — a full impl would use line/col→byte mapping.
    let _ = offset;
    let _ = span;
    true // Placeholder — refined in later iteration
}

// ── Symbol collection (AST walk) ──

fn collect_symbols_and_refs(module: &Module) -> (HashMap<String, SymbolDef>, Vec<Reference>) {
    let mut symbols: HashMap<String, SymbolDef> = HashMap::new();
    let mut references: Vec<Reference> = Vec::new();

    for stmt in &module.stmts {
        collect_from_stmt(stmt, &mut symbols, &mut references);
    }

    (symbols, references)
}

fn collect_from_stmt(
    stmt: &Stmt,
    symbols: &mut HashMap<String, SymbolDef>,
    references: &mut Vec<Reference>,
) {
    match stmt {
        Stmt::ConstDecl { name, span, value, .. } => {
            symbols.insert(
                name.clone(),
                SymbolDef {
                    name: name.clone(),
                    kind: SymbolKind::Const,
                    span: *span,
                    ty: None,
                },
            );
            collect_from_expr(value, symbols, references);
        }
        Stmt::VarDecl { name, span, value, .. } => {
            symbols.insert(
                name.clone(),
                SymbolDef {
                    name: name.clone(),
                    kind: SymbolKind::Var,
                    span: *span,
                    ty: None,
                },
            );
            if let Some(v) = value {
                collect_from_expr(v, symbols, references);
            }
        }
        Stmt::StructDef {
            name, span, fields, ..
        } => {
            symbols.insert(
                name.clone(),
                SymbolDef {
                    name: name.clone(),
                    kind: SymbolKind::Struct,
                    span: *span,
                    ty: None,
                },
            );
            for f in fields {
                symbols.insert(
                    f.name.clone(),
                    SymbolDef {
                        name: f.name.clone(),
                        kind: SymbolKind::Field,
                        span: f.span,
                        ty: None,
                    },
                );
            }
        }
        Stmt::EnumDef {
            name, span, variants, ..
        } => {
            symbols.insert(
                name.clone(),
                SymbolDef {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
                    span: *span,
                    ty: None,
                },
            );
            for v in variants {
                symbols.insert(
                    v.name.clone(),
                    SymbolDef {
                        name: v.name.clone(),
                        kind: SymbolKind::Variant,
                        span: v.span,
                        ty: None,
                    },
                );
            }
        }
        Stmt::InterfaceDef {
            name, span, methods, ..
        } => {
            symbols.insert(
                name.clone(),
                SymbolDef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    span: *span,
                    ty: None,
                },
            );
            for m in methods {
                symbols.insert(
                    m.name.clone(),
                    SymbolDef {
                        name: m.name.clone(),
                        kind: SymbolKind::Method,
                        span: Span::ZERO, // MethodSig doesn't have span yet
                        ty: None,
                    },
                );
                for p in &m.params {
                    symbols.insert(
                        p.name.clone(),
                        SymbolDef {
                            name: p.name.clone(),
                            kind: SymbolKind::Param,
                            span: p.span,
                            ty: None,
                        },
                    );
                }
            }
        }
        Stmt::ImplBlock {
            struct_name,
            methods,
            ..
        } => {
            for m in methods {
                let full_name = format!("{}.{}", struct_name, m.name);
                symbols.insert(
                    full_name.clone(),
                    SymbolDef {
                        name: full_name,
                        kind: SymbolKind::Method,
                        span: m.span,
                        ty: None,
                    },
                );
                collect_from_expr(&m.body, symbols, references);
            }
        }
        Stmt::ExportStmt(inner) => {
            collect_from_stmt(inner, symbols, references);
        }
        Stmt::ExprStmt(expr) => {
            collect_from_expr(expr, symbols, references);
        }
        Stmt::Import { .. } => {} // imports are resolved by driver, not collected here
    }
}

fn collect_from_expr(
    expr: &Expr,
    symbols: &mut HashMap<String, SymbolDef>,
    references: &mut Vec<Reference>,
) {
    match expr {
        Expr::VarRef { name, span } => {
            references.push(Reference {
                span: *span,
                name: name.clone(),
            });
        }
        Expr::Lambda {
            params, body, ..
        } => {
            for p in params {
                symbols.insert(
                    p.name.clone(),
                    SymbolDef {
                        name: p.name.clone(),
                        kind: SymbolKind::Param,
                        span: p.span,
                        ty: None,
                    },
                );
            }
            collect_from_expr(body, symbols, references);
        }
        Expr::Call { func, arg } => {
            collect_from_expr(func, symbols, references);
            collect_from_expr(arg, symbols, references);
        }
        Expr::Binary { left, right, .. } => {
            collect_from_expr(left, symbols, references);
            collect_from_expr(right, symbols, references);
        }
        Expr::Unary { right, .. } => {
            collect_from_expr(right, symbols, references);
        }
        Expr::Block(stmts) => {
            for s in stmts {
                collect_from_stmt(s, symbols, references);
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_from_expr(cond, symbols, references);
            collect_from_expr(then_branch, symbols, references);
            if let Some(e) = else_branch {
                collect_from_expr(e, symbols, references);
            }
        }
        Expr::While { cond, body } => {
            collect_from_expr(cond, symbols, references);
            collect_from_expr(body, symbols, references);
        }
        Expr::For {
            var, iterable, body, ..
        } => {
            symbols.insert(
                var.name.clone(),
                SymbolDef {
                    name: var.name.clone(),
                    kind: SymbolKind::Var,
                    span: var.span,
                    ty: None,
                },
            );
            collect_from_expr(iterable, symbols, references);
            collect_from_expr(body, symbols, references);
        }
        Expr::Member { object, .. } => {
            collect_from_expr(object, symbols, references);
        }
        Expr::Index { object, index } => {
            collect_from_expr(object, symbols, references);
            collect_from_expr(index, symbols, references);
        }
        Expr::StructLit { fields, spread, .. } => {
            for (_, val) in fields {
                collect_from_expr(val, symbols, references);
            }
            if let Some(s) = spread {
                collect_from_expr(s, symbols, references);
            }
        }
        Expr::Assign { target, value } => {
            collect_from_expr(target, symbols, references);
            collect_from_expr(value, symbols, references);
        }
        Expr::Return(val) => {
            if let Some(v) = val {
                collect_from_expr(v, symbols, references);
            }
        }
        Expr::VariantLit { fields, .. } => {
            for f in fields {
                collect_from_expr(f, symbols, references);
            }
        }
        Expr::ListLit(items) => {
            for item in items {
                collect_from_expr(item, symbols, references);
            }
        }
        Expr::Tuple(items) => {
            for item in items {
                collect_from_expr(item, symbols, references);
            }
        }
        Expr::GetVariantTag(e) | Expr::GetVariantField { object: e, .. } => {
            collect_from_expr(e, symbols, references);
        }
        Expr::Async(e) | Expr::Await(e) => {
            collect_from_expr(e, symbols, references);
        }
        Expr::LitInt(_)
        | Expr::LitFloat(_)
        | Expr::LitString(_)
        | Expr::LitTrue
        | Expr::LitFalse
        | Expr::LitNull
        | Expr::Break
        | Expr::Continue => {}
    }
}

// ── Inlay hints: AST walk to find type annotations ──

fn collect_hints_stmt(
    stmt: &Stmt,
    type_env: &HashMap<String, Scheme>,
    source: &str,
    hints: &mut Vec<InlayHint>,
) {
    match stmt {
        Stmt::ConstDecl { name, span, value, .. } => {
            push_hint_with_value(name, span, Some(value), type_env, source, hints);
            collect_hints_expr(value, type_env, source, hints);
        }
        Stmt::VarDecl { name, span, value, .. } => {
            push_hint_with_value(name, span, value.as_ref(), type_env, source, hints);
            if let Some(v) = value {
                collect_hints_expr(v, type_env, source, hints);
            }
        }
        Stmt::StructDef { .. } | Stmt::EnumDef { .. } | Stmt::InterfaceDef { .. } => {
            // Types are explicit in the source — skip
        }
        Stmt::ImplBlock { struct_name, methods, .. } => {
            for m in methods {
                let full_name = format!("{}.{}", struct_name, m.name);
                push_hint(&full_name, &m.span, type_env, source, hints);
                collect_hints_expr(&m.body, type_env, source, hints);
            }
        }
        Stmt::ExprStmt(expr) => {
            collect_hints_expr(expr, type_env, source, hints);
        }
        Stmt::ExportStmt(inner) => {
            collect_hints_stmt(inner, type_env, source, hints);
        }
        Stmt::Import { .. } => {}
    }
}

fn collect_hints_expr(
    expr: &Expr,
    type_env: &HashMap<String, Scheme>,
    source: &str,
    hints: &mut Vec<InlayHint>,
) {
    match expr {
        Expr::Lambda { params, body, .. } => {
            for p in params {
                push_hint(&p.name, &p.span, type_env, source, hints);
            }
            collect_hints_expr(body, type_env, source, hints);
        }
        Expr::Call { func, arg } => {
            collect_hints_expr(func, type_env, source, hints);
            collect_hints_expr(arg, type_env, source, hints);
        }
        Expr::Binary { left, right, .. } => {
            collect_hints_expr(left, type_env, source, hints);
            collect_hints_expr(right, type_env, source, hints);
        }
        Expr::Unary { right, .. } => {
            collect_hints_expr(right, type_env, source, hints);
        }
        Expr::Block(stmts) => {
            for s in stmts {
                collect_hints_stmt(s, type_env, source, hints);
            }
        }
        Expr::If { cond, then_branch, else_branch } => {
            collect_hints_expr(cond, type_env, source, hints);
            collect_hints_expr(then_branch, type_env, source, hints);
            if let Some(e) = else_branch {
                collect_hints_expr(e, type_env, source, hints);
            }
        }
        Expr::While { cond, body } => {
            collect_hints_expr(cond, type_env, source, hints);
            collect_hints_expr(body, type_env, source, hints);
        }
        Expr::For { var, iterable, body, .. } => {
            push_hint(&var.name, &var.span, type_env, source, hints);
            collect_hints_expr(iterable, type_env, source, hints);
            collect_hints_expr(body, type_env, source, hints);
        }
        Expr::Assign { target, value } => {
            collect_hints_expr(target, type_env, source, hints);
            collect_hints_expr(value, type_env, source, hints);
        }
        Expr::Return(val) => {
            if let Some(v) = val {
                collect_hints_expr(v, type_env, source, hints);
            }
        }
        Expr::Member { object, .. } => {
            collect_hints_expr(object, type_env, source, hints);
        }
        Expr::Index { object, index } => {
            collect_hints_expr(object, type_env, source, hints);
            collect_hints_expr(index, type_env, source, hints);
        }
        Expr::StructLit { fields, spread, .. } => {
            for (_, val) in fields {
                collect_hints_expr(val, type_env, source, hints);
            }
            if let Some(s) = spread {
                collect_hints_expr(s, type_env, source, hints);
            }
        }
        Expr::ListLit(items) | Expr::Tuple(items) => {
            for item in items {
                collect_hints_expr(item, type_env, source, hints);
            }
        }
        _ => {}
    }
}

fn push_hint(
    name: &str,
    span: &Span,
    type_env: &HashMap<String, Scheme>,
    source: &str,
    hints: &mut Vec<InlayHint>,
) {
    let type_str = if let Some(scheme) = type_env.get(name) {
        Some(format!("{}", scheme.body))
    } else {
        // Fallback: try to infer type from the value expression
        None
    };

    if let Some(type_str) = type_str {
        // Skip uninformative types (type variables like "t0")
        if type_str.starts_with('t') && type_str.len() <= 3 {
            return;
        }
        if let Some(pos) = end_of_name_in_source(source, span, name) {
            hints.push(InlayHint {
                position: pos,
                label: format!(": {}", type_str),
            });
        }
    }
}

/// Like push_hint but also tries to guess the type from a value expression
/// when the name is not in the global type_env (local variables in blocks).
fn push_hint_with_value(
    name: &str,
    span: &Span,
    value: Option<&Expr>,
    type_env: &HashMap<String, Scheme>,
    source: &str,
    hints: &mut Vec<InlayHint>,
) {
    let type_str = if let Some(scheme) = type_env.get(name) {
        Some(format!("{}", scheme.body))
    } else if let Some(val) = value {
        guess_type(val)
    } else {
        None
    };

    if let Some(type_str) = type_str {
        if type_str.starts_with('t') && type_str.len() <= 3 {
            return;
        }
        if let Some(pos) = end_of_name_in_source(source, span, name) {
            hints.push(InlayHint {
                position: pos,
                label: format!(": {}", type_str),
            });
        }
    }
}

/// Guess a type from a simple expression (literals only).
fn guess_type(expr: &Expr) -> Option<String> {
    match expr {
        Expr::LitInt(_) => Some("Int64".to_string()),
        Expr::LitFloat(_) => Some("Float64".to_string()),
        Expr::LitString(_) => Some("String".to_string()),
        Expr::LitTrue | Expr::LitFalse => Some("Bool".to_string()),
        Expr::LitNull => Some("Null".to_string()),
        _ => None,
    }
}

/// Compute the char offset right after `name` in `source`, given its `span`.
fn end_of_name_in_source(source: &str, span: &Span, name: &str) -> Option<usize> {
    let chars: Vec<char> = source.chars().collect();
    let mut line = 1usize;
    let mut col = 1usize;
    let mut offset = 0usize;
    let mut found = false;

    for (i, ch) in chars.iter().enumerate() {
        if line == span.line && col == span.col {
            offset = i;
            found = true;
            break;
        }
        if *ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    if !found && (span.line > 1 || span.col > 1) {
        return None;
    }

    Some(offset + name.chars().count())
}
