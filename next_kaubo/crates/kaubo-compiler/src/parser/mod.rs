pub mod error;
pub mod expr;
pub mod module;
pub mod parser;
pub mod stmt;
pub mod type_checker;
pub mod type_expr;
pub mod utils;

// 重新导出常用类型
pub use error::{ErrorLocation, ParseResult, ParserError, ParserErrorKind, unexpected_token};
pub use expr::{
    AsExpr, Binary, Expr, ExprKind, FunctionCall, Grouping, IndexAccess, JsonLiteral, Lambda,
    LiteralFalse, LiteralFloat, LiteralInt, LiteralList, LiteralNull, LiteralString, LiteralTrue,
    MemberAccess, StructLiteral, Unary, VarRef, YieldExpr,
};
pub use module::{Module, ModuleKind};
pub use parser::Parser;
pub use stmt::{
    BlockStmt, BreakStmt, ContinueStmt, EmptyStmt, ExprStmt, FieldDef, ForStmt, IfStmt, ImplStmt,
    ImportStmt, MethodDef, PassStmt, PrintStmt, ReturnStmt, Stmt, StmtKind, StructStmt,
    VarDeclStmt, WhileStmt,
};
pub use type_checker::{TypeChecker, TypeEnv, TypeError, TypeCheckResult};
pub use type_expr::{FunctionType, NamedType, TypeExpr};

// Span type
pub use crate::lexer::types::Span;
