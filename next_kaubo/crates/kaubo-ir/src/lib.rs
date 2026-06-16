//! kaubo-ir — v2 intermediate representation
//!
//! CPS blocks, types, and AST→CPS lowering

/// CPS (Continuation-Passing Style) blocks and instructions
pub mod cps;

/// AST → CPS lowering compiler
pub mod lowering;
