//! Convert pipeline errors to JSON diagnostics.
use kaubo_ast::Stmt;
use kaubo_infer::infer_module;
use kaubo_syntax::parser::Parser;
use std::collections::HashSet;

pub fn diagnose(source: &str) -> String {
    let module = match Parser::new(source).parse() {
        Ok(m) => m,
        Err(e) => {
            return format!(
                "[{}]",
                serde_json::json!({
                    "severity": "error",
                    "line": e.line,
                    "column": e.col,
                    "message": e.to_string()
                })
            )
        }
    };

    // Collect imported names — these are resolved externally, so infer
    // shouldn't report them as "unbound".
    let imported: HashSet<String> = module
        .stmts
        .iter()
        .filter_map(|s| match s {
            Stmt::Import { names, .. } => Some(names.iter().cloned()),
            _ => None,
        })
        .flatten()
        .collect();

    match infer_module(&module) {
        Ok(_) => "[]".to_string(),
        Err(e) => {
            // Skip "unbound variable" if the name was imported
            if e.msg.starts_with("unbound variable") {
                for name in &imported {
                    if e.msg.contains(&format!("'{name}'")) {
                        return "[]".to_string();
                    }
                }
            }
            format!(
                "[{}]",
                serde_json::json!({
                    "severity": "error",
                    "line": e.line,
                    "column": e.col,
                    "message": e.msg
                })
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnose_valid_source_returns_empty() {
        let r = diagnose("const x = 42;");
        assert_eq!(r, "[]");
    }

    #[test]
    fn diagnose_parse_error() {
        let r = diagnose("var x = ;");
        assert!(r.contains("expected"));
        assert!(r.contains("error"));
        assert!(r.contains("line"));
        assert!(r.starts_with("["));
    }

    #[test]
    fn diagnose_type_error() {
        // "hello" + 1 fails type unification (String vs Int64)
        let r = diagnose("const x = \"hello\" + 1;");
        assert!(r.contains("error"), "should contain error: {r}");
        assert!(r.contains("line"), "should have line field: {r}");
    }

    #[test]
    fn diagnose_skips_imported_names() {
        let r = diagnose("import { add_a } from \"./A.kb\"; const r = add_a(10, 12);");
        assert_eq!(r, "[]", "imported name should not be reported as unbound: {r}");
    }
}
