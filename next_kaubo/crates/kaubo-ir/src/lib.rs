//! kaubo-ir — v2 intermediate representation
//!
//! CPS blocks, types, AST→CPS build, flattening, optimization passes

pub mod cps;
pub mod flatten;
pub mod cps_emit;
pub mod cps_build;
pub mod pass;
