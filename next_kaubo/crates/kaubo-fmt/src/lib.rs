//! Kaubo source code formatter — AST-based pretty-printer.
//!
//! # Architecture
//!
//! ```text
//! source → Parser → Module → Formatter → formatted output
//! ```
//!
//! Does NOT reuse LspCoordinator — only needs the parser, not type inference.

use kaubo_ast::{BinOp, Expr, Module, Stmt, TypeExpr, UnOp};
use kaubo_syntax::parser::Parser;
use std::fmt::Write as FmtWrite;

// ── Options ──

#[derive(Debug, Clone)]
pub struct FmtOptions {
    pub indent_size: u8,
    pub max_line_width: usize,
}

impl Default for FmtOptions {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_line_width: 100,
        }
    }
}

// ── Public API ──

/// Format Kaubo source code. Returns the formatted source or a parse error.
pub fn format(source: &str, options: &FmtOptions) -> Result<String, String> {
    let module = Parser::new(source).parse().map_err(|e| e.to_string())?;
    let mut f = Formatter::new(options);
    f.fmt_module(&module);
    Ok(f.output)
}

// ── Formatter ──

struct Formatter {
    options: FmtOptions,
    output: String,
    indent_level: usize,
}

impl Formatter {
    fn new(options: &FmtOptions) -> Self {
        Self {
            options: options.clone(),
            output: String::new(),
            indent_level: 0,
        }
    }

    // ── helpers ──

    fn indent_str(&self) -> String {
        " ".repeat(self.indent_level * self.options.indent_size as usize)
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn write_indent(&mut self) {
        self.output.push_str(&self.indent_str());
    }

    fn write_line(&mut self, s: &str) {
        let trimmed = s.trim_end();
        if trimmed.is_empty() {
            self.output.push('\n');
        } else {
            self.output.push_str(&self.indent_str());
            self.output.push_str(trimmed);
            self.output.push('\n');
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    // ── Module ──

    fn fmt_module(&mut self, module: &Module) {
        for (i, stmt) in module.stmts.iter().enumerate() {
            if i > 0 {
                // Blank line between top-level statements
                self.output.push('\n');
            }
            self.fmt_stmt(stmt);
        }
    }

    // ── Statements ──

    /// Write a statement ending: semicolon + newline, without extra indent.
    fn end_stmt(&mut self) {
        self.write(";\n");
    }

    fn fmt_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ConstDecl { name, ty_ann, value, .. } => {
                self.write_indent();
                self.write("const ");
                self.write(name);
                if let Some(ty) = ty_ann {
                    self.write(": ");
                    self.fmt_type_expr(ty);
                }
                self.write(" = ");
                self.fmt_expr(value, 0);
                self.end_stmt();
            }
            Stmt::VarDecl { name, ty_ann, value, .. } => {
                self.write_indent();
                self.write("var ");
                self.write(name);
                if let Some(ty) = ty_ann {
                    self.write(": ");
                    self.fmt_type_expr(ty);
                }
                if let Some(val) = value {
                    self.write(" = ");
                    self.fmt_expr(val, 0);
                }
                self.end_stmt();
            }
            Stmt::StructDef { name, fields, .. } => {
                self.write_indent();
                self.write("struct ");
                self.write(name);
                self.write_line(" {");
                self.indent();
                for f in fields {
                    self.write_indent();
                    self.write(&format!("{}: {},\n", f.name, type_expr_to_str(&f.ty)));
                }
                self.dedent();
                self.write_indent();
                self.write("}\n");
            }
            Stmt::EnumDef { name, variants, .. } => {
                self.write_indent();
                self.write("enum ");
                self.write(name);
                self.write_line(" {");
                self.indent();
                for v in variants {
                    if v.fields.is_empty() {
                        self.write_indent();
                        self.write(&format!("{},\n", v.name));
                    } else {
                        self.write_indent();
                        self.write(&v.name);
                        self.write("(");
                        let fs: Vec<String> = v
                            .fields
                            .iter()
                            .map(|f| format!("{}: {}", f.name, type_expr_to_str(&f.ty)))
                            .collect();
                        self.write(&fs.join(", "));
                        self.write("),\n");
                    }
                }
                self.dedent();
                self.write_indent();
                self.write("}\n");
            }
            Stmt::InterfaceDef { name, methods, .. } => {
                self.write_indent();
                self.write("interface ");
                self.write(name);
                self.write_line(" {");
                self.indent();
                for m in methods {
                    self.write_indent();
                    if m.operator {
                        self.write("operator ");
                    }
                    self.write(&m.name);
                    self.write(": |");
                    let ps: Vec<String> = m
                        .params
                        .iter()
                        .map(|p| {
                            if let Some(ty) = &p.ty_ann {
                                format!("{}: {}", p.name, type_expr_to_str(ty))
                            } else {
                                p.name.clone()
                            }
                        })
                        .collect();
                    self.write(&ps.join(", "));
                    self.write("|");
                    if let Some(ret) = &m.return_type {
                        self.write(" -> ");
                        self.fmt_type_expr(ret);
                    }
                    self.write(";\n");
                }
                self.dedent();
                self.write_indent();
                self.write("}\n");
            }
            Stmt::ImplBlock {
                struct_name,
                interface_name,
                methods,
                ..
            } => {
                self.write_indent();
                self.write("impl ");
                if let Some(iface) = interface_name {
                    self.write(iface);
                    self.write(" for ");
                }
                self.write(struct_name);
                self.write_line(" {");
                self.indent();
                for m in methods {
                    self.write_indent();
                    if m.operator {
                        self.write("operator ");
                    }
                    self.write(&m.name);
                    self.write(": ");
                    self.fmt_expr(&m.body, 0);
                    self.write(";\n");
                }
                self.dedent();
                self.write_indent();
                self.write("}\n");
            }
            Stmt::ExportStmt(inner) => {
                self.write_indent();
                self.write("export ");
                self.fmt_stmt(inner);
            }
            Stmt::Import {
                path,
                alias,
                names,
            } => {
                self.write_indent();
                self.write("import ");
                if names.is_empty() {
                    self.write(&format!("\"{}\"", path));
                } else {
                    self.write("{ ");
                    self.write(&names.join(", "));
                    self.write(&format!(" }} from \"{}\"", path));
                }
                if let Some(a) = alias {
                    self.write(&format!(" as {}", a));
                }
                self.end_stmt();
            }
            Stmt::ExprStmt(expr) => {
                if !matches!(expr, Expr::Block(_)) {
                    self.write_indent();
                    self.fmt_expr(expr, 0);
                    self.end_stmt();
                } else {
                    self.fmt_expr(expr, 0);
                    self.output.push('\n');
                }
            }
        }
    }

    // ── Expressions ──

    fn fmt_expr(&mut self, expr: &Expr, _parent_prec: u8) {
        match expr {
            Expr::LitInt(n) => self.write(&n.to_string()),
            Expr::LitFloat(n) => self.write(&n.to_string()),
            Expr::LitString(s) => {
                self.write("\"");
                self.write(s);
                self.write("\"");
            }
            Expr::LitTrue => self.write("true"),
            Expr::LitFalse => self.write("false"),
            Expr::LitNull => self.write("null"),

            Expr::VarRef { name, .. } => self.write(name),

            Expr::Lambda {
                params,
                ret_ty,
                body,
            } => {
                self.write("|");
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if let Some(ty) = &p.ty_ann {
                            format!("{}: {}", p.name, type_expr_to_str(ty))
                        } else {
                            p.name.clone()
                        }
                    })
                    .collect();
                self.write(&ps.join(", "));
                self.write("|");
                if let Some(ty) = ret_ty {
                    self.write(" -> ");
                    self.fmt_type_expr(ty);
                }
                self.write(" ");
                self.fmt_expr(body, 0);
            }

            Expr::Call { func, arg } => {
                self.fmt_expr(func, 0);
                self.write("(");
                let args = arg.as_args();
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.fmt_expr(a, 0);
                }
                self.write(")");
            }

            Expr::Binary { left, op, right } => {
                self.fmt_expr(left, 0);
                self.write(&format!(" {} ", binop_str(*op)));
                self.fmt_expr(right, 0);
            }

            Expr::Unary { op, right } => {
                self.write(unop_str(*op));
                self.fmt_expr(right, 0);
            }

            Expr::Block(stmts) => {
                self.write("{");
                self.output.push('\n');
                self.indent();
                for s in stmts {
                    self.fmt_stmt(s);
                }
                self.dedent();
                self.write_indent();
                self.write("}");
            }

            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.write("if (");
                self.fmt_expr(cond, 0);
                self.write(") ");
                self.fmt_expr(then_branch, 0);
                if let Some(else_b) = else_branch {
                    self.write(" else ");
                    self.fmt_expr(else_b, 0);
                }
            }

            Expr::While { cond, body } => {
                self.write("while (");
                self.fmt_expr(cond, 0);
                self.write(") ");
                self.fmt_expr(body, 0);
            }

            Expr::For {
                var,
                iterable,
                body,
            } => {
                self.write("for (");
                self.write(&var.name);
                self.write(" in ");
                self.fmt_expr(iterable, 0);
                self.write(") ");
                self.fmt_expr(body, 0);
            }

            Expr::Break => self.write("break"),
            Expr::Continue => self.write("continue"),

            Expr::Return(val) => {
                self.write("return");
                if let Some(v) = val {
                    self.write(" ");
                    self.fmt_expr(v, 0);
                }
            }

            Expr::Member { object, field } => {
                self.fmt_expr(object, 0);
                self.write(".");
                self.write(field);
            }

            Expr::Index { object, index } => {
                self.fmt_expr(object, 0);
                self.write("[");
                self.fmt_expr(index, 0);
                self.write("]");
            }

            Expr::StructLit {
                name,
                fields,
                spread,
            } => {
                self.write(name);
                self.write(" { ");
                let fs: Vec<String> = fields
                    .iter()
                    .map(|(n, v)| {
                        if let Expr::VarRef { name: vn, .. } = v {
                            if n == vn {
                                return n.clone(); // shorthand: { x } not { x: x }
                            }
                        }
                        format!("{}: {}", n, expr_to_str(v))
                    })
                    .collect();
                self.write(&fs.join(", "));
                if let Some(s) = spread {
                    if !fields.is_empty() {
                        self.write(", ");
                    }
                    self.write("...");
                    self.fmt_expr(s, 0);
                }
                self.write(" }");
            }

            Expr::VariantLit {
                variant_name,
                fields,
                ..
            } => {
                self.write(variant_name);
                if !fields.is_empty() {
                    self.write("(");
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            self.write(", ");
                        }
                        self.fmt_expr(f, 0);
                    }
                    self.write(")");
                }
            }

            Expr::ListLit(items) => {
                self.write("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.fmt_expr(item, 0);
                }
                self.write("]");
            }

            Expr::Tuple(items) => {
                self.write("(");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.fmt_expr(item, 0);
                }
                if items.len() == 1 {
                    self.write(",");
                }
                self.write(")");
            }

            Expr::Assign { target, value } => {
                self.fmt_expr(target, 0);
                self.write(" = ");
                self.fmt_expr(value, 0);
            }

            Expr::GetVariantTag(e) => {
                self.write("tag(");
                self.fmt_expr(e, 0);
                self.write(")");
            }

            Expr::GetVariantField { object, .. } => {
                self.fmt_expr(object, 0);
            }

            Expr::Async(e) => {
                self.write("async ");
                self.fmt_expr(e, 0);
            }

            Expr::Await(e) => {
                self.write("await ");
                self.fmt_expr(e, 0);
            }
        }
    }

    fn fmt_type_expr(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Named(n) => self.write(n),
            TypeExpr::List(t) => {
                self.write("List<");
                self.fmt_type_expr(t);
                self.write(">");
            }
            TypeExpr::Tuple(items) => {
                self.write("(");
                for (i, t) in items.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.fmt_type_expr(t);
                }
                self.write(")");
            }
            TypeExpr::Arrow { params, ret } => {
                self.write("|");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.fmt_type_expr(p);
                }
                self.write("| -> ");
                self.fmt_type_expr(ret);
            }
        }
    }
}

// ── Helpers ──

fn binop_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::Pipe => "|>",
        BinOp::GtGt => ">>",
        BinOp::SAdd => "+",
    }
}

fn unop_str(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "not ",
    }
}

fn type_expr_to_str(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named(n) => n.clone(),
        TypeExpr::List(t) => format!("List<{}>", type_expr_to_str(t)),
        TypeExpr::Tuple(items) => {
            let ts: Vec<String> = items.iter().map(type_expr_to_str).collect();
            format!("({})", ts.join(", "))
        }
        TypeExpr::Arrow { params, ret } => {
            let ps: Vec<String> = params.iter().map(type_expr_to_str).collect();
            format!("|{}| -> {}", ps.join(", "), type_expr_to_str(ret))
        }
    }
}

/// Quick inline expression string for struct literal fields.
fn expr_to_str(expr: &Expr) -> String {
    match expr {
        Expr::LitInt(n) => n.to_string(),
        Expr::LitFloat(n) => n.to_string(),
        Expr::LitString(s) => format!("\"{}\"", s),
        Expr::LitTrue => "true".to_string(),
        Expr::LitFalse => "false".to_string(),
        Expr::LitNull => "null".to_string(),
        Expr::VarRef { name, .. } => name.clone(),
        other => format!("{other:?}"),
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        format(src, &FmtOptions::default()).unwrap()
    }

    #[test]
    fn fmt_simple_const() {
        assert_eq!(fmt("const x=42;"), "const x = 42;\n");
    }

    #[test]
    fn fmt_const_with_type() {
        assert_eq!(
            fmt("const pi:Float64=3.14;"),
            "const pi: Float64 = 3.14;\n"
        );
    }

    #[test]
    fn fmt_var() {
        assert_eq!(fmt("var x:Int64=0;"), "var x: Int64 = 0;\n");
        assert_eq!(fmt("var x;"), "var x;\n");
    }

    #[test]
    fn fmt_struct_def() {
        let result = fmt("struct Point{x:Int64,y:Int64}");
        let expected = "struct Point {\n    x: Int64,\n    y: Int64,\n}\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn fmt_enum_def() {
        let result = fmt("enum Option{Some(value:Int64),None}");
        let expected = "enum Option {\n    Some(value: Int64),\n    None,\n}\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn fmt_interface_def() {
        // Note: parser requires all interface params to have explicit type annotations
        let result = fmt("interface Display { to_string: |self: Self|; }");
        let expected = "interface Display {\n    to_string: |self: Self|;\n}\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn fmt_lambda() {
        assert_eq!(
            fmt("const add=|a,b|{a+b};"),
            "const add = |a, b| {\n    a + b;\n};\n"
        );
    }

    #[test]
    fn fmt_if_else() {
        let result = fmt("const r=if(x<0){-x}else{x};");
        let expected = "const r = if (x < 0) {\n    -x;\n} else {\n    x;\n};\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn fmt_while_loop() {
        let result = fmt("while(i<10){i=i+1;};");
        let expected = "while (i < 10) {\n    i = i + 1;\n};\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn fmt_binary_expr() {
        assert_eq!(fmt("const x=1+2*3;"), "const x = 1 + 2 * 3;\n");
    }

    #[test]
    fn fmt_call() {
        assert_eq!(fmt("print(42);"), "print(42);\n");
        assert_eq!(fmt("f(a,b);"), "f(a, b);\n");
    }

    #[test]
    fn fmt_import() {
        assert_eq!(
            fmt("import{add}from\"./math.kb\";"),
            "import { add } from \"./math.kb\";\n"
        );
        assert_eq!(
            fmt("import\"std\"as std;"),
            "import \"std\" as std;\n"
        );
    }

    #[test]
    fn fmt_export() {
        assert_eq!(
            fmt("export const VERSION=1;"),
            "export const VERSION = 1;\n"
        );
    }

    #[test]
    fn fmt_member_access() {
        assert_eq!(fmt("p.x;"), "p.x;\n");
    }

    #[test]
    fn fmt_empty_source() {
        assert_eq!(fmt(""), "");
    }

    #[test]
    fn fmt_returns_result_on_parse_error() {
        assert!(format("const x = ;", &FmtOptions::default()).is_err());
    }
}
