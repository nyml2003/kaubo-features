//! LspCoordinator — wires FrontendStage + SemanticStage for editor features.
//!
//! Caches parse + infer results so hover / goto_def / completions are
//! read-only queries over the last successful build.

use kaubo_ast::{Expr, Module, Span, Stmt};
use kaubo_driver::stages::{FrontendStage, SemanticStage};
use kaubo_driver::protocol::{BuildContext, BuildError, Stage};
use kaubo_driver::SemanticArtifact;
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
        let (symbols, references) = collect_symbols_and_refs(&module);

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
                return Some(HoverInfo {
                    kind: "variable".to_string(),
                    ty: Some(format!("{}", scheme.body)),
                    description: format!("variable {}", name),
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
