//! 运算符定义 (Core 层)
//!
//! 纯类型定义，不依赖其他运行时类型

/// 可重载的运算符枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    // ===== 二元算术运算符 =====
    /// 加法: `a + b`
    Add,
    /// 减法: `a - b`
    Sub,
    /// 乘法: `a * b`
    Mul,
    /// 除法: `a / b`
    Div,
    /// 取模: `a % b`
    Mod,

    // ===== 一元运算符 =====
    /// 一元负号: `-a`
    Neg,

    // ===== 比较运算符 =====
    /// 相等: `a == b`
    Eq,
    /// 小于: `a < b`
    Lt,
    /// 小于等于: `a <= b`
    Le,

    // ===== 索引访问 =====
    /// 索引读取: `a[i]`
    Get,
    /// 索引赋值: `a[i] = v`
    Set,

    // ===== 其他 =====
    /// 字符串转换: `a as string`
    Str,
    /// 长度: `len(a)`
    Len,
    /// 调用: `a(args)`
    Call,

    // ===== 反向运算符（当左操作数不支持时尝试）=====
    /// 反向加法: `b + a` 当 a 不支持时
    RAdd,
    /// 反向乘法: `b * a` 当 a 不支持时
    RMul,
}

impl Operator {
    /// 获取运算符的元方法名
    pub fn method_name(&self) -> &'static str {
        match self {
            Operator::Add => "add",
            Operator::Sub => "sub",
            Operator::Mul => "mul",
            Operator::Div => "div",
            Operator::Mod => "mod",
            Operator::Neg => "neg",
            Operator::Eq => "eq",
            Operator::Lt => "lt",
            Operator::Le => "le",
            Operator::Get => "get",
            Operator::Set => "set",
            Operator::Str => "str",
            Operator::Len => "len",
            Operator::Call => "call",
            Operator::RAdd => "radd",
            Operator::RMul => "rmul",
        }
    }

    /// 从方法名解析运算符
    pub fn from_method_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Operator::Add),
            "sub" => Some(Operator::Sub),
            "mul" => Some(Operator::Mul),
            "div" => Some(Operator::Div),
            "mod" => Some(Operator::Mod),
            "neg" => Some(Operator::Neg),
            "eq" => Some(Operator::Eq),
            "lt" => Some(Operator::Lt),
            "le" => Some(Operator::Le),
            "get" => Some(Operator::Get),
            "set" => Some(Operator::Set),
            "str" => Some(Operator::Str),
            "len" => Some(Operator::Len),
            "call" => Some(Operator::Call),
            "radd" => Some(Operator::RAdd),
            "rmul" => Some(Operator::RMul),
            _ => None,
        }
    }

    /// 获取对应的反向运算符
    pub fn reverse(&self) -> Option<Self> {
        match self {
            Operator::Add => Some(Operator::RAdd),
            Operator::Mul => Some(Operator::RMul),
            _ => None,
        }
    }

    /// 是否是反向运算符
    pub fn is_reverse(&self) -> bool {
        matches!(self, Operator::RAdd | Operator::RMul)
    }

    /// 获取运算符的符号表示（用于错误信息）
    pub fn symbol(&self) -> &'static str {
        match self {
            Operator::Add | Operator::RAdd => "+",
            Operator::Sub => "-",
            Operator::Mul | Operator::RMul => "*",
            Operator::Div => "/",
            Operator::Mod => "%",
            Operator::Neg => "-",
            Operator::Eq => "==",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Get => "[]",
            Operator::Set => "[]=",
            Operator::Str => "as string",
            Operator::Len => "len()",
            Operator::Call => "()",
        }
    }

    /// 是否是一元运算符
    pub fn is_unary(&self) -> bool {
        matches!(self, Operator::Neg | Operator::Str | Operator::Len)
    }

    /// 是否是二元运算符
    pub fn is_binary(&self) -> bool {
        !self.is_unary()
            && !matches!(
                self,
                Operator::Call | Operator::Get | Operator::Set | Operator::RAdd | Operator::RMul
            )
    }
}


use crate::vm::core::object::ObjClosure;

/// 内联缓存条目
#[derive(Debug, Clone)]
pub struct InlineCacheEntry {
    /// 左操作数 Shape ID
    pub left_shape: u16,
    /// 右操作数 Shape ID（一元运算符为 0）
    pub right_shape: u16,
    /// 缓存的运算符闭包指针（运行时由 VM 设置）
    pub closure: *mut ObjClosure,
    /// 命中次数（用于统计）
    pub hit_count: u64,
    /// 未命中次数（用于统计）
    pub miss_count: u64,
}

impl InlineCacheEntry {
    /// 创建新的空缓存条目
    pub fn empty() -> Self {
        Self {
            left_shape: u16::MAX,
            right_shape: u16::MAX,
            closure: std::ptr::null_mut(),
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// 检查是否匹配给定的 Shape ID
    pub fn matches(&self, left: u16, right: u16) -> bool {
        self.left_shape == left && self.right_shape == right && !self.closure.is_null()
    }

    /// 是否是空缓存
    pub fn is_empty(&self) -> bool {
        self.closure.is_null()
    }

    /// 更新缓存
    pub fn update(&mut self, left: u16, right: u16, closure: *mut ObjClosure) {
        self.left_shape = left;
        self.right_shape = right;
        self.closure = closure;
        self.hit_count = 0;
        self.miss_count = 0;
    }

    /// 记录命中
    pub fn record_hit(&mut self) {
        self.hit_count += 1;
    }

    /// 记录未命中
    pub fn record_miss(&mut self) {
        self.miss_count += 1;
    }
}

unsafe impl Send for InlineCacheEntry {}
unsafe impl Sync for InlineCacheEntry {}

#[cfg(test)]
mod tests {
    use super::*;

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

        cache.update(1, 2, 0x1234 as *mut ObjClosure);

        assert!(cache.matches(1, 2));
        assert!(!cache.matches(1, 3));
        
        cache.record_hit();
        cache.record_hit();
        assert_eq!(cache.hit_count, 2);
    }
}
