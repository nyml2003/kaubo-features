//! Stages - 流水线阶段组件
//!
//! 这个模块提供可以被 Orchestrator 编排的 Pass 组件。
//! 每个组件都实现了 `Pass` trait，定义了输入/输出格式。

mod compiler;
mod multi_module;
mod vm;

pub use compiler::{CodeGenPass, CompilePass, ParserPass};
pub use multi_module::MultiModulePass;
pub use vm::VmExecutionPass;
