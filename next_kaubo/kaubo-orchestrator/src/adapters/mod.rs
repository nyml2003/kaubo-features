//! Adapters - 将外部功能包装为组件
//!
//! 这个模块提供从 kaubo-core 等外部 crate 到 orchestrator 组件系统的适配器。
//! 随着架构迁移，这些适配器将被原生实现取代。

mod core_adapter;
mod multi_module_pass;

pub use core_adapter::{CodeGenPass, CompilePass, ParserPass};
pub use multi_module_pass::MultiModulePass;
