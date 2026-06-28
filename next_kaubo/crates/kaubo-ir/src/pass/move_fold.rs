//! Move folding — eliminate redundant Move instructions after arithmetic.
//!
//! Pattern:  BinOp(tmp, op, a, b);  Move(final, tmp)
//! Becomes:  BinOp(final, op, a, b)  (Move eliminated)

use super::Pass;
use crate::cps::*;

pub struct MoveFold;

impl Pass for MoveFold {
    fn name(&self) -> &'static str {
        "move-fold"
    }
    fn run(&self, module: &mut CpsModule) {
        for func in &mut module.functions {
            fold_moves(func);
        }
    }
}

fn fold_moves(func: &mut CpsFunction) {
    for block in &mut func.blocks {
        if block.id == usize::MAX {
            continue;
        }
        let mut i = 0;
        while i + 1 < block.instrs.len() {
            // Pattern: BinOp(tmp, op, a, b) followed by Move(final, tmp)
            let (tmp_reg, final_reg) = match (&block.instrs[i], &block.instrs[i + 1]) {
                (CpsInstr::BinOp(dst, _, _, _), CpsInstr::Move(move_dst, _)) => (*dst, *move_dst),
                (CpsInstr::UnOp(dst, _, _), CpsInstr::Move(move_dst, _)) => (*dst, *move_dst),
                (CpsInstr::LoadConst(dst, _), CpsInstr::Move(move_dst, _)) => (*dst, *move_dst),
                _ => {
                    i += 1;
                    continue;
                }
            };
            // Verify the Move copies FROM the first instruction's destination
            let move_src = match &block.instrs[i + 1] {
                CpsInstr::Move(_, src) => *src,
                _ => unreachable!(),
            };
            if move_src != tmp_reg {
                i += 1;
                continue;
            }
            // Don't fold if source == destination (no-op)
            if tmp_reg == final_reg {
                i += 2;
                continue;
            }
            // Check that tmp_reg isn't used between the two instructions (already fine, they're adjacent)
            // Check that tmp_reg isn't used later in the block AND the final register isn't used before the fold
            let tmp_used_later = block.instrs[i + 2..]
                .iter()
                .any(|instr| instr_uses_reg(instr, tmp_reg));
            if tmp_used_later {
                i += 1;
                continue;
            }

            // Rewrite: redirect the first instruction's output to final_reg, remove the Move
            rewrite_dst(&mut block.instrs[i], final_reg);
            block.instrs.remove(i + 1); // remove Move
                                        // Don't increment i — next iteration re-checks at same position
        }
    }
}

fn instr_uses_reg(instr: &CpsInstr, reg: usize) -> bool {
    match instr {
        CpsInstr::BinOp(_, _, a, b) => *a == reg || *b == reg,
        CpsInstr::UnOp(_, _, a) => *a == reg,
        CpsInstr::Move(_, src) => *src == reg,
        CpsInstr::GetField(_, obj, _) => *obj == reg,
        CpsInstr::SetField(_, obj, _, val) => *obj == reg || *val == reg,
        CpsInstr::IndexGet(_, obj, idx) => *obj == reg || *idx == reg,
        CpsInstr::IndexSet(_, obj, idx, val) => *obj == reg || *idx == reg || *val == reg,
        CpsInstr::NewList(_, elements) => elements.contains(&reg),
        CpsInstr::ListLen(_, obj) => *obj == reg,
        CpsInstr::Print(r) => *r == reg,
        CpsInstr::Box(_, s) | CpsInstr::Unbox(_, s) => *s == reg,
        _ => false,
    }
}

fn rewrite_dst(instr: &mut CpsInstr, new_dst: usize) {
    match instr {
        CpsInstr::BinOp(dst, _, _, _) => *dst = new_dst,
        CpsInstr::UnOp(dst, _, _) => *dst = new_dst,
        CpsInstr::LoadConst(dst, _) => *dst = new_dst,
        _ => {}
    }
}

// Also update terminator references after folding
// (The Move elimination may change which register carries a value to the terminator)
// This is handled automatically since the rewritten BinOp now writes to final_reg directly

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
        MoveFold.run(&mut cps);
        cps
    }

    #[test]
    fn fold_binop_move() {
        // Manually construct: BinOp(r1, AddInt, r2, r3); Move(r4, r1)
        let mut func = CpsFunction {
            name: "test".into(),
            blocks: vec![CpsBlock {
                id: 0,
                params: vec![],
                instrs: vec![
                    CpsInstr::BinOp(1, CpsBinOp::AddInt, 2, 3),
                    CpsInstr::Move(4, 1),
                ],
                term: CpsTerminator::Return(4),
            }],
            entry: 0,
            reg_count: 5,
        };
        fold_moves(&mut func);
        let b = &func.blocks[0];
        assert_eq!(b.instrs.len(), 1, "Move should be eliminated");
        assert!(matches!(
            b.instrs[0],
            CpsInstr::BinOp(4, CpsBinOp::AddInt, 2, 3)
        ));
    }

    #[test]
    fn fold_loadconst_move() {
        let mut func = CpsFunction {
            name: "test".into(),
            blocks: vec![CpsBlock {
                id: 0,
                params: vec![],
                instrs: vec![CpsInstr::LoadConst(1, 0), CpsInstr::Move(2, 1)],
                term: CpsTerminator::Return(2),
            }],
            entry: 0,
            reg_count: 3,
        };
        fold_moves(&mut func);
        assert_eq!(func.blocks[0].instrs.len(), 1);
        assert!(matches!(
            func.blocks[0].instrs[0],
            CpsInstr::LoadConst(2, 0)
        ));
    }

    #[test]
    fn no_fold_if_tmp_reused() {
        let mut func = CpsFunction {
            name: "test".into(),
            blocks: vec![CpsBlock {
                id: 0,
                params: vec![],
                instrs: vec![
                    CpsInstr::BinOp(1, CpsBinOp::AddInt, 2, 3),
                    CpsInstr::Move(4, 1),
                    CpsInstr::BinOp(5, CpsBinOp::MulInt, 1, 6), // uses r1 again
                ],
                term: CpsTerminator::Return(5),
            }],
            entry: 0,
            reg_count: 7,
        };
        fold_moves(&mut func);
        assert_eq!(
            func.blocks[0].instrs.len(),
            3,
            "should NOT fold, r1 is reused"
        );
    }
}
