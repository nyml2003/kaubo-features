//! 运算符重载定义
//!
//! 定义所有可重载的运算符及其元方法名

use crate::core::{Operator, Value};
use std::fmt;

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "operator {}", self.method_name())
    }
}

/// 运算符查找结果（占位，后续实现）
#[derive(Debug)]
pub enum OperatorLookup {
    /// 找到内建处理函数
    Builtin(fn(Value, Value) -> Result<Value, String>),
    /// 找到用户定义的元方法（存储闭包指针）
    Metatable(*const ()),
    /// 未找到
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{InlineCacheEntry, ObjClosure};

    #[test]
    fn test_operator_method_name() {
        assert_eq!(Operator::Add.method_name(), "add");
        assert_eq!(Operator::Neg.method_name(), "neg");
        assert_eq!(Operator::Get.method_name(), "get");
    }

    #[test]
    fn test_operator_from_method_name() {
        assert_eq!(Operator::from_method_name("add"), Some(Operator::Add));
        assert_eq!(Operator::from_method_name("unknown"), None);
    }

    #[test]
    fn test_operator_reverse() {
        assert_eq!(Operator::Add.reverse(), Some(Operator::RAdd));
        assert_eq!(Operator::Mul.reverse(), Some(Operator::RMul));
        assert_eq!(Operator::Sub.reverse(), None);
    }

    #[test]
    fn test_operator_symbol() {
        assert_eq!(Operator::Add.symbol(), "+");
        assert_eq!(Operator::Eq.symbol(), "==");
        assert_eq!(Operator::Get.symbol(), "[]");
    }

    #[test]
    fn test_operator_arity() {
        assert!(Operator::Neg.is_unary());
        assert!(Operator::Add.is_binary());
        assert!(!Operator::Neg.is_binary());
        assert!(!Operator::Add.is_unary());
    }

    #[test]
    fn test_inline_cache() {
        let mut cache = InlineCacheEntry::empty();
        assert!(cache.is_empty());
        assert!(!cache.matches(1, 2));

        // 使用 update 方法设置缓存
        cache.update(1, 2, 0x1234 as *mut ObjClosure);

        assert!(cache.matches(1, 2));
        assert!(!cache.matches(1, 3));
        
        // 测试统计
        cache.record_hit();
        cache.record_hit();
        assert_eq!(cache.hit_count, 2);
    }
}
