//! Kaubo AST contract types.
//!
//! This crate owns syntax tree data structures shared by parser, infer, IR,
//! and adapters. It does not parse or infer on its own.

#[derive(Debug, Clone)]
pub struct Module {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    ConstDecl {
        name: String,
        ty_ann: Option<TypeExpr>,
        value: Expr,
    },
    VarDecl {
        name: String,
        ty_ann: Option<TypeExpr>,
        value: Option<Expr>,
    },
    StructDef {
        name: String,
        fields: Vec<FieldDef>,
    },
    ImplBlock {
        struct_name: String,
        methods: Vec<MethodDef>,
    },
    ExportStmt(Box<Stmt>),
    Import {
        path: String,
        alias: Option<String>,
        names: Vec<String>,
    },
    ExprStmt(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitTrue,
    LitFalse,
    LitNull,

    VarRef(String),
    Lambda {
        params: Vec<Param>,
        ret_ty: Option<TypeExpr>,
        body: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        right: Box<Expr>,
    },
    Block(Vec<Stmt>),

    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    For {
        var: Param,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },
    Break,
    Continue,
    Return(Option<Box<Expr>>),

    Member {
        object: Box<Expr>,
        field: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    ListLit(Vec<Expr>),

    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Async(Box<Expr>),
    Await(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Pipe,
    GtGt,
    SAdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty_ann: Option<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String),
    List(Box<TypeExpr>),
    Arrow {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
}

impl TypeExpr {
    pub fn named(name: &str) -> Self {
        Self::Named(name.to_string())
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Named(n) => n.clone(),
            Self::List(t) => format!("List<{}>", t.to_string()),
            Self::Arrow { params, ret } => format!(
                "|{}| -> {}",
                params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.to_string()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_expr_to_string_handles_nesting() {
        let ty = TypeExpr::Arrow {
            params: vec![
                TypeExpr::named("Int64"),
                TypeExpr::List(Box::new(TypeExpr::named("String"))),
            ],
            ret: Box::new(TypeExpr::named("Bool")),
        };

        assert_eq!(ty.to_string(), "|Int64, List<String>| -> Bool");
    }

    #[test]
    fn module_keeps_statement_shape() {
        let module = Module {
            stmts: vec![
                Stmt::ConstDecl {
                    name: "answer".to_string(),
                    ty_ann: Some(TypeExpr::named("Int64")),
                    value: Expr::LitInt(42),
                },
                Stmt::ExprStmt(Expr::VarRef("answer".to_string())),
            ],
        };

        assert_eq!(module.stmts.len(), 2);

        match &module.stmts[0] {
            Stmt::ConstDecl {
                name,
                ty_ann,
                value,
            } => {
                assert_eq!(name, "answer");
                assert!(matches!(ty_ann, Some(TypeExpr::Named(n)) if n == "Int64"));
                assert!(matches!(value, Expr::LitInt(42)));
            }
            other => panic!("unexpected first stmt: {other:?}"),
        }

        match &module.stmts[1] {
            Stmt::ExprStmt(Expr::VarRef(name)) => assert_eq!(name, "answer"),
            other => panic!("unexpected second stmt: {other:?}"),
        }
    }
}
