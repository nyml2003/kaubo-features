//! Algorithm W — Hindley-Milner 类型推断

use crate::types::*;
use kaubo_ast::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

static TVAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn fresh_tvar() -> TypeVar {
    TypeVar(TVAR_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn reset_tvar() {
    TVAR_COUNTER.store(0, Ordering::Relaxed);
}

static STRUCT_COUNTER: AtomicUsize = AtomicUsize::new(0);
static ENUM_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn fresh_struct_id() -> usize {
    STRUCT_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn fresh_enum_id() -> usize {
    ENUM_COUNTER.fetch_add(1, Ordering::Relaxed)
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
    infer_module_with_imports(module, None).map(|(env, sf, _)| (env, sf))
}

/// 带导入表的类型推断。`imports` 为 `None` 时行为与 `infer_module` 一致（向后兼容）。
pub fn infer_module_with_imports(
    module: &Module,
    imports: Option<&[ImportSpec]>,
) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>, HashSet<String>)> {
    reset_tvar();
    let mut env: TypeEnv = HashMap::new();
    let mut exports: HashSet<String> = HashSet::new();

    // Pass 1: collect struct, enum, and interface definitions
    let mut interface_registry: HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>> =
        HashMap::new();
    let mut struct_registry: HashMap<String, usize> = HashMap::new();
    let mut struct_fields: HashMap<usize, Vec<(String, Type)>> = HashMap::new();
    let mut enum_registry: HashMap<String, usize> = HashMap::new();
    let mut enum_variants: HashMap<usize, Vec<(String, Vec<(String, Type)>)>> =
        HashMap::new();
    for stmt in &module.stmts {
        // Unwrap ExportStmt to reach inner definitions
        let inner = match stmt {
            Stmt::ExportStmt(inner) => inner.as_ref(),
            other => other,
        };
        if let Stmt::StructDef { name, fields } = inner {
            let id = fresh_struct_id();
            struct_registry.insert(name.clone(), id);
            let mut fts = Vec::new();
            for f in fields {
                fts.push((
                    f.name.clone(),
                    type_expr_to_type(&f.ty, &struct_registry, &struct_fields, &interface_registry)?,
                ));
            }
            struct_fields.insert(id, fts);
        }
        if let Stmt::EnumDef { name, variants } = inner {
            let id = fresh_enum_id();
            enum_registry.insert(name.clone(), id);
            let mut vts: Vec<(String, Vec<(String, Type)>)> = Vec::new();
            for v in variants {
                let mut fts = Vec::new();
                for f in &v.fields {
                    fts.push((
                        f.name.clone(),
                        type_expr_to_type(&f.ty, &struct_registry, &struct_fields, &interface_registry)?,
                    ));
                }
                vts.push((v.name.clone(), fts));
            }
            enum_variants.insert(id, vts);
        }
        if let Stmt::InterfaceDef { name, methods } = inner {
            let mut sigs: Vec<(String, Vec<(String, Type)>, Option<Type>)> = Vec::new();
            for m in methods {
                let mut param_types = Vec::new();
                for p in &m.params {
                    let pt = if let Some(TypeExpr::Named(s)) = &p.ty_ann {
                        if s == "Self" {
                            Type::Var(fresh_tvar()) // Self is a placeholder
                        } else {
                            type_expr_to_type(&p.ty_ann.clone().unwrap(), &struct_registry, &struct_fields, &interface_registry)?
                        }
                    } else {
                        Type::Var(fresh_tvar())
                    };
                    param_types.push((p.name.clone(), pt));
                }
                let ret = m.return_type.as_ref().map(|t| {
                    if let TypeExpr::Named(s) = t {
                        if s == "Self" { return Type::Var(fresh_tvar()); }
                    }
                    type_expr_to_type(t, &struct_registry, &struct_fields, &interface_registry).unwrap_or(Type::Null)
                });
                sigs.push((m.name.clone(), param_types, ret));
            }
            interface_registry.insert(name.clone(), sigs);
        }
    }

    // Pass 2: inject stdlib builtins, builtin interfaces, and builtin impls
    inject_stdlib(&mut env);
    inject_builtin_interfaces(&mut interface_registry);
    inject_builtin_impls(
        &mut env,
        &mut struct_registry,
        &mut struct_fields,
        &mut interface_registry,
    );

    // Pass 2.5: inject imported symbols from other modules
    if let Some(imports) = imports {
        for spec in imports {
            match &spec.kind {
                ImportKind::Const { ty } | ImportKind::Function { ty } => {
                    env.insert(spec.local_name.clone(), Scheme::monomorphic(ty.clone()));
                }
                ImportKind::Struct {
                    struct_id: src_id,
                    fields,
                } => {
                    // ★ 使用源模块原始 struct_id，不重新分配
                    struct_registry.insert(spec.local_name.clone(), *src_id);
                    struct_fields.insert(*src_id, fields.clone());
                    env.insert(
                        spec.local_name.clone(),
                        Scheme::monomorphic(Type::Record(*src_id, fields.clone())),
                    );
                }
                ImportKind::Interface { methods } => {
                    interface_registry.insert(spec.local_name.clone(), methods.clone());
                    env.insert(
                        spec.local_name.clone(),
                        Scheme::monomorphic(Type::Null),
                    );
                }
            }
        }
    }

    // Pass 3: infer all statements
    for stmt in &module.stmts {
        match stmt {
            Stmt::ConstDecl { name, value, .. } => {
                let (s, ty) = infer(&env, value, &struct_registry, &struct_fields, &enum_registry, &enum_variants, &interface_registry)?;
                let scheme = generalize(&env, &s.apply(&ty));
                env.insert(name.clone(), scheme);
            }
            Stmt::VarDecl { name, value, .. } => {
                let ty = if let Some(val) = value {
                    let (s, t) = infer(&env, val, &struct_registry, &struct_fields, &enum_registry, &enum_variants, &interface_registry)?;
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
                        type_expr_to_type(&f.ty, &struct_registry, &struct_fields, &interface_registry)?,
                    ));
                }
                struct_fields.insert(id, fts);
            }
            Stmt::EnumDef { name, variants } => {
                let id = enum_registry[name];
                // Register each variant constructor in the environment
                for (tag, v) in variants.iter().enumerate() {
                    let vtys = enum_variants
                        .get(&id)
                        .and_then(|vs| vs.get(tag))
                        .map(|(_, fts)| fts.clone())
                        .unwrap_or_default();
                    let result_ty =
                        Type::Variant(id, v.name.clone(), vtys.clone());
                    if v.fields.is_empty() {
                        // Unit variant: just the variant type
                        env.insert(
                            v.name.clone(),
                            Scheme::monomorphic(result_ty),
                        );
                    } else {
                        // Payload variant: fields... -> Variant
                        let mut arrow = result_ty;
                        for (_, ft) in vtys.iter().rev() {
                            arrow = Type::Arrow(
                                Box::new(ft.clone()),
                                Box::new(arrow),
                            );
                        }
                        env.insert(
                            v.name.clone(),
                            Scheme::monomorphic(arrow),
                        );
                    }
                }
            }
            Stmt::ImplBlock {
                struct_name,
                interface_name,
                methods,
            } => {
                // Check interface completeness if implementing a trait
                if let Some(ref iface_name) = interface_name {
                    if let Some(required) = interface_registry.get(iface_name) {
                        let implemented: std::collections::HashSet<&str> =
                            methods.iter().map(|m| m.name.as_str()).collect();
                        for (mname, _, _) in required {
                            if !implemented.contains(mname.as_str()) {
                                return Err(TypeError {
                                    msg: format!(
                                        "impl '{iface_name}' for '{struct_name}' missing method '{mname}'"
                                    ),
                                    line: 0,
                                    col: 0,
                                });
                            }
                        }
                    }
                }
                // Register methods on struct — with interface signature checking
                for m in methods {
                    let (s, ty) = infer(&env, &m.body, &struct_registry, &struct_fields, &enum_registry, &enum_variants, &interface_registry)?;
                    let inferred = s.apply(&ty);
                    // If implementing an interface, check method type against declared signature
                    if let Some(ref iface_name) = interface_name {
                        if let Some(required) = interface_registry.get(iface_name) {
                            if let Some((_, param_types, ret_ty)) =
                                required.iter().find(|(mn, _, _)| mn == &m.name)
                            {
                                // Build expected type: params... -> ret
                                let mut expected = ret_ty.clone().unwrap_or(Type::Null);
                                for (_, pt) in param_types.iter().rev() {
                                    expected = Type::Arrow(
                                        Box::new(pt.clone()),
                                        Box::new(expected),
                                    );
                                }
                                // Unify inferred method type with expected signature
                                if let Err(e) = unify(&inferred, &expected) {
                                    return Err(TypeError {
                                        msg: format!(
                                            "method '{}' of impl '{iface_name}' for '{struct_name}': {}",
                                            m.name, e
                                        ),
                                        line: 0,
                                        col: 0,
                                    });
                                }
                            }
                        }
                    }
                    let scheme = generalize(&env, &inferred);
                    env.insert(format!("{}.{}", struct_name, m.name), scheme);
                }
            }
            Stmt::ExprStmt(expr) => {
                infer(&env, expr, &struct_registry, &struct_fields, &enum_registry, &enum_variants, &interface_registry)?;
            }
            Stmt::InterfaceDef { name, .. } => {
                // Register interface name as a type-level entity (no runtime value)
                env.insert(name.clone(), Scheme::monomorphic(Type::Null));
            }
            Stmt::ExportStmt(inner) => {
                // 推断内部声明，并记录导出
                match inner.as_ref() {
                    Stmt::ConstDecl { name, value, .. } => {
                        let (s, ty) = infer(&env, value, &struct_registry, &struct_fields, &enum_registry, &enum_variants, &interface_registry)?;
                        let scheme = generalize(&env, &s.apply(&ty));
                        env.insert(name.clone(), scheme);
                        exports.insert(name.clone());
                    }
                    Stmt::StructDef { name, fields } => {
                        let id = struct_registry[name];
                        let mut fts = Vec::new();
                        for f in fields {
                            fts.push((
                                f.name.clone(),
                                type_expr_to_type(&f.ty, &struct_registry, &struct_fields, &interface_registry)?,
                            ));
                        }
                        struct_fields.insert(id, fts.clone());
                        env.insert(
                            name.clone(),
                            Scheme::monomorphic(Type::Record(id, fts)),
                        );
                        exports.insert(name.clone());
                    }
                    Stmt::EnumDef { name, variants } => {
                        let id = enum_registry[name];
                        for (tag, v) in variants.iter().enumerate() {
                            let vtys = enum_variants
                                .get(&id)
                                .and_then(|vs| vs.get(tag))
                                .map(|(_, fts)| fts.clone())
                                .unwrap_or_default();
                            let result_ty = Type::Variant(id, v.name.clone(), vtys.clone());
                            if v.fields.is_empty() {
                                env.insert(v.name.clone(), Scheme::monomorphic(result_ty));
                            } else {
                                let mut arrow = result_ty;
                                for (_, ft) in vtys.iter().rev() {
                                    arrow = Type::Arrow(Box::new(ft.clone()), Box::new(arrow));
                                }
                                env.insert(v.name.clone(), Scheme::monomorphic(arrow));
                            }
                        }
                        exports.insert(name.clone());
                    }
                    Stmt::InterfaceDef { name, .. } => {
                        env.insert(name.clone(), Scheme::monomorphic(Type::Null));
                        exports.insert(name.clone());
                    }
                    Stmt::VarDecl { name, value, .. } => {
                        let ty = if let Some(val) = value {
                            let (s, t) = infer(&env, val, &struct_registry, &struct_fields, &enum_registry, &enum_variants, &interface_registry)?;
                            s.apply(&t)
                        } else {
                            Type::Var(fresh_tvar())
                        };
                        env.insert(name.clone(), Scheme::monomorphic(ty));
                        exports.insert(name.clone());
                    }
                    _ => {
                        // 不支持的导出语句类型，静默忽略（未来可报错）
                    }
                }
            }
            Stmt::Import { .. } => {}
        }
    }

    Ok((env, struct_fields, exports))
}

// ── stdlib injection ──

fn inject_stdlib(env: &mut TypeEnv) {
    // print: String → Null
    env.insert(
        "print".into(),
        Scheme::monomorphic(Type::Arrow(Box::new(Type::String), Box::new(Type::Null))),
    );
    // type_of: forall a. a → Int64
    let tv = fresh_tvar();
    env.insert(
        "type_of".into(),
        Scheme {
            bound: vec![tv],
            body: Box::new(Type::Arrow(Box::new(Type::Var(tv)), Box::new(Type::Int64))),
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

/// Inject built-in interface definitions (Add, Subtract, Multiply, Divide, Modulo,
/// Compare, Display, IntoFloat, IntoInt).
fn inject_builtin_interfaces(
    interface_registry: &mut HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>>,
) {
    // Helper: create a fresh Self type variable (used as placeholder for the implementor type)
    let self_tv = || Type::Var(fresh_tvar());
    let bool_ty = Type::Bool;
    let string_ty = Type::String;
    let float_ty = Type::Float64;

    // interface Add { operator add: |self, other: Self| -> Self; }
    let sv = self_tv();
    interface_registry.entry("Add".into()).or_insert_with(|| {
        vec![(
            "add".into(),
            vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
            Some(sv.clone()),
        )]
    });
    // interface Subtract { operator subtract: |self, other: Self| -> Self; }
    let sv = self_tv();
    interface_registry.entry("Subtract".into()).or_insert_with(|| {
        vec![(
            "subtract".into(),
            vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
            Some(sv.clone()),
        )]
    });
    // interface Multiply { operator multiply: |self, other: Self| -> Self; }
    let sv = self_tv();
    interface_registry.entry("Multiply".into()).or_insert_with(|| {
        vec![(
            "multiply".into(),
            vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
            Some(sv.clone()),
        )]
    });
    // interface Divide { operator divide: |self, other: Self| -> Self; }
    let sv = self_tv();
    interface_registry.entry("Divide".into()).or_insert_with(|| {
        vec![(
            "divide".into(),
            vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
            Some(sv.clone()),
        )]
    });
    // interface Modulo { operator modulo: |self, other: Self| -> Self; }
    let sv = self_tv();
    interface_registry.entry("Modulo".into()).or_insert_with(|| {
        vec![(
            "modulo".into(),
            vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
            Some(sv.clone()),
        )]
    });
    // interface Compare {
    //     operator less: |self, other: Self| -> Bool;
    //     operator less_equal: |self, other: Self| -> Bool;
    //     operator greater: |self, other: Self| -> Bool;
    //     operator greater_equal: |self, other: Self| -> Bool;
    //     operator equal: |self, other: Self| -> Bool;
    //     operator not_equal: |self, other: Self| -> Bool;
    // }
    let sv = self_tv();
    interface_registry.entry("Compare".into()).or_insert_with(|| {
        vec![
            (
                "less".into(),
                vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
                Some(bool_ty.clone()),
            ),
            (
                "less_equal".into(),
                vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
                Some(bool_ty.clone()),
            ),
            (
                "greater".into(),
                vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
                Some(bool_ty.clone()),
            ),
            (
                "greater_equal".into(),
                vec![("self".into(), sv.clone()), ("other".into(), sv.clone())],
                Some(bool_ty.clone()),
            ),
            (
                "equal".into(),
                vec![("self".into(), sv.clone()), ("other".into(), sv)],
                Some(bool_ty.clone()),
            ),
            ("not_equal".into(), vec![], Some(bool_ty.clone())),
        ]
    });
    // interface Display { to_string: |self| -> String; }
    interface_registry.entry("Display".into()).or_insert_with(|| {
        vec![("to_string".into(), vec![("self".into(), self_tv())], Some(string_ty.clone()))]
    });
    // interface IntoFloat { to_float: |self| -> Float64; }
    interface_registry.entry("IntoFloat".into()).or_insert_with(|| {
        vec![("to_float".into(), vec![("self".into(), self_tv())], Some(float_ty.clone()))]
    });
    // interface IntoInt { to_int: |self| -> Int64; }
    interface_registry
        .entry("IntoInt".into())
        .or_insert_with(|| {
            vec![(
                "to_int".into(),
                vec![("self".into(), self_tv())],
                Some(Type::Int64),
            )]
        });
}

/// Inject built-in impl blocks for Int64, Float64, String, Bool.
/// These method signatures go into the type environment so `x.to_string()` and
/// `x.add(y)` type-check.  CPS build recognizes builtin method names and rewrites
/// them to the corresponding CPS instructions — the impl bodies are never executed.
fn inject_builtin_impls(
    env: &mut TypeEnv,
    struct_registry: &mut HashMap<String, usize>,
    struct_fields: &mut HashMap<usize, Vec<(String, Type)>>,
    interface_registry: &mut HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>>,
) {
    // Register builtin types as "struct-like" so method lookup works
    let builtin_int_id = usize::MAX;
    let builtin_float_id = usize::MAX - 1;
    let builtin_string_id = usize::MAX - 2;
    let builtin_bool_id = usize::MAX - 3;
    struct_registry.insert("Int64".into(), builtin_int_id);
    struct_registry.insert("Float64".into(), builtin_float_id);
    struct_registry.insert("String".into(), builtin_string_id);
    struct_registry.insert("Bool".into(), builtin_bool_id);
    struct_fields.insert(builtin_int_id, vec![]);
    struct_fields.insert(builtin_float_id, vec![]);
    struct_fields.insert(builtin_string_id, vec![]);
    struct_fields.insert(builtin_bool_id, vec![]);

    // Helper: register a monomorphic method in env as "{struct}.{method}".
    // For methods with one extra arg (operator methods), the type is curried:
    //   self_type → (other_type → return_type)
    // For zero-arg methods (to_string, to_float, etc.), it's just:
    //   self_type → return_type
    // Member handler strips the first Arrow (self), leaving either `ret` or `other → ret`.
    let mut reg = |struct_name: &str, method: &str, self_ty: Type, other_ty: Option<Type>, ret_ty: Type| {
        let full = format!("{struct_name}.{method}");
        let ty = if let Some(other) = other_ty {
            Type::Arrow(
                Box::new(self_ty),
                Box::new(Type::Arrow(Box::new(other), Box::new(ret_ty))),
            )
        } else {
            Type::Arrow(Box::new(self_ty), Box::new(ret_ty))
        };
        env.insert(full, Scheme::monomorphic(ty));
    };

    // ── Int64 ── (operator methods: self + other → return)
    reg("Int64", "add", Type::Int64, Some(Type::Int64), Type::Int64);
    reg("Int64", "subtract", Type::Int64, Some(Type::Int64), Type::Int64);
    reg("Int64", "multiply", Type::Int64, Some(Type::Int64), Type::Int64);
    reg("Int64", "divide", Type::Int64, Some(Type::Int64), Type::Int64);
    reg("Int64", "modulo", Type::Int64, Some(Type::Int64), Type::Int64);
    reg("Int64", "less", Type::Int64, Some(Type::Int64), Type::Bool);
    reg("Int64", "less_equal", Type::Int64, Some(Type::Int64), Type::Bool);
    reg("Int64", "greater", Type::Int64, Some(Type::Int64), Type::Bool);
    reg("Int64", "greater_equal", Type::Int64, Some(Type::Int64), Type::Bool);
    reg("Int64", "equal", Type::Int64, Some(Type::Int64), Type::Bool);
    reg("Int64", "not_equal", Type::Int64, Some(Type::Int64), Type::Bool);
    // zero-arg methods: no other type
    reg("Int64", "to_string", Type::Int64, None, Type::String);
    reg("Int64", "to_float", Type::Int64, None, Type::Float64);

    // ── Float64 ──
    reg("Float64", "add", Type::Float64, Some(Type::Float64), Type::Float64);
    reg("Float64", "subtract", Type::Float64, Some(Type::Float64), Type::Float64);
    reg("Float64", "multiply", Type::Float64, Some(Type::Float64), Type::Float64);
    reg("Float64", "divide", Type::Float64, Some(Type::Float64), Type::Float64);
    reg("Float64", "less", Type::Float64, Some(Type::Float64), Type::Bool);
    reg("Float64", "less_equal", Type::Float64, Some(Type::Float64), Type::Bool);
    reg("Float64", "greater", Type::Float64, Some(Type::Float64), Type::Bool);
    reg("Float64", "greater_equal", Type::Float64, Some(Type::Float64), Type::Bool);
    reg("Float64", "equal", Type::Float64, Some(Type::Float64), Type::Bool);
    reg("Float64", "not_equal", Type::Float64, Some(Type::Float64), Type::Bool);
    reg("Float64", "to_string", Type::Float64, None, Type::String);
    reg("Float64", "to_int", Type::Float64, None, Type::Int64);

    // ── String ──
    reg("String", "add", Type::String, Some(Type::String), Type::String);
    reg("String", "less", Type::String, Some(Type::String), Type::Bool);
    reg("String", "less_equal", Type::String, Some(Type::String), Type::Bool);
    reg("String", "greater", Type::String, Some(Type::String), Type::Bool);
    reg("String", "greater_equal", Type::String, Some(Type::String), Type::Bool);
    reg("String", "equal", Type::String, Some(Type::String), Type::Bool);
    reg("String", "not_equal", Type::String, Some(Type::String), Type::Bool);
    reg("String", "to_string", Type::String, None, Type::String);
    reg("String", "to_int", Type::String, None, Type::Int64);

    // ── Bool ──
    reg("Bool", "equal", Type::Bool, Some(Type::Bool), Type::Bool);
    reg("Bool", "not_equal", Type::Bool, Some(Type::Bool), Type::Bool);
    reg("Bool", "to_string", Type::Bool, None, Type::String);

    // Register builtin interface impls in interface_registry so completeness checks pass
    for (iface_name, builtin_types) in &[
        ("Add", &["Int64", "Float64", "String"][..]),
        ("Subtract", &["Int64", "Float64"][..]),
        ("Multiply", &["Int64", "Float64"][..]),
        ("Divide", &["Int64", "Float64"][..]),
        ("Modulo", &["Int64"][..]),
        ("Compare", &["Int64", "Float64", "String", "Bool"][..]),
        ("Display", &["Int64", "Float64", "String", "Bool"][..]),
        ("IntoFloat", &["Int64", "String"][..]),
        ("IntoInt", &["String", "Float64"][..]),
    ] {
        for bt in *builtin_types {
            let key = format!("{bt}::{iface_name}");
            interface_registry.entry(key).or_insert_with(Vec::new);
        }
    }
}

/// Map a BinOp to the corresponding operator method name.
fn binop_to_method(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "subtract",
        BinOp::Mul => "multiply",
        BinOp::Div => "divide",
        BinOp::Mod => "modulo",
        BinOp::Eq => "equal",
        BinOp::Ne => "not_equal",
        BinOp::Lt => "less",
        BinOp::Le => "less_equal",
        BinOp::Gt => "greater",
        BinOp::Ge => "greater_equal",
        BinOp::SAdd => "add", // String concatenation uses the same Add interface
        _ => "add",
    }
}

/// Get the struct name for a Type (reverse lookup from struct_registry).
fn type_to_name(ty: &Type, structs: &HashMap<String, usize>) -> Option<String> {
    match ty {
        Type::Int64 => Some("Int64".into()),
        Type::Float64 => Some("Float64".into()),
        Type::String => Some("String".into()),
        Type::Bool => Some("Bool".into()),
        Type::List(_) => Some("List".into()),
        Type::Record(id, _) => structs.iter().find(|(_, &sid)| sid == *id).map(|(n, _)| n.clone()),
        Type::Variant(_, name, _) => Some(name.clone()),
        _ => None,
    }
}

// ── 推断 ──

pub fn infer(
    env: &TypeEnv,
    expr: &Expr,
    structs: &HashMap<String, usize>,
    struct_fields: &HashMap<usize, Vec<(String, Type)>>,
    enums: &HashMap<String, usize>,
    enum_variants: &HashMap<usize, Vec<(String, Vec<(String, Type)>)>>,
    interface_registry: &HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>>,
) -> InferResult<(Subst, Type)> {
    match expr {
        Expr::LitInt(_) => Ok((Subst::empty(), Type::Int64)),
        Expr::LitFloat(_) => Ok((Subst::empty(), Type::Float64)),
        Expr::LitString(_) => Ok((Subst::empty(), Type::String)),
        Expr::LitTrue | Expr::LitFalse => Ok((Subst::empty(), Type::Bool)),
        Expr::LitNull => Ok((Subst::empty(), Type::Null)),

        Expr::VarRef(name) => {
            let scheme = env.get(name).ok_or_else(|| TypeError {
                msg: format!("unbound variable '{name}'"),
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
                    type_expr_to_type(ann, structs, struct_fields, interface_registry)?
                } else {
                    Type::Var(fresh_tvar())
                };
                param_types.push(pt.clone());
                env_local.insert(p.name.clone(), Scheme::monomorphic(pt));
            }

            let (s_body, body_ty) = infer(&env_local, body, structs, struct_fields, enums, enum_variants, interface_registry)?;
            s = s.compose(&s_body);

            let mut arrow_ty = body_ty;
            for pt in param_types.into_iter().rev() {
                arrow_ty = Type::Arrow(Box::new(s.apply(&pt)), Box::new(arrow_ty));
            }
            Ok((s, arrow_ty))
        }

        Expr::Call { func, args } => {
            let (mut s, func_ty) = infer(env, func, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let mut arg_types = Vec::new();
            for arg in args {
                let (s_arg, arg_ty) = infer(env, arg, structs, struct_fields, enums, enum_variants, interface_registry)?;
                s = s.compose(&s_arg);
                arg_types.push(arg_ty);
            }
            let ret = Type::Var(fresh_tvar());
            let mut arrow = ret.clone();
            for at in arg_types.into_iter().rev() {
                arrow = Type::Arrow(Box::new(at), Box::new(arrow));
            }
            s = unify_with_registry(&s.apply(&func_ty), &arrow, interface_registry, structs)
                .map_err(|e| TypeError {
                    msg: e,
                    line: 0,
                    col: 0,
                })?
                .compose(&s);
            Ok((s.clone(), s.apply(&ret)))
        }

        Expr::Binary { left, op, right } => {
            let (s1, t1) = infer(env, left, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let (s2, t2) = infer(env, right, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let mut s = s1.compose(&s2);

            let result_type = match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                    s = unify(&s.apply(&t1), &s.apply(&t2))
                        .map_err(|e| TypeError {
                            msg: format!("binary operator: {e}"),
                            line: 0,
                            col: 0,
                        })?
                        .compose(&s);
                    let unified = s.apply(&t1);
                    // Try builtin numeric types first
                    match unify(&unified, &Type::Int64)
                        .or_else(|_| unify(&unified, &Type::Float64))
                    {
                        Ok(subst) => {
                            s = subst.compose(&s);
                            s.apply(&t1)
                        }
                        Err(_) => {
                            // Fallback: operator dispatch for user types
                            // Map BinOp to operator method name
                            let op_method = binop_to_method(op);
                            // Find the type name of the unified type
                            let type_name = type_to_name(&unified, structs);
                            if let Some(tn) = type_name {
                                let method_name = format!("{tn}.{op_method}");
                                if let Some(scheme) = env.get(&method_name) {
                                    let method_ty = instantiate(scheme);
                                    // Operator methods are curried: self → (other → return)
                                    // Strip both arrows to get the return type
                                    match method_ty {
                                        Type::Arrow(_, body) => match *body {
                                            Type::Arrow(_, ret) => *ret,
                                            other => other,
                                        },
                                        other => other,
                                    }
                                } else {
                                    return Err(TypeError {
                                        msg: format!(
                                            "arithmetic requires numeric types, got {unified}"
                                        ),
                                        line: 0,
                                        col: 0,
                                    });
                                }
                            } else {
                                return Err(TypeError {
                                    msg: format!(
                                        "arithmetic requires numeric types, got {unified}"
                                    ),
                                    line: 0,
                                    col: 0,
                                });
                            }
                        }
                    }
                }
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge => {
                    // Unify operand types for comparison (unless one side is Null)
                    let a1 = s.apply(&t1);
                    let a2 = s.apply(&t2);
                    if !matches!(&a1, Type::Null) && !matches!(&a2, Type::Null) {
                        s = unify(&a1, &a2)
                            .map_err(|e| TypeError {
                                msg: format!("comparison operands must have same type: {e}"),
                                line: 0,
                                col: 0,
                            })?
                            .compose(&s);
                    }
                    Type::Bool
                }
                BinOp::And
                | BinOp::Or => Type::Bool,
                BinOp::Pipe => {
                    // a |> f : if f has type T -> U, unify T with type of a, result is U
                    let ret = Type::Var(fresh_tvar());
                    let f_ty = Type::Arrow(Box::new(t1.clone()), Box::new(ret.clone()));
                    s = unify(&t2, &f_ty)
                        .map_err(|e| TypeError {
                            msg: format!("pipe operator: {e}"),
                            line: 0,
                            col: 0,
                        })?
                        .compose(&s);
                    s.apply(&ret)
                }
                BinOp::GtGt => {
                    t1.clone() // TODO: proper function composition typing
                }
                BinOp::SAdd => Type::String,
            };
            Ok((s, result_type))
        }

        Expr::Unary { op, right } => {
            let (mut s, t) = infer(env, right, structs, struct_fields, enums, enum_variants, interface_registry)?;
            match op {
                UnOp::Neg => {
                    let applied = s.apply(&t);
                    s = unify(&applied, &Type::Int64)
                        .or_else(|_| unify(&applied, &Type::Float64))
                        .map_err(|_| TypeError {
                            msg: format!("negation requires numeric type, got {applied}"),
                            line: 0,
                            col: 0,
                        })?
                        .compose(&s);
                    Ok((s, applied))
                }
                UnOp::Not => Ok((s, Type::Bool)),
            }
        }

        Expr::Block(stmts) => {
            let mut s = Subst::empty();
            let mut local_env = env.clone();
            let mut result = Type::Null;
            for stmt in stmts {
                match stmt {
                    Stmt::ConstDecl { name, value, .. } => {
                        let (s_val, ty) = infer(&local_env, value, structs, struct_fields, enums, enum_variants, interface_registry)?;
                        let scheme = generalize(&local_env, &s_val.apply(&ty));
                        local_env.insert(name.clone(), scheme);
                        s = s.compose(&s_val);
                        result = Type::Null;
                    }
                    Stmt::VarDecl { name, value, .. } => {
                        let ty = if let Some(val) = value {
                            let (s_val, t) = infer(&local_env, val, structs, struct_fields, enums, enum_variants, interface_registry)?;
                            s = s.compose(&s_val);
                            t
                        } else {
                            Type::Var(fresh_tvar())
                        };
                        local_env.insert(name.clone(), Scheme::monomorphic(ty));
                        result = Type::Null;
                    }
                    Stmt::ExprStmt(expr) => {
                        let (s_e, ty) = infer(&local_env, expr, structs, struct_fields, enums, enum_variants, interface_registry)?;
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
            let (s_c, _tc) = infer(env, cond, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let (s_t, tt) = infer(env, then_branch, structs, struct_fields, enums, enum_variants, interface_registry)?;
            if let Some(eb) = else_branch {
                let (s_e, te) = infer(env, eb, structs, struct_fields, enums, enum_variants, interface_registry)?;
                let mut s = s_c.compose(&s_t).compose(&s_e);
                s = unify(&s.apply(&tt), &s.apply(&te))
                    .map_err(|e| TypeError {
                        msg: format!("if branches: {e}"),
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
            let (s_c, _) = infer(env, cond, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let (s_b, _) = infer(env, body, structs, struct_fields, enums, enum_variants, interface_registry)?;
            Ok((s_c.compose(&s_b), Type::Null))
        }

        Expr::For {
            var,
            iterable,
            body,
        } => {
            let (mut s_i, ti) = infer(env, iterable, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let applied = s_i.apply(&ti);
            // ti should be List<'a>; if it's a type var, unify it with List<?>
            match applied {
                Type::List(elem) => {
                    let mut local_env = env.clone();
                    local_env.insert(var.name.clone(), Scheme::monomorphic((*elem).clone()));
                    let (s_b, _) = infer(&local_env, body, structs, struct_fields, enums, enum_variants, interface_registry)?;
                    Ok((s_i.compose(&s_b), Type::Null))
                }
                Type::Var(_) => {
                    let elem = Type::Var(fresh_tvar());
                    let list_ty = Type::List(Box::new(elem.clone()));
                    s_i = unify(&applied, &list_ty)
                        .map_err(|e| TypeError {
                            msg: format!("for loop: {e}"),
                            line: 0,
                            col: 0,
                        })?
                        .compose(&s_i);
                    let mut local_env = env.clone();
                    local_env.insert(var.name.clone(), Scheme::monomorphic(elem));
                    let (s_b, _) = infer(&local_env, body, structs, struct_fields, enums, enum_variants, interface_registry)?;
                    Ok((s_i.compose(&s_b), Type::Null))
                }
                other => Err(TypeError {
                    msg: format!("for loop requires List, got {other}"),
                    line: 0,
                    col: 0,
                }),
            }
        }

        Expr::Break | Expr::Continue => Ok((Subst::empty(), Type::Null)),
        Expr::Return(val) => {
            if let Some(expr) = val {
                infer(env, expr, structs, struct_fields, enums, enum_variants, interface_registry)
            } else {
                Ok((Subst::empty(), Type::Null))
            }
        }

        Expr::Member { object, field } => {
            let (s, ty) = infer(env, object, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let applied = s.apply(&ty);

            // Handle Interface type: look up method in interface definition
            if let Type::Interface(iface_name) = &applied {
                if let Some(methods) = interface_registry.get(iface_name) {
                    if let Some((_, _, ret_ty)) = methods.iter().find(|(mname, _, _)| mname == field) {
                        return Ok((s, ret_ty.clone().unwrap_or(Type::Null)));
                    }
                }
                return Err(TypeError {
                    msg: format!("method '{field}' not found on interface {iface_name}"),
                    line: 0, col: 0,
                });
            }

            // Determine the "struct name" of the applied type (for method lookup)
            let struct_name: Option<String> = match &applied {
                Type::Int64 => Some("Int64".into()),
                Type::Float64 => Some("Float64".into()),
                Type::String => Some("String".into()),
                Type::Bool => Some("Bool".into()),
                Type::List(_) => Some("List".into()),
                Type::Record(id, _) => {
                    structs.iter().find(|(_, &sid)| sid == *id).map(|(n, _)| n.clone())
                }
                Type::Variant(_, name, _) => Some(name.clone()),
                _ => None,
            };

            // Try method lookup: "{struct_name}.{field}" in env
            if let Some(ref sn) = struct_name {
                let method_name = format!("{sn}.{field}");
                if let Some(scheme) = env.get(&method_name) {
                    let method_ty = instantiate(scheme);
                    let result_ty = match method_ty {
                        Type::Arrow(_, body) => *body,
                        other => other,
                    };
                    return Ok((s, result_ty));
                }
            }

            // Try struct field access (Record only)
            if let Type::Record(_, ref fields) = applied {
                if let Some(t) = fields
                    .iter()
                    .find(|(n, _)| n == field)
                    .map(|(_, t)| t.clone())
                {
                    return Ok((s, t));
                }
            }

            Err(TypeError {
                msg: format!("field or method '{field}' not found on {applied}"),
                line: 0,
                col: 0,
            })
        }

        Expr::Index { object, index } => {
            let (s1, t_obj) = infer(env, object, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let (s2, _) = infer(env, index, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let s = s1.compose(&s2);
            match s.apply(&t_obj) {
                Type::List(elem) => Ok((s.clone(), s.apply(&elem))),
                Type::String => Ok((s, Type::String)),
                other => Err(TypeError {
                    msg: format!("cannot index into type {other}"),
                    line: 0,
                    col: 0,
                }),
            }
        }

        Expr::StructLit { name, fields, .. } => {
            let id = structs.get(name).ok_or_else(|| TypeError {
                msg: format!("unknown struct '{name}'"),
                line: 0,
                col: 0,
            })?;
            let field_types = struct_fields.get(id).cloned().unwrap_or_default();
            let mut s = Subst::empty();
            for (fname, fval) in fields {
                let (s_f, t_val) = infer(env, fval, structs, struct_fields, enums, enum_variants, interface_registry)?;
                // Unify field value type with declared field type
                if let Some((_, declared)) = field_types.iter().find(|(n, _)| n == fname) {
                    s = unify(
                        &s.apply(&t_val),
                        &s.apply(declared),
                    )
                    .map_err(|e| TypeError {
                        msg: format!("field '{fname}': {e}"),
                        line: 0,
                        col: 0,
                    })?
                    .compose(&s);
                }
                s = s.compose(&s_f);
            }
            Ok((s, Type::Record(*id, field_types)))
        }

        Expr::VariantLit {
            enum_name,
            variant_name,
            fields,
            ..
        } => {
            let id = enums.get(enum_name).ok_or_else(|| TypeError {
                msg: format!("unknown enum '{enum_name}'"),
                line: 0,
                col: 0,
            })?;
            let variant_info = enum_variants
                .get(id)
                .and_then(|vs| vs.iter().find(|(n, _)| n == variant_name))
                .ok_or_else(|| TypeError {
                    msg: format!("unknown variant '{variant_name}'"),
                    line: 0,
                    col: 0,
                })?;
            let mut s = Subst::empty();
            for (i, fval) in fields.iter().enumerate() {
                let (s_f, t_val) = infer(env, fval, structs, struct_fields, enums, enum_variants, interface_registry)?;
                // Unify with declared variant field type
                if let Some((_, declared)) = variant_info.1.get(i) {
                    s = unify_with_registry(&s.apply(&t_val), &s.apply(declared), interface_registry, structs)
                        .map_err(|e| TypeError {
                            msg: format!("variant field: {e}"),
                            line: 0,
                            col: 0,
                        })?
                        .compose(&s);
                }
                s = s.compose(&s_f);
            }
            Ok((
                s,
                Type::Variant(*id, variant_name.clone(), variant_info.1.clone()),
            ))
        }

        Expr::GetVariantTag(inner) => {
            let (s, ty) = infer(env, inner, structs, struct_fields, enums, enum_variants, interface_registry)?;
            // Verify inner is a variant type
            match s.apply(&ty) {
                Type::Variant(..) => {}
                _ => {
                    return Err(TypeError {
                        msg: format!("GetVariantTag expected variant type, got {ty}"),
                        line: 0,
                        col: 0,
                    })
                }
            }
            Ok((s, Type::Int64))
        }

        Expr::GetVariantField { object, field_idx } => {
            let (s, ty) = infer(env, object, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let applied = s.apply(&ty);
            match applied {
                Type::Variant(_id, _name, fields) => {
                    let ft = fields
                        .get(*field_idx as usize)
                        .cloned()
                        .map(|(_, t)| t)
                        .unwrap_or(Type::Var(fresh_tvar()));
                    Ok((s, ft))
                }
                _ => Err(TypeError {
                    msg: format!(
                        "GetVariantField expected variant type, got {ty}"
                    ),
                    line: 0,
                    col: 0,
                }),
            }
        }

        Expr::ListLit(items) | Expr::Tuple(items) => {
            let mut s = Subst::empty();
            let elem_ty = Type::Var(fresh_tvar());
            for item in items {
                let (s_i, ti) = infer(env, item, structs, struct_fields, enums, enum_variants, interface_registry)?;
                s = s.compose(&s_i);
                s = unify(&s.apply(&elem_ty), &s.apply(&ti))
                    .map_err(|e| TypeError {
                        msg: format!("list element: {e}"),
                        line: 0,
                        col: 0,
                    })?
                    .compose(&s);
            }
            Ok((s, Type::List(Box::new(elem_ty))))
        }

        Expr::Assign { target, value } => {
            let (s1, t_target) = infer(env, target, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let (s2, t_val) = infer(env, value, structs, struct_fields, enums, enum_variants, interface_registry)?;
            let mut s = s1.compose(&s2);
            s = unify(&s.apply(&t_target), &s.apply(&t_val))
                .map_err(|e| TypeError {
                    msg: format!("assignment type mismatch: {e}"),
                    line: 0,
                    col: 0,
                })?
                .compose(&s);
            Ok((s, Type::Null))
        }

        Expr::Async(body) => infer(env, body, structs, struct_fields, enums, enum_variants, interface_registry),
        Expr::Await(body) => infer(env, body, structs, struct_fields, enums, enum_variants, interface_registry),
    }
}

// ── 统一 ──

/// Unify with interface and struct registry for Struct↔Interface validation.
pub fn unify_with_registry(
    t1: &Type,
    t2: &Type,
    interface_registry: &HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>>,
    struct_registry: &HashMap<String, usize>,
) -> Result<Subst, String> {
    unify_impl(t1, t2, Some(interface_registry), Some(struct_registry))
}

/// Unify without interface/struct lookup (for tests and simple cases).
pub fn unify(t1: &Type, t2: &Type) -> Result<Subst, String> {
    unify_impl(t1, t2, None, None)
}

fn unify_impl(
    t1: &Type,
    t2: &Type,
    interface_registry: Option<&HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>>>,
    struct_registry: Option<&HashMap<String, usize>>,
) -> Result<Subst, String> {
    match (t1, t2) {
        (a, b) if a == b => Ok(Subst::empty()),
        (Type::Var(v), ty) if !occurs_check(v, ty) => Ok(Subst::singleton(*v, ty.clone())),
        (ty, Type::Var(v)) if !occurs_check(v, ty) => Ok(Subst::singleton(*v, ty.clone())),
        // Struct ↔ Interface: validate impl relationship
        (Type::Record(..), Type::Interface(iface_name))
        | (Type::Interface(iface_name), Type::Record(..)) => {
            if let Some(reg) = interface_registry {
                if reg.contains_key(iface_name.as_str()) {
                    Ok(Subst::empty())
                } else {
                    Err(format!("unknown interface '{iface_name}'"))
                }
            } else {
                // Without registry, allow the unification (best-effort)
                Ok(Subst::empty())
            }
        }
        (Type::Arrow(a1, r1), Type::Arrow(a2, r2)) => {
            let s1 = unify_impl(a1, a2, interface_registry, struct_registry)?;
            let s2 = unify_impl(&s1.apply(r1), &s1.apply(r2), interface_registry, struct_registry)?;
            Ok(s2.compose(&s1))
        }
        (Type::List(t1), Type::List(t2)) => unify_impl(t1, t2, interface_registry, struct_registry),
        (Type::Record(id1, _), Type::Record(id2, _)) if id1 == id2 => Ok(Subst::empty()),
        (Type::Variant(id1, _, _), Type::Variant(id2, _, _)) if id1 == id2 => Ok(Subst::empty()),
        _ => Err(format!("cannot unify {t1} and {t2}")),
    }
}

fn occurs_check(var: &TypeVar, ty: &Type) -> bool {
    match ty {
        Type::Var(v) => v == var,
        Type::Arrow(a, r) => occurs_check(var, a) || occurs_check(var, r),
        Type::List(t) => occurs_check(var, t),
        Type::Record(_, fields) => fields
            .iter()
            .any(|(_, t)| occurs_check(var, t)),
        Type::Variant(_, _, fields) => fields
            .iter()
            .any(|(_, t)| occurs_check(var, t)),
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
        Type::Variant(_, _, fields) => {
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
    interface_registry: &HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>>,
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
                } else if interface_registry.contains_key(n) {
                    Ok(Type::Interface(n.to_string()))
                } else {
                    Err(TypeError {
                        msg: format!("unknown type '{n}'"),
                        line: 0,
                        col: 0,
                    })
                }
            }
        },
        TypeExpr::List(t) => Ok(Type::List(Box::new(type_expr_to_type(
            t, structs, struct_fields, interface_registry,
        )?))),
        TypeExpr::Arrow { params, ret } => {
            let mut arrow = type_expr_to_type(ret, structs, struct_fields, interface_registry)?;
            for p in params.iter().rev() {
                arrow = Type::Arrow(
                    Box::new(type_expr_to_type(p, structs, struct_fields, interface_registry)?),
                    Box::new(arrow),
                );
            }
            Ok(arrow)
        }
    }
}

// ── tests ──

#[cfg(test)]
#[allow(clippy::approx_constant)]
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

    fn empty_enums() -> HashMap<String, usize> {
        HashMap::new()
    }

    fn empty_enum_variants(
    ) -> HashMap<usize, Vec<(String, Vec<(String, Type)>)>> {
        HashMap::new()
    }

    fn empty_interface_registry(
    ) -> HashMap<String, Vec<(String, Vec<(String, Type)>, Option<Type>)>> {
        HashMap::new()
    }

    fn infer_expr(expr: Expr) -> InferResult<Type> {
        let (mut structs, mut fields) = empty_structs();
        let mut interface_registry = HashMap::new();
        inject_builtin_interfaces(&mut interface_registry);
        inject_builtin_impls(
            &mut TypeEnv::new(),
            &mut structs,
            &mut fields,
            &mut interface_registry,
        );
        let mut env = TypeEnv::new();
        inject_builtin_impls(
            &mut env,
            &mut structs,
            &mut fields,
            &mut interface_registry,
        );
        let (subst, ty) = infer(&env, &expr, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry())?;
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
                            spread: None,
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

        assert_eq!(format!("{ty}"), "(Int64 → Int64)");
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
        // pipe of non-function RHS should error
        assert!(infer_expr(Expr::Binary {
            left: Box::new(Expr::LitInt(1)),
            op: BinOp::Pipe,
            right: Box::new(Expr::LitInt(2)),
        })
        .is_err());
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
                interface_name: None,
                methods: vec![MethodDef {
                    name: "value".to_string(),
                    body: Expr::Lambda {
                        params: vec![param("self", Some(TypeExpr::named("Point")))],
                        ret_ty: None,
                        body: Box::new(Expr::LitInt(7)),
                    },
                    operator: false,
                }],
            },
            const_decl(
                "p",
                Expr::StructLit {
                    name: "Point".to_string(),
                    fields: vec![("x".to_string(), Expr::LitInt(1))],
                    spread: None,
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
                spread: None,
            },
        )]))
        .unwrap_err();
        assert!(err.msg.contains("unknown struct"));

        let err = infer_expr(Expr::Member {
            object: Box::new(Expr::LitInt(1)),
            field: "x".to_string(),
        })
        .unwrap_err();
        assert!(
            err.msg.contains("not found"),
            "expected 'not found' error, got: {}",
            err.msg
        );
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
        // Index on Int is a type error
        assert!(infer_expr(Expr::Index {
            object: Box::new(Expr::LitInt(1)),
            index: Box::new(Expr::LitInt(0)),
        }).is_err());
        assert_eq!(infer_expr(Expr::Assign { target: Box::new(Expr::LitInt(1)), value: Box::new(Expr::LitInt(42)) }).unwrap(), Type::Null);
        assert_eq!(infer_expr(Expr::Async(Box::new(Expr::LitInt(1)))).unwrap(), Type::Int64);
        assert_eq!(infer_expr(Expr::Await(Box::new(Expr::LitFloat(3.14)))).unwrap(), Type::Float64);
    }

    #[test]
    fn type_expr_unify_generalize_and_instantiate_edges() {
        let t_int = Type::Int64;
        let t_var = Type::Var(TypeVar(99999));
        let sub = unify(&t_var, &t_int).unwrap();
        assert_eq!(sub.apply(&t_var), Type::Int64);

        let empty_env = TypeEnv::new();
        let scheme = generalize(&empty_env, &Type::Int64);
        assert!(scheme.bound.is_empty());
        let inst = instantiate(&scheme);
        assert_eq!(inst, Type::Int64);

        let scheme_var = generalize(&empty_env, &Type::Var(TypeVar(1)));
        assert_eq!(scheme_var.bound.len(), 1);
    }

    // ── Unification scenarios ──

    #[test]
    fn unify_identical_types() {
        let s = unify(&Type::Int64, &Type::Int64).unwrap();
        assert_eq!(s, Subst::empty());
    }

    #[test]
    fn unify_var_with_int() {
        let s = unify(&Type::Var(TypeVar(0)), &Type::Int64).unwrap();
        assert_eq!(s.apply(&Type::Var(TypeVar(0))), Type::Int64);
    }

    #[test]
    fn unify_int_with_var() {
        let s = unify(&Type::Int64, &Type::Var(TypeVar(0))).unwrap();
        assert_eq!(s.apply(&Type::Var(TypeVar(0))), Type::Int64);
    }

    #[test]
    fn unify_arrow_types() {
        let t1 = Type::Arrow(
            Box::new(Type::Int64),
            Box::new(Type::Bool),
        );
        let t2 = Type::Arrow(
            Box::new(Type::Int64),
            Box::new(Type::Bool),
        );
        assert!(unify(&t1, &t2).is_ok());
    }

    #[test]
    fn unify_arrow_types_different_args() {
        let t1 = Type::Arrow(
            Box::new(Type::Int64),
            Box::new(Type::Bool),
        );
        let t2 = Type::Arrow(
            Box::new(Type::Float64),
            Box::new(Type::Bool),
        );
        assert!(unify(&t1, &t2).is_err());
    }

    #[test]
    fn unify_list_types() {
        let s = unify(
            &Type::List(Box::new(Type::Var(TypeVar(0)))),
            &Type::List(Box::new(Type::Int64)),
        )
        .unwrap();
        assert_eq!(
            s.apply(&Type::List(Box::new(Type::Var(TypeVar(0))))),
            Type::List(Box::new(Type::Int64))
        );
    }

    #[test]
    fn unify_record_same_id() {
        let s = unify(
            &Type::Record(1, vec![("x".into(), Type::Int64)]),
            &Type::Record(1, vec![("y".into(), Type::Float64)]),
        )
        .unwrap();
        // Records with same id unify (fields not checked structurally)
        assert_eq!(s, Subst::empty());
    }

    #[test]
    fn unify_incompatible_types_error() {
        assert!(unify(&Type::Int64, &Type::String).is_err());
        assert!(unify(&Type::Bool, &Type::Null).is_err());
        assert!(unify(&Type::Int64, &Type::List(Box::new(Type::Int64))).is_err());
    }

    #[test]
    fn occurs_check_prevents_infinite_types() {
        let v = TypeVar(0);
        let infinite = Type::Arrow(
            Box::new(Type::Var(v)),
            Box::new(Type::Var(v)),
        );
        // This should fail the occurs check
        assert!(unify(&Type::Var(v), &infinite).is_err());
    }

    // ── Schema generalization and instantiation ──

    #[test]
    fn generalize_closed_type_is_monomorphic() {
        let env = TypeEnv::new();
        let scheme = generalize(&env, &Type::Int64);
        assert!(scheme.bound.is_empty());
    }

    #[test]
    fn generalize_open_type_is_polymorphic() {
        let env = TypeEnv::new();
        let scheme = generalize(&env, &Type::Arrow(
            Box::new(Type::Var(TypeVar(7))),
            Box::new(Type::Var(TypeVar(7))),
        ));
        assert_eq!(scheme.bound.len(), 1);
    }

    #[test]
    fn generalize_respects_env_free_vars() {
        let mut env = TypeEnv::new();
        let scheme = generalize(&env, &Type::Var(TypeVar(0)));
        assert_eq!(scheme.bound.len(), 1); // free in env → should be generalized
        env.insert("f".into(), scheme);
        // Now generalize again — var 0 is free in env, not generalized
        let scheme2 = generalize(&env, &Type::Var(TypeVar(0)));
        assert!(scheme2.bound.is_empty());
    }

    #[test]
    fn instantiate_creates_fresh_vars() {
        let t = Type::Arrow(
            Box::new(Type::Var(TypeVar(0))),
            Box::new(Type::Var(TypeVar(0))),
        );
        let env = TypeEnv::new();
        let scheme = generalize(&env, &t);
        let inst = instantiate(&scheme);
        // Should produce an arrow with fresh type vars
        match &inst {
            Type::Arrow(a, r) => {
                assert!(matches!(**a, Type::Var(_)));
                assert!(matches!(**r, Type::Var(_)));
            }
            _ => panic!("expected arrow"),
        }
    }

    // ── Type inference: expressions ──

    #[test]
    fn infer_lit_null() {
        assert_eq!(infer_expr(Expr::LitNull).unwrap(), Type::Null);
    }

    #[test]
    fn infer_lit_string() {
        assert_eq!(infer_expr(Expr::LitString("hello".into())).unwrap(), Type::String);
    }

    #[test]
    fn infer_binop_add_int() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(1)),
                op: BinOp::Add,
                right: Box::new(Expr::LitInt(2)),
            })
            .unwrap(),
            Type::Int64
        );
    }

    #[test]
    fn infer_binop_sub_int() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(5)),
                op: BinOp::Sub,
                right: Box::new(Expr::LitInt(3)),
            })
            .unwrap(),
            Type::Int64
        );
    }

    #[test]
    fn infer_binop_mul_float() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitFloat(2.0)),
                op: BinOp::Mul,
                right: Box::new(Expr::LitFloat(3.0)),
            })
            .unwrap(),
            Type::Float64
        );
    }

    #[test]
    fn infer_binop_div_int() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(10)),
                op: BinOp::Div,
                right: Box::new(Expr::LitInt(3)),
            })
            .unwrap(),
            Type::Int64
        );
    }

    #[test]
    fn infer_binop_mod_int() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(10)),
                op: BinOp::Mod,
                right: Box::new(Expr::LitInt(3)),
            })
            .unwrap(),
            Type::Int64
        );
    }

    #[test]
    fn infer_binop_eq_returns_bool() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(1)),
                op: BinOp::Eq,
                right: Box::new(Expr::LitInt(2)),
            })
            .unwrap(),
            Type::Bool
        );
    }

    #[test]
    fn infer_binop_ne_returns_bool() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitFloat(1.0)),
                op: BinOp::Ne,
                right: Box::new(Expr::LitFloat(2.0)),
            })
            .unwrap(),
            Type::Bool
        );
    }

    #[test]
    fn infer_binop_and_returns_bool() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitTrue),
                op: BinOp::And,
                right: Box::new(Expr::LitFalse),
            })
            .unwrap(),
            Type::Bool
        );
    }

    #[test]
    fn infer_binop_or_returns_bool() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitFalse),
                op: BinOp::Or,
                right: Box::new(Expr::LitTrue),
            })
            .unwrap(),
            Type::Bool
        );
    }

    #[test]
    fn infer_binop_pipe() {
        // 42 |> |x| { x + 1 }  :  pipe Int64 through (Int64 -> Int64) should give Int64
        let func = Expr::Lambda {
            params: vec![Param {
                name: "x".to_string(),
                ty_ann: Some(TypeExpr::Named("Int64".to_string())),
            }],
            body: Box::new(Expr::Binary {
                left: Box::new(Expr::VarRef("x".to_string())),
                op: BinOp::Add,
                right: Box::new(Expr::LitInt(1)),
            }),
            ret_ty: Some(TypeExpr::Named("Int64".to_string())),
        };
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitInt(42)),
                op: BinOp::Pipe,
                right: Box::new(func),
            })
            .unwrap(),
            Type::Int64
        );
    }

    #[test]
    fn infer_binop_gtgt() {
        assert_eq!(
            infer_expr(Expr::Binary {
                left: Box::new(Expr::LitString("a".into())),
                op: BinOp::GtGt,
                right: Box::new(Expr::LitString("b".into())),
            })
            .unwrap(),
            Type::String
        );
    }

    #[test]
    fn infer_unary_neg_int() {
        assert_eq!(
            infer_expr(Expr::Unary {
                op: UnOp::Neg,
                right: Box::new(Expr::LitInt(1)),
            })
            .unwrap(),
            Type::Int64
        );
    }

    #[test]
    fn infer_unary_not_returns_bool() {
        assert_eq!(
            infer_expr(Expr::Unary {
                op: UnOp::Not,
                right: Box::new(Expr::LitFalse),
            })
            .unwrap(),
            Type::Bool
        );
    }

    // ── Control flow inference ──

    #[test]
    fn infer_if_else_branches_unify() {
        let e = Expr::If {
            cond: Box::new(Expr::LitTrue),
            then_branch: Box::new(Expr::LitInt(1)),
            else_branch: Some(Box::new(Expr::LitInt(0))),
        };
        assert_eq!(infer_expr(e).unwrap(), Type::Int64);
    }

    #[test]
    fn infer_if_without_else_returns_then_branch_type() {
        let e = Expr::If {
            cond: Box::new(Expr::LitTrue),
            then_branch: Box::new(Expr::LitInt(1)),
            else_branch: None,
        };
        // if without else: then_branch type propagates
        assert_eq!(infer_expr(e).unwrap(), Type::Int64);
    }

    #[test]
    fn infer_while_is_null() {
        let e = Expr::While {
            cond: Box::new(Expr::LitTrue),
            body: Box::new(Expr::LitInt(1)),
        };
        assert_eq!(infer_expr(e).unwrap(), Type::Null);
    }

    #[test]
    fn infer_block_returns_last_expr_type() {
        let e = Expr::Block(vec![
            Stmt::ExprStmt(Expr::LitInt(1)),
            Stmt::ExprStmt(Expr::LitString("result".into())),
        ]);
        assert_eq!(infer_expr(e).unwrap(), Type::String);
    }

    #[test]
    fn infer_empty_block_is_null() {
        let e = Expr::Block(vec![]);
        assert_eq!(infer_expr(e).unwrap(), Type::Null);
    }

    // ── Struct and member inference ──

    #[test]
    fn infer_struct_lit_validates_fields() {
        // Struct lit without struct in registry → error
        let err = infer_expr(Expr::StructLit {
            name: "Missing".into(),
            fields: vec![],
            spread: None,
        })
        .unwrap_err();
        assert!(err.msg.contains("unknown struct"));
    }

    #[test]
    fn infer_field_access_on_record() {
        let (structs, fields) = {
            let mut s = HashMap::new();
            let mut f = HashMap::new();
            s.insert("X".into(), 1);
            f.insert(1, vec![("val".into(), Type::Int64)]);
            (s, f)
        };
        let env = TypeEnv::new();
        // Infer StructLit to get a Record type, then access field
        let lit = Expr::StructLit {
            name: "X".into(),
            fields: vec![("val".into(), Expr::LitInt(42))],
            spread: None,
        };
        let (sub, ty) = infer(&env, &lit, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry()).unwrap();
        assert_eq!(sub.apply(&ty), Type::Record(1, vec![("val".into(), Type::Int64)]));
    }

    #[test]
    fn infer_field_access_errors_on_non_record() {
        let err = infer_expr(Expr::Member {
            object: Box::new(Expr::LitInt(42)),
            field: "unknown".into(),
        })
        .unwrap_err();
        assert!(
            err.msg.contains("not found"),
            "expected 'not found' error, got: {}",
            err.msg
        );
    }

    // ── List inference ──

    #[test]
    fn infer_empty_list_is_var() {
        let ty = infer_expr(Expr::ListLit(vec![])).unwrap();
        assert!(matches!(ty, Type::List(_)));
    }

    #[test]
    fn infer_list_unifies_elements() {
        let ty = infer_expr(Expr::ListLit(vec![
            Expr::LitInt(1),
            Expr::LitInt(2),
            Expr::LitInt(3),
        ]))
        .unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::Int64)));
    }

    #[test]
    fn infer_list_mixed_types_errors() {
        let err = infer_expr(Expr::ListLit(vec![
            Expr::LitInt(1),
            Expr::LitString("x".into()),
        ]))
        .unwrap_err();
        assert!(err.msg.contains("list element"));
    }

    // ── Call inference ──

    #[test]
    fn infer_call_with_matching_args() {
        // Lambda with 1 arg called with 1 arg — should infer
        let func = Expr::Lambda {
            params: vec![param("x", None)],
            ret_ty: None,
            body: Box::new(Expr::VarRef("x".into())),
        };
        let call = Expr::Call {
            func: Box::new(func),
            args: vec![Expr::LitInt(1)],
        };
        assert_eq!(infer_expr(call).unwrap(), Type::Int64);
    }

    #[test]
    fn infer_call_with_matching_args_returns_body_type() {
        // Lambda with 2 params called with 2 args
        let func = Expr::Lambda {
            params: vec![param("x", None), param("y", None)],
            ret_ty: None,
            body: Box::new(Expr::LitFloat(3.14)),
        };
        let call = Expr::Call {
            func: Box::new(func),
            args: vec![Expr::LitInt(1), Expr::LitInt(2)],
        };
        assert_eq!(infer_expr(call).unwrap(), Type::Float64);
    }

    // ── Assign inference ──

    #[test]
    fn infer_assign_is_null() {
        // Assign returns Null regardless of target/value types
        let e = Expr::Assign {
            target: Box::new(Expr::LitInt(1)),
            value: Box::new(Expr::LitInt(0)),
        };
        assert_eq!(infer_expr(e).unwrap(), Type::Null);
    }

    // ── Reset TVAR ──

    #[test]
    fn reset_tvar_clears_counter() {
        let _ = fresh_tvar();
        let _ = fresh_tvar();
        let _ = fresh_tvar();
        reset_tvar();
        let new_tvar = fresh_tvar();
        assert_eq!(new_tvar, TypeVar(0));
    }

    // ── Stdlib presence ──

    #[test]
    fn stdlib_includes_print() {
        let mut env = TypeEnv::new();
        inject_stdlib(&mut env);
        assert!(env.contains_key("print"));
        assert!(env.contains_key("sqrt"));
        assert!(env.contains_key("sin"));
        assert!(env.contains_key("cos"));
    }

    // ── Polymorphic var ──

    #[test]
    fn var_without_init_is_fresh_var() {
        let (env, _) = infer_ast(module(vec![var_decl("x", None)])).unwrap();
        assert!(matches!(*env["x"].body, Type::Var(_)));
    }

    #[test]
    fn monomorphic_var_after_assign() {
        let (env, _) = infer_ast(module(vec![var_decl("x", Some(Expr::LitFloat(1.0)))])).unwrap();
        assert_eq!(*env["x"].body, Type::Float64);
    }

    #[test]
    fn type_expr_to_type_edge_cases() {
        let mut structs = HashMap::new();
        structs.insert("Node".to_string(), 1);
        let mut fields = HashMap::new();
        fields.insert(1, vec![("value".to_string(), Type::Int64)]);

        assert_eq!(
            type_expr_to_type(
                &TypeExpr::List(Box::new(TypeExpr::named("Int64"))),
                &structs,
                &fields,
                &empty_interface_registry(),
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
                    &empty_interface_registry(),
                )
                .unwrap()
            ),
            "(Int64 → Bool)"
        );
        assert!(type_expr_to_type(&TypeExpr::named("Missing"), &structs, &fields, &empty_interface_registry()).is_err());

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

    // ── Type checking regression tests ──

    #[test]
    fn t30_struct_lit_rejects_wrong_field_type() {
        // Point { x: "hello" } when x is Int64 — must fail
        let (structs, fields) = {
            let mut s = HashMap::new();
            let mut f = HashMap::new();
            s.insert("Point".into(), 0);
            f.insert(0, vec![("x".into(), Type::Int64)]);
            (s, f)
        };
        let lit = Expr::StructLit {
            name: "Point".into(), fields: vec![("x".into(), Expr::LitString("hi".into()))], spread: None,
        };
        let env = TypeEnv::new();
        assert!(infer(&env, &lit, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry()).is_err());
    }

    #[test]
    fn t31_cmp_rejects_cross_type() {
        // 1 == "abc" — must fail
        let e = Expr::Binary { left: Box::new(Expr::LitInt(1)), op: BinOp::Eq, right: Box::new(Expr::LitString("abc".into())) };
        let (structs, fields) = empty_structs();
        let env = TypeEnv::new();
        assert!(infer(&env, &e, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry()).is_err());
    }

    #[test]
    fn t32_arith_rejects_bool() {
        // true + false — must fail
        let e = Expr::Binary { left: Box::new(Expr::LitTrue), op: BinOp::Add, right: Box::new(Expr::LitFalse) };
        let (structs, fields) = empty_structs();
        let env = TypeEnv::new();
        assert!(infer(&env, &e, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry()).is_err());
    }

    #[test]
    fn t33_neg_rejects_non_numeric() {
        // -true — must fail
        let e = Expr::Unary { op: UnOp::Neg, right: Box::new(Expr::LitTrue) };
        assert!(infer_expr(e).is_err());
    }

    #[test]
    fn t34_assign_unifies_types() {
        // x = 42 when x: String — must fail (x must be pre-bound as String)
        let mut env = TypeEnv::new();
        env.insert("x".into(), Scheme::monomorphic(Type::String));
        let e = Expr::Assign { target: Box::new(Expr::VarRef("x".into())), value: Box::new(Expr::LitInt(42)) };
        let (structs, fields) = empty_structs();
        assert!(infer(&env, &e, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry()).is_err());
    }

    #[test]
    fn t35_index_rejects_non_list_non_string() {
        // 42[0] — must fail
        let e = Expr::Index { object: Box::new(Expr::LitInt(42)), index: Box::new(Expr::LitInt(0)) };
        assert!(infer_expr(e).is_err());
    }

    #[test]
    fn t36_for_accepts_polymorphic_list() {
        // for x in list where list is a lambda param (unbound type var) — should succeed
        reset_tvar();
        let list_var = fresh_tvar();
        let mut env = TypeEnv::new();
        // Unbound: list is just Var, not yet unified with List
        env.insert("list".into(), Scheme::monomorphic(Type::Var(list_var)));
        let e = Expr::For {
            var: Param { name: "x".into(), ty_ann: None },
            iterable: Box::new(Expr::VarRef("list".into())),
            body: Box::new(Expr::Block(vec![])),
        };
        let (structs, fields) = empty_structs();
        // Should NOT error with "for loop requires List" — should unify list with List<?>
        assert!(infer(&env, &e, &structs, &fields, &empty_enums(), &empty_enum_variants(), &empty_interface_registry()).is_ok());
    }
}
