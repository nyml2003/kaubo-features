//! kaubo-infer — Hindley-Milner 类型推断 (Algorithm W)

#![allow(clippy::type_complexity, clippy::only_used_in_recursion)]

pub mod infer;
pub mod types;

pub use infer::*;
pub use types::*;
