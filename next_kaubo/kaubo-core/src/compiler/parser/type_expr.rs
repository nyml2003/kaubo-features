//! 类型表达式 AST 定义
//!
//! 支持类型标注：var x: int = 5;
//! 支持函数类型：|int| -> int

use std::fmt;

/// 类型表达式
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// 命名类型（int, string, bool, float, 自定义类型）
    Named(NamedType),
    /// List<T>
    List(Box<TypeExpr>),
    /// Tuple<T1, T2, ...>
    Tuple(Vec<TypeExpr>),
    /// 函数类型：|Param1, Param2| -> Return
    Function(FunctionType),
}

/// 命名类型
#[derive(Debug, Clone, PartialEq)]
pub struct NamedType {
    pub name: String,
}

/// 函数类型
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<TypeExpr>,              // 参数类型列表
    pub return_type: Option<Box<TypeExpr>>, // 返回类型（None 表示 void）
}

impl TypeExpr {
    /// 创建命名类型
    pub fn named(name: impl Into<String>) -> Self {
        TypeExpr::Named(NamedType { name: name.into() })
    }

    /// 创建 List<T>
    pub fn list(elem_type: TypeExpr) -> Self {
        TypeExpr::List(Box::new(elem_type))
    }

    /// 创建 Tuple<T1, T2>
    pub fn tuple(types: Vec<TypeExpr>) -> Self {
        TypeExpr::Tuple(types)
    }

    /// 创建函数类型
    pub fn function(params: Vec<TypeExpr>, return_type: Option<TypeExpr>) -> Self {
        TypeExpr::Function(FunctionType {
            params,
            return_type: return_type.map(Box::new),
        })
    }

    /// 创建 void 函数类型
    pub fn function_void(params: Vec<TypeExpr>) -> Self {
        TypeExpr::Function(FunctionType {
            params,
            return_type: None,
        })
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeExpr::Named(n) => write!(f, "{}", n.name),
            TypeExpr::List(elem) => write!(f, "List<{elem}>"),
            TypeExpr::Tuple(elems) => {
                let types: Vec<String> = elems.iter().map(|t| t.to_string()).collect();
                write!(f, "Tuple<{}>", types.join(", "))
            }
            TypeExpr::Function(func) => {
                let params: Vec<String> = func.params.iter().map(|t| t.to_string()).collect();
                match &func.return_type {
                    Some(ret) => write!(f, "|{}| -> {}", params.join(", "), ret),
                    None => write!(f, "|{}| -> void", params.join(", ")),
                }
            }
        }
    }
}

impl fmt::Display for NamedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_type() {
        let t = TypeExpr::named("int");
        assert_eq!(t.to_string(), "int");
    }

    #[test]
    fn test_list_type() {
        let t = TypeExpr::list(TypeExpr::named("int"));
        assert_eq!(t.to_string(), "List<int>");
    }

    #[test]
    fn test_tuple_type() {
        let t = TypeExpr::tuple(vec![TypeExpr::named("int"), TypeExpr::named("string")]);
        assert_eq!(t.to_string(), "Tuple<int, string>");
    }

    #[test]
    fn test_function_type() {
        let t = TypeExpr::function(
            vec![TypeExpr::named("int"), TypeExpr::named("int")],
            Some(TypeExpr::named("int")),
        );
        assert_eq!(t.to_string(), "|int, int| -> int");
    }

    #[test]
    fn test_void_function_type() {
        let t = TypeExpr::function_void(vec![TypeExpr::named("int")]);
        assert_eq!(t.to_string(), "|int| -> void");
    }
}
