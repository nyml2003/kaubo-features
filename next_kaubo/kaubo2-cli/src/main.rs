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
        let name = &func.name;
        println!("\n── {} ──", name);
        vm.regs.ints.resize(func.reg_count, 0);
        match vm.execute(func.entry) {
            Ok(result) => println!("= {}", result),
            Err(e) => println!("  error: {:?}", e),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline_add() {
        let source = "const add = |a, b| { a + b };";
        let module = Parser::new(source).parse().unwrap();
        assert!(infer_module(&module).is_ok());
        let cps = lower_module(&module).unwrap();
        assert_eq!(cps.functions.len(), 1);

        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        vm.regs.ints.resize(cps.functions[0].reg_count, 0);
        let result = vm.execute(cps.functions[0].entry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_full_pipeline_id() {
        let source = "const id = |x| { x };";
        let module = Parser::new(source).parse().unwrap();
        let (_env, _) = infer_module(&module).unwrap();
        let cps = lower_module(&module).unwrap();
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        vm.regs.ints.resize(cps.functions[0].reg_count, 0);
        assert!(vm.execute(cps.functions[0].entry).is_ok());
    }

    #[test]
    fn test_hello_world() {
        // Simple expression test — full CPS lowering for complex expressions is P1
        let source = "const answer = 42;";
        let module = Parser::new(source).parse().unwrap();
        infer_module(&module).unwrap();
        let cps = lower_module(&module).unwrap();
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        vm.regs.ints.resize(cps.functions[0].reg_count, 0);
        let result = vm.execute(cps.functions[0].entry).unwrap();
        assert_eq!(result, 42);
    }
}
