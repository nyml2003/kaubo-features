use super::super::lexer::token_kind::KauboTokenKind;
use super::stmt::Stmt;
use std::fmt;

// 表达式类型别名（对应C++的ExprPtr）
pub type Expr = Box<ExprKind>;

/// 解析器表达式枚举（对应C++的Expr::ValueType变体集合）
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // 整数字面量表达式
    LiteralInt(LiteralInt),
    // 字符串字面量表达式
    LiteralString(LiteralString),
    // 布尔true字面量
    LiteralTrue(LiteralTrue),
    // 布尔false字面量
    LiteralFalse(LiteralFalse),
    // Null字面量
    LiteralNull(LiteralNull),
    // 列表字面量表达式
    LiteralList(LiteralList),
    // 二元运算符表达式
    Binary(Binary),
    // 一元运算符表达式
    Unary(Unary),
    // 括号表达式
    Grouping(Grouping),
    // 变量引用表达式
    VarRef(VarRef),
    // 函数调用表达式
    FunctionCall(FunctionCall),
    // 匿名函数表达式
    Lambda(Lambda),
    // 成员访问表达式
    MemberAccess(MemberAccess),
    // Yield 表达式 (用于协程)
    Yield(YieldExpr),
}

// 整数字面量结构体
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralInt {
    pub value: i64,
}

// 字符串字面量结构体
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralString {
    pub value: String,
}

// 布尔true字面量（无数据）
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LiteralTrue;

// 布尔false字面量（无数据）
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LiteralFalse;

// Null字面量（无数据）
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LiteralNull;

// 列表字面量结构体
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralList {
    pub elements: Vec<Expr>,
}

// 二元运算符表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub left: Expr,
    pub op: KauboTokenKind,
    pub right: Expr,
}

// 一元运算符表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct Unary {
    pub op: KauboTokenKind,
    pub operand: Expr,
}

// 括号表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct Grouping {
    pub expression: Expr,
}

// 变量引用表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct VarRef {
    pub name: String,
}

// 函数调用表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub function_expr: Expr,
    pub arguments: Vec<Expr>,
}

// 匿名函数表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    pub params: Vec<String>,
    pub body: Stmt,
}

// 成员访问表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct MemberAccess {
    pub object: Expr,
    pub member: String,
}

// Yield 表达式结构体
#[derive(Debug, Clone, PartialEq)]
pub struct YieldExpr {
    pub value: Option<Expr>,  // yield 的值，None 表示 yield;
}

// 实现Display trait（可选，用于调试输出）
impl fmt::Display for ExprKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprKind::LiteralInt(int) => write!(f, "{}", int.value),
            ExprKind::LiteralString(s) => write!(f, "\"{}\"", s.value),
            ExprKind::LiteralTrue(_) => write!(f, "true"),
            ExprKind::LiteralFalse(_) => write!(f, "false"),
            ExprKind::LiteralNull(_) => write!(f, "null"),
            ExprKind::LiteralList(list) => {
                let elements = list
                    .elements
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "[{}]", elements)
            }
            ExprKind::Binary(bin) => write!(f, "({} {:?} {})", bin.left, bin.op, bin.right),
            ExprKind::Unary(un) => write!(f, "({:?} {})", un.op, un.operand),
            ExprKind::Grouping(g) => write!(f, "({})", g.expression),
            ExprKind::VarRef(v) => write!(f, "{}", v.name),
            ExprKind::FunctionCall(call) => {
                let args = call
                    .arguments
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}({})", call.function_expr, args)
            }
            ExprKind::Lambda(l) => {
                let params = l.params.join(", ");
                write!(f, "({}) => {:?}", params, l.body)
            }
            ExprKind::MemberAccess(m) => write!(f, "{}.{}", m.object, m.member),
            ExprKind::Yield(y) => match &y.value {
                Some(v) => write!(f, "yield {}", v),
                None => write!(f, "yield"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_int_display() {
        let expr = ExprKind::LiteralInt(LiteralInt { value: 42 });
        assert_eq!(format!("{}", expr), "42");
    }

    #[test]
    fn test_literal_string_display() {
        let expr = ExprKind::LiteralString(LiteralString {
            value: "hello".to_string(),
        });
        assert_eq!(format!("{}", expr), "\"hello\"");
    }

    #[test]
    fn test_literal_bool_display() {
        let expr_true = ExprKind::LiteralTrue(LiteralTrue);
        let expr_false = ExprKind::LiteralFalse(LiteralFalse);
        assert_eq!(format!("{}", expr_true), "true");
        assert_eq!(format!("{}", expr_false), "false");
    }

    #[test]
    fn test_literal_null_display() {
        let expr = ExprKind::LiteralNull(LiteralNull);
        assert_eq!(format!("{}", expr), "null");
    }

    #[test]
    fn test_literal_list_display() {
        let expr = ExprKind::LiteralList(LiteralList { elements: vec![] });
        assert_eq!(format!("{}", expr), "[]");
    }

    #[test]
    fn test_var_ref_display() {
        let expr = ExprKind::VarRef(VarRef {
            name: "x".to_string(),
        });
        assert_eq!(format!("{}", expr), "x");
    }

    #[test]
    fn test_expr_kind_clone() {
        let expr = ExprKind::LiteralInt(LiteralInt { value: 42 });
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    #[test]
    fn test_struct_defaults() {
        let _ = LiteralTrue::default();
        let _ = LiteralFalse::default();
        let _ = LiteralNull::default();
    }
}
