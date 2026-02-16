//! 字节码定义（已移至 core 层）
//!
//! 本模块保留以兼容旧代码，所有类型定义已移至 `crate::core`。

pub use crate::core::{Chunk, OpCode, MethodTableEntry, OperatorTableEntry, InlineCacheSlot};
