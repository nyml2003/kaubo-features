//! Binary serialization — CpsModule ↔ compact bytecode.
//!
//! Header: "KAUB" (4) + version u32 (4) = 8 bytes

use crate::cps::*;
use std::io::{Cursor, Read};

const MAGIC: &[u8; 4] = b"KAUB";
const VERSION: u32 = 1;

// ── Write helpers ──

fn w_u16(w: &mut Vec<u8>, v: u16) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_u64(w: &mut Vec<u8>, v: u64) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_i64(w: &mut Vec<u8>, v: i64) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_f64(w: &mut Vec<u8>, v: f64) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_u8(w: &mut Vec<u8>, v: u8) {
    w.push(v);
}

// ── Read helpers ──

fn r_u16(r: &mut Cursor<&[u8]>) -> Result<u16, String> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|e| format!("read: {e}"))?;
    Ok(u16::from_le_bytes(b))
}
fn r_u32(r: &mut Cursor<&[u8]>) -> Result<u32, String> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| format!("read: {e}"))?;
    Ok(u32::from_le_bytes(b))
}
fn r_u64(r: &mut Cursor<&[u8]>) -> Result<u64, String> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| format!("read: {e}"))?;
    Ok(u64::from_le_bytes(b))
}
fn r_i64(r: &mut Cursor<&[u8]>) -> Result<i64, String> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| format!("read: {e}"))?;
    Ok(i64::from_le_bytes(b))
}
fn r_f64(r: &mut Cursor<&[u8]>) -> Result<f64, String> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| format!("read: {e}"))?;
    Ok(f64::from_le_bytes(b))
}
fn r_u8(r: &mut Cursor<&[u8]>) -> Result<u8, String> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|e| format!("read: {e}"))?;
    Ok(b[0])
}

// ── Public API ──

pub fn encode_module(module: &CpsModule) -> Vec<u8> {
    let mut w = Vec::new();
    w.extend_from_slice(MAGIC);
    w_u32(&mut w, VERSION);

    // Constants
    w_u16(&mut w, module.constants.len() as u16);
    for c in &module.constants {
        encode_constant(&mut w, c);
    }

    // Structs
    w_u16(&mut w, module.structs.len() as u16);
    for s in &module.structs {
        encode_struct_def(&mut w, s);
    }

    // Functions
    w_u16(&mut w, module.functions.len() as u16);
    for f in &module.functions {
        encode_function(&mut w, f);
    }
    w
}

pub fn decode_module(bytes: &[u8]) -> Result<CpsModule, String> {
    let mut r = Cursor::new(bytes);

    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)
        .map_err(|e| format!("magic: {e}"))?;
    if &magic != MAGIC {
        return Err("bad magic".into());
    }

    let version = r_u32(&mut r)?;
    if version != VERSION {
        return Err(format!("unsupported version {version}"));
    }

    let const_count = r_u16(&mut r)? as usize;
    let mut constants = Vec::with_capacity(const_count);
    for _ in 0..const_count {
        constants.push(decode_constant(&mut r)?);
    }

    let struct_count = r_u16(&mut r)? as usize;
    let mut structs = Vec::with_capacity(struct_count);
    for _ in 0..struct_count {
        structs.push(decode_struct_def(&mut r)?);
    }

    let func_count = r_u16(&mut r)? as usize;
    let mut functions = Vec::with_capacity(func_count);
    for _ in 0..func_count {
        functions.push(decode_function(&mut r)?);
    }

    Ok(CpsModule {
        functions,
        constants,
        structs,
        enums: vec![],
    })
}

// ── Constant encode/decode ──

fn encode_constant(w: &mut Vec<u8>, c: &Constant) {
    match c {
        Constant::Int(n) => {
            w_u8(w, 0);
            w_i64(w, *n);
        }
        Constant::Float(f) => {
            w_u8(w, 1);
            w_f64(w, *f);
        }
        Constant::String(s) => {
            w_u8(w, 2);
            let b = s.as_bytes();
            w_u16(w, b.len() as u16);
            w.extend_from_slice(b);
        }
        Constant::Bool(b) => {
            w_u8(w, 3);
            w_u8(w, *b as u8);
        }
        Constant::Null => {
            w_u8(w, 4);
        }
    }
}

fn decode_constant(r: &mut Cursor<&[u8]>) -> Result<Constant, String> {
    let tag = r_u8(r)?;
    Ok(match tag {
        0 => Constant::Int(r_i64(r)?),
        1 => Constant::Float(r_f64(r)?),
        2 => {
            let len = r_u16(r)? as usize;
            let mut b = vec![0u8; len];
            r.read_exact(&mut b).map_err(|e| format!("str: {e}"))?;
            Constant::String(String::from_utf8(b).map_err(|e| format!("utf8: {e}"))?)
        }
        3 => Constant::Bool(r_u8(r)? != 0),
        4 => Constant::Null,
        _ => return Err(format!("bad const tag {tag}")),
    })
}

// ── StructDef encode/decode ──

fn encode_struct_def(w: &mut Vec<u8>, s: &StructDef) {
    w_u32(w, s.id as u32);
    w_u16(w, s.name.len() as u16);
    w.extend_from_slice(s.name.as_bytes());
    w_u16(w, s.fields.len() as u16);
    for (n, t) in &s.fields {
        w_u16(w, n.len() as u16);
        w.extend_from_slice(n.as_bytes());
        w_u16(w, t.len() as u16);
        w.extend_from_slice(t.as_bytes());
    }
    w_u64(w, s.type_bitmap);
}

fn decode_struct_def(r: &mut Cursor<&[u8]>) -> Result<StructDef, String> {
    let id = r_u32(r)? as usize;
    let nlen = r_u16(r)? as usize;
    let mut nb = vec![0u8; nlen];
    r.read_exact(&mut nb).map_err(|e| format!("sname: {e}"))?;
    let name = String::from_utf8(nb).map_err(|e| format!("sname utf8: {e}"))?;

    let fcount = r_u16(r)? as usize;
    let mut fields = Vec::with_capacity(fcount);
    for _ in 0..fcount {
        let flen = r_u16(r)? as usize;
        let mut fb = vec![0u8; flen];
        r.read_exact(&mut fb).map_err(|e| format!("fname: {e}"))?;
        let fn2 = String::from_utf8(fb).map_err(|e| format!("fname utf8: {e}"))?;
        let tlen = r_u16(r)? as usize;
        let mut tb = vec![0u8; tlen];
        r.read_exact(&mut tb).map_err(|e| format!("ftype: {e}"))?;
        let ft = String::from_utf8(tb).map_err(|e| format!("ftype utf8: {e}"))?;
        fields.push((fn2, ft));
    }
    let type_bitmap = r_u64(r)?;
    Ok(StructDef {
        id,
        name,
        fields,
        type_bitmap,
    })
}

// ── Function encode/decode ──

fn encode_function(w: &mut Vec<u8>, f: &CpsFunction) {
    w_u16(w, f.name.len() as u16);
    w.extend_from_slice(f.name.as_bytes());
    let valid: Vec<&CpsBlock> = f.blocks.iter().filter(|b| b.id != usize::MAX).collect();
    let entry_idx = valid
        .iter()
        .position(|b| b.id == f.blocks[f.entry].id)
        .unwrap_or(0);
    w_u32(w, entry_idx as u32);
    w_u32(w, f.reg_count as u32);
    w_u16(w, valid.len() as u16);
    for b in &valid {
        encode_block(w, b);
    }
}

fn decode_function(r: &mut Cursor<&[u8]>) -> Result<CpsFunction, String> {
    let nlen = r_u16(r)? as usize;
    let mut nb = vec![0u8; nlen];
    r.read_exact(&mut nb).map_err(|e| format!("fname: {e}"))?;
    let name = String::from_utf8(nb).map_err(|e| format!("fname utf8: {e}"))?;
    let entry = r_u32(r)? as usize;
    let reg_count = r_u32(r)? as usize;
    let bcount = r_u16(r)? as usize;
    let mut blocks = Vec::with_capacity(bcount);
    for _ in 0..bcount {
        blocks.push(decode_block(r)?);
    }
    Ok(CpsFunction {
        name,
        blocks,
        entry,
        reg_count,
    })
}

// ── Block encode/decode ──

fn encode_block(w: &mut Vec<u8>, b: &CpsBlock) {
    w_u32(w, b.id as u32);
    w_u16(w, b.params.len() as u16);
    for &p in &b.params {
        w_u32(w, p as u32);
    }
    w_u16(w, b.instrs.len() as u16);
    for i in &b.instrs {
        encode_instr(w, i);
    }
    encode_term(w, &b.term);
}

fn decode_block(r: &mut Cursor<&[u8]>) -> Result<CpsBlock, String> {
    let id = r_u32(r)? as usize;
    let pcount = r_u16(r)? as usize;
    let mut params = Vec::with_capacity(pcount);
    for _ in 0..pcount {
        params.push(r_u32(r)? as usize);
    }
    let icount = r_u16(r)? as usize;
    let mut instrs = Vec::with_capacity(icount);
    for _ in 0..icount {
        instrs.push(decode_instr(r)?);
    }
    let term = decode_term(r)?;
    Ok(CpsBlock {
        id,
        params,
        instrs,
        term,
    })
}

// ── Instruction encode/decode ──

fn encode_instr(w: &mut Vec<u8>, i: &CpsInstr) {
    match i {
        CpsInstr::BinOp(d, op, s1, s2) => {
            w_u8(w, 0x00);
            w_u16(w, *d as u16);
            w_u8(w, binop_to_u8(*op));
            w_u16(w, *s1 as u16);
            w_u16(w, *s2 as u16);
        }
        CpsInstr::UnOp(d, op, s) => {
            w_u8(w, 0x01);
            w_u16(w, *d as u16);
            w_u8(w, unop_to_u8(*op));
            w_u16(w, *s as u16);
        }
        CpsInstr::LoadConst(d, idx) => {
            w_u8(w, 0x02);
            w_u16(w, *d as u16);
            w_u32(w, *idx as u32);
        }
        CpsInstr::Move(d, s) => {
            w_u8(w, 0x03);
            w_u16(w, *d as u16);
            w_u16(w, *s as u16);
        }
        CpsInstr::NewStruct(d, sid, _) => {
            w_u8(w, 0x04);
            w_u16(w, *d as u16);
            w_u32(w, *sid as u32);
        }
        CpsInstr::GetField(d, o, idx) => {
            w_u8(w, 0x05);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
            w_u16(w, *idx);
        }
        CpsInstr::SetField(d, o, idx, _) => {
            w_u8(w, 0x06);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
            w_u16(w, *idx);
        }
        CpsInstr::NewList(d, _) => {
            w_u8(w, 0x07);
            w_u16(w, *d as u16);
        }
        CpsInstr::ListLen(d, obj) => {
            w_u8(w, 0x1A);
            w_u16(w, *d as u16);
            w_u16(w, *obj as u16);
        }
        CpsInstr::IndexGet(d, o, i) => {
            w_u8(w, 0x08);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
            w_u16(w, *i as u16);
        }
        CpsInstr::IndexSet(d, o, i, v) => {
            w_u8(w, 0x09);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
            w_u16(w, *i as u16);
            w_u16(w, *v as u16);
        }
        CpsInstr::Box(d, s) => {
            w_u8(w, 0x0A);
            w_u16(w, *d as u16);
            w_u16(w, *s as u16);
        }
        CpsInstr::Unbox(d, s) => {
            w_u8(w, 0x0B);
            w_u16(w, *d as u16);
            w_u16(w, *s as u16);
        }
        CpsInstr::Print(r) => {
            w_u8(w, 0x0C);
            w_u16(w, *r as u16);
        }
        CpsInstr::NewVariant(d, eid, tag, _) => {
            w_u8(w, 0x0E);
            w_u16(w, *d as u16);
            w_u16(w, *eid as u16);
            w_u16(w, *tag);
        }
        CpsInstr::GetVariantTag(d, o) => {
            w_u8(w, 0x0F);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
        }
        CpsInstr::SetVariantField(d, o, fi, _) => {
            w_u8(w, 0x11);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
            w_u16(w, *fi);
        }
        CpsInstr::GetVariantField(d, o, fi) => {
            w_u8(w, 0x10);
            w_u16(w, *d as u16);
            w_u16(w, *o as u16);
            w_u16(w, *fi);
        }
        CpsInstr::Nop => {
            w_u8(w, 0x0D);
        }
    }
}

fn decode_instr(r: &mut Cursor<&[u8]>) -> Result<CpsInstr, String> {
    let tag = r_u8(r)?;
    Ok(match tag {
        0x00 => CpsInstr::BinOp(
            r_u16(r)? as usize,
            u8_to_binop(r_u8(r)?)?,
            r_u16(r)? as usize,
            r_u16(r)? as usize,
        ),
        0x01 => CpsInstr::UnOp(
            r_u16(r)? as usize,
            u8_to_unop(r_u8(r)?)?,
            r_u16(r)? as usize,
        ),
        0x02 => CpsInstr::LoadConst(r_u16(r)? as usize, r_u32(r)? as usize),
        0x03 => CpsInstr::Move(r_u16(r)? as usize, r_u16(r)? as usize),
        0x04 => CpsInstr::NewStruct(r_u16(r)? as usize, r_u32(r)? as usize, vec![]),
        0x05 => CpsInstr::GetField(r_u16(r)? as usize, r_u16(r)? as usize, r_u16(r)?),
        0x06 => CpsInstr::SetField(r_u16(r)? as usize, r_u16(r)? as usize, r_u16(r)?, 0),
        0x07 => CpsInstr::NewList(r_u16(r)? as usize, vec![]),
        0x1A => CpsInstr::ListLen(r_u16(r)? as usize, r_u16(r)? as usize),
        0x08 => CpsInstr::IndexGet(r_u16(r)? as usize, r_u16(r)? as usize, r_u16(r)? as usize),
        0x09 => CpsInstr::IndexSet(
            r_u16(r)? as usize,
            r_u16(r)? as usize,
            r_u16(r)? as usize,
            r_u16(r)? as usize,
        ),
        0x0A => CpsInstr::Box(r_u16(r)? as usize, r_u16(r)? as usize),
        0x0B => CpsInstr::Unbox(r_u16(r)? as usize, r_u16(r)? as usize),
        0x0E => CpsInstr::NewVariant(
            r_u16(r)? as usize,
            r_u32(r)? as usize,
            r_u16(r)?,
            vec![],
        ),
        0x0F => CpsInstr::GetVariantTag(
            r_u16(r)? as usize,
            r_u16(r)? as usize,
        ),
        0x11 => CpsInstr::SetVariantField(
            r_u16(r)? as usize,
            r_u16(r)? as usize,
            r_u16(r)?,
            0,
        ),
        0x10 => CpsInstr::GetVariantField(
            r_u16(r)? as usize,
            r_u16(r)? as usize,
            r_u16(r)?,
        ),
        0x0C => CpsInstr::Print(r_u16(r)? as usize),
        0x0D => CpsInstr::Nop,
        _ => return Err(format!("bad instr tag {tag:02x}")),
    })
}

fn encode_term(w: &mut Vec<u8>, t: &CpsTerminator) {
    match t {
        CpsTerminator::Jump(b, args) => {
            w_u8(w, 0x10);
            w_u32(w, *b as u32);
            w_u16(w, args.len() as u16);
            for a in args {
                w_u16(w, *a as u16);
            }
        }
        CpsTerminator::Branch(c, tb, _, fb, _) => {
            w_u8(w, 0x11);
            w_u16(w, *c as u16);
            w_u32(w, *tb as u32);
            w_u32(w, *fb as u32);
        }
        CpsTerminator::Return(r) => {
            w_u8(w, 0x12);
            w_u16(w, *r as u16);
        }
        CpsTerminator::Call(fi, args, ret) => {
            w_u8(w, 0x13);
            w_u16(w, *fi as u16);
            w_u32(w, *ret as u32);
            w_u16(w, args.len() as u16);
            for a in args {
                w_u16(w, *a as u16);
            }
        }
        CpsTerminator::TailCall(fi, args) => {
            w_u8(w, 0x14);
            w_u16(w, *fi as u16);
            w_u16(w, args.len() as u16);
            for a in args {
                w_u16(w, *a as u16);
            }
        }
        CpsTerminator::CallNative(fi, args, ret) => {
            w_u8(w, 0x16);
            w_u16(w, *fi as u16);
            w_u32(w, *ret as u32);
            w_u16(w, args.len() as u16);
            for a in args {
                w_u16(w, *a as u16);
            }
        }
        CpsTerminator::Suspend => {
            w_u8(w, 0x15);
        }
    }
}

fn decode_term(r: &mut Cursor<&[u8]>) -> Result<CpsTerminator, String> {
    let tag = r_u8(r)?;
    Ok(match tag {
        0x10 => {
            let b = r_u32(r)? as usize;
            let n = r_u16(r)? as usize;
            let mut args = vec![];
            for _ in 0..n {
                args.push(r_u16(r)? as usize);
            }
            CpsTerminator::Jump(b, args)
        }
        0x11 => {
            let c = r_u16(r)? as usize;
            let tb = r_u32(r)? as usize;
            let fb = r_u32(r)? as usize;
            CpsTerminator::Branch(c, tb, vec![], fb, vec![])
        }
        0x12 => CpsTerminator::Return(r_u16(r)? as usize),
        0x13 => {
            let fi = r_u16(r)? as usize;
            let ret = r_u32(r)? as usize;
            let n = r_u16(r)? as usize;
            let mut args = vec![];
            for _ in 0..n {
                args.push(r_u16(r)? as usize);
            }
            CpsTerminator::Call(fi, args, ret)
        }
        0x14 => {
            let fi = r_u16(r)? as usize;
            let n = r_u16(r)? as usize;
            let mut args = vec![];
            for _ in 0..n {
                args.push(r_u16(r)? as usize);
            }
            CpsTerminator::TailCall(fi, args)
        }
        0x15 => CpsTerminator::Suspend,
        0x16 => {
            let fi = r_u16(r)? as usize;
            let ret = r_u32(r)? as usize;
            let n = r_u16(r)? as usize;
            let mut args = vec![];
            for _ in 0..n {
                args.push(r_u16(r)? as usize);
            }
            CpsTerminator::CallNative(fi, args, ret)
        }
        _ => return Err(format!("bad term tag {tag:02x}")),
    })
}

// ── Opcode maps ──

fn binop_to_u8(op: CpsBinOp) -> u8 {
    match op {
        CpsBinOp::AddInt => 0,
        CpsBinOp::SubInt => 1,
        CpsBinOp::MulInt => 2,
        CpsBinOp::DivInt => 3,
        CpsBinOp::ModInt => 4,
        CpsBinOp::FAdd => 5,
        CpsBinOp::FSub => 6,
        CpsBinOp::FMul => 7,
        CpsBinOp::FDiv => 8,
        CpsBinOp::SAdd => 9,
        CpsBinOp::EqInt => 10,
        CpsBinOp::NeInt => 11,
        CpsBinOp::LtInt => 12,
        CpsBinOp::LeInt => 13,
        CpsBinOp::GtInt => 14,
        CpsBinOp::GeInt => 15,
        CpsBinOp::FEq => 16,
        CpsBinOp::FLt => 17,
        CpsBinOp::IToF => 18,
        CpsBinOp::FToI => 19,
        CpsBinOp::IToS => 20,
        CpsBinOp::FToS => 21,
        CpsBinOp::SToI => 22,
        CpsBinOp::FNe => 23,
        CpsBinOp::FLe => 24,
        CpsBinOp::FGt => 25,
        CpsBinOp::FGe => 26,
    }
}

fn u8_to_binop(v: u8) -> Result<CpsBinOp, String> {
    Ok(match v {
        0 => CpsBinOp::AddInt,
        1 => CpsBinOp::SubInt,
        2 => CpsBinOp::MulInt,
        3 => CpsBinOp::DivInt,
        4 => CpsBinOp::ModInt,
        5 => CpsBinOp::FAdd,
        6 => CpsBinOp::FSub,
        7 => CpsBinOp::FMul,
        8 => CpsBinOp::FDiv,
        9 => CpsBinOp::SAdd,
        10 => CpsBinOp::EqInt,
        11 => CpsBinOp::NeInt,
        12 => CpsBinOp::LtInt,
        13 => CpsBinOp::LeInt,
        14 => CpsBinOp::GtInt,
        15 => CpsBinOp::GeInt,
        16 => CpsBinOp::FEq,
        17 => CpsBinOp::FLt,
        18 => CpsBinOp::IToF,
        19 => CpsBinOp::FToI,
        20 => CpsBinOp::IToS,
        21 => CpsBinOp::FToS,
        22 => CpsBinOp::SToI,
        23 => CpsBinOp::FNe,
        24 => CpsBinOp::FLe,
        25 => CpsBinOp::FGt,
        26 => CpsBinOp::FGe,
        _ => return Err(format!("bad binop tag {v}")),
    })
}

fn unop_to_u8(op: CpsUnOp) -> u8 {
    match op {
        CpsUnOp::NegInt => 0,
        CpsUnOp::FNeg => 1,
        CpsUnOp::Not => 2,
    }
}
fn u8_to_unop(v: u8) -> Result<CpsUnOp, String> {
    Ok(match v {
        0 => CpsUnOp::NegInt,
        1 => CpsUnOp::FNeg,
        2 => CpsUnOp::Not,
        _ => return Err(format!("bad unop tag {v}")),
    })
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps_build::build_module;
    use crate::flatten::flatten_module;
    use crate::pass::fold::ConstantFold;
    use crate::pass::Pass;
    use crate::test_fixtures;

    fn roundtrip(src: &str) -> CpsModule {
        let m = test_fixtures::module(src);
        let mut cps = build_module(&m, None).unwrap();
        flatten_module(&mut cps);
        ConstantFold.run(&mut cps);
        let bytes = encode_module(&cps);
        decode_module(&bytes).unwrap()
    }

    #[test]
    fn roundtrip_lit() {
        let cps = roundtrip("const x = 42;");
        assert_eq!(cps.functions.len(), 1);
        assert!(cps.functions[0].blocks.iter().any(|b| b
            .instrs
            .iter()
            .any(|i| matches!(i, CpsInstr::LoadConst(..)))));
    }

    #[test]
    fn roundtrip_add() {
        let cps = roundtrip("const x = 2 + 3;");
        let f = &cps.functions[0];
        // Walk reachable blocks from entry
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![f.entry];
        while let Some(bid) = stack.pop() {
            if !visited.insert(bid) {
                continue;
            }
            if let Some(b) = f.blocks.iter().find(|b| b.id == bid) {
                for i in &b.instrs {
                    assert!(
                        !matches!(i, CpsInstr::BinOp(_, CpsBinOp::AddInt, _, _)),
                        "binop should be folded: {i:?}"
                    );
                }
                match &b.term {
                    CpsTerminator::Jump(t, _) => stack.push(*t),
                    CpsTerminator::Branch(_, t, _, f, _) => {
                        stack.push(*t);
                        stack.push(*f);
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn roundtrip_while() {
        let cps = roundtrip("var i = 0; while i < 3 { i = i + 1; };");
        assert!(cps.functions[0].blocks.len() >= 2);
    }

    #[test]
    fn roundtrip_lambda() {
        let cps = roundtrip("const f = |x| { x + 1 }; f(41);");
        assert!(cps.functions.len() >= 2);
    }

    #[test]
    fn roundtrip_struct() {
        let cps = roundtrip("struct Point { x: Int64, y: Int64 }; const p = Point { x: 1, y: 2 };");
        assert!(!cps.structs.is_empty());
    }

    #[test]
    fn binary_size_reasonable() {
        let m = test_fixtures::module("const x = 42;");
        let mut cps = build_module(&m, None).unwrap();
        flatten_module(&mut cps);
        let bin = encode_module(&cps);
        assert!(
            bin.len() < 500,
            "binary size {} should be < 500 bytes",
            bin.len()
        );
        assert!(&bin[0..4] == b"KAUB", "should start with KAUB magic");
    }

    #[test]
    fn roundtrip_covers_all_binary_tags() {
        let binops = [
            CpsBinOp::AddInt,
            CpsBinOp::SubInt,
            CpsBinOp::MulInt,
            CpsBinOp::DivInt,
            CpsBinOp::ModInt,
            CpsBinOp::FAdd,
            CpsBinOp::FSub,
            CpsBinOp::FMul,
            CpsBinOp::FDiv,
            CpsBinOp::SAdd,
            CpsBinOp::EqInt,
            CpsBinOp::NeInt,
            CpsBinOp::LtInt,
            CpsBinOp::LeInt,
            CpsBinOp::GtInt,
            CpsBinOp::GeInt,
            CpsBinOp::FEq,
            CpsBinOp::FNe,
            CpsBinOp::FLt,
            CpsBinOp::FLe,
            CpsBinOp::FGt,
            CpsBinOp::FGe,
            CpsBinOp::IToF,
            CpsBinOp::FToI,
            CpsBinOp::IToS,
            CpsBinOp::FToS,
            CpsBinOp::SToI,
        ];
        let mut instrs = Vec::new();
        for (i, op) in binops.iter().enumerate() {
            instrs.push(CpsInstr::BinOp(i, *op, 0, 1));
        }
        instrs.extend([
            CpsInstr::UnOp(28, CpsUnOp::NegInt, 0),
            CpsInstr::UnOp(29, CpsUnOp::FNeg, 0),
            CpsInstr::UnOp(30, CpsUnOp::Not, 0),
            CpsInstr::LoadConst(31, 0),
            CpsInstr::Move(32, 31),
            CpsInstr::NewStruct(33, 0, vec![0, 1]),
            CpsInstr::GetField(34, 33, 1),
            CpsInstr::SetField(35, 33, 1, 34),
            CpsInstr::NewList(36, vec![0, 1]),
            CpsInstr::IndexGet(37, 36, 0),
            CpsInstr::IndexSet(36, 36, 0, 37),
            CpsInstr::Box(38, 37),
            CpsInstr::Unbox(39, 38),
            CpsInstr::Print(39),
            CpsInstr::Nop,
        ]);

        let module = CpsModule {
            constants: vec![
                Constant::Int(-7),
                Constant::Float(3.5),
                Constant::String("tagged".to_string()),
                Constant::Bool(true),
                Constant::Null,
            ],
            structs: vec![StructDef {
                id: 9,
                name: "Pair".to_string(),
                fields: vec![
                    ("left".to_string(), "Int64".to_string()),
                    ("right".to_string(), "String".to_string()),
                ],
                type_bitmap: 0b10,
            }],
            enums: vec![],
            functions: vec![
                CpsFunction {
                    name: "main".to_string(),
                    entry: 0,
                    reg_count: 44,
                    blocks: vec![
                        CpsBlock {
                            id: 0,
                            params: vec![0, 1],
                            instrs,
                            term: CpsTerminator::Jump(1, vec![0, 1]),
                        },
                        CpsBlock {
                            id: 1,
                            params: vec![],
                            instrs: vec![],
                            term: CpsTerminator::Branch(0, 2, vec![1], 3, vec![2]),
                        },
                        CpsBlock {
                            id: 2,
                            params: vec![],
                            instrs: vec![],
                            term: CpsTerminator::Return(0),
                        },
                        CpsBlock {
                            id: 3,
                            params: vec![],
                            instrs: vec![],
                            term: CpsTerminator::Call(1, vec![0], 4),
                        },
                        CpsBlock {
                            id: 4,
                            params: vec![],
                            instrs: vec![],
                            term: CpsTerminator::TailCall(1, vec![0]),
                        },
                        CpsBlock {
                            id: 5,
                            params: vec![],
                            instrs: vec![],
                            term: CpsTerminator::CallNative(0, vec![0], 2),
                        },
                        CpsBlock {
                            id: 6,
                            params: vec![],
                            instrs: vec![],
                            term: CpsTerminator::Suspend,
                        },
                        CpsBlock {
                            id: usize::MAX,
                            params: vec![],
                            instrs: vec![CpsInstr::Nop],
                            term: CpsTerminator::Return(0),
                        },
                    ],
                },
                CpsFunction {
                    name: "callee".to_string(),
                    entry: 0,
                    reg_count: 1,
                    blocks: vec![CpsBlock {
                        id: 0,
                        params: vec![],
                        instrs: vec![],
                        term: CpsTerminator::Return(0),
                    }],
                },
            ],
        };

        let decoded = decode_module(&encode_module(&module)).unwrap();

        assert_eq!(decoded.constants.len(), 5);
        assert_eq!(decoded.structs[0].name, "Pair");
        assert_eq!(decoded.structs[0].type_bitmap, 0b10);
        assert_eq!(decoded.functions.len(), 2);
        assert_eq!(decoded.functions[0].blocks.len(), 7);
        assert_eq!(decoded.functions[0].blocks[0].params, vec![0, 1]);
        assert_eq!(decoded.functions[0].blocks[0].instrs.len(), 42);
        assert!(matches!(
            decoded.functions[0].blocks[0].instrs[0],
            CpsInstr::BinOp(_, CpsBinOp::AddInt, _, _)
        ));
        assert!(decoded.functions[0].blocks[0].instrs.iter().any(|instr| matches!(
            instr,
            CpsInstr::BinOp(_, CpsBinOp::SToI, _, _)
        )));
        assert!(matches!(
            decoded.functions[0].blocks[1].term,
            CpsTerminator::Branch(_, 2, _, 3, _)
        ));
        assert!(matches!(
            decoded.functions[0].blocks[3].term,
            CpsTerminator::Call(1, _, 4)
        ));
        assert!(matches!(
            decoded.functions[0].blocks[4].term,
            CpsTerminator::TailCall(1, _)
        ));
        assert!(matches!(
            decoded.functions[0].blocks[5].term,
            CpsTerminator::CallNative(0, _, 2)
        ));
        assert!(matches!(
            decoded.functions[0].blocks[6].term,
            CpsTerminator::Suspend
        ));
    }

    #[test]
    fn decode_reports_invalid_headers_and_tags() {
        assert_eq!(decode_module(b"NOPE").unwrap_err(), "bad magic");

        let mut unsupported = Vec::new();
        unsupported.extend_from_slice(b"KAUB");
        unsupported.extend_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            decode_module(&unsupported).unwrap_err(),
            "unsupported version 99"
        );

        let module = CpsModule {
            constants: vec![Constant::Null],
            structs: vec![],
            enums: vec![],
            functions: vec![],
        };
        let mut bytes = encode_module(&module);
        bytes[10] = 0xFF;
        assert!(decode_module(&bytes).unwrap_err().contains("bad const tag"));

        assert!(u8_to_binop(0xFF).unwrap_err().contains("bad binop tag"));
        assert!(u8_to_unop(0xFF).unwrap_err().contains("bad unop tag"));
    }
}
