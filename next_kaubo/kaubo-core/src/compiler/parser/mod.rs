pub mod error;
pub mod expr;
mod module;
pub mod parser;
pub mod stmt;
pub mod type_checker;
pub mod type_expr;
mod utils;

// 重新导出常用类型
pub use error::{ErrorLocation, ParseResult, ParserError, ParserErrorKind};
pub use expr::{Binary, Expr, ExprKind, LiteralInt, MemberAccess, StructLiteral, Unary};
pub use module::{Module, ModuleKind};
pub use stmt::{FieldDef, PrintStmt, Stmt, StmtKind, StructStmt};
pub use type_checker::{TypeChecker, TypeEnv, TypeError};
pub use type_expr::{FunctionType, NamedType, TypeExpr};
