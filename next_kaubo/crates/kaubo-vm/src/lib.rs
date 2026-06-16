//! kaubo-vm — 寄存器 VM (CPS block scheduler)
//!
//! 44 opcodes, 零栈操作, 零控制流 opcode
//! 分层寄存器: int_regs / float_regs / ptr_regs
//! 引用计数 GC

pub mod regfile;
pub mod execute;
pub mod async_runtime;
pub mod stdlib;

pub use regfile::*;
pub use execute::*;
pub use async_runtime::*;
