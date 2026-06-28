//! kaubo-ir — v2 intermediate representation
//!
//! CPS blocks, types, AST→CPS build, flattening, optimization passes

#![allow(clippy::type_complexity, clippy::len_zero, dead_code)]

pub mod cps;
pub mod cps_build;
pub mod cps_emit;
pub mod flatten;
pub mod pass;

#[cfg(test)]
mod test_fixtures {
    use kaubo_ast::*;

    pub fn module(src: &str) -> Module {
        Module {
            stmts: match src {
                "const x = 42;" => vec![const_decl("x", Expr::LitInt(42))],
                "const x = 1 + 2;" => vec![const_decl("x", int_bin(1, BinOp::Add, 2))],
                "const x = 1 + 2 + 3;" => vec![const_decl(
                    "x",
                    Expr::Binary {
                        left: Box::new(int_bin(1, BinOp::Add, 2)),
                        op: BinOp::Add,
                        right: Box::new(Expr::LitInt(3)),
                    },
                )],
                "const x = if true { 1 } else { 2 };" => vec![const_decl(
                    "x",
                    Expr::If {
                        cond: Box::new(Expr::LitTrue),
                        then_branch: Box::new(Expr::LitInt(1)),
                        else_branch: Some(Box::new(Expr::LitInt(2))),
                    },
                )],
                "const x = 2 + 3;" => vec![const_decl("x", int_bin(2, BinOp::Add, 3))],
                "const x = 6 * 7;" => vec![const_decl("x", int_bin(6, BinOp::Mul, 7))],
                "const x = 5 < 10;" => vec![const_decl("x", int_bin(5, BinOp::Lt, 10))],
                "var x = 2; var y = x + 3;" => vec![
                    var_decl("x", Expr::LitInt(2)),
                    var_decl("y", var_bin("x", BinOp::Add, Expr::LitInt(3))),
                ],
                "var x = 2; x = x + 1; var y = x + 3;" => vec![
                    var_decl("x", Expr::LitInt(2)),
                    Stmt::ExprStmt(Expr::Assign {
                        target: Box::new(Expr::VarRef("x".to_string())),
                        value: Box::new(var_bin("x", BinOp::Add, Expr::LitInt(1))),
                    }),
                    var_decl("y", var_bin("x", BinOp::Add, Expr::LitInt(3))),
                ],
                "const x = -(42);" => vec![const_decl(
                    "x",
                    Expr::Unary {
                        op: UnOp::Neg,
                        right: Box::new(Expr::LitInt(42)),
                    },
                )],
                "var i = 0; while i < 3 { i = i + 1; };" => vec![
                    var_decl("i", Expr::LitInt(0)),
                    Stmt::ExprStmt(while_assign("i", BinOp::Lt, Expr::LitInt(3), BinOp::Add)),
                ],
                "const f = |x| { x + 1 }; f(41);" => vec![
                    const_decl(
                        "f",
                        lambda(
                            "x",
                            Expr::Block(vec![Stmt::ExprStmt(var_bin(
                                "x",
                                BinOp::Add,
                                Expr::LitInt(1),
                            ))]),
                        ),
                    ),
                    call_stmt("f", vec![Expr::LitInt(41)]),
                ],
                "struct Point { x: Int64, y: Int64 }; const p = Point { x: 1, y: 2 };" => {
                    vec![
                        Stmt::StructDef {
                            name: "Point".to_string(),
                            fields: vec![
                                FieldDef {
                                    name: "x".to_string(),
                                    ty: TypeExpr::Named("Int64".to_string()),
                                },
                                FieldDef {
                                    name: "y".to_string(),
                                    ty: TypeExpr::Named("Int64".to_string()),
                                },
                            ],
                        },
                        const_decl(
                            "p",
                            Expr::StructLit {
                                name: "Point".to_string(),
                                fields: vec![
                                    ("x".to_string(), Expr::LitInt(1)),
                                    ("y".to_string(), Expr::LitInt(2)),
                                ],
                                spread: None,
                            },
                        ),
                    ]
                }
                _ => panic!("missing IR AST fixture for {src}"),
            },
        }
    }

    fn const_decl(name: &str, value: Expr) -> Stmt {
        Stmt::ConstDecl {
            name: name.to_string(),
            ty_ann: None,
            value,
        }
    }

    fn var_decl(name: &str, value: Expr) -> Stmt {
        Stmt::VarDecl {
            name: name.to_string(),
            ty_ann: None,
            value: Some(value),
        }
    }

    fn int_bin(left: i64, op: BinOp, right: i64) -> Expr {
        Expr::Binary {
            left: Box::new(Expr::LitInt(left)),
            op,
            right: Box::new(Expr::LitInt(right)),
        }
    }

    fn var_bin(name: &str, op: BinOp, right: Expr) -> Expr {
        Expr::Binary {
            left: Box::new(Expr::VarRef(name.to_string())),
            op,
            right: Box::new(right),
        }
    }

    fn while_assign(name: &str, cmp: BinOp, rhs: Expr, update: BinOp) -> Expr {
        Expr::While {
            cond: Box::new(Expr::Binary {
                left: Box::new(Expr::VarRef(name.to_string())),
                op: cmp,
                right: Box::new(rhs),
            }),
            body: Box::new(Expr::Block(vec![Stmt::ExprStmt(Expr::Assign {
                target: Box::new(Expr::VarRef(name.to_string())),
                value: Box::new(var_bin(name, update, Expr::LitInt(1))),
            })])),
        }
    }

    fn lambda(param: &str, body: Expr) -> Expr {
        Expr::Lambda {
            params: vec![Param {
                name: param.to_string(),
                ty_ann: None,
            }],
            ret_ty: None,
            body: Box::new(body),
        }
    }

    fn call_stmt(name: &str, args: Vec<Expr>) -> Stmt {
        Stmt::ExprStmt(Expr::Call {
            func: Box::new(Expr::VarRef(name.to_string())),
            arg: Expr::call_arg(args),
        })
    }
}
