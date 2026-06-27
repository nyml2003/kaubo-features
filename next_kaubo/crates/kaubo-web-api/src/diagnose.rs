//! Convert pipeline errors to JSON diagnostics.
use kaubo_infer::infer_module;
use kaubo_syntax::parser::Parser;

pub fn diagnose(source: &str) -> String {
    let module = match Parser::new(source).parse() {
        Ok(m) => m,
        Err(e) => {
            return format!(
                "[{}]",
                serde_json::json!({
                    "severity": "error",
                    "from": 0,
                    "to": 0,
                    "message": e
                })
            )
        }
    };
    match infer_module(&module) {
        Ok(_) => "[]".to_string(),
        Err(e) => format!(
            "[{}]",
            serde_json::json!({
                "severity": "error",
                "from": e.line,
                "to": e.line,
                "message": e.msg
            })
        ),
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
        assert!(r.starts_with("["));
    }

    #[test]
    fn diagnose_type_error() {
        // "hello" + 1 fails type unification (String vs Int64)
        let r = diagnose("const x = \"hello\" + 1;");
        assert!(r.contains("error"), "should contain error: {r}");
    }
}
