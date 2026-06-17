//! CPS (Continuation-Passing Style) blocks — kaubo v2
//!
//! 三地址码 + block terminators, 用于代码生成和优化

use serde::{Serialize, Deserialize};

/// CPS 模块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpsModule {
    pub functions: Vec<CpsFunction>,
    pub constants: Vec<Constant>,
    pub structs: Vec<StructDef>,
}

/// CPS 函数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpsFunction {
    pub name: String,
    pub blocks: Vec<CpsBlock>,
    pub entry: usize,
    pub reg_count: usize,
}

/// CPS 基本块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpsBlock {
    pub id: usize,
    pub params: Vec<usize>,
    pub instrs: Vec<CpsInstr>,
    pub term: CpsTerminator,
}

/// 三地址码指令 (纯计算)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpsInstr {
    BinOp(usize, CpsBinOp, usize, usize),
    UnOp(usize, CpsUnOp, usize),
    LoadConst(usize, usize),
    Move(usize, usize),
    NewStruct(usize, usize, Vec<usize>),
    GetField(usize, usize, u16),
    SetField(usize, usize, u16, usize),
    NewList(usize, Vec<usize>),
    IndexGet(usize, usize, usize),
    IndexSet(usize, usize, usize, usize),
    Box(usize, usize),
    Unbox(usize, usize),
    Print(usize),
    Nop,
}

/// 块终结器 (唯一的控制流出口)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpsTerminator {
    Jump(usize, Vec<usize>),
    Branch(usize, usize, Vec<usize>, usize, Vec<usize>),
    Return(usize),
    Call(usize, Vec<usize>, usize),
    TailCall(usize, Vec<usize>),
    CallNative(usize, Vec<usize>, usize),  // (native_index, arg_regs, cont_block)
    Suspend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CpsBinOp {
    AddInt, SubInt, MulInt, DivInt, ModInt,
    FAdd, FSub, FMul, FDiv,
    SAdd,
    EqInt, NeInt, LtInt, LeInt, GtInt, GeInt,
    FEq, FLt,
    IToF, FToI, IToS, FToS, SToI,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CpsUnOp { NegInt, FNeg, Not }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant { Int(i64), Float(f64), String(String), Bool(bool), Null }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub id: usize,
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub type_bitmap: u64,
}
