//! Module system for multi-file compilation
//!
//! Provides module resolution, compilation, and caching.

pub mod module_id;
pub mod compile_context;

pub use module_id::{ModuleId, ParseError};
pub use compile_context::{CompileContext, CompileUnit, CompileError};
