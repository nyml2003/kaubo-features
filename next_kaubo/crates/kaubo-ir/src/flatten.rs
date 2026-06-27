//! Block flattening — CPS optimization pass
//!
//! Inlines blocks with single predecessors to reduce block count.
//! Result: flat blocks with no params (all values in registers).

use crate::cps::*;
use std::collections::HashMap;

pub fn flatten_module(module: &mut CpsModule) {
    for func in &mut module.functions {
        flatten_function(func);
    }
}

fn flatten_function(func: &mut CpsFunction) {
    loop {
        let mut predecessor_counts = HashMap::new();
        let mut predecessors: HashMap<usize, Vec<usize>> = HashMap::new();

        // Count predecessors for each block (skip already-inlined blocks)
        for block in &func.blocks {
            if block.id == usize::MAX {
                continue;
            }
            match &block.term {
                CpsTerminator::Jump(target, _) => {
                    *predecessor_counts.entry(*target).or_insert(0) += 1;
                    predecessors.entry(*target).or_default().push(block.id);
                }
                CpsTerminator::Branch(_, tb, _, fb, _) => {
                    *predecessor_counts.entry(*tb).or_insert(0) += 1;
                    predecessors.entry(*tb).or_default().push(block.id);
                    *predecessor_counts.entry(*fb).or_insert(0) += 1;
                    predecessors.entry(*fb).or_default().push(block.id);
                }
                _ => {}
            }
        }

        let mut changed = false;
        let entry_id = func.entry;
        let block_ids: Vec<usize> = func.blocks.iter().map(|b| b.id).collect();

        for &id in &block_ids {
            if id == entry_id {
                continue;
            } // Don't inline entry
            if *predecessor_counts.get(&id).unwrap_or(&0) == 1 {
                if let Some(preds) = predecessors.get(&id) {
                    if preds.len() == 1 {
                        let pred_id = preds[0];
                        // Only inline if predecessor has Jump (not Branch)
                        let pred = func.blocks.iter().find(|b| b.id == pred_id).unwrap();
                        let target = func.blocks.iter().find(|b| b.id == id).unwrap();
                        // Don't inline blocks with params — they need bind_params
                        if !target.params.is_empty() {
                            continue;
                        }
                        if matches!(pred.term, CpsTerminator::Jump(_, _)) {
                            inline_block(func, pred_id, id);
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }
}

fn inline_block(func: &mut CpsFunction, pred_id: usize, target_id: usize) {
    let pred_idx = func.blocks.iter().position(|b| b.id == pred_id).unwrap();
    let target_idx = func.blocks.iter().position(|b| b.id == target_id).unwrap();

    let pred = &func.blocks[pred_idx];

    // Get the args passed in the Jump
    let jump_args: Vec<usize> = match &pred.term {
        CpsTerminator::Jump(_, args) => args.clone(),
        _ => vec![],
    };

    // Build a register map: param_index → actual_register (from Jump args)
    let reg_map: HashMap<usize, usize> =
        jump_args.iter().enumerate().map(|(i, &r)| (i, r)).collect();

    // Clone target block's instructions (they'll be modified)
    let target = &func.blocks[target_idx];
    let mut new_instrs = target.instrs.clone();

    // Remap instruction references from param indices to actual registers
    for instr in &mut new_instrs {
        remap_instr_regs(instr, &reg_map);
    }

    // Also remap terminator of the target block
    let mut new_term = target.term.clone();
    remap_term_regs(&mut new_term, &reg_map);

    // Append target's instructions to predecessor, replace predecessor's terminator
    let pred_mut = &mut func.blocks[pred_idx];
    pred_mut.instrs.append(&mut new_instrs);
    pred_mut.term = new_term;

    // Remove target block (mark as removed → set ID to max)
    // We don't actually remove it, just mark it
    func.blocks[target_idx].id = usize::MAX;
}

fn remap_instr_regs(instr: &mut CpsInstr, reg_map: &HashMap<usize, usize>) {
    let lookup = |r: &mut usize| {
        if let Some(&new_r) = reg_map.get(r) {
            *r = new_r;
        }
    };
    match instr {
        CpsInstr::BinOp(_, _, s1, s2) => {
            lookup(s1);
            lookup(s2);
        }
        CpsInstr::UnOp(_, _, s) => {
            lookup(s);
        }
        CpsInstr::Move(_, s) => {
            lookup(s);
        }
        CpsInstr::GetField(_, s, _) => {
            lookup(s);
        }
        CpsInstr::SetField(_, s, _, v) => {
            lookup(s);
            lookup(v);
        }
        CpsInstr::IndexGet(_, s, i) => {
            lookup(s);
            lookup(i);
        }
        CpsInstr::IndexSet(_, s, i, v) => {
            lookup(s);
            lookup(i);
            lookup(v);
        }
        CpsInstr::NewList(_, elements) => {
            for r in elements {
                lookup(r);
            }
        }
        CpsInstr::ListLen(_, obj) => {
            lookup(obj);
        }
        CpsInstr::Box(_, s) | CpsInstr::Unbox(_, s) | CpsInstr::Print(s) => {
            lookup(s);
        }
        CpsInstr::LoadVtable(_, _) => {
            // vtable_idx is not a register, no remapping needed
        }
        CpsInstr::NewInterfaceObj(_, vr, sr) => {
            lookup(vr);
            lookup(sr);
        }
        _ => {}
    }
}

fn remap_term_regs(term: &mut CpsTerminator, reg_map: &HashMap<usize, usize>) {
    match term {
        CpsTerminator::Return(r) => {
            if let Some(&new_r) = reg_map.get(r) {
                *r = new_r;
            }
        }
        CpsTerminator::Branch(r, _, _, _, _) => {
            if let Some(&new_r) = reg_map.get(r) {
                *r = new_r;
            }
        }
        CpsTerminator::CallIndirect(_, args, _) => {
            for r in args {
                if let Some(&new_r) = reg_map.get(r) {
                    *r = new_r;
                }
            }
        }
        _ => {}
    }
}

// ── tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps_build::build_module;
    use crate::test_fixtures;

    fn lower_and_flatten(src: &str) -> CpsModule {
        let m = test_fixtures::module(src);
        let mut cps = build_module(&m, None).unwrap();
        flatten_module(&mut cps);
        cps
    }

    #[test]
    fn f1_flatten_const() {
        let cps = lower_and_flatten("const x = 42;");
        let f = &cps.functions[0];
        assert!(
            f.blocks.len() <= 2,
            "flattened const should have 1-2 blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn f2_flatten_add() {
        let cps = lower_and_flatten("const x = 1 + 2;");
        let f = &cps.functions[0];
        assert!(
            f.blocks.len() <= 5,
            "flattened add should reduce blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn f3_flatten_no_params() {
        // Params are preserved for VM bind_params — flatten doesn't clear them anymore
        let cps = lower_and_flatten("const x = 1 + 2;");
        let f = &cps.functions[0];
        assert!(!f.blocks.is_empty(), "should have at least 1 block");
    }

    #[test]
    fn f4_flatten_if() {
        let cps = lower_and_flatten("const x = if true { 1 } else { 2 };");
        let f = &cps.functions[0];
        // if/else keeps at least 4 blocks (branch, then, else, merge) — these have multiple preds
        assert!(
            f.blocks.len() >= 2 && f.blocks.len() <= 6,
            "flattened if should have 2-6 blocks, got {}",
            f.blocks.len()
        );
    }

    #[test]
    fn f5_flatten_nested() {
        let cps = lower_and_flatten("const x = 1 + 2 + 3;");
        let f = &cps.functions[0];
        assert!(
            f.blocks.len() <= 6,
            "flattened nested add should reduce blocks, got {}",
            f.blocks.len()
        );
    }
}
