//! NaN-boxed Value 实现（Core 层）
//!
//! 基于 IEEE 754 double 的 NaN 空间存储非浮点值
//! 位布局: [1位符号][11位指数][7位Tag][45位Payload]
//!          S         E(0x7FF)   Tag      Payload

/// NaN-boxed 值 (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq)]
pub struct Value(pub u64);

// ==================== 常量定义 ====================

/// Quiet NaN 基础值: E=0x7FF, bit 51 = 1 (用于区分 NaN 类型)
const QNAN: u64 = 0x7FF8_0000_0000_0000;

/// Tag 掩码: bits 50-44 (7位)
const TAG_MASK: u64 = 0x7F << 44;

/// Payload 掩码: bits 43-0 (44位)
const PAYLOAD_MASK: u64 = 0xFFFFFFFFFFF;

// ==================== Tag 定义 ====================
// bits 50-44 (7位) = Tag
// 0-7: 特殊值
#[allow(dead_code)]
const TAG_NAN: u64 = 0 << 44; // 0: 语言级 NaN
const TAG_NULL: u64 = 1 << 44; // 1: null
const TAG_TRUE: u64 = 2 << 44; // 2: true
const TAG_FALSE: u64 = 3 << 44; // 3: false
const TAG_SMI: u64 = 4 << 44; // 4: 小整数 (SMI，31位有符号)

// 8-23: 内联整数 (-8 ~ +7)
#[allow(dead_code)]
const TAG_INLINE_INT_START: u64 = 8 << 44;
#[allow(dead_code)]
const TAG_INLINE_INT_END: u64 = 23 << 44;

// 32+: 堆类型（具体类型在 Tag 常量中定义，供 runtime 使用）
pub const TAG_HEAP_OBJECT: u64 = 32 << 44;
pub const TAG_STRING: u64 = 33 << 44;
pub const TAG_FUNCTION: u64 = 34 << 44;
pub const TAG_LIST: u64 = 35 << 44;
pub const TAG_ITERATOR: u64 = 36 << 44;
pub const TAG_CLOSURE: u64 = 37 << 44;
pub const TAG_COROUTINE: u64 = 38 << 44;
pub const TAG_RESULT: u64 = 39 << 44;
pub const TAG_OPTION: u64 = 40 << 44;
pub const TAG_JSON: u64 = 41 << 44;
pub const TAG_MODULE: u64 = 42 << 44;
pub const TAG_NATIVE: u64 = 43 << 44;
pub const TAG_NATIVE_VM: u64 = 44 << 44;
pub const TAG_STRUCT: u64 = 45 << 44;
pub const TAG_SHAPE: u64 = 46 << 44;

/// SMI 最大值 (2^30 - 1)
const SMI_MAX: i32 = (1 << 30) - 1;
/// SMI 最小值 (-2^30)
const SMI_MIN: i32 = -(1 << 30);

impl Value {
    // ==================== 基础构造方法 ====================

    /// 创建 SMI (小整数)
    /// 范围: -2^30 ~ 2^30-1 (约 ±10亿)
    #[inline]
    pub fn smi(n: i32) -> Self {
        debug_assert!((SMI_MIN..=SMI_MAX).contains(&n), "SMI out of range: {n}");
        let payload = (n as u64) & ((1 << 31) - 1);
        Self(QNAN | TAG_SMI | payload)
    }

    /// 创建整数（自动选择最优编码）
    /// - -8~7: 内联整数（Tag 编码）
    /// - 其他 SMI 范围: SMI（31位 Payload 编码）
    #[inline]
    pub fn int(n: i32) -> Self {
        match n {
            -8..=7 => {
                let tag = ((n + 16) as u64) << 44;
                Self(QNAN | tag)
            }
            SMI_MIN..=SMI_MAX => Self::smi(n),
            _ => panic!("Integer {n} out of SMI range"),
        }
    }

    /// 创建浮点数
    #[inline]
    pub fn float(f: f64) -> Self {
        Self(f.to_bits())
    }

    /// 创建布尔值
    #[inline]
    pub fn bool_from(b: bool) -> Self {
        if b { Self::TRUE } else { Self::FALSE }
    }

    /// 内部: 编码堆指针（供 runtime 层使用）
    #[inline]
    pub fn encode_heap_ptr<T>(ptr: *mut T, tag: u64) -> Self {
        let addr = ptr as u64;
        debug_assert!(addr & 0x7 == 0, "Pointer must be 8-byte aligned");
        let compressed = (addr >> 3) & PAYLOAD_MASK;
        Self(QNAN | tag | compressed)
    }

    /// 内部: 解码堆指针（供 runtime 层使用）
    #[inline]
    pub fn decode_heap_ptr<T>(&self, expected_tag: u64) -> Option<*mut T> {
        if self.is_boxed() && (self.0 & TAG_MASK) == expected_tag {
            let compressed = self.0 & PAYLOAD_MASK;
            Some(((compressed << 3) as usize) as *mut T)
        } else {
            None
        }
    }

    // ==================== 类型判断 ====================

    /// 是否为我们的 boxing 值 (非普通浮点数)
    #[inline]
    fn is_boxed(&self) -> bool {
        (self.0 & 0x7FF8_0000_0000_0000) == 0x7FF8_0000_0000_0000
    }

    /// 是否为浮点数
    #[inline]
    pub fn is_float(&self) -> bool {
        !self.is_boxed()
    }

    /// 获取原始 Tag 值 (0-127)
    #[inline]
    pub fn raw_tag(&self) -> u64 {
        (self.0 & TAG_MASK) >> 44
    }

    /// 是否为 SMI
    #[inline]
    pub fn is_smi(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 4
    }

    /// 是否为内联整数（-8~7）
    #[inline]
    pub fn is_inline_int(&self) -> bool {
        if !self.is_boxed() {
            return false;
        }
        let tag = self.raw_tag();
        let payload = self.0 & PAYLOAD_MASK;
        (8..=23).contains(&tag) && payload == 0
    }

    /// 是否为整数（SMI 或内联整数）
    #[inline]
    pub fn is_int(&self) -> bool {
        self.is_smi() || self.is_inline_int()
    }

    /// 是否为 null
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == (QNAN | TAG_NULL)
    }

    /// 是否为 true
    #[inline]
    pub fn is_true(&self) -> bool {
        self.0 == (QNAN | TAG_TRUE)
    }

    /// 是否为 false
    #[inline]
    pub fn is_false(&self) -> bool {
        self.0 == (QNAN | TAG_FALSE)
    }

    /// 是否为布尔值
    #[inline]
    pub fn is_bool(&self) -> bool {
        self.is_true() || self.is_false()
    }

    /// 是否为真值（只有 false 和 null 为假）
    #[inline]
    pub fn is_truthy(&self) -> bool {
        !self.is_false() && !self.is_null()
    }

    /// 是否为堆对象（检查是否为堆类型 Tag）
    #[inline]
    pub fn is_heap(&self) -> bool {
        self.is_boxed() && self.raw_tag() >= 32
    }

    /// 是否为特定 Tag 的堆对象
    #[inline]
    pub fn is_tagged(&self, tag: u64) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == tag
    }

    // ==================== 解包方法 ====================

    /// 解包为 SMI (i32)
    #[inline]
    pub fn as_smi(&self) -> Option<i32> {
        if self.is_smi() {
            let payload = self.0 & ((1 << 31) - 1);
            if payload & (1 << 30) != 0 {
                Some((payload | (!0 << 31)) as i32)
            } else {
                Some(payload as i32)
            }
        } else {
            None
        }
    }

    /// 解包为内联整数 (-8~7)
    #[inline]
    pub fn as_inline_int(&self) -> Option<i32> {
        if !self.is_boxed() {
            return None;
        }
        let tag = self.raw_tag();
        let payload = self.0 & PAYLOAD_MASK;
        if (8..=23).contains(&tag) && payload == 0 {
            Some((tag as i32) - 16)
        } else {
            None
        }
    }

    /// 统一解包为 i32（支持 SMI 和内联整数）
    #[inline]
    pub fn as_int(&self) -> Option<i32> {
        self.as_inline_int().or_else(|| self.as_smi())
    }

    /// 解包为浮点数
    #[inline]
    pub fn as_float(&self) -> f64 {
        f64::from_bits(self.0)
    }

    /// 解包为布尔值
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.is_true() {
            Some(true)
        } else if self.is_false() {
            Some(false)
        } else {
            None
        }
    }

    // ==================== 常量 ====================

    pub const NULL: Value = Value(QNAN | TAG_NULL);
    pub const TRUE: Value = Value(QNAN | TAG_TRUE);
    pub const FALSE: Value = Value(QNAN | TAG_FALSE);
}

// ==================== Debug/Display ====================

// Debug 和 Display 实现在 runtime/value_ext.rs 中提供

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smi() {
        let v = Value::smi(42);
        assert!(v.is_smi());
        assert_eq!(v.as_smi(), Some(42));
        assert_eq!(v.as_int(), Some(42));
    }

    #[test]
    fn test_inline_int() {
        for n in -8..=7 {
            let v = Value::int(n);
            assert!(v.is_inline_int(), "{n} should be inline");
            assert_eq!(v.as_int(), Some(n));
        }
    }

    #[test]
    #[allow(clippy::approximate_constant)]
    fn test_float() {
        let v = Value::float(3.14);
        assert!(v.is_float());
        assert_eq!(v.as_float(), 3.14);
    }

    #[test]
    fn test_special_values() {
        assert!(Value::NULL.is_null());
        assert!(Value::TRUE.is_true());
        assert!(Value::FALSE.is_false());
        assert!(!Value::NULL.is_truthy());
        assert!(Value::TRUE.is_truthy());
    }
}
