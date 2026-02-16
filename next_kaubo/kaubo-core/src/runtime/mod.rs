//! Kaubo 运行时 (Runtime 层)
//!
//! 字节码虚拟机实现。
//! 
//! 本模块为 core 层类型提供实现：
//! - `Value` 操作（as_string, is_list 等方法）
//! - `VM` 执行逻辑
//! - 运算符实现
//! - 标准库

// ==================== 核心类型（从 core 层重新导出）====================

pub use crate::core::{
    Chunk, InlineCacheEntry, InterpretResult, ObjClosure, ObjCoroutine, ObjFunction,
    ObjIterator, ObjJson, ObjList, ObjModule, ObjNative, ObjNativeVm, ObjOption, ObjResult,
    ObjShape, ObjString, ObjStruct, ObjUpvalue, OpCode, Operator, Value, VMConfig, VM,
};

// ==================== Runtime 实现模块 ====================

/// 字节码模块（重新导出以保持向后兼容）
pub mod bytecode {
    pub use crate::core::{Chunk, InlineCacheSlot, MethodTableEntry, OpCode, OperatorTableEntry};
}

/// 运算符实现
pub mod operators;

/// 标准库
pub mod stdlib;

/// 编译器接口
pub mod compiler;

/// VM 实现（包含执行逻辑）
pub mod vm;

// ==================== Value 扩展方法 ====================

/// 为 Value 类型提供扩展方法（as_string, is_list 等）
pub mod value_ext;

// ==================== 向后兼容导出 ====================

pub use compiler::{compile, CompileError, Compiler};
