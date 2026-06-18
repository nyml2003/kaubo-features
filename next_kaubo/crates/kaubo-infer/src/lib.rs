//! kaubo-infer — Hindley-Milner 类型推断 (Algorithm W)
//!
//! v2.0 支持: Int64, Float64, String, Bool, Null, Arrow, Record, List
//! let-多态: const 绑密 generalize, var 绑密单态
//! v2.1 收尾: ADT/Variant, match pattern 类型检查

pub mod infer;
pub mod types;

pub use infer::*;
pub use types::*;
