//! Kaubo 运行时
//!
//! 字节码虚拟机实现

pub mod bytecode;
pub mod compiler;
pub mod object;
pub mod stdlib;
pub mod value;
pub mod vm;

pub use compiler::{compile, CompileError, Compiler};
pub use value::Value;
pub use vm::{InterpretResult, VM};
