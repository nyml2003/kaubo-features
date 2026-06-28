//! kaubo-syntax — v2 词法分析和语法分析

#![allow(clippy::unnecessary_parentheses)]

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;

// re-exports
pub use ast::{BinOp, Expr, FieldDef, MethodDef, Module, Param, Stmt, UnOp};
pub use lexer::Lexer;
pub use parser::Parser;
pub use token::TokenKind;
