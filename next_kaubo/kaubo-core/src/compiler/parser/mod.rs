pub mod error;
pub mod expr;
mod module;
pub mod parser;
pub mod stmt;
mod utils;
pub mod type_expr;
pub mod type_checker;

// 重新导出常用类型
pub use error::{ParserError, ParserErrorKind, ErrorLocation, ParseResult};
pub use expr::{Expr, ExprKind, Binary, Unary, LiteralInt};
pub use stmt::{Stmt, StmtKind, PrintStmt};
pub use module::{Module, ModuleKind};
pub use type_expr::{TypeExpr, NamedType, FunctionType};
pub use type_checker::{TypeChecker, TypeEnv, TypeError};
