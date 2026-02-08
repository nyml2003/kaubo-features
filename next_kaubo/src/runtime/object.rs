//! 运行时对象定义

use crate::runtime::bytecode::chunk::Chunk;

/// 函数对象
#[derive(Debug)]
pub struct ObjFunction {
    /// 函数的字节码
    pub chunk: Chunk,
    /// 参数数量
    pub arity: u8,
    /// 函数名（用于调试）
    pub name: Option<String>,
}

impl ObjFunction {
    /// 创建新的函数对象
    pub fn new(chunk: Chunk, arity: u8, name: Option<String>) -> Self {
        Self { chunk, arity, name }
    }
}

/// 字符串对象
#[derive(Debug)]
pub struct ObjString {
    /// 字符串内容
    pub chars: String,
}

impl ObjString {
    /// 创建新的字符串对象
    pub fn new(chars: String) -> Self {
        Self { chars }
    }
}
