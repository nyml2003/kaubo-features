//! Hindley-Milner type inference (Algorithm W)
//!
//! v2.0: 支持 Int64, Float64, String, Bool, Null, Arrow, Record, List
//! v2.1: 支持 ADT/Variant

use std::collections::HashMap;
use std::fmt;

/// 类型变量 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub usize);

/// 类型
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Var(TypeVar),
    Int64,
    Float64,
    String,
    Bool,
    Null,
    Arrow(Box<Type>, Box<Type>),
    Record(usize, Vec<(String, Type)>), // struct_id, fields
    Variant(usize, String, Vec<(String, Type)>), // enum_id, variant_name, field_types
    List(Box<Type>),
    Interface(String), // interface name (for type annotations like `x: Add`)
}

/// 多态类型方案 — ∀ bound. body
#[derive(Debug, Clone)]
pub struct Scheme {
    pub bound: Vec<TypeVar>,
    pub body: Box<Type>,
}

/// 类型代换 — Var → Type
#[derive(Debug, Clone)]
#[derive(PartialEq)]
pub struct Subst(HashMap<TypeVar, Type>);

/// 类型环境 — Name → Scheme
pub type TypeEnv = HashMap<String, Scheme>;

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Var(v) => write!(f, "t{}", v.0),
            Type::Int64 => write!(f, "Int64"),
            Type::Float64 => write!(f, "Float64"),
            Type::String => write!(f, "String"),
            Type::Bool => write!(f, "Bool"),
            Type::Null => write!(f, "Null"),
            Type::Arrow(a, b) => write!(f, "({a} → {b})"),
            Type::Record(_, fields) => {
                let fs: Vec<_> = fields
                    .iter()
                    .map(|(n, t)| format!("{n}: {t}"))
                    .collect();
                write!(f, "{{{}}}", fs.join(", "))
            }
            Type::Variant(id, name, _) => write!(f, "{name}#{id}"),
            Type::List(t) => write!(f, "List<{t}>"),
            Type::Interface(name) => write!(f, "interface {name}"),
        }
    }
}

impl Subst {
    pub fn empty() -> Self {
        Subst(HashMap::new())
    }

    pub fn singleton(var: TypeVar, ty: Type) -> Self {
        let mut m = HashMap::new();
        m.insert(var, ty);
        Subst(m)
    }

    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => self.0.get(v).cloned().unwrap_or_else(|| ty.clone()),
            Type::Arrow(a, b) => Type::Arrow(Box::new(self.apply(a)), Box::new(self.apply(b))),
            Type::Record(id, fields) => Type::Record(
                *id,
                fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.apply(t)))
                    .collect(),
            ),
            Type::Variant(id, name, fields) => Type::Variant(
                *id,
                name.clone(),
                fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.apply(t)))
                    .collect(),
            ),
            Type::List(t) => Type::List(Box::new(self.apply(t))),
            other => other.clone(),
        }
    }

    pub fn compose(mut self, other: &Subst) -> Self {
        for (var, ty) in &other.0 {
            let applied = self.apply(ty);
            self.0.insert(*var, applied);
        }
        self
    }

    pub fn extend(&mut self, var: TypeVar, ty: Type) {
        self.0.insert(var, ty);
    }
}

impl Scheme {
    pub fn monomorphic(ty: Type) -> Self {
        Scheme {
            bound: vec![],
            body: Box::new(ty),
        }
    }
}

// ── 模块系统：导入规格 ──

/// 从外部模块导入的符号信息。
///
/// 由 driver 从 `ExportTable` 翻译，传入 `infer_module`。
/// kaubo-infer 不依赖 kaubo-driver，这里是纯类型层。
#[derive(Debug, Clone)]
pub struct ImportSpec {
    pub local_name: String,
    pub source_path: String,
    pub kind: ImportKind,
}

/// 导入符号的种类。
#[derive(Debug, Clone)]
pub enum ImportKind {
    Const { ty: Type },
    Function { ty: Type },
    Struct { fields: Vec<(String, Type)> },
    Interface { methods: Vec<(String, Vec<(String, Type)>, Option<Type>)> },
}
