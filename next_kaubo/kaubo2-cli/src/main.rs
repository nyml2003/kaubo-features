//! kaubo2 — v2 pipeline: parse → lower → flatten → execute
use std::env;
use std::fs;
use kaubo_syntax::parser::Parser;
use kaubo_infer::infer_module;
use kaubo_ir::lowering::lower_module;
use kaubo_ir::flatten::flatten_module;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let file = args.get(1).ok_or("Usage: kaubo2 <file.kaubo>")?;
    let source = fs::read_to_string(file).map_err(|e| format!("read {}: {}", file, e))?;
    let module = Parser::new(&source).parse().map_err(|e| format!("parse: {}", e))?;
    infer_module(&module).map_err(|e| format!("infer: {:?}", e))?;
    let mut cps = lower_module(&module).map_err(|e| format!("lower: {}", e))?;
    flatten_module(&mut cps);
    if cps.functions.is_empty() { return Ok(()); }
    let mut vm = kaubo_vm::VM::new();
    vm.load(&cps).map_err(|e| format!("load: {}", e))?;
    let func = cps.functions.last().unwrap();
    match vm.execute(cps.functions.len() - 1, func.reg_count) {
        Ok(r) => { for l in &vm.output { println!("{}", l); } println!("= {}", r); }
        Err(e) => eprintln!("error: {:?}", e),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_src(src: &str) -> Result<i64, String> {
        let m = Parser::new(src).parse()?;
        infer_module(&m).map_err(|e| format!("infer: {:?}", e))?;
        let mut cps = lower_module(&m)?; flatten_module(&mut cps);
        if cps.functions.is_empty() { return Ok(0); }
        let mut vm = kaubo_vm::VM::new(); vm.load(&cps)?;
        let e = cps.functions.len() - 1;
        vm.execute(e, cps.functions[e].reg_count).map_err(|e| format!("{:?}", e))
    }

    #[test] fn e2e_lit() { assert_eq!(run_src("const x = 42;").unwrap(), 42); }
    #[test] fn e2e_add() { assert_eq!(run_src("const x = 40 + 2;").unwrap_or(-1), 42); }
    #[test] fn e2e_sub() { assert_eq!(run_src("const x = 50 - 8;").unwrap_or(-1), 42); }
    #[test] fn e2e_mul() { assert_eq!(run_src("const x = 6 * 7;").unwrap_or(-1), 42); }
    #[test] fn e2e_if_else() { assert_eq!(run_src("const x = if true { 42 } else { 0 };").unwrap_or(-1), 42); }
    #[test] fn e2e_if_false() { assert_eq!(run_src("const x = if false { 0 } else { 42 };").unwrap_or(-1), 42); }
}
