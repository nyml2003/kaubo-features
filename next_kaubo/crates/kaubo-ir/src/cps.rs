//! CPS (Continuation-Passing Style) blocks — kaubo v2
//!
//! 三地址码 + block terminators, 用于代码生成和优化

/// CPS 模块
#[derive(Debug, Clone)]
pub struct CpsModule {
    pub functions: Vec<CpsFunction>,
    pub constants: Vec<Constant>,
    pub structs: Vec<StructDef>,
}

/// CPS 函数
#[derive(Debug, Clone)]
pub struct CpsFunction {
    pub name: String,
    pub blocks: Vec<CpsBlock>,
    pub entry: usize,
    pub reg_count: usize,
}

/// CPS 基本块
#[derive(Debug, Clone)]
pub struct CpsBlock {
    pub id: usize,
    pub params: Vec<usize>,    // 虚拟寄存器
    pub instrs: Vec<CpsInstr>,
    pub term: CpsTerminator,
}

/// 三地址码指令 (纯计算)
#[derive(Debug, Clone)]
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
    Print(usize),    // Print register value
    Nop,
}

/// 块终结器 (唯一的控制流出口)
#[derive(Debug, Clone)]
pub enum CpsTerminator {
    Jump(usize, Vec<usize>),
    Branch(usize, usize, Vec<usize>, usize, Vec<usize>),
    Return(usize),
    Call(usize, Vec<usize>, usize),
    TailCall(usize, Vec<usize>),
    Suspend,
}

#[derive(Debug, Clone)]
pub enum CpsBinOp {
    AddInt, SubInt, MulInt, DivInt, ModInt,
    FAdd, FSub, FMul, FDiv,
    SAdd,
    EqInt, NeInt, LtInt, LeInt, GtInt, GeInt,
    FEq, FLt,
    IToF, FToI, IToS, FToS, SToI,
}

#[derive(Debug, Clone)]
pub enum CpsUnOp { NegInt, FNeg, Not }

#[derive(Debug, Clone)]
pub enum Constant { Int(i64), Float(f64), String(String), Bool(bool), Null }

#[derive(Debug, Clone)]
pub struct StructDef {
    pub id: usize,
    pub name: String,
    pub fields: Vec<(String, String)>, // name, type_name
}
