//! Kaubo AST contract types.
//!
//! This crate owns syntax tree data structures shared by parser, infer, IR,
//! and adapters. It does not parse or infer on its own.

/// Source position (1-based line and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    /// Synthetic span (line=0, col=0) for compiler-generated code.
    pub const ZERO: Self = Self { line: 0, col: 0 };
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    ConstDecl {
        name: String,
        span: Span,
        ty_ann: Option<TypeExpr>,
        value: Expr,
    },
    VarDecl {
        name: String,
        span: Span,
        ty_ann: Option<TypeExpr>,
        value: Option<Expr>,
    },
    StructDef {
        name: String,
        span: Span,
        fields: Vec<FieldDef>,
    },
    EnumDef {
        name: String,
        span: Span,
        variants: Vec<VariantDef>,
    },
    ImplBlock {
        struct_name: String,
        span: Span,
        interface_name: Option<String>,
        methods: Vec<MethodDef>,
    },
    InterfaceDef {
        name: String,
        span: Span,
        methods: Vec<MethodSig>,
    },
    ExportStmt(Box<Stmt>),
    Import {
        path: String,
        alias: Option<String>,
        names: Vec<String>,
    },
    ExprStmt(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    LitInt(i64),
    LitFloat(f64),
    LitString(String),
    LitTrue,
    LitFalse,
    LitNull,

    VarRef { name: String, span: Span },
    Lambda {
        params: Vec<Param>,
        ret_ty: Option<TypeExpr>,
        body: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        arg: Box<Expr>,
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
        spread: Option<Box<Expr>>,
    },
    VariantLit {
        enum_name: String,
        variant_name: String,
        tag: u16,
        fields: Vec<Expr>,
    },
    ListLit(Vec<Expr>),
    Tuple(Vec<Expr>),
    GetVariantTag(Box<Expr>),
    GetVariantField {
        object: Box<Expr>,
        field_idx: u16,
    },

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

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub span: Span,
    pub ty_ann: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub span: Span,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantDef {
    pub name: String,
    pub span: Span,
    pub fields: Vec<FieldDef>, // empty = unit variant
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub name: String,
    pub span: Span,
    pub body: Expr,
    pub operator: bool,
}

/// Interface method signature (name + type, no body)
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub operator: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(String),
    List(Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),
    Arrow {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
}

impl std::fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Named(n) => write!(f, "{n}"),
            Self::List(t) => write!(f, "List<{t}>"),
            Self::Tuple(elements) => write!(
                f,
                "({})",
                elements
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Arrow { params, ret } => write!(
                f,
                "|{}| -> {ret}",
                params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl TypeExpr {
    pub fn named(name: &str) -> Self {
        Self::Named(name.to_string())
    }
}

impl Expr {
    /// 将表达式列表转为 Call 的单 arg：0 → Tuple([]), 1 → 直接取, 2+ → Tuple
    pub fn call_arg(args: Vec<Expr>) -> Box<Expr> {
        Box::new(match args.len() {
            0 => Expr::Tuple(vec![]),
            1 => args.into_iter().next().unwrap(),
            _ => Expr::Tuple(args),
        })
    }

    /// 从 Call 的 arg 中提取参数列表（模式匹配后使用）
    pub fn as_args(&self) -> Vec<&Expr> {
        match self {
            Expr::Tuple(elements) => elements.iter().collect(),
            other => vec![other],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Span = Span { line: 0, col: 0 };

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
                    span: S,
                    ty_ann: Some(TypeExpr::named("Int64")),
                    value: Expr::LitInt(42),
                },
                Stmt::ExprStmt(Expr::VarRef { name: "answer".to_string(), span: S }),
            ],
        };

        assert_eq!(module.stmts.len(), 2);

        match &module.stmts[0] {
            Stmt::ConstDecl {
                name,
                ty_ann,
                value,
                ..
            } => {
                assert_eq!(name, "answer");
                assert!(matches!(ty_ann, Some(TypeExpr::Named(n)) if n == "Int64"));
                assert!(matches!(value, Expr::LitInt(42)));
            }
            other => panic!("unexpected first stmt: {other:?}"),
        }

        match &module.stmts[1] {
            Stmt::ExprStmt(Expr::VarRef { name, .. }) => assert_eq!(name, "answer"),
            other => panic!("unexpected second stmt: {other:?}"),
        }
    }

    // ── TypeExpr Display ──

    #[test]
    fn type_expr_named_to_string() {
        assert_eq!(TypeExpr::named("Int64").to_string(), "Int64");
        assert_eq!(TypeExpr::named("String").to_string(), "String");
        assert_eq!(TypeExpr::named("Bool").to_string(), "Bool");
    }

    #[test]
    fn type_expr_list_to_string() {
        let list_int = TypeExpr::List(Box::new(TypeExpr::named("Int64")));
        assert_eq!(list_int.to_string(), "List<Int64>");
    }

    #[test]
    fn type_expr_arrow_to_string() {
        let arrow = TypeExpr::Arrow {
            params: vec![TypeExpr::named("Int64")],
            ret: Box::new(TypeExpr::named("Bool")),
        };
        assert_eq!(arrow.to_string(), "|Int64| -> Bool");
    }

    #[test]
    fn type_expr_deep_nesting_to_string() {
        let ty = TypeExpr::Arrow {
            params: vec![TypeExpr::List(Box::new(TypeExpr::List(Box::new(
                TypeExpr::named("Int64"),
            ))))],
            ret: Box::new(TypeExpr::Arrow {
                params: vec![TypeExpr::named("String")],
                ret: Box::new(TypeExpr::named("Bool")),
            }),
        };
        assert_eq!(ty.to_string(), "|List<List<Int64>>| -> |String| -> Bool");
    }

    // ── BinOp and UnOp derivations ──

    #[test]
    fn binop_copy_and_eq() {
        let a = BinOp::Add;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn unop_copy_and_eq() {
        let a = UnOp::Neg;
        let b = a;
        assert_eq!(a, b);
    }

    // ── Expr constructors ──

    #[test]
    fn expr_lit_int_partial_eq() {
        assert_eq!(Expr::LitInt(42), Expr::LitInt(42));
        assert_ne!(Expr::LitInt(42), Expr::LitInt(0));
    }

    #[test]
    fn expr_var_ref_partial_eq() {
        assert_eq!(
            Expr::VarRef { name: "x".into(), span: S },
            Expr::VarRef { name: "x".into(), span: S },
        );
        assert_ne!(
            Expr::VarRef { name: "x".into(), span: S },
            Expr::VarRef { name: "y".into(), span: S },
        );
    }

    #[test]
    fn expr_lambda_partial_eq() {
        let l1 = Expr::Lambda {
            params: vec![Param {
                name: "x".into(),
                span: S,
                ty_ann: None,
            }],
            ret_ty: None,
            body: Box::new(Expr::VarRef { name: "x".into(), span: S }),
        };
        let l2 = Expr::Lambda {
            params: vec![Param {
                name: "x".into(),
                span: S,
                ty_ann: None,
            }],
            ret_ty: None,
            body: Box::new(Expr::VarRef { name: "x".into(), span: S }),
        };
        assert_eq!(l1, l2);
    }

    // ── Stmt constructors ──

    #[test]
    fn stmt_partial_eq() {
        let s1 = Stmt::ConstDecl {
            name: "x".into(),
            span: S,
            ty_ann: None,
            value: Expr::LitInt(1),
        };
        let s2 = Stmt::ConstDecl {
            name: "x".into(),
            span: S,
            ty_ann: None,
            value: Expr::LitInt(1),
        };
        assert_eq!(s1, s2);
    }

    // ── Module PartialEq ──

    #[test]
    fn module_partial_eq() {
        let m1 = Module { stmts: vec![] };
        let m2 = Module { stmts: vec![] };
        assert_eq!(m1, m2);
    }

    #[test]
    fn module_debug_format() {
        let m = Module { stmts: vec![] };
        let s = format!("{m:?}");
        assert!(s.contains("Module"));
    }
}
