//! kaubo2 — v2 pipeline: parse → infer → build → flatten → execute
use std::env;
use std::fs;
use kaubo_syntax::parser::Parser;
use kaubo_infer::infer_module;
use kaubo_ir::cps_build::build_module;
use kaubo_ir::flatten::flatten_module;
use kaubo_ir::cps::CpsModule;
use kaubo_ir::pass::{run_passes, fold::ConstantFold};
use kaubo_ir::pass::binary;

fn compile_pipeline(source: &str) -> Result<CpsModule, String> {
    let module = Parser::new(source).parse().map_err(|e| format!("parse: {}", e))?;
    infer_module(&module).map_err(|e| format!("infer: {:?}", e))?;
    let mut cps = build_module(&module).map_err(|e| format!("build: {}", e))?;
    flatten_module(&mut cps);
    run_passes(&mut cps, &[&ConstantFold]);
    Ok(cps)
}

fn run_compiled(cps: &CpsModule) -> Result<(), String> {
    if cps.functions.is_empty() { return Ok(()); }
    let mut vm = kaubo_vm::VM::new();
    vm.load(cps).map_err(|e| format!("load: {}", e))?;
    let func = cps.functions.last().unwrap();
    match vm.execute(cps.functions.len() - 1, func.reg_count) {
        Ok(r) => { for l in &vm.output { println!("{}", l); } println!("= {}", r); }
        Err(e) => eprintln!("error: {:?}", e),
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("run");
    let file = match sub {
        "compile" => args.get(2),
        "run" => args.get(2),
        _ => args.get(1),
    }.ok_or("Usage: kaubo2 [compile|run] <file>")?;

    match sub {
        "compile" => {
            let source = fs::read_to_string(file).map_err(|e| format!("read {}: {}", file, e))?;
            let cps = compile_pipeline(&source)?;
            let out = file.replace(".kaubo", ".kauboc");
            let bytes = binary::encode_module(&cps);
            let len = bytes.len();
            fs::write(&out, bytes).map_err(|e| format!("write {}: {}", out, e))?;
            println!("Compiled: ({:.1}KB)", len as f64 / 1024.0);
        }
        "run" => {
            if file.ends_with(".kauboc") {
                let bytes = fs::read(file).map_err(|e| format!("read {}: {}", file, e))?;
                let cps = binary::decode_module(&bytes)?;
                run_compiled(&cps)?;
            } else {
                let source = fs::read_to_string(file).map_err(|e| format!("read {}: {}", file, e))?;
                let cps = compile_pipeline(&source)?;
                run_compiled(&cps)?;
            }
        }
        _ => {
            let source = fs::read_to_string(file).map_err(|e| format!("read {}: {}", file, e))?;
            let cps = compile_pipeline(&source)?;
            run_compiled(&cps)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_src(src: &str) -> Result<i64, String> {
        let m = Parser::new(src).parse()?;
        infer_module(&m).map_err(|e| format!("infer: {:?}", e))?;
        let mut cps = build_module(&m)?; flatten_module(&mut cps);
        if cps.functions.is_empty() { return Ok(0); }
        let mut vm = kaubo_vm::VM::new(); vm.load(&cps)?;
        let e = cps.functions.len() - 1;
        vm.execute(e, cps.functions[e].reg_count).map_err(|e| format!("{:?}", e))
    }

    fn run_src_with_output(src: &str) -> Result<(i64, Vec<String>), String> {
        let m = Parser::new(src).parse()?;
        infer_module(&m).map_err(|e| format!("infer: {:?}", e))?;
        let mut cps = build_module(&m)?; flatten_module(&mut cps);
        if cps.functions.is_empty() { return Ok((0, vec![])); }
        let mut vm = kaubo_vm::VM::new(); vm.load(&cps)?;
        let e = cps.functions.len() - 1;
        let r = vm.execute(e, cps.functions[e].reg_count).map_err(|e| format!("{:?}", e))?;
        Ok((r, vm.output.clone()))
    }

    #[test] fn e2e_lit() { assert_eq!(run_src("const x = 42;").unwrap(), 42); }
    #[test] fn e2e_add() { assert_eq!(run_src("const x = 40 + 2;").unwrap_or(-1), 42); }
    #[test] fn e2e_sub() { assert_eq!(run_src("const x = 50 - 8;").unwrap_or(-1), 42); }
    #[test] fn e2e_mul() { assert_eq!(run_src("const x = 6 * 7;").unwrap_or(-1), 42); }
    #[test] fn e2e_if_else() { assert_eq!(run_src("const x = if true { 42 } else { 0 };").unwrap_or(-1), 42); }
    #[test] fn e2e_if_false() { assert_eq!(run_src("const x = if false { 0 } else { 42 };").unwrap_or(-1), 42); }

    // ── Phase 1: 新 e2e 测试（先红）──

    #[test]
    fn e2e_var_multi_stmt() {
        assert_eq!(run_src("var x = 10; var y = 32; x + y;").unwrap_or(-1), 42);
    }

    #[test]
    fn e2e_const_multi_stmt() {
        assert_eq!(run_src("const x = 10; const y = x + 22; y;").unwrap_or(-1), 32);
    }

    #[test]
    fn e2e_while_skip() {
        assert_eq!(run_src("while false { const x = 0; }; const r = 5; r;").unwrap_or(-1), 5);
    }

    #[test]
    fn e2e_while_count() {
        assert_eq!(run_src("var n = 0; while n < 3 { n = n + 1; }; n;").unwrap_or(-1), 3);
    }

    #[test]
    fn e2e_lambda_call() {
        assert_eq!(run_src("const f = |x| { x + 1 }; f(41);").unwrap_or(-1), 42);
    }

    #[test]
    fn e2e_print() {
        let result = run_src_with_output("print(\"hi\");").unwrap();
        assert!(result.1.iter().any(|s| s.contains("hi")), "output should contain 'hi', got {:?}", result.1);
    }

    #[test]
    fn debug_impl_dis() {
        use kaubo_ir::pass::{run_passes, fold::ConstantFold};
        let src = r#"
struct Point { x: Int64, y: Int64 };
impl Point {
  dis: |self: Point, other: Point| -> Float64 {
    const dx = (self.x - other.x);
    const dy = (self.y - other.y);
    return sqrt((dx*dx + dy*dy).to_float());
  }
};
const p1 = Point { x: 200, y: 300 };
const p2 = Point { x: 300, y: 400 };
print(p1.dis(p2).to_string());
"#;
        let m = Parser::new(src).parse().unwrap();
        infer_module(&m).unwrap();
        let mut cps = build_module(&m).unwrap();
        flatten_module(&mut cps);
        run_passes(&mut cps, &[&ConstantFold]);
        eprintln!("=== CPS DUMP ===");
        for (fi, func) in cps.functions.iter().enumerate() {
            eprintln!("fn {} '{}' entry={} regs={}",
                fi, func.name, func.entry, func.reg_count);
            for b in func.blocks.iter().filter(|b| b.id != usize::MAX) {
                eprintln!("  blk{} {}: {:?} | {:?}",
                    b.id, if b.id == func.entry {"(entry)"} else {""}, b.instrs, b.term);
            }
        }
        if cps.functions.is_empty() { panic!("no funcs"); }
        let mut vm = kaubo_vm::VM::new();
        vm.debug = true;
        vm.load(&cps).unwrap();
        let e = cps.functions.len() - 1;
        let r = vm.execute(e, cps.functions[e].reg_count);
        eprintln!("[DEBUG] result={:?} output={:?}", r, vm.output);
    }
}
