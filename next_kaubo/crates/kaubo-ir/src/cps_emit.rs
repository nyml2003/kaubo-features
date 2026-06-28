//! cps_emit — pure CPS instruction emission
//!
//! Each function maps an AST concept to (instructions, terminator).
//! No global state, no block management, independently testable.

use crate::cps::*;

pub type EmitResult = (Vec<CpsInstr>, CpsTerminator);

pub fn emit_literal(reg: usize, const_idx: usize) -> EmitResult {
    (
        vec![CpsInstr::LoadConst(reg, const_idx)],
        CpsTerminator::Return(reg),
    )
}

pub fn emit_binary(dst: usize, op: CpsBinOp, left: usize, right: usize) -> EmitResult {
    (
        vec![CpsInstr::BinOp(dst, op, left, right)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_unary(dst: usize, op: CpsUnOp, src: usize) -> EmitResult {
    (
        vec![CpsInstr::UnOp(dst, op, src)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_varref(reg: usize) -> EmitResult {
    (vec![], CpsTerminator::Return(reg))
}

pub fn emit_move(dst: usize, src: usize) -> EmitResult {
    (vec![CpsInstr::Move(dst, src)], CpsTerminator::Return(dst))
}

pub fn emit_print(reg: usize) -> EmitResult {
    (vec![CpsInstr::Print(reg)], CpsTerminator::Return(reg))
}

pub fn emit_get_field(dst: usize, obj: usize, field_idx: u16) -> EmitResult {
    (
        vec![CpsInstr::GetField(dst, obj, field_idx)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_set_field(val: usize, obj: usize, field_idx: u16) -> EmitResult {
    (
        vec![CpsInstr::SetField(val, obj, field_idx, 0)],
        CpsTerminator::Return(val),
    )
}

pub fn emit_new_struct(dst: usize, struct_id: usize, field_regs: Vec<usize>) -> EmitResult {
    (
        vec![CpsInstr::NewStruct(dst, struct_id, field_regs)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_new_variant(
    dst: usize,
    enum_id: usize,
    tag: u16,
    field_regs: Vec<usize>,
) -> EmitResult {
    (
        vec![CpsInstr::NewVariant(dst, enum_id, tag, field_regs)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_get_variant_tag(dst: usize, obj: usize) -> EmitResult {
    (
        vec![CpsInstr::GetVariantTag(dst, obj)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_set_variant_field(val: usize, obj: usize, field_idx: u16) -> EmitResult {
    (
        vec![CpsInstr::SetVariantField(val, obj, field_idx, 0)],
        CpsTerminator::Return(val),
    )
}

pub fn emit_get_variant_field(dst: usize, obj: usize, field_idx: u16) -> EmitResult {
    (
        vec![CpsInstr::GetVariantField(dst, obj, field_idx)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_list_len(dst: usize, obj: usize) -> EmitResult {
    (
        vec![CpsInstr::ListLen(dst, obj)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_new_list(dst: usize, elements: Vec<usize>) -> EmitResult {
    (
        vec![CpsInstr::NewList(dst, elements)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_new_tuple(dst: usize, elements: Vec<usize>) -> EmitResult {
    (
        vec![CpsInstr::NewTuple(dst, elements)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_tuple_index(dst: usize, tuple_reg: usize, index: u16) -> EmitResult {
    (
        vec![CpsInstr::TupleIndex(dst, tuple_reg, index)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_new_int64_array(dst: usize, elements: Vec<usize>) -> EmitResult {
    (
        vec![CpsInstr::NewInt64Array(dst, elements)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_new_float64_array(dst: usize, elements: Vec<usize>) -> EmitResult {
    (
        vec![CpsInstr::NewFloat64Array(dst, elements)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_index_set(dst: usize, obj: usize, idx: usize, val: usize) -> EmitResult {
    (
        vec![CpsInstr::IndexSet(dst, obj, idx, val)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_index_get(dst: usize, obj: usize, idx: usize) -> EmitResult {
    (
        vec![CpsInstr::IndexGet(dst, obj, idx)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_return(reg: usize) -> CpsTerminator {
    CpsTerminator::Return(reg)
}

pub fn emit_jump(target: usize, args: Vec<usize>) -> CpsTerminator {
    CpsTerminator::Jump(target, args)
}

pub fn emit_branch(cond: usize, then_block: usize, else_block: usize) -> CpsTerminator {
    CpsTerminator::Branch(cond, then_block, vec![], else_block, vec![])
}

pub fn emit_call(func_idx: usize, args: Vec<usize>, ret_block: usize) -> CpsTerminator {
    CpsTerminator::Call(func_idx, args, ret_block)
}

pub fn emit_nop() -> EmitResult {
    (vec![], CpsTerminator::Return(0))
}

pub fn emit_load_vtable(dst: usize, vtable_idx: usize) -> EmitResult {
    (
        vec![CpsInstr::LoadVtable(dst, vtable_idx)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_new_interface_obj(dst: usize, vtable_reg: usize, struct_reg: usize) -> EmitResult {
    (
        vec![CpsInstr::NewInterfaceObj(dst, vtable_reg, struct_reg)],
        CpsTerminator::Return(dst),
    )
}

pub fn emit_call_indirect(slot: usize, args: Vec<usize>, cont_block: usize) -> CpsTerminator {
    CpsTerminator::CallIndirect(slot, args, cont_block)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::approx_constant)]
    use super::*;

    // ── emit_literal ──

    #[test]
    fn emit_lit_int() {
        let (instrs, term) = emit_literal(0, 3);
        assert_eq!(instrs.len(), 1);
        assert!(matches!(instrs[0], CpsInstr::LoadConst(0, 3)));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    #[test]
    fn emit_lit_uses_given_register() {
        let (instrs, term) = emit_literal(5, 0);
        assert!(matches!(instrs[0], CpsInstr::LoadConst(5, 0)));
        assert!(matches!(term, CpsTerminator::Return(5)));
    }

    // ── emit_binary ──

    #[test]
    fn emit_binop_add_int() {
        let (instrs, term) = emit_binary(0, CpsBinOp::AddInt, 1, 2);
        assert_eq!(instrs.len(), 1);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::AddInt, 1, 2)
        ));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    #[test]
    fn emit_binop_eq_int() {
        let (instrs, term) = emit_binary(3, CpsBinOp::EqInt, 0, 1);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(3, CpsBinOp::EqInt, 0, 1)
        ));
        assert!(matches!(term, CpsTerminator::Return(3)));
    }

    #[test]
    fn emit_binop_sub_int() {
        let (instrs, term) = emit_binary(2, CpsBinOp::SubInt, 0, 1);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(2, CpsBinOp::SubInt, 0, 1)
        ));
        assert!(matches!(term, CpsTerminator::Return(2)));
    }

    // ── emit_unary ──

    #[test]
    fn emit_unary_neg() {
        let (instrs, term) = emit_unary(0, CpsUnOp::NegInt, 1);
        assert!(matches!(instrs[0], CpsInstr::UnOp(0, CpsUnOp::NegInt, 1)));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    #[test]
    fn emit_unary_not() {
        let (instrs, term) = emit_unary(2, CpsUnOp::Not, 0);
        assert!(matches!(instrs[0], CpsInstr::UnOp(2, CpsUnOp::Not, 0)));
        assert!(matches!(term, CpsTerminator::Return(2)));
    }

    // ── emit_varref ──

    #[test]
    fn emit_varref_returns_register() {
        let (instrs, term) = emit_varref(5);
        assert!(instrs.is_empty(), "varref should have no instructions");
        assert!(matches!(term, CpsTerminator::Return(5)));
    }

    // ── emit_get_field ──

    #[test]
    fn emit_getfield() {
        let (instrs, term) = emit_get_field(0, 1, 2);
        assert!(matches!(instrs[0], CpsInstr::GetField(0, 1, 2)));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    // ── emit_new_struct ──

    #[test]
    fn emit_newstruct_empty() {
        let (instrs, term) = emit_new_struct(0, 0, vec![]);
        assert!(matches!(instrs[0], CpsInstr::NewStruct(0, 0, _)));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    // ── emit_new_list ──

    #[test]
    fn emit_newlist() {
        let (instrs, term) = emit_new_list(0, vec![1, 2, 3]);
        assert!(matches!(instrs[0], CpsInstr::NewList(0, _)));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    // ── emit_return ──

    #[test]
    fn emit_return_terminator() {
        let term = emit_return(3);
        assert!(matches!(term, CpsTerminator::Return(3)));
    }

    #[test]
    fn test_emit_print() {
        let (instrs, term) = super::emit_print(3);
        assert!(matches!(instrs[0], CpsInstr::Print(3)));
        assert!(matches!(term, CpsTerminator::Return(3)));
    }

    #[test]
    fn test_emit_itos() {
        let (instrs, term) = emit_binary(0, CpsBinOp::IToS, 1, 0);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::IToS, 1, 0)
        ));
        assert!(matches!(term, CpsTerminator::Return(0)));
    }

    #[test]
    fn emit_binop_mul_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::MulInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::MulInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_div_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::DivInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::DivInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_mod_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::ModInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::ModInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_lt_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::LtInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::LtInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_le_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::LeInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::LeInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_gt_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::GtInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::GtInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_ge_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::GeInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::GeInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_ne_int() {
        let (instrs, _) = emit_binary(0, CpsBinOp::NeInt, 1, 2);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::NeInt, 1, 2)
        ));
    }

    #[test]
    fn emit_binop_ftos() {
        let (instrs, _) = emit_binary(0, CpsBinOp::FToS, 1, 0);
        assert!(matches!(
            instrs[0],
            CpsInstr::BinOp(0, CpsBinOp::FToS, 1, 0)
        ));
    }

    #[test]
    fn emit_unary_neg_int() {
        let (instrs, term) = emit_unary(7, CpsUnOp::NegInt, 3);
        assert!(matches!(instrs[0], CpsInstr::UnOp(7, CpsUnOp::NegInt, 3)));
        assert!(matches!(term, CpsTerminator::Return(7)));
    }

    #[test]
    fn emit_varref_multiple_regs() {
        let (instrs, term) = emit_varref(10);
        assert!(instrs.is_empty());
        assert!(matches!(term, CpsTerminator::Return(10)));
    }

    #[test]
    fn emit_newstruct_with_fields() {
        let (instrs, term) = emit_new_struct(3, 7, vec![1, 2, 3]);
        assert!(matches!(instrs[0], CpsInstr::NewStruct(3, 7, _)));
        assert!(matches!(term, CpsTerminator::Return(3)));
    }
}
