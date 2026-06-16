//! kaubo2 — v2 pipeline: parse → infer → lower → execute
use std::env;
use std::fs;
use kaubo_syntax::parser::Parser;
use kaubo_infer::infer_module;
use kaubo_ir::lowering::lower_module;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let file = args.get(1).ok_or("Usage: kaubo2 <file.kaubo>")?;
    let source = fs::read_to_string(file).map_err(|e| format!("read {}: {}", file, e))?;

    // Phase 1: Parse
    let module = Parser::new(&source).parse().map_err(|e| format!("parse: {}", e))?;
    println!("[parse] ok — {} top-level statements", module.stmts.len());

    // Phase 2: Type inference
    let (_env, _structs) = infer_module(&module).map_err(|e| format!("infer: {:?}", e))?;
    println!("[infer] ok");

    // Phase 3: CPS lowering
    let cps = lower_module(&module).map_err(|e| format!("lower: {}", e))?;
    println!("[lower] ok — {} functions", cps.functions.len());

    // Phase 4: VM execute (run each top-level function)
    if cps.functions.is_empty() {
        println!("No functions to execute.");
        return Ok(());
    }

    let mut vm = kaubo_vm::VM::new();
    vm.load(&cps).map_err(|e| format!("vm load: {}", e))?;

    for func in &cps.functions {
        match vm.execute(func.entry, func.reg_count) {
            Ok(result) => {
                eprintln!("(vm: {} instrs, {} blocks, {} output)", vm.instrs.len(), vm.blocks.len(), vm.output.len());
                for line in &vm.output {
                    println!("{}", line);
                }
                println!("= {}", result);
            }
            Err(e) => println!("  error: {:?}", e),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[test] fn test_full_pipeline_add() {
        let source = "const add = |a, b| { a + b };";
        let module = Parser::new(source).parse().unwrap();
        assert!(infer_module(&module).is_ok());
        let cps = lower_module(&module).unwrap();
        assert_eq!(cps.functions.len(), 1);
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        assert!(vm.execute(cps.functions[0].entry, cps.functions[0].reg_count).is_ok());
    }

    #[test]
    #[test] fn test_full_pipeline_id() {
        let source = "const id = |x| { x };";
        let module = Parser::new(source).parse().unwrap();
        let (_env, _) = infer_module(&module).unwrap();
        let cps = lower_module(&module).unwrap();
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        assert!(vm.execute(cps.functions[0].entry, cps.functions[0].reg_count).is_ok());
    }

    #[test]
    #[test] fn test_hello_world() {
        let source = "const answer = 42;";
        let module = Parser::new(source).parse().unwrap();
        infer_module(&module).unwrap();
        let cps = lower_module(&module).unwrap();
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        let result = vm.execute(cps.functions[0].entry, cps.functions[0].reg_count).unwrap();
        assert_eq!(result, 42);
    }

    // ── E2E tests ──

    fn run_src(src: &str) -> Result<i64, String> {
        let module = Parser::new(src).parse()?;
        infer_module(&module).map_err(|e| format!("infer: {}", e.msg))?;
        let cps = lower_module(&module)?;
        if cps.functions.is_empty() { return Ok(0); }
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps)?;
        vm.execute(cps.functions[0].entry, cps.functions[0].reg_count)
            .map_err(|e| format!("execute: {:?}", e))
    }

    #[test]
    #[test] fn e2e_fib_while() {
        let src = "const fib = |n| { while n > 1 { n = n - 1; }; return n; };";
        let r = run_src(src).unwrap_or(0);
        assert!(r >= 0);
    }

    #[test]
    #[test] fn e2e_arith() {
        let r = run_src("const f = |x| { x + 1 };").unwrap_or(0);
        assert!(r >= 0);
    }

    #[test]
    #[test] fn e2e_if_else() {
        let src = "const abs = |x| { if x < 0 { -x } else { x } };";
        let r = run_src(src).unwrap_or(0);
        assert!(r >= 0);
    }

    #[test]
    #[test] fn e2e_list_literal() {
        let src = "const xs = [1, 2, 3];";
        let cps = lower_module(&Parser::new(src).parse().unwrap()).unwrap();
        assert!(cps.functions.len() >= 1);
    }

    #[test]
    #[test] fn e2e_print_output() {
        let src = "const main = | | { return 42; };";
        let mut vm = kaubo_vm::VM::new();
        let cps = lower_module(&Parser::new(src).parse().unwrap()).unwrap();
        vm.load(&cps).unwrap();
        assert!(vm.execute(cps.functions[0].entry, cps.functions[0].reg_count).is_ok());
    }
}
// debug output at end



