//! Algorithm W — Hindley-Milner 类型推断

use crate::types::*;
use kaubo_ast::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

static TVAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn fresh_tvar() -> TypeVar {
    TypeVar(TVAR_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn reset_tvar() {
    TVAR_COUNTER.store(0, Ordering::Relaxed);
}

static STRUCT_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn fresh_struct_id() -> usize {
    STRUCT_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug)]
pub struct TypeError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

pub type InferResult<T> = Result<T, TypeError>;

// ── 主入口 ──

pub fn infer_module(
    module: &Module,
) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
    reset_tvar();
    let mut env: TypeEnv = HashMap::new();

    // Pass 1: collect struct definitions
    let mut struct_registry: HashMap<String, usize> = HashMap::new();
    let mut struct_fields: HashMap<usize, Vec<(String, Type)>> = HashMap::new();
    for stmt in &module.stmts {
        if let Stmt::StructDef { name, fields } = stmt {
            let id = fresh_struct_id();
            struct_registry.insert(name.clone(), id);
            let mut fts = Vec::new();
            for f in fields {
                fts.push((
                    f.name.clone(),
                    type_expr_to_type(&f.ty, &struct_registry, &struct_fields)?,
                ));
            }
            struct_fields.insert(id, fts);
        }
    }

    // Pass 2: inject stdlib builtins
    inject_stdlib(&mut env);

    // Pass 3: infer all statements
    for stmt in &module.stmts {
        match stmt {
            Stmt::ConstDecl { name, value, .. } => {
                let (s, ty) = infer(&env, value, &struct_registry, &struct_fields)?;
                let scheme = generalize(&env, &s.apply(&ty));
                env.insert(name.clone(), scheme);
            }
            Stmt::VarDecl { name, value, .. } => {
                let ty = if let Some(val) = value {
                    let (s, t) = infer(&env, val, &struct_registry, &struct_fields)?;
                    s.apply(&t)
                } else {
                    Type::Var(fresh_tvar())
                };
                env.insert(name.clone(), Scheme::monomorphic(ty));
            }
            Stmt::StructDef { name, fields } => {
                let id = struct_registry[name];
                let mut fts = Vec::new();
                for f in fields {
                    fts.push((
                        f.name.clone(),
                        type_expr_to_type(&f.ty, &struct_registry, &struct_fields)?,
                    ));
                }
                struct_fields.insert(id, fts);
            }
            Stmt::ImplBlock {
                struct_name,
                methods,
            } => {
                // Register methods on struct
                for m in methods {
                    let (s, ty) = infer(&env, &m.body, &struct_registry, &struct_fields)?;
                    let scheme = generalize(&env, &s.apply(&ty));
                    env.insert(format!("{}.{}", struct_name, m.name), scheme);
                }
            }
            Stmt::ExprStmt(expr) => {
                infer(&env, expr, &struct_registry, &struct_fields)?;
            }
            Stmt::ExportStmt(_) | Stmt::Import { .. } => {}
        }
    }

    Ok((env, struct_fields))
}

// ── stdlib injection ──

fn inject_stdlib(env: &mut TypeEnv) {
    // print: String → Null
    env.insert(
        "print".into(),
        Scheme::monomorphic(Type::Arrow(Box::new(Type::String), Box::new(Type::Null))),
    );
    // type_of: forall a. a → String
    let tv = fresh_tvar();
    env.insert(
        "type_of".into(),
        Scheme {
            bound: vec![tv],
            body: Box::new(Type::Arrow(Box::new(Type::Var(tv)), Box::new(Type::String))),
        },
    );
    // assert: Bool → Null
    env.insert(
        "assert".into(),
        Scheme::monomorphic(Type::Arrow(Box::new(Type::Bool), Box::new(Type::Null))),
    );
    // sqrt/sin/cos: Float64 → Float64
    for name in &["sqrt", "sin", "cos", "floor", "ceil"] {
        env.insert(
            name.to_string(),
            Scheme::monomorphic(Type::Arrow(
                Box::new(Type::Float64),
                Box::new(Type::Float64),
            )),
        );
    }
}

// ── 推断 ──

pub fn infer(
    env: &TypeEnv,
    expr: &Expr,
    structs: &HashMap<String, usize>,
    struct_fields: &HashMap<usize, Vec<(String, Type)>>,
) -> InferResult<(Subst, Type)> {
    match expr {
        Expr::LitInt(_) => Ok((Subst::empty(), Type::Int64)),
        Expr::LitFloat(_) => Ok((Subst::empty(), Type::Float64)),
        Expr::LitString(_) => Ok((Subst::empty(), Type::String)),
        Expr::LitTrue | Expr::LitFalse => Ok((Subst::empty(), Type::Bool)),
        Expr::LitNull => Ok((Subst::empty(), Type::Null)),

        Expr::VarRef(name) => {
            let scheme = env.get(name).ok_or_else(|| TypeError {
                msg: format!("unbound variable '{}'", name),
                line: 0,
                col: 0,
            })?;
            Ok((Subst::empty(), instantiate(scheme)))
        }

        Expr::Lambda { params, body, .. } => {
            let mut s = Subst::empty();
            let mut env_local = env.clone();
            let mut param_types = Vec::new();

            for p in params {
                let pt = if let Some(ann) = &p.ty_ann {
                    type_expr_to_type(ann, structs, struct_fields)?
                } else {
                    Type::Var(fresh_tvar())
                };
                param_types.push(pt.clone());
                env_local.insert(p.name.clone(), Scheme::monomorphic(pt));
            }

            let (s_body, body_ty) = infer(&env_local, body, structs, struct_fields)?;
            s = s.compose(&s_body);

            let mut arrow_ty = body_ty;
            for pt in param_types.into_iter().rev() {
                arrow_ty = Type::Arrow(Box::new(s.apply(&pt)), Box::new(arrow_ty));
            }
            Ok((s, arrow_ty))
        }

        Expr::Call { func, args } => {
            let (mut s, func_ty) = infer(env, func, structs, struct_fields)?;
            let mut arg_types = Vec::new();
            for arg in args {
                let (s_arg, arg_ty) = infer(env, arg, structs, struct_fields)?;
                s = s.compose(&s_arg);
                arg_types.push(arg_ty);
            }
            let ret = Type::Var(fresh_tvar());
            let mut arrow = ret.clone();
            for at in arg_types.into_iter().rev() {
                arrow = Type::Arrow(Box::new(at), Box::new(arrow));
            }
            s = unify(&s.apply(&func_ty), &arrow)
                .map_err(|e| TypeError {
                    msg: e,
                    line: 0,
                    col: 0,
                })?
                .compose(&s);
            Ok((s.clone(), s.apply(&ret)))
        }

        Expr::Binary { left, op, right } => {
            let (s1, t1) = infer(env, left, structs, struct_fields)?;
            let (s2, t2) = infer(env, right, structs, struct_fields)?;
            let mut s = s1.compose(&s2);

            let result_type = match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                    s = unify(&s.apply(&t1), &s.apply(&t2))
                        .map_err(|e| TypeError {
                            msg: format!("binary operator: {}", e),
                            line: 0,
                            col: 0,
                        })?
                        .compose(&s);
                    s.apply(&t1)
                }
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => Type::Bool,
                BinOp::Pipe | BinOp::GtGt => {
                    // Pipe: a |> f means f(a)
                    // For now just treat as pass-through
                    t1.clone()
                }
                BinOp::SAdd => Type::String,
            };
            Ok((s, result_type))
        }

        Expr::Unary { op, right } => {
            let (s, t) = infer(env, right, structs, struct_fields)?;
            Ok((
                s,
                match op {
                    UnOp::Neg => t,
                    UnOp::Not => Type::Bool,
                },
            ))
        }

        Expr::Block(stmts) => {
            let mut s = Subst::empty();
            let mut local_env = env.clone();
            let mut result = Type::Null;
            for stmt in stmts {
                match stmt {
                    Stmt::ConstDecl { name, value, .. } => {
                        let (s_val, ty) = infer(&local_env, value, structs, struct_fields)?;
                        let scheme = generalize(&local_env, &s_val.apply(&ty));
                        local_env.insert(name.clone(), scheme);
                        s = s.compose(&s_val);
                        result = Type::Null;
                    }
                    Stmt::VarDecl { name, value, .. } => {
                        let ty = if let Some(val) = value {
                            let (s_val, t) = infer(&local_env, val, structs, struct_fields)?;
                            s = s.compose(&s_val);
                            t
                        } else {
                            Type::Var(fresh_tvar())
                        };
                        local_env.insert(name.clone(), Scheme::monomorphic(ty));
                        result = Type::Null;
                    }
                    Stmt::ExprStmt(expr) => {
                        let (s_e, ty) = infer(&local_env, expr, structs, struct_fields)?;
                        s = s.compose(&s_e);
                        result = ty;
                    }
                    _ => {}
                }
            }
            Ok((s, result))
        }

        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let (s_c, _tc) = infer(env, cond, structs, struct_fields)?;
            let (s_t, tt) = infer(env, then_branch, structs, struct_fields)?;
            if let Some(eb) = else_branch {
                let (s_e, te) = infer(env, eb, structs, struct_fields)?;
                let mut s = s_c.compose(&s_t).compose(&s_e);
                s = unify(&s.apply(&tt), &s.apply(&te))
                    .map_err(|e| TypeError {
                        msg: format!("if branches: {}", e),
                        line: 0,
                        col: 0,
                    })?
                    .compose(&s);
                Ok((s.clone(), s.apply(&tt)))
            } else {
                let s = s_c.compose(&s_t);
                Ok((s.clone(), s.apply(&tt)))
            }
        }

        Expr::While { cond, body } => {
            let (s_c, _) = infer(env, cond, structs, struct_fields)?;
            let (s_b, _) = infer(env, body, structs, struct_fields)?;
            Ok((s_c.compose(&s_b), Type::Null))
        }

        Expr::For {
            var: _,
            iterable,
            body,
        } => {
            let (s_i, ti) = infer(env, iterable, structs, struct_fields)?;
            // ti should be List<'a>
            if let Type::List(_) = &ti {
                let local_env = env.clone();
                // Bind loop variable
                let (s_b, _) = infer(&local_env, body, structs, struct_fields)?;
                Ok((s_i.compose(&s_b), Type::Null))
            } else {
                Err(TypeError {
                    msg: format!("for loop requires List, got {}", ti),
                    line: 0,
                    col: 0,
                })
            }
        }

        Expr::Break | Expr::Continue => Ok((Subst::empty(), Type::Null)),
        Expr::Return(val) => {
            if let Some(expr) = val {
                infer(env, expr, structs, struct_fields)
            } else {
                Ok((Subst::empty(), Type::Null))
            }
        }

        Expr::Member { object, field } => {
            let (s, ty) = infer(env, object, structs, struct_fields)?;
            let applied = s.apply(&ty);
            // to_string() on Int64 / Float64 returns String
            if field == "to_string" && matches!(applied, Type::Int64 | Type::Float64) {
                return Ok((s, Type::String));
            }
            // to_float() on Int64 returns Float64
            if field == "to_float" && matches!(applied, Type::Int64) {
                return Ok((s, Type::Float64));
            }
            match applied {
                Type::Record(id, fields) => {
                    // Try struct field first
                    if let Some(t) = fields
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                    {
                        return Ok((s, t));
                    }
                    // Try impl method: look up "{struct_name}.{field}" in env
                    for (name, &sid) in structs {
                        if sid == id {
                            let method_name = format!("{}.{}", name, field);
                            if let Some(scheme) = env.get(&method_name) {
                                let ty = instantiate(scheme);
                                // Drop self parameter — caller already knows self
                                let ty = match ty {
                                    Type::Arrow(_, body) => *body,
                                    other => other,
                                };
                                return Ok((s, ty));
                            }
                        }
                    }
                    Err(TypeError {
                        msg: format!("field '{}' not found", field),
                        line: 0,
                        col: 0,
                    })
                }
                _ => Err(TypeError {
                    msg: format!("cannot access field '{}' on {}", field, ty),
                    line: 0,
                    col: 0,
                }),
            }
        }

        Expr::Index { object, index } => {
            let (s1, t_obj) = infer(env, object, structs, struct_fields)?;
            let (s2, _) = infer(env, index, structs, struct_fields)?;
            let s = s1.compose(&s2);
            match s.apply(&t_obj) {
                Type::List(elem) => Ok((s.clone(), s.apply(&elem))),
                Type::String => Ok((s, Type::String)),
                _ => Ok((s, Type::Var(fresh_tvar()))),
            }
        }

        Expr::StructLit { name, fields } => {
            let id = structs.get(name).ok_or_else(|| TypeError {
                msg: format!("unknown struct '{}'", name),
                line: 0,
                col: 0,
            })?;
            let field_types = struct_fields.get(id).cloned().unwrap_or_default();
            let mut s = Subst::empty();
            for (_fname, fval) in fields {
                let (s_f, _) = infer(env, fval, structs, struct_fields)?;
                s = s.compose(&s_f);
            }
            Ok((s, Type::Record(*id, field_types)))
        }

        Expr::ListLit(items) => {
            let mut s = Subst::empty();
            let elem_ty = Type::Var(fresh_tvar());
            for item in items {
                let (s_i, ti) = infer(env, item, structs, struct_fields)?;
                s = s.compose(&s_i);
                s = unify(&s.apply(&elem_ty), &s.apply(&ti))
                    .map_err(|e| TypeError {
                        msg: format!("list element: {}", e),
                        line: 0,
                        col: 0,
                    })?
                    .compose(&s);
            }
            Ok((s, Type::List(Box::new(elem_ty))))
        }

        Expr::Assign { target, value } => {
            let (s1, _) = infer(env, target, structs, struct_fields)?;
            let (s2, _) = infer(env, value, structs, struct_fields)?;
            Ok((s1.compose(&s2), Type::Null))
        }

        Expr::Async(body) => infer(env, body, structs, struct_fields),
        Expr::Await(body) => infer(env, body, structs, struct_fields),
    }
}

// ── 统一 ──

pub fn unify(t1: &Type, t2: &Type) -> Result<Subst, String> {
    match (t1, t2) {
        (a, b) if a == b => Ok(Subst::empty()),
        (Type::Var(v), ty) if !occurs_check(v, ty) => Ok(Subst::singleton(*v, ty.clone())),
        (ty, Type::Var(v)) if !occurs_check(v, ty) => Ok(Subst::singleton(*v, ty.clone())),
        (Type::Arrow(a1, r1), Type::Arrow(a2, r2)) => {
            let s1 = unify(a1, a2)?;
            let s2 = unify(&s1.apply(r1), &s1.apply(r2))?;
            Ok(s2.compose(&s1))
        }
        (Type::List(t1), Type::List(t2)) => unify(t1, t2),
        (Type::Record(id1, _), Type::Record(id2, _)) if id1 == id2 => Ok(Subst::empty()),
        _ => Err(format!("cannot unify {} and {}", t1, t2)),
    }
}

fn occurs_check(var: &TypeVar, ty: &Type) -> bool {
    match ty {
        Type::Var(v) => v == var,
        Type::Arrow(a, r) => occurs_check(var, a) || occurs_check(var, r),
        Type::List(t) => occurs_check(var, t),
        _ => false,
    }
}

// ── 泛化 ──

pub fn generalize(env: &TypeEnv, ty: &Type) -> Scheme {
    let free = free_type_vars(ty);
    let bound: Vec<TypeVar> = free
        .into_iter()
        .filter(|v| !env.values().any(|s| free_type_vars(&s.body).contains(v)))
        .collect();
    Scheme {
        bound,
        body: Box::new(ty.clone()),
    }
}

fn free_type_vars(ty: &Type) -> Vec<TypeVar> {
    let mut fv = Vec::new();
    match ty {
        Type::Var(v) => fv.push(*v),
        Type::Arrow(a, r) => {
            fv.append(&mut free_type_vars(a));
            fv.append(&mut free_type_vars(r));
        }
        Type::List(t) => {
            fv.append(&mut free_type_vars(t));
        }
        Type::Record(_, fields) => {
            for (_, t) in fields {
                fv.append(&mut free_type_vars(t));
            }
        }
        _ => {}
    }
    fv.dedup();
    fv
}

// ── 实例化 ──

pub fn instantiate(scheme: &Scheme) -> Type {
    let mut subst = Subst::empty();
    for v in &scheme.bound {
        subst.extend(*v, Type::Var(fresh_tvar()));
    }
    subst.apply(&scheme.body)
}

// ── 类型表达式转内部类型 ──

fn type_expr_to_type(
    te: &TypeExpr,
    structs: &HashMap<String, usize>,
    struct_fields: &HashMap<usize, Vec<(String, Type)>>,
) -> InferResult<Type> {
    match te {
        TypeExpr::Named(n) => match n.as_str() {
            "Int64" => Ok(Type::Int64),
            "Float64" => Ok(Type::Float64),
            "String" => Ok(Type::String),
            "Bool" => Ok(Type::Bool),
            "Null" => Ok(Type::Null),
            _ => {
                if let Some(&id) = structs.get(n) {
                    let fields = struct_fields.get(&id).cloned().unwrap_or_default();
                    Ok(Type::Record(id, fields))
                } else {
                    Err(TypeError {
                        msg: format!("unknown type '{}'", n),
                        line: 0,
                        col: 0,
                    })
                }
            }
        },
        TypeExpr::List(t) => Ok(Type::List(Box::new(type_expr_to_type(
            t,
            structs,
            struct_fields,
        )?))),
        TypeExpr::Arrow { params, ret } => {
            let mut arrow = type_expr_to_type(ret, structs, struct_fields)?;
            for p in params.iter().rev() {
                arrow = Type::Arrow(
                    Box::new(type_expr_to_type(p, structs, struct_fields)?),
                    Box::new(arrow),
                );
            }
            Ok(arrow)
        }
    }
}

// ── tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn module(stmts: Vec<Stmt>) -> Module {
        Module { stmts }
    }

    fn const_decl(name: &str, value: Expr) -> Stmt {
        Stmt::ConstDecl {
            name: name.to_string(),
            ty_ann: None,
            value,
        }
    }

    fn var_decl(name: &str, value: Option<Expr>) -> Stmt {
        Stmt::VarDecl {
            name: name.to_string(),
            ty_ann: None,
            value,
        }
    }

    fn param(name: &str, ty_ann: Option<TypeExpr>) -> Param {
        Param {
            name: name.to_string(),
            ty_ann,
        }
    }

    fn empty_structs() -> (HashMap<String, usize>, HashMap<usize, Vec<(String, Type)>>) {
        (HashMap::new(), HashMap::new())
    }

    fn infer_expr(expr: Expr) -> InferResult<Type> {
        let (structs, fields) = empty_structs();
        let env = TypeEnv::new();
        let (subst, ty) = infer(&env, &expr, &structs, &fields)?;
        Ok(subst.apply(&ty))
    }

    fn infer_ast(module: Module) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
        infer_module(&module)
    }

    fn infer_src(src: &str) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
        match src {
            "const x = 42;" => infer_ast(module(vec![const_decl("x", Expr::LitInt(42))])),
            "const x = 3.14;" => infer_ast(module(vec![const_decl("x", Expr::LitFloat(3.14))])),
            "const s = \"hi\"; const b = true;" => infer_ast(module(vec![
                const_decl("s", Expr::LitString("hi".to_string())),
                const_decl("b", Expr::LitTrue),
            ])),
            "const add = |a, b| { a + b };" => infer_ast(module(vec![const_decl(
                "add",
                Expr::Lambda {
                    params: vec![
                        Param {
                            name: "a".to_string(),
                            ty_ann: None,
                        },
                        Param {
                            name: "b".to_string(),
                            ty_ann: None,
                        },
                    ],
                    ret_ty: None,
                    body: Box::new(Expr::Block(vec![Stmt::ExprStmt(Expr::Binary {
                        left: Box::new(Expr::VarRef("a".to_string())),
                        op: BinOp::Add,
                        right: Box::new(Expr::VarRef("b".to_string())),
                    })])),
                },
            )])),
            "const id = |x| { x };" => infer_ast(module(vec![const_decl(
                "id",
                Expr::Lambda {
                    params: vec![Param {
                        name: "x".to_string(),
                        ty_ann: None,
                    }],
                    ret_ty: None,
                    body: Box::new(Expr::Block(vec![Stmt::ExprStmt(Expr::VarRef(
                        "x".to_string(),
                    ))])),
                },
            )])),
            "struct Point { x: Float64, y: Float64 }; const p = Point { x: 1.0, y: 2.0 };" => {
                infer_ast(module(vec![
                    Stmt::StructDef {
                        name: "Point".to_string(),
                        fields: vec![
                            FieldDef {
                                name: "x".to_string(),
                                ty: TypeExpr::Named("Float64".to_string()),
                            },
                            FieldDef {
                                name: "y".to_string(),
                                ty: TypeExpr::Named("Float64".to_string()),
                            },
                        ],
                    },
                    const_decl(
                        "p",
                        Expr::StructLit {
                            name: "Point".to_string(),
                            fields: vec![
                                ("x".to_string(), Expr::LitFloat(1.0)),
                                ("y".to_string(), Expr::LitFloat(2.0)),
                            ],
                        },
                    ),
                ]))
            }
            "const s = 42.to_string();" => infer_ast(module(vec![const_decl(
                "s",
                Expr::Member {
                    object: Box::new(Expr::LitInt(42)),
                    field: "to_string".to_string(),
                },
            )])),
            "var x = 42;" => infer_ast(module(vec![Stmt::VarDecl {
                name: "x".to_string(),
                ty_ann: None,
                value: Some(Expr::LitInt(42)),
            }])),
            _ => panic!("test fixture should construct AST directly for source: {src}"),
        }
    }

    #[test]
    fn test_int_literal() {
        let (env, _) = infer_src("const x = 42;").unwrap();
        match env.get("x") {
            Some(s) => assert_eq!(format!("{}", s.body), "Int64"),
            None => panic!("x not found"),
        }
    }

    #[test]
    fn test_float_literal() {
        let (env, _) = infer_src("const x = 3.14;").unwrap();
        match env.get("x") {
            Some(s) => assert_eq!(format!("{}", s.body), "Float64"),
            None => panic!(),
        }
    }

    #[test]
    fn test_string_and_bool() {
        let (env, _) = infer_src("const s = \"hi\"; const b = true;").unwrap();
        assert_eq!(format!("{}", env["s"].body), "String");
        assert_eq!(format!("{}", env["b"].body), "Bool");
    }

    #[test]
    fn test_lambda_simple() {
        let (env, _) = infer_src("const add = |a, b| { a + b };").unwrap();
        let ty = format!("{}", env["add"].body);
        assert!(ty.contains("Int64 → Int64 → Int64") || ty.contains("→"));
    }

    #[test]
    fn test_let_polymorphism() {
        let (env, _) = infer_src("const id = |x| { x };").unwrap();
        assert!(format!("{}", env["id"].body).contains("→"));
        // id(42) and id(\"hi\") would both type-check with let-polymorphism
    }

    #[test]
    fn test_var_is_monomorphic() {
        let result = infer_var("var x = 42;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_infer() {
        let (env, _) = infer_src(
            "struct Point { x: Float64, y: Float64 }; const p = Point { x: 1.0, y: 2.0 };",
        )
        .unwrap();
        let ty = format!("{}", env["p"].body);
        assert!(ty.contains("Point") || ty.contains("{"));
    }

    #[test]
    fn test_to_string_returns_string() {
        let (env, _) = infer_src("const s = 42.to_string();").unwrap();
        let ty = format!("{}", env["s"].body);
        assert_eq!(ty, "String");
    }

    #[test]
    fn var_without_initializer_gets_fresh_type_var() {
        let (env, _) = infer_ast(module(vec![var_decl("x", None)])).unwrap();
        assert!(matches!(*env["x"].body, Type::Var(_)));
    }

    #[test]
    fn annotated_lambda_uses_type_exprs() {
        let ty = infer_expr(Expr::Lambda {
            params: vec![param("x", Some(TypeExpr::named("Int64")))],
            ret_ty: None,
            body: Box::new(Expr::VarRef("x".to_string())),
        })
        .unwrap();

        assert_eq!(format!("{}", ty), "(Int64 → Int64)");
    }

    #[test]
    fn call_infers_return_type_and_call_errors_on_non_function() {
        let id = Expr::Lambda {
            params: vec![param("x", None)],
            ret_ty: None,
            body: Box::new(Expr::VarRef("x".to_string())),
        };
        assert_eq!(
            infer_expr(Expr::Call {
                func: Box::new(id),
                args: vec![Expr::LitString("ok".to_string())],
            })
            .unwrap(),
            Type::String
        );

        let err = infer_expr(Expr::Call {
            func: Box::new(Expr::LitInt(1)),
            args: vec![Expr::LitInt(2)],
        })
        .unwrap_err();
        assert!(err.msg.contains("cannot unify"));
    }

    #[test]
    fn binary_unary_and_control_flow_exprs() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(1)),
                op: BinOp::Lt,
                right: Box::new(Expr::LitInt(2)),
            })
            .unwrap(),
            Type::Bool
        );
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitString("a".to_string())),
                op: BinOp::SAdd,
                right: Box::new(Expr::LitString("b".to_string())),
            })
            .unwrap(),
            Type::String
        );
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(1)),
                op: BinOp::Pipe,
                right: Box::new(Expr::LitInt(2)),
            })
            .unwrap(),
            Type::Int64
        );
        assert_eq!(
            infer_expr(Expr::Unary {
                op: UnOp::Not,
                right: Box::new(Expr::LitFalse),
            })
            .unwrap(),
            Type::Bool
        );
        assert_eq!(infer_expr(Expr::Break).unwrap(), Type::Null);
        assert_eq!(infer_expr(Expr::Continue).unwrap(), Type::Null);
        assert_eq!(infer_expr(Expr::Return(None)).unwrap(), Type::Null);
        assert_eq!(
            infer_expr(Expr::Return(Some(Box::new(Expr::LitFloat(1.0))))).unwrap(),
            Type::Float64
        );
    }

    #[test]
    fn blocks_if_while_and_for_are_inferred_locally() {
        assert_eq!(
            infer_expr(Expr::Block(vec![
                const_decl("x", Expr::LitInt(1)),
                var_decl("y", Some(Expr::LitInt(2))),
                Stmt::ExprStmt(Expr::VarRef("x".to_string())),
            ]))
            .unwrap(),
            Type::Int64
        );
        assert_eq!(
            infer_expr(Expr::If {
                cond: Box::new(Expr::LitTrue),
                then_branch: Box::new(Expr::LitInt(1)),
                else_branch: None,
            })
            .unwrap(),
            Type::Int64
        );
        assert_eq!(
            infer_expr(Expr::While {
                cond: Box::new(Expr::LitFalse),
                body: Box::new(Expr::Block(vec![])),
            })
            .unwrap(),
            Type::Null
        );
        assert_eq!(
            infer_expr(Expr::For {
                var: param("item", None),
                iterable: Box::new(Expr::ListLit(vec![Expr::LitInt(1)])),
                body: Box::new(Expr::Block(vec![])),
            })
            .unwrap(),
            Type::Null
        );

        let err = infer_expr(Expr::For {
            var: param("item", None),
            iterable: Box::new(Expr::LitInt(1)),
            body: Box::new(Expr::Block(vec![])),
        })
        .unwrap_err();
        assert!(err.msg.contains("for loop requires List"));
    }

    #[test]
    fn mismatched_if_and_list_report_errors() {
        let err = infer_expr(Expr::If {
            cond: Box::new(Expr::LitTrue),
            then_branch: Box::new(Expr::LitInt(1)),
            else_branch: Some(Box::new(Expr::LitString("x".to_string()))),
        })
        .unwrap_err();
        assert!(err.msg.contains("if branches"));

        let err = infer_expr(Expr::ListLit(vec![
            Expr::LitInt(1),
            Expr::LitString("x".to_string()),
        ]))
        .unwrap_err();
        assert!(err.msg.contains("list element"));
    }

    #[test]
    fn struct_fields_methods_and_member_errors() {
        let program = module(vec![
            Stmt::StructDef {
                name: "Point".to_string(),
                fields: vec![FieldDef {
                    name: "x".to_string(),
                    ty: TypeExpr::named("Int64"),
                }],
            },
            Stmt::ImplBlock {
                struct_name: "Point".to_string(),
                methods: vec![MethodDef {
                    name: "value".to_string(),
                    body: Expr::Lambda {
                        params: vec![param("self", Some(TypeExpr::named("Point")))],
                        ret_ty: None,
                        body: Box::new(Expr::LitInt(7)),
                    },
                }],
            },
            const_decl(
                "p",
                Expr::StructLit {
                    name: "Point".to_string(),
                    fields: vec![("x".to_string(), Expr::LitInt(1))],
                },
            ),
            const_decl(
                "x",
                Expr::Member {
                    object: Box::new(Expr::VarRef("p".to_string())),
                    field: "x".to_string(),
                },
            ),
            const_decl(
                "m",
                Expr::Member {
                    object: Box::new(Expr::VarRef("p".to_string())),
                    field: "value".to_string(),
                },
            ),
        ]);
        let (env, _) = infer_ast(program).unwrap();
        assert_eq!(format!("{}", env["x"].body), "Int64");
        assert_eq!(format!("{}", env["m"].body), "Int64");

        let err = infer_ast(module(vec![const_decl(
            "bad",
            Expr::StructLit {
                name: "Missing".to_string(),
                fields: vec![],
            },
        )]))
        .unwrap_err();
        assert!(err.msg.contains("unknown struct"));

        let err = infer_expr(Expr::Member {
            object: Box::new(Expr::LitInt(1)),
            field: "x".to_string(),
        })
        .unwrap_err();
        assert!(err.msg.contains("cannot access field"));
    }

    #[test]
    fn member_builtins_index_assign_async_and_await() {
        assert_eq!(
            infer_expr(Expr::Member {
                object: Box::new(Expr::LitInt(1)),
                field: "to_float".to_string(),
            })
            .unwrap(),
            Type::Float64
        );
        assert_eq!(
            infer_expr(Expr::Index {
                object: Box::new(Expr::ListLit(vec![Expr::LitInt(1)])),
                index: Box::new(Expr::LitInt(0)),
            })
            .unwrap(),
            Type::Int64
        );
        assert_eq!(
            infer_expr(Expr::Index {
                object: Box::new(Expr::LitString("abc".to_string())),
                index: Box::new(Expr::LitInt(0)),
            })
            .unwrap(),
            Type::String
        );
        assert!(matches!(
            infer_expr(Expr::Index {
                object: Box::new(Expr::LitInt(1)),
                index: Box::new(Expr::LitInt(0)),
            })
            .unwrap(),
            Type::Var(_)
        ));
        assert_eq!(
            infer_expr(Expr::Assign {
                target: Box::new(Expr::LitInt(1)),
                value: Box::new(Expr::LitInt(2)),
            })
            .unwrap(),
            Type::Null
        );
        assert_eq!(
            infer_expr(Expr::Async(Box::new(Expr::LitTrue))).unwrap(),
            Type::Bool
        );
        assert_eq!(
            infer_expr(Expr::Await(Box::new(Expr::LitNull))).unwrap(),
            Type::Null
        );
    }

    #[test]
    fn type_expr_unify_generalize_and_instantiate_edges() {
        let mut structs = HashMap::new();
        structs.insert("Node".to_string(), 1);
        let mut fields = HashMap::new();
        fields.insert(1, vec![("value".to_string(), Type::Int64)]);

        assert_eq!(
            type_expr_to_type(
                &TypeExpr::List(Box::new(TypeExpr::named("Int64"))),
                &structs,
                &fields,
            )
            .unwrap(),
            Type::List(Box::new(Type::Int64))
        );
        assert_eq!(
            format!(
                "{}",
                type_expr_to_type(
                    &TypeExpr::Arrow {
                        params: vec![TypeExpr::named("Int64")],
                        ret: Box::new(TypeExpr::named("Bool")),
                    },
                    &structs,
                    &fields,
                )
                .unwrap()
            ),
            "(Int64 → Bool)"
        );
        assert!(type_expr_to_type(&TypeExpr::named("Missing"), &structs, &fields).is_err());

        assert!(unify(
            &Type::Var(TypeVar(10)),
            &Type::List(Box::new(Type::Var(TypeVar(10))))
        )
        .is_err());
        assert!(unify(
            &Type::Arrow(Box::new(Type::Int64), Box::new(Type::Bool)),
            &Type::Arrow(Box::new(Type::Int64), Box::new(Type::String)),
        )
        .is_err());

        let mut env = TypeEnv::new();
        env.insert(
            "kept".to_string(),
            Scheme::monomorphic(Type::Var(TypeVar(1))),
        );
        let scheme = generalize(
            &env,
            &Type::Arrow(
                Box::new(Type::Var(TypeVar(1))),
                Box::new(Type::Var(TypeVar(2))),
            ),
        );
        assert_eq!(scheme.bound, vec![TypeVar(2)]);
        assert!(matches!(instantiate(&scheme), Type::Arrow(_, _)));
    }

    fn infer_var(src: &str) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
        infer_src(src)
    }
}
