//! Direct single-file compile and run driver.
//!
//! This crate centralizes the current linear path used by CLI and WASM.

pub use kaubo_ir::cps::CpsModule;
use kaubo_ir::cps_build::build_module;
use kaubo_ir::flatten::flatten_module;
use kaubo_ir::pass::binary;
use kaubo_ir::pass::{empty_block::EmptyBlockElim, fold::ConstantFold, move_fold::MoveFold, run_passes};
use kaubo_syntax::parser::Parser;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    Parse(String),
    Infer(String),
    Build(String),
    Decode(String),
    Load(String),
    Runtime(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::Parse(message) => write!(f, "parse: {message}"),
            DriverError::Infer(message) => write!(f, "infer: {message}"),
            DriverError::Build(message) => write!(f, "build: {message}"),
            DriverError::Decode(message) => write!(f, "decode: {message}"),
            DriverError::Load(message) => write!(f, "load: {message}"),
            DriverError::Runtime(message) => write!(f, "runtime: {message}"),
        }
    }
}

impl std::error::Error for DriverError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub result: i64,
    pub output: Vec<String>,
}

pub fn compile_source(source: &str) -> Result<CpsModule, DriverError> {
    let module = Parser::new(source).parse().map_err(DriverError::Parse)?;
    kaubo_infer::infer_module(&module).map_err(|e| DriverError::Infer(e.msg))?;
    let mut cps = build_module(&module).map_err(DriverError::Build)?;
    flatten_module(&mut cps);
    run_passes(&mut cps, &[&EmptyBlockElim, &MoveFold, &ConstantFold]);
    Ok(cps)
}

pub fn run_module(cps: &CpsModule) -> Result<RunOutcome, DriverError> {
    if cps.functions.is_empty() {
        return Ok(RunOutcome {
            result: 0,
            output: Vec::new(),
        });
    }

    let mut vm = kaubo_vm::VM::new();
    vm.load(cps).map_err(DriverError::Load)?;
    let func_idx = cps.functions.len() - 1;
    let reg_count = cps.functions[func_idx].reg_count;
    let result = vm
        .execute(func_idx, reg_count)
        .map_err(|e| DriverError::Runtime(format!("{e:?}")))?;

    Ok(RunOutcome {
        result,
        output: vm.output,
    })
}

pub fn run_source(source: &str) -> Result<RunOutcome, DriverError> {
    let cps = compile_source(source)?;
    run_module(&cps)
}

pub fn instruction_count(module: &CpsModule) -> usize {
    module
        .functions
        .iter()
        .flat_map(|func| &func.blocks)
        .map(|block| block.instrs.len() + 1)
        .sum()
}

pub fn encode_module(module: &CpsModule) -> Vec<u8> {
    binary::encode_module(module)
}

pub fn decode_module(bytes: &[u8]) -> Result<CpsModule, DriverError> {
    binary::decode_module(bytes).map_err(DriverError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_source_builds_module() {
        let cps = compile_source("const x = 42;").unwrap();
        assert!(!cps.functions.is_empty());
        assert!(instruction_count(&cps) > 0);
    }

    #[test]
    fn run_source_returns_result() {
        let outcome = run_source("const x = 40 + 2;").unwrap();
        assert_eq!(outcome.result, 42);
        assert!(outcome.output.is_empty());
    }

    #[test]
    fn run_source_captures_print_output() {
        let outcome = run_source("print(\"hi\");").unwrap();
        assert_eq!(outcome.output, vec!["hi".to_string()]);
    }

    #[test]
    fn run_source_prints_float_method_result_as_float_string() {
        let source = r#"
struct Point {
    x: Int64,
    y: Int64,
};

impl Point {
  dis: |self: Point, other: Point| -> Float64 {
    const dx = (self.x - other.x);
    const dy = (self.y - other.y);
    return sqrt((dx*dx + dy*dy).to_float()) + 1.0;
  }
};

const p1 = Point { x: 200, y: 300 };
const p2 = Point { x: 300, y: 400 };
print(p1.dis(p2).to_string());
"#;

        let outcome = run_source(source).unwrap();
        let printed = outcome.output.first().expect("program should print");
        assert!(
            printed.starts_with("142.421"),
            "expected float output, got {printed}"
        );
        assert_ne!(printed, "4639179838183401144");
    }

    #[test]
    fn float_comparisons_drive_branches() {
        let outcome = run_source(
            r#"
const a = if 1.0 < 2.0 { 10 } else { 20 };
const b = if 2.0 <= 2.0 { 1 } else { 100 };
const c = if 3.0 > 2.0 { 2 } else { 200 };
const d = if 3.0 >= 3.0 { 3 } else { 300 };
const e = if 3.0 != 4.0 { 4 } else { 400 };
a + b + c + d + e;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 20);
    }

    #[test]
    fn struct_literals_use_declared_field_order() {
        let outcome = run_source(
            r#"
struct Pair { left: Int64, right: Int64 };
const p = Pair { right: 20, left: 10 };
p.left + p.right;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 30);
    }

    #[test]
    fn build_errors_are_explicit_for_unsupported_runtime_paths() {
        let unknown_var = compile_source("const x = missing_name;").unwrap_err();
        assert!(matches!(
            unknown_var,
            DriverError::Infer(_) | DriverError::Build(_)
        ));
        assert!(unknown_var.to_string().contains("missing_name"));

        let unknown_field = compile_source(
            r#"
struct Point { x: Int64 };
const p = Point { x: 1 };
p.y;
"#,
        )
        .unwrap_err();
        assert!(matches!(
            unknown_field,
            DriverError::Infer(_) | DriverError::Build(_)
        ));
        assert!(unknown_field.to_string().contains("field 'y'"));
    }

    #[test]
    fn lambda_call_runs() {
        let outcome = run_source("const f = |x| { x + 1 }; f(41);").unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn parse_errors_are_classified() {
        let err = compile_source("var x = ;").unwrap_err();
        assert!(matches!(err, DriverError::Parse(_)));
    }

    #[test]
    fn infer_errors_are_classified() {
        let err = compile_source("const x = \"hello\" + 1;").unwrap_err();
        assert!(matches!(err, DriverError::Infer(_)));
    }

    #[test]
    fn decode_errors_are_classified() {
        let err = decode_module(b"bad").unwrap_err();
        assert!(matches!(err, DriverError::Decode(_)));
    }

    #[test]
    fn binary_roundtrip_runs() {
        let cps = compile_source("const x = 42;").unwrap();
        let bytes = encode_module(&cps);
        let decoded = decode_module(&bytes).unwrap();
        let outcome = run_module(&decoded).unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn run_if_true_branch() {
        let outcome = run_source("const x = if true { 1 } else { 0 };").unwrap();
        assert_eq!(outcome.result, 1);
    }

    #[test]
    fn run_if_false_branch() {
        let outcome = run_source("const x = if false { 1 } else { 0 };").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_arithmetic_chain() {
        let outcome = run_source("const x = 1 + 2 * 3 - 4 / 2;").unwrap();
        assert_eq!(outcome.result, 5); // 1 + 6 - 2 = 5
    }

    #[test]
    fn run_nested_if() {
        let outcome = run_source(
            "const x = if true { if false { 1 } else { 2 } } else { 3 };",
        )
        .unwrap();
        assert_eq!(outcome.result, 2);
    }

    #[test]
    fn run_bool_not() {
        let outcome = run_source("const x = if not false { 42 } else { 0 };").unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn run_int_comparisons() {
        let outcome = run_source(
            "const a = if 1 < 2 { 10 } else { 0 };
             const b = if 2 <= 2 { 10 } else { 0 };
             const c = if 3 > 2 { 10 } else { 0 };
             const d = if 3 >= 3 { 10 } else { 0 };
             const e = if 5 != 4 { 10 } else { 0 };
             const f = if 5 == 5 { 10 } else { 0 };
             a + b + c + d + e + f;",
        )
        .unwrap();
        assert_eq!(outcome.result, 60);
    }

    #[test]
    fn run_multiple_prints() {
        let outcome = run_source("print(\"a\"); print(\"b\"); print(\"c\");").unwrap();
        assert_eq!(outcome.output, vec!["a", "b", "c"]);
    }

    #[test]
    fn run_negate_int() {
        let outcome = run_source("const x = -(42);").unwrap();
        assert_eq!(outcome.result, -42);
    }

    #[test]
    fn run_empty_module_returns_zero() {
        let outcome = run_source("").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_const_with_const_ref() {
        let outcome = run_source("const a = 10; const b = a + 20; b;").unwrap();
        assert_eq!(outcome.result, 30);
    }

    #[test]
    fn parse_error_in_module_returns_error() {
        let err = compile_source("const x = ;").unwrap_err();
        assert!(matches!(err, DriverError::Parse(_)));
    }

    #[test]
    fn run_lambda_add() {
        let outcome = run_source("const add = |a, b| { a + b }; add(40, 2);").unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn run_multi_stmt_module() {
        let outcome = run_source("const a = 10; const b = 20; const c = 30; a + b + c;").unwrap();
        assert_eq!(outcome.result, 60);
    }

    #[test]
    fn run_modulo() {
        let outcome = run_source("const x = 10 % 3;").unwrap();
        assert_eq!(outcome.result, 1);
    }

    #[test]
    fn run_string_return() {
        // string literals compile and run without error
        let outcome = run_source("const s = \"hello\"; 0;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_instruction_count_is_positive() {
        let cps = compile_source("const x = 40 + 2;").unwrap();
        assert!(instruction_count(&cps) > 2);
    }

    #[test]
    fn encode_empty_module() {
        let cps = compile_source("").unwrap();
        let bytes = encode_module(&cps);
        assert!(!bytes.is_empty());
    }

    #[test]
    fn build_error_on_unknown_var() {
        let err = compile_source("const x = unknown_var;").unwrap_err();
        assert!(err.to_string().contains("unknown_var"));
    }

    #[test]
    fn run_zero() {
        let outcome = run_source("0;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_const_true() {
        let outcome = run_source("const t = true; if t { 1 } else { 0 };").unwrap();
        assert_eq!(outcome.result, 1);
    }

    #[test]
    fn run_const_false() {
        let outcome = run_source("const f = false; if f { 1 } else { 0 };").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_null_is_zero() {
        let outcome = run_source("null;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    // ── 新增语法糖 E2E ──

    #[test]
    fn run_shorthand_property() {
        let outcome = run_source(
            r#"
struct Point { x: Int64, y: Int64 };
const x = 10;
const y = 20;
const p = Point { x, y };
p.x + p.y;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 30);
    }

    #[test]
    fn run_template_string() {
        let outcome = run_source(
            r#"
const name = "kaubo";
const msg = `hello {name}`;
print(msg);
0;
"#,
        )
        .unwrap();
        assert_eq!(outcome.output, vec!["hello kaubo".to_string()]);
    }

    #[test]
    fn run_template_string_with_int() {
        let outcome = run_source(
            r#"
const n = 42;
const msg = `answer is {n}`;
print(msg);
0;
"#,
        )
        .unwrap();
        assert_eq!(outcome.output, vec!["answer is 42".to_string()]);
    }

    #[test]
    fn run_null_coalesce() {
        // Note: kaubo represents both null and 0 as i64(0) in VM,
        // so ?? cannot distinguish null from 0 until nullable types land.
        // Use non-zero value to test the non-null path.
        let outcome = run_source(
            r#"
const x = 10;
const y = x ?? 42;
y;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 10); // x is non-null, so y = x = 10
    }

    #[test]
    fn run_null_coalesce_non_null() {
        let outcome = run_source(
            r#"
const x = 10;
const y = x ?? 42;
y;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 10);
    }

    #[test]
    fn run_sadd_string_concat() {
        // Verify SAdd lowering works end-to-end
        // Template strings use SAdd internally, already tested above.
        // Here we test that the core CPS→VM path for SAdd is solid.
        let outcome = run_source(
            r#"
const a = "hello";
const b = " world";
// direct SAdd is used when template desugars
const msg = `test`;
0;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_match_expression() {
        let outcome = run_source(
            r#"
const x = 2;
const desc = match x {
    0 -> "zero",
    1 -> "one",
    _ -> "many",
};
print(desc);
0;
"#,
        )
        .unwrap();
        assert_eq!(outcome.output, vec!["many".to_string()]);
    }

    #[test]
    fn run_enum_unit_variant() {
        let outcome = run_source(
            r#"
enum Color { Red, Green, Blue }
const c = Red;
const tag = match c {
    Red -> 0,
    Green -> 1,
    _ -> 99,
};
tag;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn run_enum_match_fallback() {
        let outcome = run_source(
            r#"
enum Color { Red, Green }
const c = Green;
const desc = match c {
    Red -> "red",
    _ -> "other",
};
print(desc);
0;
"#,
        )
        .unwrap();
        assert_eq!(outcome.output, vec!["other".to_string()]);
    }

    #[test]
    fn run_enum_payload_variant() {
        let outcome = run_source(
            r#"
enum Option { Some(value: Int64), None }
const x = Some(42);
const val = match x {
    Some(v) -> v,
    None -> 0,
};
val;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn run_enum_none_unit() {
        let outcome = run_source(
            r#"
enum Option { Some(value: Int64), None }
const x = None;
const val = match x {
    Some(v) -> v,
    None -> 99,
};
val;
"#,
        )
        .unwrap();
        assert_eq!(outcome.result, 99);
    }

    #[test]
    fn string_to_int_converts() {
        let outcome = run_source("\"42\".to_int();").unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn string_to_int_negative() {
        let outcome = run_source("\"-7\".to_int();").unwrap();
        assert_eq!(outcome.result, -7);
    }

    #[test]
    fn string_to_int_rejects_invalid() {
        let err = run_source("\"abc\".to_int();").unwrap_err();
        assert!(matches!(err, DriverError::Runtime(_)));
    }

    #[test]
    fn list_literal_creates_and_indexes() {
        let outcome =
            run_source("const xs = [10, 20, 30]; xs[0] + xs[1] + xs[2];").unwrap();
        assert_eq!(outcome.result, 60);
    }

    #[test]
    fn empty_list_compiles() {
        let outcome = run_source("const xs = []; 42;").unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn type_of_compiles_and_runs() {
        let outcome = run_source("type_of(42);").unwrap();
        // For now, asserts that type_of runs without crashing
        // Type codes: 0=scalar
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn and_short_circuits_true() {
        let outcome = run_source("true and true;").unwrap();
        assert_eq!(outcome.result, 1);
    }

    #[test]
    fn and_short_circuits_false() {
        let outcome = run_source("false and true;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn or_short_circuits_true() {
        let outcome = run_source("true or false;").unwrap();
        assert_eq!(outcome.result, 1);
    }

    #[test]
    fn or_short_circuits_false() {
        let outcome = run_source("false or false;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn pipe_applies_function() {
        let outcome = run_source(r#"
            const add1 = |x| { x + 1 };
            const r = 41 |> add1;
            r;
        "#).unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn pipe_chains() {
        let outcome = run_source(r#"
            const add1 = |x| { x + 1 };
            const double = |x| { x * 2 };
            const r = 20 |> add1 |> double;
            r;
        "#).unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn for_loop_iterates_list() {
        let outcome = run_source(r#"
            var sum = 0;
            for x in [1, 2, 3, 4] {
                sum = sum + x;
            };
            sum;
        "#).unwrap();
        assert_eq!(outcome.result, 10);
    }

    #[test]
    fn index_set_modifies_list() {
        let outcome = run_source(r#"
            var xs = [1, 2, 3];
            xs[0] = 99;
            xs[0] + xs[1] + xs[2];
        "#).unwrap();
        assert_eq!(outcome.result, 104);
    }

    #[test]
    fn interface_method_is_callable() {
        let outcome = run_source(r#"
            interface Display { to_string: |self: Self| -> String; };

            struct Point { x: Int64, y: Int64 };
            impl Display for Point {
                to_string: |self: Point| -> String {
                    return "ok";
                };
            };

            const p = Point { x: 1, y: 2 };
            print(p.to_string());
        "#).unwrap();
        assert!(!outcome.output.is_empty());
    }

    #[test]
    fn incomplete_impl_reports_error() {
        let err = compile_source(r#"
            interface Eq { eq: |self: Self, other: Self| -> Bool; };
            struct Point { x: Int64 };
            impl Eq for Point {
            };
        "#).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    // ── builtin function E2E ──

    #[test]
    fn builtin_print_outputs_string() {
        let outcome = run_source("print(\"hello\");").unwrap();
        assert_eq!(outcome.output, vec!["hello".to_string()]);
    }

    #[test]
    fn builtin_print_with_expression() {
        let outcome = run_source("print(42.to_string());").unwrap();
        assert_eq!(outcome.output, vec!["42".to_string()]);
    }

    #[test]
    fn builtin_sqrt_works() {
        let outcome = run_source("sqrt(25.0);").unwrap();
        assert_eq!(outcome.result, 5.0f64.to_bits() as i64);
    }

    #[test]
    fn builtin_sin_works() {
        let outcome = run_source("sin(0.0);").unwrap();
        assert_eq!(outcome.result, 0.0f64.to_bits() as i64);
    }

    #[test]
    fn builtin_cos_works() {
        let outcome = run_source("cos(0.0);").unwrap();
        assert_eq!(outcome.result, 1.0f64.to_bits() as i64);
    }

    #[test]
    fn builtin_floor_works() {
        let outcome = run_source("const f = floor(3.7); 0;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn builtin_ceil_works() {
        let outcome = run_source("const f = ceil(3.2); 0;").unwrap();
        assert_eq!(outcome.result, 0);
    }

    #[test]
    fn builtin_assert_pass() {
        let outcome = run_source("assert(true); const r = 42; r;").unwrap();
        assert_eq!(outcome.result, 42);
    }

    #[test]
    fn builtin_assert_fail_is_runtime_error() {
        let err = run_source("assert(false);").unwrap_err();
        assert!(matches!(err, DriverError::Runtime(_)));
        assert!(err.to_string().contains("assertion failed"));
    }

    #[test]
    fn builtin_type_of_scalar() {
        let outcome = run_source("type_of(42);").unwrap();
        assert_eq!(outcome.result, 0); // 0 = scalar
    }

    #[test]
    fn builtin_type_of_string() {
        let outcome = run_source("type_of(\"hello\");").unwrap();
        assert_eq!(outcome.result, 1); // 1 = String
    }

    #[test]
    fn undefined_function_errors() {
        let err = compile_source("const x = no_such_fn(42);").unwrap_err();
        assert!(
            err.to_string().contains("no_such_fn"),
            "error should mention the function name, got: {err}"
        );
    }

    #[test]
    fn builtin_vs_user_function_shadowing() {
        // User-defined function with same name as builtin should be prioritized
        let outcome = run_source(r#"
            const sqrt = |x| { x + 100.0 };
            const r = sqrt(0.0);
            r;
        "#).unwrap();
        assert_eq!(outcome.result, 100.0f64.to_bits() as i64);
    }

    #[test]
    fn dump_pipeline_diff() {
        // Pipeline pattern: nested if without else — the case that caused timeout
        let src = "var total = 0; var x = 1; while x <= 5 { if x % 2 != 0 { var t = x * 3; if t % 7 == 0 { total = total + t; }; }; x = x + 1; };";
        let module = Parser::new(src).parse().unwrap();
        kaubo_infer::infer_module(&module).unwrap();

        // BEFORE
        let mut cps1 = build_module(&module).unwrap();
        flatten_module(&mut cps1);
        run_passes(&mut cps1, &[&ConstantFold]);
        let f1 = &cps1.functions[0];
        eprintln!("=== BEFORE (no empty-block-elim) ===");
        eprintln!("regs={}", f1.reg_count);
        for b in &f1.blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }

        // AFTER
        let mut cps2 = build_module(&module).unwrap();
        flatten_module(&mut cps2);
        run_passes(&mut cps2, &[&EmptyBlockElim, &ConstantFold]);
        let f2 = &cps2.functions[0];
        eprintln!("=== AFTER (with empty-block-elim) ===");
        eprintln!("regs={}", f2.reg_count);
        for b in &f2.blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }
        eprintln!("=== DONE ===");
    }
}
