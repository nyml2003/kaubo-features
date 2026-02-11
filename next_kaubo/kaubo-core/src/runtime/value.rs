//! NaN-boxed Value 实现（7-bit Tag 版本）
//!
//! 基于 IEEE 754 double 的 NaN 空间存储非浮点值
//! 位布局: [1位符号][11位指数][7位Tag][45位Payload]
//!          S         E(0x7FF)   Tag      Payload

use super::object::{
    ObjClosure, ObjCoroutine, ObjFunction, ObjIterator, ObjJson, ObjList, ObjModule, ObjNative, ObjOption,
    ObjResult, ObjString,
};

/// NaN-boxed 值 (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq)]
pub struct Value(u64);

// ==================== 常量定义 ====================

/// Quiet NaN 基础值: E=0x7FF, bit 51 = 1 (用于区分 NaN 类型)
/// 注意: QNAN 本身占据 bit 62-51 (指数位 + 最高位尾数)
const QNAN: u64 = 0x7FF8_0000_0000_0000;

/// 用于检测是否为我们的 boxing 值的掩码 (检查指数位)
#[allow(dead_code)]
const EXP_MASK: u64 = 0x7FF0_0000_0000_0000; // bits 62-52

/// Tag 掩码: bits 50-44 (7位)
/// 注意: bit 51 是 QNAN 标志位，所以我们用 bits 50-44 存储 Tag
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
// 5-7: 预留

// 8-23: 内联整数 (-8 ~ +7)
#[allow(dead_code)]
const TAG_INLINE_INT_START: u64 = 8 << 44; // 对应 -8
#[allow(dead_code)]
const TAG_INLINE_INT_END: u64 = 23 << 44; // 对应 +7

// 24-31: 预留内联值

// 32+: 堆类型
const TAG_HEAP_OBJECT: u64 = 32 << 44; // 通用堆对象
const TAG_STRING: u64 = 33 << 44; // 字符串对象
const TAG_FUNCTION: u64 = 34 << 44; // 函数对象
const TAG_LIST: u64 = 35 << 44; // 列表对象
const TAG_ITERATOR: u64 = 36 << 44; // 迭代器对象
const TAG_CLOSURE: u64 = 37 << 44; // 闭包对象
const TAG_COROUTINE: u64 = 38 << 44; // 协程对象
const TAG_RESULT: u64 = 39 << 44; // Result 对象
const TAG_OPTION: u64 = 40 << 44; // Option 对象
const TAG_JSON: u64 = 41 << 44; // JSON 对象
const TAG_MODULE: u64 = 42 << 44; // 模块对象
const TAG_NATIVE: u64 = 43 << 44; // 原生函数对象
const TAG_NATIVE_VM: u64 = 44 << 44; // VM-aware 原生函数对象
// 44-127: 预留其他堆类型

/// SMI 最大值 (2^30 - 1)
const SMI_MAX: i32 = (1 << 30) - 1;
/// SMI 最小值 (-2^30)
const SMI_MIN: i32 = -(1 << 30);

impl Value {
    // ==================== 构造方法 ====================

    /// 创建 SMI (小整数)
    /// 范围: -2^30 ~ 2^30-1 (约 ±10亿)
    #[inline]
    pub fn smi(n: i32) -> Self {
        debug_assert!(n >= SMI_MIN && n <= SMI_MAX, "SMI out of range: {}", n);
        // 保留低 31 位 (30 位数值 + 符号)
        let payload = (n as u64) & ((1 << 31) - 1);
        Self(QNAN | TAG_SMI | payload)
    }

    /// 创建整数（自动选择最优编码）
    /// - -8~7: 内联整数（Tag 编码）
    /// - 其他 SMI 范围: SMI（31位 Payload 编码）
    /// - 超出 SMI 范围: panic（未来可扩展为堆分配 BigInt）
    #[inline]
    pub fn int(n: i32) -> Self {
        match n {
            // 内联整数范围: -8 ~ 7 → Tag 8 ~ 23
            -8..=7 => {
                let tag = ((n + 16) as u64) << 44; // -8→8, 0→16, 7→23
                Self(QNAN | tag)
            }
            // SMI 范围
            SMI_MIN..=SMI_MAX => Self::smi(n),
            // 超出范围（未来应返回 Result 或使用 BigInt）
            _ => panic!("Integer {} out of SMI range", n),
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

    /// 内部: 编码堆指针
    #[inline]
    fn encode_heap_ptr<T>(ptr: *mut T, tag: u64) -> Self {
        let addr = ptr as u64;
        debug_assert!(addr & 0x7 == 0, "Pointer must be 8-byte aligned");
        // 右移 3 位压缩地址（低 3 位恒为 0）
        // 44位 payload 足够存储 47位地址 (>>3 后)
        let compressed = (addr >> 3) & PAYLOAD_MASK;
        Self(QNAN | tag | compressed)
    }

    /// 创建通用堆对象
    #[inline]
    pub fn object<T>(ptr: *mut T) -> Self {
        Self::encode_heap_ptr(ptr, TAG_HEAP_OBJECT)
    }

    /// 创建字符串对象
    #[inline]
    pub fn string(ptr: *mut ObjString) -> Self {
        Self::encode_heap_ptr(ptr, TAG_STRING)
    }

    /// 创建函数对象
    #[inline]
    pub fn function(ptr: *mut ObjFunction) -> Self {
        Self::encode_heap_ptr(ptr, TAG_FUNCTION)
    }

    /// 创建列表对象
    #[inline]
    pub fn list(ptr: *mut ObjList) -> Self {
        Self::encode_heap_ptr(ptr, TAG_LIST)
    }

    /// 创建迭代器对象
    #[inline]
    pub fn iterator(ptr: *mut ObjIterator) -> Self {
        Self::encode_heap_ptr(ptr, TAG_ITERATOR)
    }

    /// 创建闭包对象
    #[inline]
    pub fn closure(ptr: *mut ObjClosure) -> Self {
        Self::encode_heap_ptr(ptr, TAG_CLOSURE)
    }

    /// 创建协程对象
    #[inline]
    pub fn coroutine(ptr: *mut ObjCoroutine) -> Self {
        Self::encode_heap_ptr(ptr, TAG_COROUTINE)
    }

    /// 创建 Result 对象
    #[inline]
    pub fn result(ptr: *mut ObjResult) -> Self {
        Self::encode_heap_ptr(ptr, TAG_RESULT)
    }

    /// 创建 Option 对象
    #[inline]
    pub fn option(ptr: *mut ObjOption) -> Self {
        Self::encode_heap_ptr(ptr, TAG_OPTION)
    }

    /// 创建 JSON 对象
    #[inline]
    pub fn json(ptr: *mut ObjJson) -> Self {
        Self::encode_heap_ptr(ptr, TAG_JSON)
    }

    /// 创建模块对象
    #[inline]
    pub fn module(ptr: *mut ObjModule) -> Self {
        Self::encode_heap_ptr(ptr, TAG_MODULE)
    }

    /// 创建原生函数对象
    #[inline]
    pub fn native_fn(ptr: *mut ObjNative) -> Self {
        Self::encode_heap_ptr(ptr, TAG_NATIVE)
    }

    /// 创建 VM-aware 原生函数对象
    #[inline]
    pub fn native_vm_fn(ptr: *mut crate::runtime::object::ObjNativeVm) -> Self {
        Self::encode_heap_ptr(ptr, TAG_NATIVE_VM)
    }

    // ==================== 类型判断 ====================

    /// 是否为我们的 boxing 值 (非普通浮点数)
    /// 检测: 指数位全为 1 (E=0x7FF) 且 bit 51 = 1 (QNAN 标志)
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
    fn raw_tag(&self) -> u64 {
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
        tag >= 8 && tag <= 23 && payload == 0
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

    /// 是否为真值
    /// Kaubo 中只有 false 和 null 为假，其他都为真（包括 0 和空字符串）
    #[inline]
    pub fn is_truthy(&self) -> bool {
        !self.is_false() && !self.is_null()
    }

    /// 是否为通用堆对象
    #[inline]
    pub fn is_heap(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 32
    }

    /// 是否为字符串对象
    #[inline]
    pub fn is_string(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 33
    }

    /// 是否为函数对象
    #[inline]
    pub fn is_function(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 34
    }

    /// 是否为列表对象
    #[inline]
    pub fn is_list(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 35
    }

    /// 是否为迭代器对象
    #[inline]
    pub fn is_iterator(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 36
    }

    /// 是否为闭包对象
    #[inline]
    pub fn is_closure(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 37
    }

    /// 是否为协程对象
    #[inline]
    pub fn is_coroutine(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 38
    }

    /// 是否为 Result 对象
    #[inline]
    pub fn is_result(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 39
    }

    /// 是否为 Option 对象
    #[inline]
    pub fn is_option(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 40
    }

    /// 是否为 JSON 对象
    #[inline]
    pub fn is_json(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 41
    }

    /// 是否为模块对象
    #[inline]
    pub fn is_module(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 42
    }

    /// 是否为原生函数对象
    #[inline]
    pub fn is_native(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 43
    }

    /// 是否为 VM-aware 原生函数对象
    #[inline]
    pub fn is_native_vm(&self) -> bool {
        self.is_boxed() && self.raw_tag() == 44
    }

    // ==================== 解包方法 ====================

    /// 解包为 SMI (i32)
    #[inline]
    pub fn as_smi(&self) -> Option<i32> {
        if self.is_smi() {
            let payload = self.0 & ((1 << 31) - 1);
            // 符号扩展
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
        // 内联整数: Tag >= 8 且 Payload = 0 (没有额外 payload)
        let tag = self.raw_tag();
        let payload = self.0 & PAYLOAD_MASK;
        if tag >= 8 && tag <= 23 && payload == 0 {
            Some((tag as i32) - 16) // 8->-8, 16->0, 23->7
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

    /// 内部: 解码堆指针
    #[inline]
    fn decode_heap_ptr<T>(&self, expected_tag: u64) -> Option<*mut T> {
        if self.is_boxed() && (self.0 & TAG_MASK) == expected_tag {
            let compressed = self.0 & PAYLOAD_MASK;
            Some(((compressed << 3) as usize) as *mut T)
        } else {
            None
        }
    }

    /// 解包为通用堆对象指针
    #[inline]
    pub fn as_object<T>(&self) -> Option<*mut T> {
        self.decode_heap_ptr(TAG_HEAP_OBJECT)
    }

    /// 解包为字符串对象指针
    #[inline]
    pub fn as_string(&self) -> Option<*mut ObjString> {
        self.decode_heap_ptr(TAG_STRING)
    }

    /// 解包为函数对象指针
    #[inline]
    pub fn as_function(&self) -> Option<*mut ObjFunction> {
        self.decode_heap_ptr(TAG_FUNCTION)
    }

    /// 解包为列表对象指针
    #[inline]
    pub fn as_list(&self) -> Option<*mut ObjList> {
        self.decode_heap_ptr(TAG_LIST)
    }

    /// 解包为迭代器对象指针
    #[inline]
    pub fn as_iterator(&self) -> Option<*mut ObjIterator> {
        self.decode_heap_ptr(TAG_ITERATOR)
    }

    /// 解包为闭包对象指针
    #[inline]
    pub fn as_closure(&self) -> Option<*mut ObjClosure> {
        self.decode_heap_ptr(TAG_CLOSURE)
    }

    /// 解包为协程对象
    #[inline]
    pub fn as_coroutine(&self) -> Option<*mut ObjCoroutine> {
        self.decode_heap_ptr(TAG_COROUTINE)
    }

    /// 解包为 Result 对象
    #[inline]
    pub fn as_result(&self) -> Option<*mut ObjResult> {
        self.decode_heap_ptr(TAG_RESULT)
    }

    /// 解包为 Option 对象
    #[inline]
    pub fn as_option(&self) -> Option<*mut ObjOption> {
        self.decode_heap_ptr(TAG_OPTION)
    }

    /// 解包为 JSON 对象
    #[inline]
    pub fn as_json(&self) -> Option<*mut ObjJson> {
        self.decode_heap_ptr(TAG_JSON)
    }

    /// 解包为模块对象
    #[inline]
    pub fn as_module(&self) -> Option<*mut ObjModule> {
        self.decode_heap_ptr(TAG_MODULE)
    }

    /// 解包为原生函数对象
    #[inline]
    pub fn as_native(&self) -> Option<*mut ObjNative> {
        self.decode_heap_ptr(TAG_NATIVE)
    }

    /// 解包为 VM-aware 原生函数对象
    #[inline]
    pub fn as_native_vm(&self) -> Option<*mut crate::runtime::object::ObjNativeVm> {
        self.decode_heap_ptr(TAG_NATIVE_VM)
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

// ==================== Debug 输出 ====================

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_float() {
            write!(f, "Float({})", self.as_float())
        } else if self.is_inline_int() {
            write!(f, "InlineInt({})", self.as_inline_int().unwrap())
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
        } else if self.is_string() {
            write!(f, "String({:p})", self.as_string().unwrap())
        } else if self.is_list() {
            write!(f, "List({:p})", self.as_list().unwrap())
        } else if self.is_iterator() {
            write!(f, "Iterator({:p})", self.as_iterator().unwrap())
        } else if self.is_closure() {
            write!(f, "Closure")
        } else if self.is_heap() {
            write!(f, "Object({:p})", self.as_object::<()>().unwrap())
        } else if self.is_coroutine() {
            write!(f, "Coroutine")
        } else if self.is_result() {
            write!(f, "Result")
        } else if self.is_option() {
            write!(f, "Option")
        } else if self.is_json() {
            write!(f, "Json")
        } else if self.is_module() {
            write!(f, "Module")
        } else if self.is_native() {
            write!(f, "Native")
        } else if self.is_native_vm() {
            write!(f, "NativeVm")
        } else {
            write!(f, "Value({:016x})", self.0)
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_float() {
            write!(f, "{}", self.as_float())
        } else if self.is_inline_int() {
            write!(f, "{}", self.as_inline_int().unwrap())
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
        } else if self.is_string() {
            if let Some(ptr) = self.as_string() {
                unsafe { write!(f, "{}", (*ptr).chars) }
            } else {
                write!(f, "<string>")
            }
        } else if self.is_list() {
            if let Some(ptr) = self.as_list() {
                unsafe {
                    write!(f, "[")?;
                    let list = &*ptr;
                    for (i, elem) in list.elements.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", elem)?;
                    }
                    write!(f, "]")
                }
            } else {
                write!(f, "<list>")
            }
        } else if self.is_iterator() {
            write!(f, "<iterator>")
        } else if self.is_closure() {
            write!(f, "<closure>")
        } else if self.is_coroutine() {
            write!(f, "<coroutine>")
        } else if self.is_result() {
            write!(f, "<result>")
        } else if self.is_option() {
            write!(f, "<option>")
        } else if self.is_json() {
            if let Some(ptr) = self.as_json() {
                unsafe {
                    write!(f, "{{")?;
                    let json = &*ptr;
                    let entries: Vec<_> = json.entries.iter().collect();
                    for (i, (key, value)) in entries.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "\"{}\": {}", key, value)?;
                    }
                    write!(f, "}}")
                }
            } else {
                write!(f, "{{}}")
            }
        } else if self.is_module() {
            write!(f, "<module>")
        } else if self.is_native() {
            write!(f, "<native>")
        } else if self.is_native_vm() {
            write!(f, "<native_vm>")
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
        assert!(!v.is_inline_int());
        assert_eq!(v.as_smi(), Some(42));
        assert_eq!(v.as_int(), Some(42));
    }

    #[test]
    fn test_smi_negative() {
        let v = Value::smi(-100);
        assert!(v.is_smi());
        assert_eq!(v.as_smi(), Some(-100));
        assert_eq!(v.as_int(), Some(-100));
    }

    #[test]
    fn test_smi_bounds() {
        let max = SMI_MAX;
        let min = SMI_MIN;
        assert_eq!(Value::smi(max).as_smi(), Some(max));
        assert_eq!(Value::smi(min).as_smi(), Some(min));
    }

    #[test]
    fn test_inline_int() {
        // 测试内联整数范围 -8 ~ 7
        for n in -8..=7 {
            let v = Value::int(n);
            assert!(v.is_inline_int(), "{} should be inline int", n);
            assert!(!v.is_smi(), "{} should not be SMI", n);
            assert!(v.is_int(), "{} should be int", n);
            assert_eq!(v.as_inline_int(), Some(n));
            assert_eq!(v.as_int(), Some(n));
        }
    }

    #[test]
    fn test_int_auto_selection() {
        // -8~7 应该使用内联编码
        assert!(Value::int(-8).is_inline_int());
        assert!(Value::int(0).is_inline_int());
        assert!(Value::int(7).is_inline_int());

        // 超出内联范围的 SMI 应该使用 SMI 编码
        assert!(Value::int(8).is_smi());
        assert!(Value::int(-9).is_smi());
        assert!(Value::int(100).is_smi());
    }

    #[test]
    fn test_float_creation() {
        let v = Value::float(3.14);
        assert!(v.is_float());
        assert!(!v.is_smi());
        assert!(!v.is_inline_int());
        assert!(!v.is_heap());
        assert_eq!(v.as_float(), 3.14);
    }

    #[test]
    fn test_special_values() {
        assert!(Value::NULL.is_null());
        assert!(!Value::NULL.is_smi());
        assert!(!Value::NULL.is_inline_int());

        assert!(Value::TRUE.is_true());
        assert!(Value::TRUE.is_bool());
        assert!(Value::TRUE.is_truthy());
        assert_eq!(Value::TRUE.as_bool(), Some(true));

        assert!(Value::FALSE.is_false());
        assert!(Value::FALSE.is_bool());
        assert!(!Value::FALSE.is_truthy());
        assert_eq!(Value::FALSE.as_bool(), Some(false));

        assert!(!Value::NULL.is_truthy());
    }

    #[test]
    fn test_smi_zero_is_truthy() {
        let zero = Value::smi(0);
        assert!(zero.is_truthy());
        assert!(zero.is_int());
    }

    #[test]
    fn test_inline_zero_is_truthy() {
        let zero = Value::int(0);
        assert!(zero.is_truthy());
        assert!(zero.is_inline_int());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Value::smi(42)), "42");
        assert_eq!(format!("{}", Value::int(5)), "5"); // 内联
        assert_eq!(format!("{}", Value::int(-5)), "-5"); // 内联
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
    }

    #[test]
    fn test_bool_from() {
        assert!(Value::bool_from(true).is_true());
        assert!(Value::bool_from(false).is_false());
    }

    #[test]
    fn test_debug_output() {
        assert!(format!("{:?}", Value::smi(42)).contains("SMI"));
        assert!(format!("{:?}", Value::int(5)).contains("InlineInt"));
        assert!(format!("{:?}", Value::NULL).contains("Null"));
        assert!(format!("{:?}", Value::TRUE).contains("True"));
    }
}
