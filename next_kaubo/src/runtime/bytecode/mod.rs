//! 字节码定义

pub mod chunk;

/// 操作码定义
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    // ===== 常量加载 (0x00-0x1F) =====
    LoadConst0 = 0x00,
    LoadConst1,
    LoadConst2,
    LoadConst3,
    LoadConst4,
    LoadConst5,
    LoadConst6,
    LoadConst7,
    LoadConst8,
    LoadConst9,
    LoadConst10,
    LoadConst11,
    LoadConst12,
    LoadConst13,
    LoadConst14,
    LoadConst15,
    LoadConst,    // 0x10 + u8 索引
    LoadConstWide, // 0x11 + u16 索引

    LoadNull = 0x18,
    LoadTrue,
    LoadFalse,
    LoadZero, // SMI 0 优化
    LoadOne,  // SMI 1 优化

    // ===== 栈操作 (0x20-0x2F) =====
    Pop = 0x20,
    Dup,
    Swap,

    // ===== 局部变量 (0x30-0x47) =====
    LoadLocal0 = 0x30,
    LoadLocal1,
    LoadLocal2,
    LoadLocal3,
    LoadLocal4,
    LoadLocal5,
    LoadLocal6,
    LoadLocal7,
    LoadLocal, // 0x38 + u8 索引

    StoreLocal0 = 0x40,
    StoreLocal1,
    StoreLocal2,
    StoreLocal3,
    StoreLocal4,
    StoreLocal5,
    StoreLocal6,
    StoreLocal7,
    StoreLocal, // 0x48 + u8 索引

    // ===== 全局变量 (0x50-0x57) =====
    LoadGlobal = 0x50, // + u8 索引
    StoreGlobal,       // + u8 索引
    DefineGlobal,      // + u8 索引

    // ===== 算术运算 (0x60-0x6F) =====
    Add = 0x60,
    Sub,
    Mul,
    Div,

    Neg = 0x68, // 一元取负

    // ===== 比较运算 (0x70-0x77) =====
    Equal = 0x70,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // ===== 逻辑运算 (0x78-0x7B) =====
    Not = 0x78,

    // ===== 控制流 (0x80-0x8F) =====
    Jump = 0x80,          // + i16 偏移
    JumpIfFalse,          // + i16 偏移
    JumpBack,             // + i16 偏移 (负向跳转)

    // ===== 函数 (0x90-0x9F) =====
    Call = 0x90,          // + u8 参数个数
    Closure = 0x91,        // 创建闭包对象
    GetUpvalue = 0x92,     // 读取 upvalue（预留）
    SetUpvalue = 0x93,     // 设置 upvalue（预留）
    Return,
    ReturnValue,

    // ===== 列表 (0xB0-0xBF) =====
    BuildList = 0xB0,     // + u8 元素个数
    IndexGet,             // 列表索引读取
    GetIter,              // 获取迭代器
    IterNext,             // 获取迭代器下一个值，null 表示结束

    // ===== 调试 (0xF0-0xFF) =====
    Print = 0xF0,         // 调试用
    Invalid = 0xFF,
}

impl OpCode {
    /// 获取操作码名称
    pub fn name(&self) -> &'static str {
        match self {
            OpCode::LoadConst0 => "LOAD_CONST_0",
            OpCode::LoadConst1 => "LOAD_CONST_1",
            OpCode::LoadConst2 => "LOAD_CONST_2",
            OpCode::LoadConst3 => "LOAD_CONST_3",
            OpCode::LoadConst4 => "LOAD_CONST_4",
            OpCode::LoadConst5 => "LOAD_CONST_5",
            OpCode::LoadConst6 => "LOAD_CONST_6",
            OpCode::LoadConst7 => "LOAD_CONST_7",
            OpCode::LoadConst8 => "LOAD_CONST_8",
            OpCode::LoadConst9 => "LOAD_CONST_9",
            OpCode::LoadConst10 => "LOAD_CONST_10",
            OpCode::LoadConst11 => "LOAD_CONST_11",
            OpCode::LoadConst12 => "LOAD_CONST_12",
            OpCode::LoadConst13 => "LOAD_CONST_13",
            OpCode::LoadConst14 => "LOAD_CONST_14",
            OpCode::LoadConst15 => "LOAD_CONST_15",
            OpCode::LoadConst => "LOAD_CONST",
            OpCode::LoadConstWide => "LOAD_CONST_WIDE",
            OpCode::LoadNull => "LOAD_NULL",
            OpCode::LoadTrue => "LOAD_TRUE",
            OpCode::LoadFalse => "LOAD_FALSE",
            OpCode::LoadZero => "LOAD_ZERO",
            OpCode::LoadOne => "LOAD_ONE",
            OpCode::Pop => "POP",
            OpCode::Dup => "DUP",
            OpCode::Swap => "SWAP",
            OpCode::LoadLocal0 => "LOAD_LOCAL_0",
            OpCode::LoadLocal1 => "LOAD_LOCAL_1",
            OpCode::LoadLocal2 => "LOAD_LOCAL_2",
            OpCode::LoadLocal3 => "LOAD_LOCAL_3",
            OpCode::LoadLocal4 => "LOAD_LOCAL_4",
            OpCode::LoadLocal5 => "LOAD_LOCAL_5",
            OpCode::LoadLocal6 => "LOAD_LOCAL_6",
            OpCode::LoadLocal7 => "LOAD_LOCAL_7",
            OpCode::LoadLocal => "LOAD_LOCAL",
            OpCode::StoreLocal0 => "STORE_LOCAL_0",
            OpCode::StoreLocal1 => "STORE_LOCAL_1",
            OpCode::StoreLocal2 => "STORE_LOCAL_2",
            OpCode::StoreLocal3 => "STORE_LOCAL_3",
            OpCode::StoreLocal4 => "STORE_LOCAL_4",
            OpCode::StoreLocal5 => "STORE_LOCAL_5",
            OpCode::StoreLocal6 => "STORE_LOCAL_6",
            OpCode::StoreLocal7 => "STORE_LOCAL_7",
            OpCode::StoreLocal => "STORE_LOCAL",
            OpCode::LoadGlobal => "LOAD_GLOBAL",
            OpCode::StoreGlobal => "STORE_GLOBAL",
            OpCode::DefineGlobal => "DEFINE_GLOBAL",
            OpCode::Add => "ADD",
            OpCode::Sub => "SUB",
            OpCode::Mul => "MUL",
            OpCode::Div => "DIV",
            OpCode::Neg => "NEG",
            OpCode::Equal => "EQUAL",
            OpCode::NotEqual => "NOT_EQUAL",
            OpCode::Greater => "GREATER",
            OpCode::GreaterEqual => "GREATER_EQUAL",
            OpCode::Less => "LESS",
            OpCode::LessEqual => "LESS_EQUAL",
            OpCode::Not => "NOT",
            OpCode::Jump => "JUMP",
            OpCode::JumpIfFalse => "JUMP_IF_FALSE",
            OpCode::JumpBack => "JUMP_BACK",
            OpCode::Call => "CALL",
            OpCode::Closure => "CLOSURE",
            OpCode::GetUpvalue => "GET_UPVALUE",
            OpCode::SetUpvalue => "SET_UPVALUE",
            OpCode::Return => "RETURN",
            OpCode::ReturnValue => "RETURN_VALUE",
            OpCode::BuildList => "BUILD_LIST",
            OpCode::IndexGet => "INDEX_GET",
            OpCode::GetIter => "GET_ITER",
            OpCode::IterNext => "ITER_NEXT",
            OpCode::Print => "PRINT",
            OpCode::Invalid => "INVALID",
        }
    }

    /// 操作数大小 (bytes)
    pub fn operand_size(&self) -> usize {
        match self {
            // 无操作数
            OpCode::LoadConst0
            | OpCode::LoadConst1
            | OpCode::LoadConst2
            | OpCode::LoadConst3
            | OpCode::LoadConst4
            | OpCode::LoadConst5
            | OpCode::LoadConst6
            | OpCode::LoadConst7
            | OpCode::LoadConst8
            | OpCode::LoadConst9
            | OpCode::LoadConst10
            | OpCode::LoadConst11
            | OpCode::LoadConst12
            | OpCode::LoadConst13
            | OpCode::LoadConst14
            | OpCode::LoadConst15
            | OpCode::LoadNull
            | OpCode::LoadTrue
            | OpCode::LoadFalse
            | OpCode::LoadZero
            | OpCode::LoadOne
            | OpCode::Pop
            | OpCode::Dup
            | OpCode::Swap
            | OpCode::LoadLocal0
            | OpCode::LoadLocal1
            | OpCode::LoadLocal2
            | OpCode::LoadLocal3
            | OpCode::LoadLocal4
            | OpCode::LoadLocal5
            | OpCode::LoadLocal6
            | OpCode::LoadLocal7
            | OpCode::StoreLocal0
            | OpCode::StoreLocal1
            | OpCode::StoreLocal2
            | OpCode::StoreLocal3
            | OpCode::StoreLocal4
            | OpCode::StoreLocal5
            | OpCode::StoreLocal6
            | OpCode::StoreLocal7
            | OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Neg
            | OpCode::Equal
            | OpCode::NotEqual
            | OpCode::Greater
            | OpCode::GreaterEqual
            | OpCode::Less
            | OpCode::LessEqual
            | OpCode::Not
            | OpCode::Return
            | OpCode::ReturnValue
            | OpCode::IndexGet
            | OpCode::GetIter
            | OpCode::IterNext
            | OpCode::Print
            | OpCode::Invalid => 0,

            // u8 操作数
            OpCode::LoadConst
            | OpCode::LoadLocal
            | OpCode::StoreLocal
            | OpCode::LoadGlobal
            | OpCode::StoreGlobal
            | OpCode::DefineGlobal
            | OpCode::Call
            | OpCode::Closure
            | OpCode::GetUpvalue
            | OpCode::SetUpvalue
            | OpCode::BuildList => 1,

            // u16/i16 操作数
            OpCode::LoadConstWide => 2,
            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpBack => 2,
        }
    }
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        // SAFETY: 所有 0-255 都映射到某个枚举值
        // 无效值会映射到未定义的行为，但我们在 VM 中处理 Invalid
        unsafe { std::mem::transmute(value) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_name() {
        assert_eq!(OpCode::Add.name(), "ADD");
        assert_eq!(OpCode::LoadConst0.name(), "LOAD_CONST_0");
        assert_eq!(OpCode::Return.name(), "RETURN");
    }

    #[test]
    fn test_operand_size() {
        assert_eq!(OpCode::Add.operand_size(), 0);
        assert_eq!(OpCode::LoadConst.operand_size(), 1);
        assert_eq!(OpCode::Jump.operand_size(), 2);
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(OpCode::from(0x60), OpCode::Add);
        assert_eq!(OpCode::from(0x00), OpCode::LoadConst0);
    }
}
