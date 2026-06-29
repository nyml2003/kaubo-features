pub mod ast;
pub mod cps;
pub mod linked_cps;
pub mod module_graph;
pub mod per_module_cps;
pub mod semantic;

use kaubo_dag::Kind;

/// Kind constants reused across fetchers.
pub const KIND_SOURCE: &str = Kind::SOURCE;
pub const KIND_AST: &str = Kind::AST;
pub const KIND_SEMANTIC: &str = Kind::SEMANTIC;
pub const KIND_CPS: &str = Kind::CPS;
pub const KIND_MODULE_GRAPH: &str = Kind::MODULE_GRAPH;
pub const KIND_LINKED_CPS: &str = Kind::LINKED_CPS;
