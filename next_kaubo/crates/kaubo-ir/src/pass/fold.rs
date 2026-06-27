//! Constant folding — evaluate constant expressions at compile time.

use super::Pass;
use crate::cps::*;
use std::collections::HashMap;

pub struct ConstantFold;

impl Pass for ConstantFold {
    fn name(&self) -> &'static str {
        "constant-fold"
    }
    fn run(&self, module: &mut CpsModule) {
        for func in &mut module.functions {
            fold_function(func, &mut module.constants);
        }
    }
}

fn fold_function(func: &mut CpsFunction, constants: &mut Vec<Constant>) {
    // Count predecessors
    let mut pred_count: HashMap<usize, usize> = HashMap::new();
    let mut pred: HashMap<usize, usize> = HashMap::new();
    for block in &func.blocks {
        if block.id == usize::MAX {
            continue;
        }
        match &block.term {
            CpsTerminator::Jump(t, _) => {
                *pred_count.entry(*t).or_insert(0) += 1;
                pred.entry(*t).or_insert(block.id);
            }
            CpsTerminator::Branch(_, t, _, f, _) => {
                *pred_count.entry(*t).or_insert(0) += 1;
                pred.entry(*t).or_insert(block.id);
                *pred_count.entry(*f).or_insert(0) += 1;
                pred.entry(*f).or_insert(block.id);
            }
            _ => {}
        }
    }

    let mut block_consts: HashMap<usize, HashMap<usize, i64>> = HashMap::new();
    let mut visited: std::collections::HashSet<usize> = Default::default();
    let mut queue: Vec<usize> = vec![func.entry];
    visited.insert(func.entry);

    while let Some(bid) = queue.pop() {
        // Inherit constants from single predecessor
        let inherited = if pred_count.get(&bid).copied().unwrap_or(0) == 1 {
            let p = pred[&bid];
            block_consts.get(&p).cloned().unwrap_or_default()
        } else {
            HashMap::new()
        };

        if let Some(block) = func.blocks.iter_mut().find(|b| b.id == bid) {
            let result = fold_block_with(block, constants, inherited);
            block_consts.insert(bid, result);

            // Enqueue successors
            match &block.term {
                CpsTerminator::Jump(t, _) => {
                    if !visited.contains(t) {
                        visited.insert(*t);
                        queue.push(*t);
                    }
                }
                CpsTerminator::Branch(_, t, _, f, _) => {
                    if !visited.contains(t) {
                        visited.insert(*t);
                        queue.push(*t);
                    }
                    if !visited.contains(f) {
                        visited.insert(*f);
                        queue.push(*f);
                    }
                }
                _ => {}
            }
        }
    }
}

fn fold_block_with(
    block: &mut CpsBlock,
    constants: &mut Vec<Constant>,
    mut reg_val: HashMap<usize, i64>,
) -> HashMap<usize, i64> {
    for instr in &mut block.instrs {
        let replacement = match instr {
            CpsInstr::LoadConst(r, idx) => {
                if let Some(Constant::Int(n)) = constants.get(*idx) {
                    reg_val.insert(*r, *n);
                }
                None
            }
            CpsInstr::BinOp(r, op, a, b) => {
                let (dst, src1, src2) = (*r, *a, *b);
                let va = reg_val.get(&src1).copied();
                let vb = reg_val.get(&src2).copied();
                if let (Some(va), Some(vb)) = (va, vb) {
                    if let Some(result) = eval_binop(*op, va, vb) {
                        let new_idx = add_or_get_const(constants, Constant::Int(result));
                        reg_val.insert(dst, result);
                        Some(CpsInstr::LoadConst(dst, new_idx))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            CpsInstr::UnOp(r, op, a) => {
                let (dst, src) = (*r, *a);
                let va = reg_val.get(&src).copied();
                if let Some(va) = va {
                    if let Some(result) = eval_unop(*op, va) {
                        let new_idx = add_or_get_const(constants, Constant::Int(result));
                        reg_val.insert(dst, result);
                        Some(CpsInstr::LoadConst(dst, new_idx))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            CpsInstr::Move(r, a) => {
                let va = reg_val.get(a).copied();
                if let Some(va) = va {
                    reg_val.insert(*r, va);
                }
                None
            }
            _ => None,
        };
        if let Some(new_instr) = replacement {
            *instr = new_instr;
        }
    }
    reg_val
}

fn eval_binop(op: CpsBinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        CpsBinOp::AddInt => Some(a.wrapping_add(b)),
        CpsBinOp::SubInt => Some(a.wrapping_sub(b)),
        CpsBinOp::MulInt => Some(a.wrapping_mul(b)),
        CpsBinOp::DivInt => {
            if b != 0 {
                Some(a / b)
            } else {
                None
            }
        }
        CpsBinOp::ModInt => {
            if b != 0 {
                Some(a % b)
            } else {
                None
            }
        }
        CpsBinOp::EqInt => Some((a == b) as i64),
        CpsBinOp::NeInt => Some((a != b) as i64),
        CpsBinOp::LtInt => Some((a < b) as i64),
        CpsBinOp::LeInt => Some((a <= b) as i64),
        CpsBinOp::GtInt => Some((a > b) as i64),
        CpsBinOp::GeInt => Some((a >= b) as i64),
        _ => None,
    }
}

fn eval_unop(op: CpsUnOp, a: i64) -> Option<i64> {
    match op {
        CpsUnOp::NegInt => Some(-a),
        CpsUnOp::Not => Some((a == 0) as i64),
        _ => None,
    }
}

fn add_or_get_const(constants: &mut Vec<Constant>, c: Constant) -> usize {
    for (i, existing) in constants.iter().enumerate() {
        if const_eq(existing, &c) {
            return i;
        }
    }
    let i = constants.len();
    constants.push(c);
    i
}

fn const_eq(a: &Constant, b: &Constant) -> bool {
    match (a, b) {
        (Constant::Int(a), Constant::Int(b)) => a == b,
        (Constant::Float(a), Constant::Float(b)) => a.to_bits() == b.to_bits(),
        (Constant::String(a), Constant::String(b)) => a == b,
        (Constant::Bool(a), Constant::Bool(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps_build::build_module;
    use crate::flatten::flatten_module;
    use crate::test_fixtures;

    fn fold_src(src: &str) -> CpsModule {
        let m = test_fixtures::module(src);
        let mut cps = build_module(&m, None).unwrap();
        flatten_module(&mut cps);
        ConstantFold.run(&mut cps);
        cps
    }

    #[test]
    fn fold_add_constants() {
        let cps = fold_src("const x = 2 + 3;");
        let main = cps.functions.last().unwrap();
        let has_add = main.blocks.iter().filter(|b| b.id != usize::MAX).any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::AddInt, _, _)))
        });
        assert!(!has_add, "2 + 3 should fold to LoadConst(5)");
    }

    #[test]
    fn fold_mul_constants() {
        let cps = fold_src("const x = 6 * 7;");
        let main = cps.functions.last().unwrap();
        let has_mul = main.blocks.iter().filter(|b| b.id != usize::MAX).any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::MulInt, _, _)))
        });
        assert!(!has_mul, "6 * 7 should fold to LoadConst(42)");
    }

    #[test]
    fn fold_comparison() {
        let cps = fold_src("const x = 5 < 10;");
        let main = cps.functions.last().unwrap();
        let has_lt = main.blocks.iter().filter(|b| b.id != usize::MAX).any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::LtInt, _, _)))
        });
        assert!(!has_lt, "5 < 10 should fold to LoadConst(1)");
    }

    #[test]
    fn fold_var_with_known_value() {
        let cps = fold_src("var x = 2; var y = x + 3;");
        let main = cps.functions.last().unwrap();
        let has_add = main.blocks.iter().filter(|b| b.id != usize::MAX).any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::AddInt, _, _)))
        });
        assert!(!has_add, "x+3 with x=2 should fold (constant propagation)");
    }

    #[test]
    fn fold_through_assign() {
        // x=2, then x=x+1 makes x known as 3, so x+3 should fold to 6
        let cps = fold_src("var x = 2; x = x + 1; var y = x + 3;");
        let main = cps.functions.last().unwrap();
        let has_add = main.blocks.iter().filter(|b| b.id != usize::MAX).any(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::AddInt, _, _)))
        });
        assert!(
            !has_add,
            "x+3 after x=x+1 should fold (x becomes known constant 3)"
        );
    }

    #[test]
    fn fold_unary_neg() {
        let cps = fold_src("const x = -(42);");
        let main = cps.functions.last().unwrap();
        let has_neg = main
            .blocks
            .iter()
            .filter(|b| b.id != usize::MAX)
            .any(|b| b.instrs.iter().any(|i| matches!(i, CpsInstr::UnOp(..))));
        assert!(!has_neg, "-(42) should fold to LoadConst(-42)");
    }
}
