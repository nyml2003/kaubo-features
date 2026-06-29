//! DagLspCoordinator — DAG-powered LSP coordinator.
//!
//! Uses the `kaubo_dag` scheduler for compilation, giving LSP features:
//! automatic caching, cancellation support, and WASM compatibility
//! (via WasmSpawner).

use kaubo_ast::{Expr, Module, Span, Stmt};
use kaubo_dag::{Artifact, ArtifactKey, DagScheduler, FetcherRegistry, Kind};
use kaubo_driver::fetchers::semantic::SemanticFetcher;
use kaubo_driver::SemanticArtifact;
use kaubo_infer::Scheme;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use kaubo_dag::NativeSpawner;
#[cfg(target_arch = "wasm32")]
use kaubo_dag::WasmSpawner;

use crate::{CompletionItem, HoverInfo, InlayHint, Reference, SymbolDef, SymbolKind};

// ── DagLspCoordinator ────────────────────────────────────────────────

/// An LSP coordinator backed by the DAG scheduler.
///
/// On each source change, the scheduler runs parse→infer via Fetchers,
/// with automatic caching. Query methods (hover, goto_def, completions)
/// read from locally cached data — they are synchronous.
pub struct DagLspCoordinator {
    scheduler: Arc<DagScheduler<String>>,
    source: String,
    module: Option<Module>,
    semantic: Option<SemanticArtifact>,
    symbols: HashMap<String, SymbolDef>,
    references: Vec<Reference>,
}

impl DagLspCoordinator {
    /// Create a new DagLspCoordinator with the platform-appropriate spawner.
    ///
    /// Registers SemanticFetcher. AstFetcher is not registered — the
    /// coordinator parses inline and seeds the AST artifact directly.
    pub fn new() -> Self {
        let registry = FetcherRegistry::<String>::new();

        // Register SemanticFetcher — depends on AST artifact
        registry.register(
            Kind::new(Kind::SEMANTIC),
            Box::new(|key| Box::new(SemanticFetcher::new(key.module_id.clone()))),
        );

        #[cfg(not(target_arch = "wasm32"))]
        let scheduler = DagScheduler::new(registry, Arc::new(NativeSpawner));
        #[cfg(target_arch = "wasm32")]
        let scheduler = DagScheduler::new(registry, Arc::new(WasmSpawner));

        DagLspCoordinator {
            scheduler,
            source: String::new(),
            module: None,
            semantic: None,
            symbols: HashMap::new(),
            references: Vec::new(),
        }
    }

    /// Process a source change asynchronously: parse → seed AST → request semantic.
    pub async fn on_change(&mut self, source: &str) -> Result<(), kaubo_dag::DagError<String>> {
        self.source = source.to_string();
        let module_id = "lsp".to_string();

        // 1. Parse
        let module = kaubo_syntax::parser::Parser::new(source).parse().map_err(|e| {
            kaubo_dag::DagError::fetcher_error(
                ArtifactKey::new(module_id.clone(), Kind::new(Kind::AST)),
                format!("parse: {e}"),
            )
        })?;

        // 2. Collect symbols and references from AST
        let (mut symbols, references) = collect_symbols_and_refs(&module);

        // 3. Seed AST into scheduler so SemanticFetcher can find it
        let ast_artifact = Artifact::new(module_id.clone(), Kind::new(Kind::AST), module.clone());
        self.scheduler.seed_artifact(ast_artifact);

        // 4. Request Semantic via DAG — this triggers SemanticFetcher
        let semantic_key = ArtifactKey::new(module_id.clone(), Kind::new(Kind::SEMANTIC));
        // We need to go through the scheduler's build mechanism.
        // Use a simple builder that returns the SemanticArtifact.
        let builder = Box::new(SemanticBuilder { module_id: module_id.clone() });
        let stream = self.scheduler.build::<SemanticArtifact>(builder);
        futures::pin_mut!(stream);
        let semantic = match futures::StreamExt::next(&mut stream).await {
            Some(kaubo_dag::BuilderEvent::Done(sem)) => sem,
            Some(kaubo_dag::BuilderEvent::Error(e)) => return Err((*e).clone()),
            None => return Err(kaubo_dag::DagError::Internal("semantic stream empty".into())),
        };

        // 5. Cross-reference symbols with inferred types
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

    // ── Queries (identical to LspCoordinator) ──────────────────────

    pub fn symbol_at(&self, offset: usize) -> Option<&SymbolDef> {
        let name = identifier_at(&self.source, offset)?;
        self.symbols.get(&name)
    }

    pub fn goto_def(&self, offset: usize) -> Option<Span> {
        let name = identifier_at(&self.source, offset)?;
        for r in &self.references {
            if spans_contain(&r.span, offset, &self.source) && r.name == name {
                return self.symbols.get(&r.name).map(|s| s.span);
            }
        }
        self.symbols.get(&name).map(|s| s.span)
    }

    pub fn hover(&self, offset: usize) -> Option<HoverInfo> {
        let name = identifier_at(&self.source, offset)?;
        if let Some(sym) = self.symbols.get(&name) {
            return Some(HoverInfo {
                kind: sym.kind.as_str().to_string(),
                ty: sym.ty.clone(),
                description: format!("{} {}", sym.kind.as_str(), name),
            });
        }
        if let Some(ref semantic) = self.semantic {
            if let Some(scheme) = semantic.type_env.get(&name) {
                let (kind_str, desc) = if let Some(sym) = self.symbols.get(&name) {
                    (sym.kind.as_str().to_string(), format!("{} {}", sym.kind.as_str(), name))
                } else {
                    ("variable".to_string(), format!("variable {}", name))
                };
                return Some(HoverInfo { kind: kind_str, ty: Some(format!("{}", scheme.body)), description: desc });
            }
        }
        None
    }

    pub fn completions(&self, offset: usize) -> Vec<CompletionItem> {
        let token_items = crate::completions(&self.source, offset);
        if !token_items.is_empty() { return token_items; }
        let prefix = prefix_at(&self.source, offset).unwrap_or_default();
        let mut items = Vec::new();
        for (_, sym) in &self.symbols {
            if prefix.is_empty() || sym.name.starts_with(&prefix) {
                items.push(CompletionItem { label: sym.name.clone(), kind: sym.kind.as_str().to_string(), detail: sym.ty.clone() });
            }
        }
        if let Some(ref semantic) = self.semantic {
            for (name, scheme) in &semantic.type_env {
                if !self.symbols.contains_key(name) {
                    if prefix.is_empty() || name.starts_with(&prefix) {
                        items.push(CompletionItem { label: name.clone(), kind: "variable".to_string(), detail: Some(format!("{}", scheme.body)) });
                    }
                }
            }
        }
        items
    }

    pub fn inlay_hints(&self) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        let Some(ref module) = self.module else { return hints; };
        let Some(ref semantic) = self.semantic else { return hints; };
        for stmt in &module.stmts {
            collect_hints_stmt(stmt, &semantic.type_env, &self.source, &mut hints);
        }
        hints
    }

    pub fn is_ready(&self) -> bool { self.semantic.is_some() }
    pub fn semantic(&self) -> Option<&SemanticArtifact> { self.semantic.as_ref() }
    pub fn module(&self) -> Option<&Module> { self.module.as_ref() }
}

impl Default for DagLspCoordinator {
    fn default() -> Self { Self::new() }
}

// ── Internal builder for requesting Semantic ─────────────────────────

use std::future::Future;
use std::pin::Pin;

struct SemanticBuilder {
    module_id: String,
}

impl kaubo_dag::Builder<String, SemanticArtifact> for SemanticBuilder {
    fn name(&self) -> &str { "Semantic" }
    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::SEMANTIC))]
    }
    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut kaubo_dag::FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<SemanticArtifact, kaubo_dag::DagError<String>>> + Send + 'a>> {
        let sem = inputs.into_iter().next().unwrap().downcast_clone::<SemanticArtifact>();
        Box::pin(async move { Ok(sem) })
    }
}

// ── Helpers (shared with LspCoordinator) ─────────────────────────────

fn prefix_at(source: &str, offset: usize) -> Option<String> {
    let bytes: Vec<char> = source.chars().collect();
    let idx = offset.min(bytes.len());
    let mut start = idx;
    while start > 0 && is_ident_char(bytes[start - 1]) { start -= 1; }
    if start == idx { return None; }
    Some(bytes[start..idx].iter().collect())
}

fn identifier_at(source: &str, offset: usize) -> Option<String> {
    if offset >= source.len() { return None; }
    let bytes: Vec<char> = source.chars().collect();
    if offset >= bytes.len() || !is_ident_char(bytes[offset]) { return None; }
    let mut start = offset;
    while start > 0 && is_ident_char(bytes[start - 1]) { start -= 1; }
    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes[end]) { end += 1; }
    Some(bytes[start..end].iter().collect())
}

fn is_ident_char(c: char) -> bool { c.is_alphanumeric() || c == '_' }
fn spans_contain(_span: &Span, _offset: usize, _source: &str) -> bool { true }

// ── Symbol collection ────────────────────────────────────────────────

fn collect_symbols_and_refs(module: &Module) -> (HashMap<String, SymbolDef>, Vec<Reference>) {
    let mut symbols: HashMap<String, SymbolDef> = HashMap::new();
    let mut references: Vec<Reference> = Vec::new();
    for stmt in &module.stmts { collect_from_stmt(stmt, &mut symbols, &mut references); }
    (symbols, references)
}

fn collect_from_stmt(stmt: &Stmt, symbols: &mut HashMap<String, SymbolDef>, references: &mut Vec<Reference>) {
    match stmt {
        Stmt::ConstDecl { name, span, value, .. } => {
            symbols.insert(name.clone(), SymbolDef { name: name.clone(), kind: SymbolKind::Const, span: *span, ty: None });
            collect_from_expr(value, symbols, references);
        }
        Stmt::VarDecl { name, span, value, .. } => {
            symbols.insert(name.clone(), SymbolDef { name: name.clone(), kind: SymbolKind::Var, span: *span, ty: None });
            if let Some(v) = value { collect_from_expr(v, symbols, references); }
        }
        Stmt::StructDef { name, span, fields, .. } => {
            symbols.insert(name.clone(), SymbolDef { name: name.clone(), kind: SymbolKind::Struct, span: *span, ty: None });
            for f in fields { symbols.insert(f.name.clone(), SymbolDef { name: f.name.clone(), kind: SymbolKind::Field, span: f.span, ty: None }); }
        }
        Stmt::EnumDef { name, span, variants, .. } => {
            symbols.insert(name.clone(), SymbolDef { name: name.clone(), kind: SymbolKind::Enum, span: *span, ty: None });
            for v in variants { symbols.insert(v.name.clone(), SymbolDef { name: v.name.clone(), kind: SymbolKind::Variant, span: v.span, ty: None }); }
        }
        Stmt::InterfaceDef { name, span, methods, .. } => {
            symbols.insert(name.clone(), SymbolDef { name: name.clone(), kind: SymbolKind::Interface, span: *span, ty: None });
            for m in methods {
                symbols.insert(m.name.clone(), SymbolDef { name: m.name.clone(), kind: SymbolKind::Method, span: Span::ZERO, ty: None });
                for p in &m.params { symbols.insert(p.name.clone(), SymbolDef { name: p.name.clone(), kind: SymbolKind::Param, span: p.span, ty: None }); }
            }
        }
        Stmt::ImplBlock { struct_name, methods, .. } => {
            for m in methods {
                let full_name = format!("{}.{}", struct_name, m.name);
                symbols.insert(full_name.clone(), SymbolDef { name: full_name, kind: SymbolKind::Method, span: m.span, ty: None });
                collect_from_expr(&m.body, symbols, references);
            }
        }
        Stmt::ExportStmt(inner) => collect_from_stmt(inner, symbols, references),
        Stmt::ExprStmt(expr) => collect_from_expr(expr, symbols, references),
        Stmt::Import { .. } => {}
    }
}

fn collect_from_expr(expr: &Expr, symbols: &mut HashMap<String, SymbolDef>, references: &mut Vec<Reference>) {
    match expr {
        Expr::VarRef { name, span } => { references.push(Reference { span: *span, name: name.clone() }); }
        Expr::Lambda { params, body, .. } => {
            for p in params { symbols.insert(p.name.clone(), SymbolDef { name: p.name.clone(), kind: SymbolKind::Param, span: p.span, ty: None }); }
            collect_from_expr(body, symbols, references);
        }
        Expr::Call { func, arg } => { collect_from_expr(func, symbols, references); collect_from_expr(arg, symbols, references); }
        Expr::Binary { left, right, .. } => { collect_from_expr(left, symbols, references); collect_from_expr(right, symbols, references); }
        Expr::Unary { right, .. } => { collect_from_expr(right, symbols, references); }
        Expr::Block(stmts) => { for s in stmts { collect_from_stmt(s, symbols, references); } }
        Expr::If { cond, then_branch, else_branch } => {
            collect_from_expr(cond, symbols, references); collect_from_expr(then_branch, symbols, references);
            if let Some(e) = else_branch { collect_from_expr(e, symbols, references); }
        }
        Expr::While { cond, body } => { collect_from_expr(cond, symbols, references); collect_from_expr(body, symbols, references); }
        Expr::For { var, iterable, body, .. } => {
            symbols.insert(var.name.clone(), SymbolDef { name: var.name.clone(), kind: SymbolKind::Var, span: var.span, ty: None });
            collect_from_expr(iterable, symbols, references); collect_from_expr(body, symbols, references);
        }
        Expr::Member { object, .. } => { collect_from_expr(object, symbols, references); }
        Expr::Index { object, index } => { collect_from_expr(object, symbols, references); collect_from_expr(index, symbols, references); }
        Expr::StructLit { fields, spread, .. } => {
            for (_, val) in fields { collect_from_expr(val, symbols, references); }
            if let Some(s) = spread { collect_from_expr(s, symbols, references); }
        }
        Expr::Assign { target, value } => { collect_from_expr(target, symbols, references); collect_from_expr(value, symbols, references); }
        Expr::Return(val) => { if let Some(v) = val { collect_from_expr(v, symbols, references); } }
        Expr::VariantLit { fields, .. } => { for f in fields { collect_from_expr(f, symbols, references); } }
        Expr::ListLit(items) | Expr::Tuple(items) => { for item in items { collect_from_expr(item, symbols, references); } }
        Expr::GetVariantTag(e) | Expr::GetVariantField { object: e, .. } => { collect_from_expr(e, symbols, references); }
        Expr::Async(e) | Expr::Await(e) => { collect_from_expr(e, symbols, references); }
        Expr::LitInt(_) | Expr::LitFloat(_) | Expr::LitString(_) | Expr::LitTrue | Expr::LitFalse | Expr::LitNull | Expr::Break | Expr::Continue => {}
    }
}

// ── Inlay hints ──────────────────────────────────────────────────────

fn collect_hints_stmt(stmt: &Stmt, type_env: &HashMap<String, Scheme>, source: &str, hints: &mut Vec<InlayHint>) {
    match stmt {
        Stmt::ConstDecl { name, span, value, .. } => {
            push_hint_with_value(name, span, Some(value), type_env, source, hints);
            collect_hints_expr(value, type_env, source, hints);
        }
        Stmt::VarDecl { name, span, value, .. } => {
            push_hint_with_value(name, span, value.as_ref(), type_env, source, hints);
            if let Some(v) = value { collect_hints_expr(v, type_env, source, hints); }
        }
        Stmt::StructDef { .. } | Stmt::EnumDef { .. } | Stmt::InterfaceDef { .. } => {}
        Stmt::ImplBlock { struct_name, methods, .. } => {
            for m in methods {
                let full_name = format!("{}.{}", struct_name, m.name);
                push_hint_qualified(&full_name, &m.name, &m.span, type_env, source, hints);
                collect_hints_expr(&m.body, type_env, source, hints);
            }
        }
        Stmt::ExprStmt(expr) => collect_hints_expr(expr, type_env, source, hints),
        Stmt::ExportStmt(inner) => collect_hints_stmt(inner, type_env, source, hints),
        Stmt::Import { .. } => {}
    }
}

fn collect_hints_expr(expr: &Expr, type_env: &HashMap<String, Scheme>, source: &str, hints: &mut Vec<InlayHint>) {
    match expr {
        Expr::Lambda { params, body, .. } => {
            for p in params { if p.ty_ann.is_none() { push_hint(&p.name, &p.span, type_env, source, hints); } }
            collect_hints_expr(body, type_env, source, hints);
        }
        Expr::Call { func, arg } => { collect_hints_expr(func, type_env, source, hints); collect_hints_expr(arg, type_env, source, hints); }
        Expr::Binary { left, right, .. } => { collect_hints_expr(left, type_env, source, hints); collect_hints_expr(right, type_env, source, hints); }
        Expr::Unary { right, .. } => { collect_hints_expr(right, type_env, source, hints); }
        Expr::Block(stmts) => { for s in stmts { collect_hints_stmt(s, type_env, source, hints); } }
        Expr::If { cond, then_branch, else_branch } => {
            collect_hints_expr(cond, type_env, source, hints); collect_hints_expr(then_branch, type_env, source, hints);
            if let Some(e) = else_branch { collect_hints_expr(e, type_env, source, hints); }
        }
        Expr::While { cond, body } => { collect_hints_expr(cond, type_env, source, hints); collect_hints_expr(body, type_env, source, hints); }
        Expr::For { var, iterable, body, .. } => {
            push_hint(&var.name, &var.span, type_env, source, hints);
            collect_hints_expr(iterable, type_env, source, hints); collect_hints_expr(body, type_env, source, hints);
        }
        Expr::Assign { target, value } => { collect_hints_expr(target, type_env, source, hints); collect_hints_expr(value, type_env, source, hints); }
        Expr::Return(val) => { if let Some(v) = val { collect_hints_expr(v, type_env, source, hints); } }
        Expr::Member { object, .. } => { collect_hints_expr(object, type_env, source, hints); }
        Expr::Index { object, index } => { collect_hints_expr(object, type_env, source, hints); collect_hints_expr(index, type_env, source, hints); }
        Expr::StructLit { fields, spread, .. } => {
            for (_, val) in fields { collect_hints_expr(val, type_env, source, hints); }
            if let Some(s) = spread { collect_hints_expr(s, type_env, source, hints); }
        }
        Expr::ListLit(items) | Expr::Tuple(items) => { for item in items { collect_hints_expr(item, type_env, source, hints); } }
        _ => {}
    }
}

fn push_hint(name: &str, span: &Span, type_env: &HashMap<String, Scheme>, source: &str, hints: &mut Vec<InlayHint>) {
    if let Some(scheme) = type_env.get(name) {
        let type_str = format!("{}", scheme.body);
        if type_str.starts_with('t') && type_str.len() <= 3 { return; }
        if let Some(pos) = end_of_name_in_source(source, span, name) {
            hints.push(InlayHint { position: pos, label: format!(": {}", type_str) });
        }
    }
}

fn push_hint_qualified(lookup_name: &str, display_name: &str, span: &Span, type_env: &HashMap<String, Scheme>, source: &str, hints: &mut Vec<InlayHint>) {
    if let Some(scheme) = type_env.get(lookup_name) {
        let type_str = format!("{}", scheme.body);
        if type_str.starts_with('t') && type_str.len() <= 3 { return; }
        if let Some(pos) = end_of_name_in_source(source, span, display_name) {
            hints.push(InlayHint { position: pos, label: format!(": {}", type_str) });
        }
    }
}

fn push_hint_with_value(name: &str, span: &Span, value: Option<&Expr>, type_env: &HashMap<String, Scheme>, source: &str, hints: &mut Vec<InlayHint>) {
    let type_str = if let Some(scheme) = type_env.get(name) {
        Some(format!("{}", scheme.body))
    } else if let Some(val) = value {
        guess_type(val)
    } else { None };

    if let Some(type_str) = type_str {
        if type_str.starts_with('t') && type_str.len() <= 3 { return; }
        if let Some(pos) = end_of_name_in_source(source, span, name) {
            hints.push(InlayHint { position: pos, label: format!(": {}", type_str) });
        }
    }
}

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

fn end_of_name_in_source(source: &str, span: &Span, name: &str) -> Option<usize> {
    let chars: Vec<char> = source.chars().collect();
    let mut line = 1usize; let mut col = 1usize; let mut approx_offset = 0usize; let mut found = false;
    for (i, ch) in chars.iter().enumerate() {
        if line == span.line && col == span.col { approx_offset = i; found = true; break; }
        if *ch == '\n' { line += 1; col = 1; } else { col += 1; }
    }
    if !found { return None; }
    let start = approx_offset;
    let end = (start + 50).min(chars.len());
    let window: String = chars[start..end].iter().collect();
    if window.starts_with(name) { Some(start + name.chars().count()) }
    else { window.find(name).map(|rel| start + rel + name.chars().count()) }
}
