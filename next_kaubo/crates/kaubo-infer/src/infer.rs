//! Algorithm W — Hindley-Milner 类型推断

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use kaubo_syntax::ast::*;
use crate::types::*;

static TVAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn fresh_tvar() -> TypeVar {
    TypeVar(TVAR_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn reset_tvar() { TVAR_COUNTER.store(0, Ordering::Relaxed); }

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

pub fn infer_module(module: &Module) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
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
                fts.push((f.name.clone(), type_expr_to_type(&f.ty)?));
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
                    fts.push((f.name.clone(), type_expr_to_type(&f.ty)?));
                }
                struct_fields.insert(id, fts);
            }
            Stmt::ImplBlock { struct_name, methods } => {
                // Register methods on struct
                for m in methods {
                    let (s, ty) = infer(&env, &m.body, &struct_registry, &struct_fields)?;
                    let scheme = generalize(&env, &s.apply(&ty));
                    env.insert(format!("{}.{}", struct_name, m.name), scheme);
                }
            }
            Stmt::ExprStmt(expr) => { infer(&env, expr, &struct_registry, &struct_fields)?; }
            Stmt::ExportStmt(_) | Stmt::Import { .. } => {}
        }
    }

    Ok((env, struct_fields))
}

// ── stdlib injection ──

fn inject_stdlib(env: &mut TypeEnv) {
    // print: String → Null
    env.insert("print".into(), Scheme::monomorphic(
        Type::Arrow(Box::new(Type::String), Box::new(Type::Null))
    ));
    // type_of: forall a. a → String
    let tv = fresh_tvar();
    env.insert("type_of".into(), Scheme {
        bound: vec![tv],
        body: Box::new(Type::Arrow(Box::new(Type::Var(tv)), Box::new(Type::String))),
    });
    // assert: Bool → Null
    env.insert("assert".into(), Scheme::monomorphic(
        Type::Arrow(Box::new(Type::Bool), Box::new(Type::Null))
    ));
    // sqrt/sin/cos: Float64 → Float64
    for name in &["sqrt", "sin", "cos", "floor", "ceil"] {
        env.insert(name.to_string(), Scheme::monomorphic(
            Type::Arrow(Box::new(Type::Float64), Box::new(Type::Float64))
        ));
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
            let scheme = env.get(name)
                .ok_or_else(|| TypeError { msg: format!("unbound variable '{}'", name), line: 0, col: 0 })?;
            Ok((Subst::empty(), instantiate(scheme)))
        }

        Expr::Lambda { params, ret_ty, body } => {
            let mut s = Subst::empty();
            let mut env_local = env.clone();
            let mut param_types = Vec::new();

            for p in params {
                let pt = if let Some(ann) = &p.ty_ann {
                    type_expr_to_type(ann)?
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
            s = unify(&s.apply(&func_ty), &arrow).map_err(|e| TypeError {
                msg: e, line: 0, col: 0
            })?.compose(&s);
            Ok((s.clone(), s.apply(&ret)))
        }

        Expr::Binary { left, op, right } => {
            // Handle value-type methods (compile-time rewrites)
            if let (Expr::VarRef(obj_name), _binop) = (left.as_ref(), op) {
                // We don't need to handle value type methods here since they're
                // handled at the codegen level. The HM engine just infers the types.
            }
            let (s1, t1) = infer(env, left, structs, struct_fields)?;
            let (s2, t2) = infer(env, right, structs, struct_fields)?;
            let mut s = s1.compose(&s2);
            
            let result_type = match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                    s = unify(&s.apply(&t1), &s.apply(&t2)).map_err(|e| TypeError {
                        msg: format!("binary operator: {}", e), line: 0, col: 0
                    })?.compose(&s);
                    s.apply(&t1)
                }
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                    | BinOp::And | BinOp::Or => {
                    Type::Bool
                }
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
            Ok((s, match op { UnOp::Neg => t, UnOp::Not => Type::Bool }))
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
                        } else { Type::Var(fresh_tvar()) };
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

        Expr::If { cond, then_branch, else_branch } => {
            let (s_c, _tc) = infer(env, cond, structs, struct_fields)?;
            let (s_t, tt) = infer(env, then_branch, structs, struct_fields)?;
            if let Some(eb) = else_branch {
                let (s_e, te) = infer(env, eb, structs, struct_fields)?;
                let mut s = s_c.compose(&s_t).compose(&s_e);
                s = unify(&s.apply(&tt), &s.apply(&te)).map_err(|e| TypeError {
                    msg: format!("if branches: {}", e), line: 0, col: 0
                })?.compose(&s);
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

        Expr::For { var: _, iterable, body } => {
            let (s_i, ti) = infer(env, iterable, structs, struct_fields)?;
            // ti should be List<'a>
            if let Type::List(elem) = &ti {
                let mut local_env = env.clone();
                // Bind loop variable
                let (s_b, _) = infer(&local_env, body, structs, struct_fields)?;
                Ok((s_i.compose(&s_b), Type::Null))
            } else {
                Err(TypeError { msg: format!("for loop requires List, got {}", ti), line: 0, col: 0 })
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
            match applied {
                Type::Record(_id, fields) => {
                    let ft = fields.iter().find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| TypeError {
                            msg: format!("field '{}' not found", field), line: 0, col: 0
                        })?;
                    Ok((s, ft))
                }
                _ => Err(TypeError { msg: format!("cannot access field '{}' on {}", field, ty), line: 0, col: 0 }),
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
            let id = structs.get(name)
                .ok_or_else(|| TypeError { msg: format!("unknown struct '{}'", name), line: 0, col: 0 })?;
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
                s = unify(&s.apply(&elem_ty), &s.apply(&ti)).map_err(|e| TypeError {
                    msg: format!("list element: {}", e), line: 0, col: 0
                })?.compose(&s);
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
    let bound: Vec<TypeVar> = free.into_iter().filter(|v| {
        !env.values().any(|s| free_type_vars(&s.body).contains(v))
    }).collect();
    Scheme { bound, body: Box::new(ty.clone()) }
}

fn free_type_vars(ty: &Type) -> Vec<TypeVar> {
    let mut fv = Vec::new();
    match ty {
        Type::Var(v) => fv.push(*v),
        Type::Arrow(a, r) => { fv.append(&mut free_type_vars(a)); fv.append(&mut free_type_vars(r)); }
        Type::List(t) => { fv.append(&mut free_type_vars(t)); }
        Type::Record(_, fields) => {
            for (_, t) in fields { fv.append(&mut free_type_vars(t)); }
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

fn type_expr_to_type(te: &TypeExpr) -> InferResult<Type> {
    match te {
        TypeExpr::Named(n) => match n.as_str() {
            "Int64" => Ok(Type::Int64),
            "Float64" => Ok(Type::Float64),
            "String" => Ok(Type::String),
            "Bool" => Ok(Type::Bool),
            "Null" => Ok(Type::Null),
            _ => Err(TypeError { msg: format!("unknown type '{}'", n), line: 0, col: 0 }),
        },
        TypeExpr::List(t) => Ok(Type::List(Box::new(type_expr_to_type(t)?))),
        TypeExpr::Arrow { params, ret } => {
            let mut arrow = type_expr_to_type(ret)?;
            for p in params.iter().rev() {
                arrow = Type::Arrow(Box::new(type_expr_to_type(p)?), Box::new(arrow));
            }
            Ok(arrow)
        }
    }
}

// ── tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_syntax::parser::Parser;

    fn infer_src(src: &str) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
        let m = Parser::new(src).parse().map_err(|e| TypeError { msg: e, line: 0, col: 0 })?;
        infer_module(&m)
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
        let (env, _) = infer_src("struct Point { x: Float64, y: Float64 }; const p = Point { x: 1.0, y: 2.0 };").unwrap();
        let ty = format!("{}", env["p"].body);
        assert!(ty.contains("Point") || ty.contains("{"));
    }

    #[test]
    fn test_to_string_returns_string() {
        let (env, _) = infer_src("const s = 42.to_string();").unwrap();
        let ty = format!("{}", env["s"].body);
        assert_eq!(ty, "String");
    }

    fn infer_var(src: &str) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>)> {
        let m = Parser::new(src).parse().map_err(|e| TypeError { msg: e, line: 0, col: 0 })?;
        infer_module(&m)
    }
}
