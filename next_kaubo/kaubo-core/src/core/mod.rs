//! Core 模块 - Kaubo 运行时核心类型定义
//!
//! 本模块包含所有核心类型的纯定义，不依赖实现细节。
//! 这是三层架构的中间层：
//! - Core 层：类型定义（本模块）
//! - Runtime 层：实现逻辑（runtime/ 目录）
//! - API 层：对外接口（kaubo-api）

// ==================== 基础类型 ====================

/// 值类型（NaN-boxed）
pub mod value;
pub use value::Value;

// ==================== 运算符 ====================

/// 运算符定义
pub mod operators;
pub use operators::{InlineCacheEntry, Operator};

// ==================== 字节码 ====================

/// 字节码定义
pub mod bytecode;
pub use bytecode::{
    InlineCacheSlot, MethodTableEntry, OpCode, OperatorTableEntry,
};

/// 字节码块
pub mod chunk;
pub use chunk::Chunk;

// ==================== 对象类型 ====================

/// 对象定义
pub mod object;
pub use object::{
    CallFrame, CoroutineState, IteratorSource, NativeFn, NativeVmFn, ObjClosure,
    ObjCoroutine, ObjFunction, ObjIterator, ObjJson, ObjList, ObjModule, ObjNative,
    ObjNativeVm, ObjOption, ObjResult, ObjShape, ObjString, ObjStruct, ObjUpvalue,
    OptionVariant, ResultVariant,
};

// ==================== 虚拟机 ====================

/// VM 定义
pub mod vm;
pub use vm::{InterpretResult, VMConfig, VM};

// ==================== 错误 ====================

/// 错误类型
pub mod error;
pub use error::RuntimeError;
