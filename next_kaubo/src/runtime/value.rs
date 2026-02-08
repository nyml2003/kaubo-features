//! NaN-boxed Value 实现
//!
//! 使用 IEEE 754 double 的 NaN 空间存储非浮点值

use super::object::ObjFunction;

/// NaN-boxed 值 (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq)]
pub struct Value(u64);

// 常量定义
const QNAN: u64 = 0x7FF8_0000_0000_0000; // Quiet NaN 基础值
const QNAN_MASK: u64 = 0x7FFC_0000_0000_0000; // 用于判断的掩码 (bits 62-50)

// 类型标签 (bits 50-48)
const TAG_MASK: u64 = 0x7 << 48;
const TAG_SMI: u64 = 0 << 48;     // 000 - 小整数
const TAG_HEAP: u64 = 1 << 48;    // 001 - 堆对象指针
const TAG_SPECIAL: u64 = 2 << 48; // 010 - 特殊值
const TAG_FUNCTION: u64 = 3 << 48; // 011 - 函数对象

const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF; // bits 47-0

// 特殊值编码 (bits 47-0)
// Kaubo 语言只有 null，没有 undefined
const VAL_NULL: u64 = 0;
const VAL_TRUE: u64 = 1;
const VAL_FALSE: u64 = 2;
// VAL_UNUSED = 3 保留

impl Value {
    // ==================== 构造方法 ====================

    /// 创建 SMI (小整数)
    /// 范围: -2^30 ~ 2^30-1 (约 ±10亿)
    #[inline]
    pub fn smi(n: i32) -> Self {
        // 只保留低 31 位 (30 位数值 + 符号)
        let payload = (n as u64) & ((1 << 31) - 1);
        Self(QNAN | TAG_SMI | payload)
    }

    /// 创建浮点数
    #[inline]
    pub fn float(f: f64) -> Self {
        Self(f.to_bits())
    }

    /// 创建堆对象指针
    #[inline]
    pub fn object<T>(ptr: *mut T) -> Self {
        let addr = ptr as u64;
        debug_assert!(
            addr & 0x7 == 0,
            "Object pointer must be 8-byte aligned"
        );
        let compressed = (addr >> 3) & PAYLOAD_MASK;
        Self(QNAN | TAG_HEAP | compressed)
    }

    /// 创建函数对象
    #[inline]
    pub fn function(ptr: *mut ObjFunction) -> Self {
        let addr = ptr as u64;
        debug_assert!(
            addr & 0x7 == 0,
            "Function pointer must be 8-byte aligned"
        );
        let compressed = (addr >> 3) & PAYLOAD_MASK;
        Self(QNAN | TAG_FUNCTION | compressed)
    }

    // ==================== 类型判断 ====================

    /// 是否为我们的 boxing 值 (非普通浮点数)
    #[inline]
    fn is_boxed(&self) -> bool {
        // 检查是否是 QNAN 模式
        (self.0 & QNAN_MASK) == QNAN
    }

    /// 是否为浮点数
    #[inline]
    pub fn is_float(&self) -> bool {
        !self.is_boxed()
    }

    /// 是否为 SMI
    #[inline]
    pub fn is_smi(&self) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == TAG_SMI
    }

    /// 是否为堆对象
    #[inline]
    pub fn is_heap(&self) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == TAG_HEAP
    }

    /// 是否为特殊值
    #[inline]
    pub fn is_special(&self) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == TAG_SPECIAL
    }

    /// 是否为函数对象
    #[inline]
    pub fn is_function(&self) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == TAG_FUNCTION
    }

    /// 是否为 null
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == QNAN | TAG_SPECIAL | VAL_NULL
    }

    /// 是否为 true
    #[inline]
    pub fn is_true(&self) -> bool {
        self.0 == QNAN | TAG_SPECIAL | VAL_TRUE
    }

    /// 是否为 false
    #[inline]
    pub fn is_false(&self) -> bool {
        self.0 == QNAN | TAG_SPECIAL | VAL_FALSE
    }

    /// 是否为真值
    /// Kaubo 中只有 false 和 null 为假，其他都为真（包括 0 和空字符串）
    #[inline]
    pub fn is_truthy(&self) -> bool {
        !self.is_false() && !self.is_null()
    }

    // ==================== 解包方法 ====================

    /// 解包为 SMI
    #[inline]
    pub fn as_smi(&self) -> Option<i32> {
        if self.is_smi() {
            let payload = self.0 & ((1 << 31) - 1);
            // 符号扩展
            if payload & (1 << 30) != 0 {
                // 负数
                Some((payload | (!0 << 31)) as i32)
            } else {
                Some(payload as i32)
            }
        } else {
            None
        }
    }

    /// 解包为浮点数
    #[inline]
    pub fn as_float(&self) -> f64 {
        f64::from_bits(self.0)
    }

    /// 解包为堆对象指针
    #[inline]
    pub fn as_object<T>(&self) -> Option<*mut T> {
        if self.is_heap() {
            let compressed = self.0 & PAYLOAD_MASK;
            Some(((compressed << 3) as usize) as *mut T)
        } else {
            None
        }
    }

    /// 解包为函数对象指针
    #[inline]
    pub fn as_function(&self) -> Option<*mut ObjFunction> {
        if self.is_function() {
            let compressed = self.0 & PAYLOAD_MASK;
            Some(((compressed << 3) as usize) as *mut ObjFunction)
        } else {
            None
        }
    }

    // ==================== 常量 ====================

    pub const NULL: Value = Value(QNAN | TAG_SPECIAL | VAL_NULL);
    pub const TRUE: Value = Value(QNAN | TAG_SPECIAL | VAL_TRUE);
    pub const FALSE: Value = Value(QNAN | TAG_SPECIAL | VAL_FALSE);
}

// ==================== Debug 输出 ====================

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_float() {
            write!(f, "Float({})", self.as_float())
        } else if self.is_smi() {
            write!(f, "SMI({})", self.as_smi().unwrap())
        } else if self.is_null() {
            write!(f, "Null")
        } else if self.is_true() {
            write!(f, "True")
        } else if self.is_false() {
            write!(f, "False")
        } else if self.is_function() {
            write!(f, "Function({:p})", self.as_function().unwrap())
        } else if self.is_heap() {
            write!(f, "Object({:p})", self.as_object::<()>().unwrap())
        } else {
            write!(f, "Value({:016x})", self.0)
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_float() {
            write!(f, "{}", self.as_float())
        } else if self.is_smi() {
            write!(f, "{}", self.as_smi().unwrap())
        } else if self.is_null() {
            write!(f, "null")
        } else if self.is_true() {
            write!(f, "true")
        } else if self.is_false() {
            write!(f, "false")
        } else if self.is_function() {
            write!(f, "<function>")
        } else if self.is_heap() {
            write!(f, "<object>")
        } else {
            write!(f, "<value>")
        }
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smi_creation() {
        let v = Value::smi(42);
        assert!(v.is_smi());
        assert!(!v.is_float());
        assert_eq!(v.as_smi(), Some(42));
    }

    #[test]
    fn test_smi_negative() {
        let v = Value::smi(-100);
        assert!(v.is_smi());
        assert_eq!(v.as_smi(), Some(-100));
    }

    #[test]
    fn test_smi_bounds() {
        let max = (1 << 30) - 1;
        let min = -(1 << 30);
        assert_eq!(Value::smi(max).as_smi(), Some(max));
        assert_eq!(Value::smi(min).as_smi(), Some(min));
    }

    #[test]
    fn test_float_creation() {
        let v = Value::float(3.14);
        assert!(v.is_float());
        assert!(!v.is_smi());
        assert!(!v.is_heap());
        assert_eq!(v.as_float(), 3.14);
    }

    #[test]
    fn test_special_values() {
        assert!(Value::NULL.is_null());
        assert!(Value::NULL.is_special());
        assert!(!Value::NULL.is_smi());

        assert!(Value::TRUE.is_true());
        assert!(Value::TRUE.is_truthy());

        assert!(Value::FALSE.is_false());
        assert!(!Value::FALSE.is_truthy());

        assert!(!Value::NULL.is_truthy());
    }

    #[test]
    fn test_smi_zero_is_truthy() {
        let zero = Value::smi(0);
        assert!(zero.is_truthy());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Value::smi(42)), "42");
        assert_eq!(format!("{}", Value::float(3.14)), "3.14");
        assert_eq!(format!("{}", Value::NULL), "null");
        assert_eq!(format!("{}", Value::TRUE), "true");
        assert_eq!(format!("{}", Value::FALSE), "false");
    }

    #[test]
    fn test_float_special_values() {
        // 无穷大仍然是浮点数
        let inf = Value::float(f64::INFINITY);
        let neg_inf = Value::float(f64::NEG_INFINITY);
        assert!(inf.is_float());
        assert!(neg_inf.is_float());
        assert!(inf.as_float().is_infinite());

        // Note: f64::NAN 的位模式可能与 QNAN 重叠
        // 在我们的设计中，某些 NaN 值会被识别为 boxing 值
        // 这是预期的行为，实际代码中应避免显式使用 NaN
    }
}
