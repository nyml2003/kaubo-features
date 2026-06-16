//! AST 定义 — Kaubo v2
//!
//! 规范: https://github.com/kaubo-lang/kaubo

/// 模块
#[derive(Debug, Clone)]
pub struct Module {
    pub stmts: Vec<Stmt>,
}

/// 语句 (模块级声明)
#[derive(Debug, Clone)]
pub enum Stmt {
    ConstDecl { name: String, ty_ann: Option<TypeExpr>, value: Expr },
    VarDecl { name: String, ty_ann: Option<TypeExpr>, value: Option<Expr> },
    StructDef { name: String, fields: Vec<FieldDef> },
    ImplBlock { struct_name: String, methods: Vec<MethodDef> },
    ExportStmt(Box<Stmt>),
    Import { path: String, alias: Option<String>, names: Vec<String> },
    ExprStmt(Expr),
}

/// 表达式
#[derive(Debug, Clone)]
pub enum Expr {
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitTrue,
    LitFalse,
    LitNull,

    VarRef(String),
    Lambda { params: Vec<Param>, ret_ty: Option<TypeExpr>, body: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Unary { op: UnOp, right: Box<Expr> },
    Block(Vec<Stmt>),

    If { cond: Box<Expr>, then_branch: Box<Expr>, else_branch: Option<Box<Expr>> },
    While { cond: Box<Expr>, body: Box<Expr> },
    For { var: Param, iterable: Box<Expr>, body: Box<Expr> },
    Break,
    Continue,
    Return(Option<Box<Expr>>),

    Member { object: Box<Expr>, field: String },
    Index { object: Box<Expr>, index: Box<Expr> },

    StructLit { name: String, fields: Vec<(String, Expr)> },
    ListLit(Vec<Expr>),

    Assign { target: Box<Expr>, value: Box<Expr> },
    Async(Box<Expr>),
    Await(Box<Expr>),
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Pipe,       // |>
    GtGt,       // >>
    SAdd,       // 字符串拼接 (编译期确定)
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

/// 函数参数
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty_ann: Option<TypeExpr>,
}

/// 结构体字段定义
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeExpr,
}

/// 方法定义
#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub body: Expr, // Lambda
}

/// 类型表达式
#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String),           // Int64, Float64, String, Bool, Null
    List(Box<TypeExpr>),     // List<Int64>
    Arrow { params: Vec<TypeExpr>, ret: Box<TypeExpr> },
}

impl TypeExpr {
    pub fn named(name: &str) -> Self { Self::Named(name.to_string()) }
    pub fn to_string(&self) -> String {
        match self {
            Self::Named(n) => n.clone(),
            Self::List(t) => format!("List<{}>", t.to_string()),
            Self::Arrow { params, ret } => format!("|{}| -> {}", 
                params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                ret.to_string()),
        }
    }
}
