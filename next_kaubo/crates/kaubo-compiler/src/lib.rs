//! kaubo-compiler — 编译器

pub mod lexer;
pub mod parser;
pub mod codegen;
pub mod ring_buffer;
pub mod stages;
pub mod hir;
pub mod module;

pub use stages::*;
