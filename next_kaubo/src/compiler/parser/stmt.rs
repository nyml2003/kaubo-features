use super::expr::{Expr}; // 引用之前定义的Expr类型
use std::fmt;

// 语句类型别名（对应C++的StmtPtr）
pub type Stmt = Box<StmtKind>;

/// 解析器语句枚举（对应C++的Stmt::ValueType变体集合）
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    // 表达式语句（如 `a + b;`）
    Expr(ExprStmt),
    // 空语句（如单独的 `;`）
    Empty(EmptyStmt),
    // 代码块语句（由{}包裹的语句列表）
    Block(BlockStmt),
    // 变量声明语句（如 `var x = 5;`）
    VarDecl(VarDeclStmt),
    // If条件语句
    If(IfStmt),
    // While循环语句
    While(WhileStmt),
    // For循环语句
    For(ForStmt),
    // Return返回语句
    Return(ReturnStmt),
}

// 表达式语句结构体（包装一个表达式）
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expression: Expr, // 对应C++的ExprPtr
}

// 空语句结构体（无实际数据）
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmptyStmt;

// 代码块语句结构体（包含多个语句）
#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>, // 对应C++的std::vector<StmtPtr>
}

// 变量声明语句结构体
#[derive(Debug, Clone, PartialEq)]
pub struct VarDeclStmt {
    pub name: String,      // 变量名
    pub initializer: Expr, // 初始化表达式（对应C++的ExprPtr）
}

// If语句结构体
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub if_condition: Expr,         // if条件表达式（对应if_condition）
    pub elif_conditions: Vec<Expr>, // elif条件列表（对应elif_conditions）
    pub elif_bodies: Vec<Stmt>,     // elif代码块列表（对应elif_bodies）
    pub else_body: Option<Stmt>,    // else代码块（可能为空，用Option表示）
    pub then_body: Stmt,            // if条件满足时的代码块（对应then_body）
}

// While循环语句结构体
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr, // 循环条件表达式（对应condition）
    pub body: Stmt,      // 循环体（对应body）
}

// For循环语句结构体
#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub iterator: Expr, // 迭代变量表达式（对应iterator）
    pub iterable: Expr, // 可迭代对象表达式（对应iterable）
    pub body: Stmt,     // 循环体（对应body）
}

// Return返回语句结构体
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>, // 返回值表达式（可能为空，如return;，用Option表示）
}

// 实现Display trait（可选，用于调试输出）
impl fmt::Display for StmtKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StmtKind::Expr(expr_stmt) => write!(f, "{};", expr_stmt.expression),
            StmtKind::Empty(_) => write!(f, ";"),
            StmtKind::Block(block) => {
                let stmts = block
                    .statements
                    .iter()
                    .map(|s| format!("  {}", s))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{{\n{}\n}}", stmts)
            }
            StmtKind::VarDecl(var_decl) => {
                write!(f, "var {} = {};", var_decl.name, var_decl.initializer)
            }
            StmtKind::If(if_stmt) => {
                let mut s = format!("if ({}) {}", if_stmt.if_condition, if_stmt.then_body);
                // 拼接elif
                for (cond, body) in if_stmt
                    .elif_conditions
                    .iter()
                    .zip(if_stmt.elif_bodies.iter())
                {
                    s.push_str(&format!(" elif ({}) {}", cond, body));
                }
                // 拼接else
                if let Some(else_body) = &if_stmt.else_body {
                    s.push_str(&format!(" else {}", else_body));
                }
                write!(f, "{}", s)
            }
            StmtKind::While(while_stmt) => {
                write!(f, "while ({}) {}", while_stmt.condition, while_stmt.body)
            }
            StmtKind::For(for_stmt) => {
                write!(
                    f,
                    "for ({}) in ({}) {}",
                    for_stmt.iterator, for_stmt.iterable, for_stmt.body
                )
            }
            StmtKind::Return(ret_stmt) => {
                if let Some(value) = &ret_stmt.value {
                    write!(f, "return {};", value)
                } else {
                    write!(f, "return;")
                }
            }
        }
    }
}
