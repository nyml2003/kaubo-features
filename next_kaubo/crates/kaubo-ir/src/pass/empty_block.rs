//! Empty block elimination — remove blocks that are just `Jump(target, [])` passthroughs.

use super::Pass;
use crate::cps::*;
use std::collections::{HashMap, HashSet};

pub struct EmptyBlockElim;

impl Pass for EmptyBlockElim {
    fn name(&self) -> &'static str {
        "empty-block-elim"
    }
    fn run(&self, module: &mut CpsModule) {
        for func in &mut module.functions {
            eliminate(func);
        }
    }
}

fn eliminate(func: &mut CpsFunction) {
    // Build predecessor map and identify empty forwarder blocks
    let mut preds: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut empty_forwarders: HashSet<usize> = HashSet::new();

    for block in &func.blocks {
        if block.id == usize::MAX {
            continue;
        }
        // Empty forwarder: no instructions, no params, Jump with no args
        if block.params.is_empty()
            && block.instrs.is_empty()
            && matches!(&block.term, CpsTerminator::Jump(_, args) if args.is_empty())
        {
            empty_forwarders.insert(block.id);
        }
        // Record predecessors
        match &block.term {
            CpsTerminator::Jump(t, _) => {
                preds.entry(*t).or_default().push(block.id);
            }
            CpsTerminator::Branch(_, t, _, f, _) => {
                preds.entry(*t).or_default().push(block.id);
                preds.entry(*f).or_default().push(block.id);
            }
            _ => {}
        }
    }

    let entry = func.entry;

    // Eliminate empty forwarders
    for &ef_id in &empty_forwarders {
        // Don't eliminate entry block
        if ef_id == entry {
            continue;
        }
        // Find the target of this forwarder
        let target = match func.blocks.iter().find(|b| b.id == ef_id) {
            Some(b) => {
                if let CpsTerminator::Jump(t, _) = &b.term {
                    *t
                } else {
                    continue;
                }
            }
            None => continue,
        };
        // Don't create self-loop
        if target == ef_id {
            continue;
        }
        // Rewrite all predecessors to jump directly to target
        if let Some(pred_list) = preds.get(&ef_id).cloned() {
            for pred_id in &pred_list {
                if let Some(pred) = func.blocks.iter_mut().find(|b| b.id == *pred_id) {
                    match &mut pred.term {
                        CpsTerminator::Jump(t, _) if *t == ef_id => {
                            *t = target;
                        }
                        CpsTerminator::Branch(_, t, _, f, _) => {
                            if *t == ef_id {
                                *t = target;
                            }
                            if *f == ef_id {
                                *f = target;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // Mark the empty forwarder as dead
        if let Some(block) = func.blocks.iter_mut().find(|b| b.id == ef_id) {
            block.id = usize::MAX;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps_build::build_module;
    use crate::flatten::flatten_module;
    use crate::test_fixtures;

    fn optimize(src: &str) -> CpsModule {
        let m = test_fixtures::module(src);
        let mut cps = build_module(&m, None).unwrap();
        flatten_module(&mut cps);
        EmptyBlockElim.run(&mut cps);
        cps
    }

    #[test]
    fn elim_preserves_const_fixture() {
        let cps = optimize("const x = 42;");
        let f = &cps.functions[0];
        let live: Vec<_> = f.blocks.iter().filter(|b| b.id != usize::MAX).collect();
        assert!(!live.is_empty(), "const fixture should survive elimination");
        assert!(live.iter().any(|b| b.id == f.entry), "entry block must survive");
    }

    #[test]
    fn compare_bytecode_before_after() {
        let m = test_fixtures::module("var i = 0; while i < 3 { i = i + 1; };");

        let mut cps_before = build_module(&m, None).unwrap();
        flatten_module(&mut cps_before);
        let fb = &cps_before.functions[0];
        eprintln!("=== BEFORE (while loop) ===");
        eprintln!("regs={}", fb.reg_count);
        for b in &fb.blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }

        let mut cps_after = build_module(&m, None).unwrap();
        flatten_module(&mut cps_after);
        EmptyBlockElim.run(&mut cps_after);
        let fa = &cps_after.functions[0];
        eprintln!("=== AFTER (while loop) ===");
        eprintln!("regs={}", fa.reg_count);
        for b in &fa.blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }
        eprintln!("=== DONE ===");
    }

    #[test]
    fn compare_if_else_bytecode() {
        let m = test_fixtures::module("const x = if true { 1 } else { 2 };");

        let mut cps_before = build_module(&m, None).unwrap();
        flatten_module(&mut cps_before);
        let fb = &cps_before.functions[0];
        eprintln!("=== BEFORE (if/else) ===");
        eprintln!("regs={}", fb.reg_count);
        for b in &fb.blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }

        let mut cps_after = build_module(&m, None).unwrap();
        flatten_module(&mut cps_after);
        EmptyBlockElim.run(&mut cps_after);
        let fa = &cps_after.functions[0];
        eprintln!("=== AFTER (if/else) ===");
        eprintln!("regs={}", fa.reg_count);
        for b in &fa.blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }
        eprintln!("=== DONE ===");
    }

    // Pipeline-specific: nested if without else — the pattern that caused timeouts
    #[test]
    fn compare_nested_if_no_else() {
        // Build CPS for a minimal pipeline pattern manually
        // if cond { body } without else creates empty skip blocks
        let mut module = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![
                    // blk0: compute cond → blk1
                    CpsBlock { id: 0, params: vec![], instrs: vec![CpsInstr::LoadConst(0, 0)], term: CpsTerminator::Jump(1, vec![]) },
                    // blk1: Branch(cond, then=2, skip=3)
                    CpsBlock { id: 1, params: vec![], instrs: vec![], term: CpsTerminator::Branch(0, 2, vec![], 3, vec![]) },
                    // blk2: then-body → Jump(3)  (to skip/merge block)
                    CpsBlock { id: 2, params: vec![], instrs: vec![CpsInstr::LoadConst(1, 1)], term: CpsTerminator::Jump(3, vec![]) },
                    // blk3: empty skip block → Jump(4)  ← THIS is the pattern
                    CpsBlock { id: 3, params: vec![], instrs: vec![], term: CpsTerminator::Jump(4, vec![]) },
                    // blk4: merge → Return
                    CpsBlock { id: 4, params: vec![], instrs: vec![], term: CpsTerminator::Return(1) },
                ],
                entry: 0, reg_count: 2,
            }],
            constants: vec![Constant::Int(1), Constant::Int(42)],
            structs: vec![], enums: vec![],
            vtables: vec![],
            symbol_map: std::collections::HashMap::new(),
            func_owners: vec![],
        };
        eprintln!("=== BEFORE (nested if no else pattern) ===");
        for b in &module.functions[0].blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }

        // Run pass and check afterwards
        EmptyBlockElim.run(&mut module);
        eprintln!("=== AFTER ===");
        for b in &module.functions[0].blocks { if b.id != usize::MAX { eprintln!("  blk{} p{:?} {:?} | {:?}", b.id, b.params, b.instrs, b.term); } }
        eprintln!("=== DONE ===");
    }
}
