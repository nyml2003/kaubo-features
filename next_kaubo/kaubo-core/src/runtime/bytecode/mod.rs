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
    LoadConst,     // 0x10 + u8 索引
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
    // 带内联缓存索引的运算符指令
    // 操作数: u8 cache_idx (0xFF 表示不使用缓存)
    Add = 0x60,  // + u8 cache_idx
    Sub,         // + u8 cache_idx
    Mul,         // + u8 cache_idx
    Div,         // + u8 cache_idx
    Mod,         // + u8 cache_idx (取模/求余)

    Neg = 0x68, // 一元取负

    // ===== 比较运算 (0x70-0x77) =====
    // 带内联缓存索引的比较指令
    Equal = 0x70,      // + u8 cache_idx (或不带，视实现而定)
    NotEqual,          // 不带缓存
    Greater,           // + u8 cache_idx
    GreaterEqual,      // + u8 cache_idx
    Less,              // + u8 cache_idx
    LessEqual,         // + u8 cache_idx

    // ===== 逻辑运算 (0x78-0x7B) =====
    Not = 0x78,

    // ===== 控制流 (0x80-0x8F) =====
    Jump = 0x80, // + i16 偏移
    JumpIfFalse, // + i16 偏移
    JumpBack,    // + i16 偏移 (负向跳转)

    // ===== 函数 (0x90-0x9F) =====
    Call = 0x90,          // + u8 参数个数
    Closure = 0x91,       // 创建闭包对象
    GetUpvalue = 0x92,    // + u8 读取 upvalue
    SetUpvalue = 0x93,    // + u8 设置 upvalue
    CloseUpvalues = 0x94, // + u8 关闭指定槽位以上的所有 upvalue
    Return,
    ReturnValue,

    // ===== 协程 (0x98-0x9F) =====
    CreateCoroutine = 0x98, // 创建协程 (操作数: 函数常量索引)
    Resume = 0x99,          // 恢复协程执行 (操作数: 传入值个数)
    Yield = 0x9A,           // 挂起协程并返回值
    CoroutineStatus = 0x9B, // 获取协程状态 (0=Suspended, 1=Running, 2=Dead)

    // ===== 列表 (0xB0-0xBF) =====
    BuildList = 0xB0, // + u8 元素个数
    IndexGet,         // 列表索引读取
    IndexSet,         // 列表索引赋值
    GetIter,          // 获取迭代器
    IterNext,         // 获取迭代器下一个值，null 表示结束

    // ===== JSON (0xC0-0xCF) =====
    BuildJson = 0xC0, // + u8 键值对个数，键和值从栈弹出
    JsonGet,          // JSON 字符串键获取: 栈顶[key, json] → value
    JsonSet,          // JSON 字符串键设置: 栈顶[value, key, json] → null

    // ===== 模块 (0xD0-0xD7) =====
    BuildModule = 0xD0, // + u8 导出项个数，值从栈弹出，创建模块对象
    ModuleGet,          // + u16 ShapeID，从模块获取字段（编译期确定）
    GetModuleExport,    // 从模块动态获取导出项：栈顶[module, name] -> value
    GetModule,          // 根据模块名获取模块对象：栈顶[name] -> module
    // ModuleSet 预留（未来支持模块字段可变性）

    // ===== Struct (0xD8-0xDF) =====
    BuildStruct = 0xD8, // + u16 shape_id + u8 field_count，从栈弹出字段值，创建 struct
    GetField,           // + u8 字段索引，栈顶[struct] -> field_value
    SetField,           // + u8 字段索引，栈顶[value, struct] -> null
    LoadMethod,         // + u8 方法索引，栈顶[struct] -> [struct, method]

    // ===== 类型转换 (0xE0-0xE3) =====
    CastToInt = 0xE0,    // 栈顶[value] -> int
    CastToFloat,         // 栈顶[value] -> float
    CastToString,        // 栈顶[value] -> string
    CastToBool,          // 栈顶[value] -> bool

    // ===== 调试 (0xF0-0xFF) =====
    Print = 0xF0, // 调试用
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
            OpCode::Mod => "MOD",
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
            OpCode::CloseUpvalues => "CLOSE_UPVALUES",
            OpCode::Return => "RETURN",
            OpCode::ReturnValue => "RETURN_VALUE",
            OpCode::CreateCoroutine => "CREATE_COROUTINE",
            OpCode::Resume => "RESUME",
            OpCode::Yield => "YIELD",
            OpCode::CoroutineStatus => "COROUTINE_STATUS",
            OpCode::BuildList => "BUILD_LIST",
            OpCode::BuildJson => "BUILD_JSON",
            OpCode::BuildModule => "BUILD_MODULE",
            OpCode::ModuleGet => "MODULE_GET",
            OpCode::GetModuleExport => "GET_MODULE_EXPORT",
            OpCode::GetModule => "GET_MODULE",
            OpCode::JsonGet => "JSON_GET",
            OpCode::JsonSet => "JSON_SET",
            OpCode::IndexGet => "INDEX_GET",
            OpCode::IndexSet => "INDEX_SET",
            OpCode::GetIter => "GET_ITER",
            OpCode::IterNext => "ITER_NEXT",
            OpCode::BuildStruct => "BUILD_STRUCT",
            OpCode::GetField => "GET_FIELD",
            OpCode::SetField => "SET_FIELD",
            OpCode::LoadMethod => "LOAD_METHOD",

            OpCode::CastToInt => "CAST_TO_INT",
            OpCode::CastToFloat => "CAST_TO_FLOAT",
            OpCode::CastToString => "CAST_TO_STRING",
            OpCode::CastToBool => "CAST_TO_BOOL",

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
            | OpCode::Neg
            | OpCode::NotEqual
            | OpCode::Not
            | OpCode::Return
            | OpCode::ReturnValue
            | OpCode::IndexGet
            | OpCode::IndexSet
            | OpCode::JsonGet
            | OpCode::JsonSet
            | OpCode::GetIter
            | OpCode::IterNext
            | OpCode::Yield
            | OpCode::Print
            | OpCode::CastToInt
            | OpCode::CastToFloat
            | OpCode::CastToString
            | OpCode::CastToBool
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
            | OpCode::CloseUpvalues
            | OpCode::CreateCoroutine
            | OpCode::Resume
            | OpCode::CoroutineStatus
            | OpCode::BuildList
            | OpCode::BuildJson
            | OpCode::BuildModule => 1,

            // u8 操作数（常量池索引）
            OpCode::GetModuleExport => 1,

            // u8 操作数（Struct 相关）
            OpCode::GetField | OpCode::SetField | OpCode::LoadMethod => 1,

            // u8 操作数（内联缓存索引）
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod
            | OpCode::Greater | OpCode::GreaterEqual | OpCode::Less | OpCode::LessEqual
            | OpCode::Equal => 1,

            // u16 + u8 操作数（BuildStruct）
            OpCode::BuildStruct => 3,

            // 无操作数（运行时从栈获取参数）
            OpCode::GetModule => 0,

            // u16/i16 操作数
            OpCode::LoadConstWide => 2,
            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpBack => 2,
            OpCode::ModuleGet => 2,
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
        assert_eq!(OpCode::Add.operand_size(), 1);  // cache_idx
        assert_eq!(OpCode::LoadConst.operand_size(), 1);
        assert_eq!(OpCode::Jump.operand_size(), 2);
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(OpCode::from(0x60), OpCode::Add);
        assert_eq!(OpCode::from(0x00), OpCode::LoadConst0);
    }
}
