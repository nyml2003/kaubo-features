//! Value 扩展方法
//!
//! 为 core::Value 提供堆对象操作方法。
//! 这些方法需要知道具体的 ObjXxx 类型，所以在 runtime 层实现。

use crate::vm::core::object::{
    ObjClosure, ObjCoroutine, ObjFunction, ObjIterator, ObjJson, ObjList, ObjModule, ObjNative,
    ObjNativeVm, ObjOption, ObjResult, ObjShape, ObjString, ObjStruct,
};
use crate::vm::core::value::{
    Value, TAG_CLOSURE, TAG_COROUTINE, TAG_FUNCTION, TAG_ITERATOR, TAG_JSON, TAG_LIST, TAG_MODULE,
    TAG_NATIVE, TAG_NATIVE_VM, TAG_OPTION, TAG_RESULT, TAG_SHAPE, TAG_STRING, TAG_STRUCT,
};

impl Value {
    // ==================== 堆对象构造方法 ====================

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
    pub fn native_vm_fn(ptr: *mut ObjNativeVm) -> Self {
        Self::encode_heap_ptr(ptr, TAG_NATIVE_VM)
    }

    /// 创建 Struct 实例对象
    #[inline]
    pub fn struct_instance(ptr: *mut ObjStruct) -> Self {
        Self::encode_heap_ptr(ptr, TAG_STRUCT)
    }

    /// 创建 Shape 描述符对象
    #[inline]
    pub fn shape(ptr: *mut ObjShape) -> Self {
        Self::encode_heap_ptr(ptr, TAG_SHAPE)
    }

    // ==================== 类型判断 ====================

    /// 是否为字符串对象
    #[inline]
    pub fn is_string(&self) -> bool {
        self.is_tagged(TAG_STRING)
    }

    /// 是否为函数对象
    #[inline]
    pub fn is_function(&self) -> bool {
        self.is_tagged(TAG_FUNCTION)
    }

    /// 是否为列表对象
    #[inline]
    pub fn is_list(&self) -> bool {
        self.is_tagged(TAG_LIST)
    }

    /// 是否为迭代器对象
    #[inline]
    pub fn is_iterator(&self) -> bool {
        self.is_tagged(TAG_ITERATOR)
    }

    /// 是否为闭包对象
    #[inline]
    pub fn is_closure(&self) -> bool {
        self.is_tagged(TAG_CLOSURE)
    }

    /// 是否为协程对象
    #[inline]
    pub fn is_coroutine(&self) -> bool {
        self.is_tagged(TAG_COROUTINE)
    }

    /// 是否为 Result 对象
    #[inline]
    pub fn is_result(&self) -> bool {
        self.is_tagged(TAG_RESULT)
    }

    /// 是否为 Option 对象
    #[inline]
    pub fn is_option(&self) -> bool {
        self.is_tagged(TAG_OPTION)
    }

    /// 是否为 JSON 对象
    #[inline]
    pub fn is_json(&self) -> bool {
        self.is_tagged(TAG_JSON)
    }

    /// 是否为模块对象
    #[inline]
    pub fn is_module(&self) -> bool {
        self.is_tagged(TAG_MODULE)
    }

    /// 是否为原生函数对象
    #[inline]
    pub fn is_native(&self) -> bool {
        self.is_tagged(TAG_NATIVE)
    }

    /// 是否为 VM-aware 原生函数对象
    #[inline]
    pub fn is_native_vm(&self) -> bool {
        self.is_tagged(TAG_NATIVE_VM)
    }

    /// 是否为 Struct 实例对象
    #[inline]
    pub fn is_struct(&self) -> bool {
        self.is_tagged(TAG_STRUCT)
    }

    /// 是否为 Shape 描述符对象
    #[inline]
    pub fn is_shape(&self) -> bool {
        self.is_tagged(TAG_SHAPE)
    }

    // ==================== 解包方法 ====================

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

    /// 解包为协程对象指针
    #[inline]
    pub fn as_coroutine(&self) -> Option<*mut ObjCoroutine> {
        self.decode_heap_ptr(TAG_COROUTINE)
    }

    /// 解包为 Result 对象指针
    #[inline]
    pub fn as_result(&self) -> Option<*mut ObjResult> {
        self.decode_heap_ptr(TAG_RESULT)
    }

    /// 解包为 Option 对象指针
    #[inline]
    pub fn as_option(&self) -> Option<*mut ObjOption> {
        self.decode_heap_ptr(TAG_OPTION)
    }

    /// 解包为 JSON 对象指针
    #[inline]
    pub fn as_json(&self) -> Option<*mut ObjJson> {
        self.decode_heap_ptr(TAG_JSON)
    }

    /// 解包为模块对象指针
    #[inline]
    pub fn as_module(&self) -> Option<*mut ObjModule> {
        self.decode_heap_ptr(TAG_MODULE)
    }

    /// 解包为原生函数对象指针
    #[inline]
    pub fn as_native(&self) -> Option<*mut ObjNative> {
        self.decode_heap_ptr(TAG_NATIVE)
    }

    /// 解包为 VM-aware 原生函数对象指针
    #[inline]
    pub fn as_native_vm(&self) -> Option<*mut ObjNativeVm> {
        self.decode_heap_ptr(TAG_NATIVE_VM)
    }

    /// 解包为 Struct 实例对象指针
    #[inline]
    pub fn as_struct(&self) -> Option<*mut ObjStruct> {
        self.decode_heap_ptr(TAG_STRUCT)
    }

    /// 解包为 Shape 描述符对象指针
    #[inline]
    pub fn as_shape(&self) -> Option<*mut ObjShape> {
        self.decode_heap_ptr(TAG_SHAPE)
    }
}

// ==================== Debug 扩展 ====================

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
        } else if self.is_struct() {
            write!(f, "Struct({:p})", self.as_struct().unwrap())
        } else if self.is_shape() {
            write!(f, "Shape({:p})", self.as_shape().unwrap())
        } else {
            write!(f, "Value({:016x})", self.0)
        }
    }
}

// ==================== Display 扩展 ====================

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
                unsafe { write!(f, "\'{}\'", (*ptr).chars) }
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
                        write!(f, "{elem}")?;
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
                        write!(f, "\"{key}\": {value}")?;
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
        } else if self.is_struct() {
            if let Some(ptr) = self.as_struct() {
                unsafe {
                    let shape = &*(*ptr).shape;
                    write!(f, "{} {{ ", shape.name)?;
                    for (i, field) in (*ptr).fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        if let Some(name) = shape.field_names.get(i) {
                            write!(f, "{name}: {field}")?;
                        } else {
                            write!(f, "{field}")?;
                        }
                    }
                    write!(f, " }}")
                }
            } else {
                write!(f, "<struct>")
            }
        } else if self.is_shape() {
            write!(f, "<shape>")
        } else {
            write!(f, "<value>")
        }
    }
}
