//! Direct single-file compile and run driver.
//!
//! This crate centralizes the current linear path without using
//! `kaubo-pipeline` scheduling.

pub use kaubo_ir::cps::CpsModule;
use kaubo_ir::cps_build::build_module;
use kaubo_ir::flatten::flatten_module;
use kaubo_ir::pass::binary;
use kaubo_ir::pass::{fold::ConstantFold, run_passes};
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
    run_passes(&mut cps, &[&ConstantFold]);
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
        .map_err(|e| DriverError::Runtime(format!("{:?}", e)))?;

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
}
