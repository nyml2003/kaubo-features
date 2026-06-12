//! HIR (High-level Intermediate Representation)
//!
//! 位于 AST 和 Bytecode 之间的中间表示。
//! 使用基本块 + 三地址码，方便优化 pass 操作。

use crate::bytecode::OpCode;
use std::fmt;

// ============================================================
// HIR Module
// ============================================================

/// HIR 模块：一个编译单元的优化中间表示
#[derive(Debug, Clone)]
pub struct HirModule {
    pub functions: Vec<HirFunction>,
    pub constants: Vec<ConstantValue>,
    pub struct_infos: Vec<StructInfo>,
}

/// HIR 函数：一个函数体，由基本块组成
#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: Option<String>,
    pub arity: u8,
    pub blocks: Vec<HirBlock>,
    /// entry block index
    pub entry: usize,
    /// 局部变量数量
    pub local_count: usize,
    /// 返回类型（None = void）
    pub return_type: Option<String>,
}

/// HIR 基本块
#[derive(Debug, Clone)]
pub struct HirBlock {
    pub id: usize,
    pub instrs: Vec<HirInstr>,
    /// terminator: 控制如何离开此块
    pub term: HirTerminator,
}

/// HIR 指令（三地址码形式）
#[derive(Debug, Clone)]
pub enum HirInstr {
    /// result = left op right
    Binary {
        dst: HirOperand,
        op: HirBinaryOp,
        left: HirOperand,
        right: HirOperand,
    },
    /// result = op src
    Unary {
        dst: HirOperand,
        op: HirUnaryOp,
        src: HirOperand,
    },
    /// dst = constant value
    LoadConst {
        dst: HirOperand,
        value: ConstantValue,
    },
    /// dst = src (copy)
    Move {
        dst: HirOperand,
        src: HirOperand,
    },
    /// call(dst, callee, args)
    Call {
        dst: Option<HirOperand>,
        callee: HirOperand,
        args: Vec<HirOperand>,
    },
    /// 返回 dst
    Return {
        value: Option<HirOperand>,
    },
    /// 打印（debug 用途）
    Print {
        value: HirOperand,
    },
    /// 无操作
    Nop,
}

/// HIR 终止指令
#[derive(Debug, Clone)]
pub enum HirTerminator {
    /// 无条件跳转到块
    Jump { target: usize },
    /// 条件跳转：if cond is truthy goto true_target else false_target
    Branch {
        cond: HirOperand,
        true_target: usize,
        false_target: usize,
    },
    /// 函数返回
    Return { value: Option<HirOperand> },
    /// 结束（没有更多指令）
    End,
}

/// HIR 操作数
#[derive(Debug, Clone, PartialEq)]
pub enum HirOperand {
    /// 临时变量（三地址码中的虚拟寄存器）
    Temp(usize),
    /// 局部变量（按名称索引）
    Local(String),
    /// 常量引用
    Const(usize),
    /// 全局变量
    Global(String),
    /// 字面量常量
    Immediate(ConstantValue),
    /// 空操作数
    None,
}

/// HIR 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

impl HirBinaryOp {
    pub fn to_opcode(self) -> OpCode {
        match self {
            HirBinaryOp::Add => OpCode::Add,
            HirBinaryOp::Sub => OpCode::Sub,
            HirBinaryOp::Mul => OpCode::Mul,
            HirBinaryOp::Div => OpCode::Div,
            HirBinaryOp::Mod => OpCode::Mod,
            HirBinaryOp::Eq => OpCode::Equal,
            HirBinaryOp::Neq => OpCode::NotEqual,
            HirBinaryOp::Lt => OpCode::Less,
            HirBinaryOp::Gt => OpCode::Greater,
            HirBinaryOp::Le => OpCode::LessEqual,
            HirBinaryOp::Ge => OpCode::GreaterEqual,
            HirBinaryOp::And => OpCode::Return, // logical And uses short-circuit jump
            HirBinaryOp::Or => OpCode::Return,  // logical Or uses short-circuit jump
        }
    }
}

/// HIR 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnaryOp {
    Neg,
    Not,
    CastInt,
    CastFloat,
    CastString,
    CastBool,
}

/// 常量值
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Int(i32),
    Float(f64),
    Bool(bool),
    String(String),
    Null,
}

impl ConstantValue {
    pub fn is_zero(&self) -> bool {
        match self {
            ConstantValue::Int(0) | ConstantValue::Float(0.0) => true,
            _ => false,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            ConstantValue::Int(1) | ConstantValue::Float(1.0) => true,
            _ => false,
        }
    }
}

/// Struct 信息
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub shape_id: u16,
    pub field_names: Vec<String>,
    pub field_types: Vec<String>,
}

impl HirModule {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            struct_infos: Vec::new(),
        }
    }

    /// 添加常量并返回索引
    pub fn add_constant(&mut self, val: ConstantValue) -> usize {
        let idx = self.constants.len();
        self.constants.push(val);
        idx
    }
}

impl Default for HirModule {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for HirOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirOperand::Temp(n) => write!(f, "%{n}"),
            HirOperand::Local(n) => write!(f, "${n}"),
            HirOperand::Const(n) => write!(f, "#{n}"),
            HirOperand::Global(n) => write!(f, "@{n}"),
            HirOperand::Immediate(v) => match v {
                ConstantValue::Int(n) => write!(f, "{n}"),
                ConstantValue::Float(n) => write!(f, "{n}"),
                ConstantValue::Bool(b) => write!(f, "{b}"),
                ConstantValue::String(s) => write!(f, "\"{s}\""),
                ConstantValue::Null => write!(f, "null"),
            },
            HirOperand::None => write!(f, "_"),
        }
    }
}
