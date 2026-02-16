//! 运行时对象定义 (Core 层)
//!
//! 纯类型定义，与 Value 形成循环依赖的解决方式：
//! - ObjXxx 中存储 Value 的地方直接使用 Value 类型
//! - Value 的堆对象方法在 runtime/value_impl.rs 中实现

use super::chunk::Chunk;
use super::operators::Operator;
use super::value::Value;
use std::collections::HashMap;

// ==================== Upvalue ====================

/// Upvalue 对象 - 表示对外部变量的引用
#[derive(Debug)]
pub struct ObjUpvalue {
    /// 指向外部变量的指针（栈上位置）
    pub location: *mut Value,
    /// 如果变量离开栈，转储到这里
    pub closed: Option<Value>,
}

impl ObjUpvalue {
    pub fn new(location: *mut Value) -> Self {
        Self { location, closed: None }
    }

    pub fn get(&self) -> Value {
        match self.closed {
            Some(value) => value,
            None => unsafe { *self.location },
        }
    }

    pub fn set(&mut self, value: Value) {
        match self.closed {
            Some(_) => self.closed = Some(value),
            None => unsafe { *self.location = value },
        }
    }

    pub fn close(&mut self) {
        if self.closed.is_none() {
            self.closed = Some(unsafe { *self.location });
            self.location = std::ptr::null_mut();
        }
    }
}

// ==================== Function & Closure ====================

/// 函数对象
#[derive(Debug)]
pub struct ObjFunction {
    pub chunk: Chunk,
    pub arity: u8,
    pub name: Option<String>,
}

impl ObjFunction {
    pub fn new(chunk: Chunk, arity: u8, name: Option<String>) -> Self {
        Self { chunk, arity, name }
    }
}

/// 闭包对象
#[derive(Debug)]
pub struct ObjClosure {
    pub function: *mut ObjFunction,
    pub upvalues: Vec<*mut ObjUpvalue>,
}

impl ObjClosure {
    pub fn new(function: *mut ObjFunction) -> Self {
        Self { function, upvalues: Vec::new() }
    }

    pub fn add_upvalue(&mut self, upvalue: *mut ObjUpvalue) {
        self.upvalues.push(upvalue);
    }

    pub fn get_upvalue(&self, index: usize) -> Option<*mut ObjUpvalue> {
        self.upvalues.get(index).copied()
    }
}

// ==================== String & List ====================

/// 字符串对象
#[derive(Debug)]
pub struct ObjString {
    pub chars: String,
}

impl ObjString {
    pub fn new(chars: String) -> Self {
        Self { chars }
    }
}

/// 列表对象
#[derive(Debug)]
pub struct ObjList {
    pub elements: Vec<Value>,
}

impl ObjList {
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    pub fn from_vec(elements: Vec<Value>) -> Self {
        Self { elements }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<Value> {
        self.elements.get(index).copied()
    }
}

impl Default for ObjList {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Iterator ====================

/// 迭代器源
#[derive(Debug)]
pub enum IteratorSource {
    List {
        list: *mut ObjList,
        index: usize,
        done: bool,
    },
    Coroutine {
        coroutine: *mut ObjCoroutine,
        done: bool,
    },
    Json {
        json: *mut ObjJson,
        keys: Vec<String>,
        index: usize,
        done: bool,
    },
}

/// 迭代器对象
#[derive(Debug)]
pub struct ObjIterator {
    pub source: IteratorSource,
}

impl ObjIterator {
    pub fn from_list(list: *mut ObjList) -> Self {
        Self {
            source: IteratorSource::List { list, index: 0, done: false },
        }
    }

    pub fn from_coroutine(coroutine: *mut ObjCoroutine) -> Self {
        Self {
            source: IteratorSource::Coroutine { coroutine, done: false },
        }
    }

    /// # Safety
    /// `json` 必须是有效的、非空的指向 `ObjJson` 的指针
    pub unsafe fn from_json(json: *mut ObjJson) -> Self {
        let keys = { (*json).entries.keys().cloned().collect() };
        Self {
            source: IteratorSource::Json { json, keys, index: 0, done: false },
        }
    }

    pub fn is_done(&self) -> bool {
        match &self.source {
            IteratorSource::List { done, .. } => *done,
            IteratorSource::Coroutine { done, .. } => *done,
            IteratorSource::Json { done, .. } => *done,
        }
    }

    pub fn as_coroutine(&self) -> Option<*mut ObjCoroutine> {
        match &self.source {
            IteratorSource::Coroutine { coroutine, .. } => Some(*coroutine),
            _ => None,
        }
    }

    /// 获取下一个元素
    pub fn next(&mut self) -> Option<Value> {
        match &mut self.source {
            IteratorSource::List { list, index, done } => {
                if *done {
                    return None;
                }
                unsafe {
                    let list_ref = &**list;
                    if *index < list_ref.len() {
                        let value = list_ref.get(*index)?;
                        *index += 1;
                        Some(value)
                    } else {
                        *done = true;
                        None
                    }
                }
            }
            IteratorSource::Coroutine { coroutine, done } => {
                if *done {
                    return None;
                }
                unsafe {
                    let coro = &mut **coroutine;
                    if coro.state == CoroutineState::Dead {
                        *done = true;
                        return None;
                    }
                    Some(Value::coroutine(*coroutine))
                }
            }
            IteratorSource::Json { keys, index, done, .. } => {
                if *done {
                    return None;
                }
                if *index < keys.len() {
                    let key = keys[*index].clone();
                    *index += 1;
                    let key_obj = Box::new(ObjString::new(key));
                    Some(Value::string(Box::into_raw(key_obj)))
                } else {
                    *done = true;
                    None
                }
            }
        }
    }
}

// ==================== CallFrame & Coroutine ====================

/// 调用帧
#[derive(Debug)]
pub struct CallFrame {
    pub closure: *mut ObjClosure,
    pub ip: *const u8,
    pub locals: Vec<Value>,
    pub stack_base: usize,
}

impl CallFrame {
    pub fn chunk(&self) -> &Chunk {
        unsafe { &(*(*self.closure).function).chunk }
    }
}

/// 协程状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoroutineState {
    Suspended,
    Running,
    Dead,
}

/// 协程对象
#[derive(Debug)]
pub struct ObjCoroutine {
    pub state: CoroutineState,
    pub frames: Vec<CallFrame>,
    pub stack: Vec<Value>,
    pub open_upvalues: Vec<*mut ObjUpvalue>,
    pub entry_closure: *mut ObjClosure,
}

impl ObjCoroutine {
    pub fn new(entry_closure: *mut ObjClosure) -> Self {
        Self {
            state: CoroutineState::Suspended,
            frames: Vec::with_capacity(64),
            stack: Vec::with_capacity(256),
            open_upvalues: Vec::new(),
            entry_closure,
        }
    }

    pub fn is_resumable(&self) -> bool {
        self.state == CoroutineState::Suspended
    }

    pub fn is_dead(&self) -> bool {
        self.state == CoroutineState::Dead
    }
}

// ==================== Result & Option ====================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultVariant {
    Ok,
    Err,
}

#[derive(Debug)]
pub struct ObjResult {
    pub variant: ResultVariant,
    pub value: Value,
}

impl ObjResult {
    pub fn ok(value: Value) -> Self {
        Self { variant: ResultVariant::Ok, value }
    }

    pub fn err(value: Value) -> Self {
        Self { variant: ResultVariant::Err, value }
    }

    pub fn is_ok(&self) -> bool {
        self.variant == ResultVariant::Ok
    }

    pub fn is_err(&self) -> bool {
        self.variant == ResultVariant::Err
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionVariant {
    Some,
    None,
}

#[derive(Debug)]
pub struct ObjOption {
    pub variant: OptionVariant,
    pub value: Value,
}

impl ObjOption {
    pub fn some(value: Value) -> Self {
        Self { variant: OptionVariant::Some, value }
    }

    pub fn none() -> Self {
        Self { variant: OptionVariant::None, value: Value::NULL }
    }

    pub fn is_some(&self) -> bool {
        self.variant == OptionVariant::Some
    }

    pub fn is_none(&self) -> bool {
        self.variant == OptionVariant::None
    }
}

// ==================== JSON ====================

#[derive(Debug)]
pub struct ObjJson {
    pub entries: HashMap<String, Value>,
}

impl ObjJson {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    pub fn from_hashmap(entries: HashMap<String, Value>) -> Self {
        Self { entries }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.entries.get(key).copied()
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.entries.insert(key, value);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ObjJson {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Module ====================

#[derive(Debug)]
pub struct ObjModule {
    pub name: String,
    pub exports: Box<[Value]>,
    pub name_to_index: HashMap<String, u16>,
}

impl ObjModule {
    pub fn new(name: String, exports: Vec<Value>, name_to_index: HashMap<String, u16>) -> Self {
        Self {
            name,
            exports: exports.into_boxed_slice(),
            name_to_index,
        }
    }

    pub fn get_by_shape_id(&self, shape_id: u16) -> Option<Value> {
        self.exports.get(shape_id as usize).copied()
    }

    pub fn get_shape_id(&self, name: &str) -> Option<u16> {
        self.name_to_index.get(name).copied()
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.get_shape_id(name).and_then(|id| self.get_by_shape_id(id))
    }
}

// ==================== Native Functions ====================

/// 原生函数类型
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

#[derive(Debug)]
pub struct ObjNative {
    pub function: NativeFn,
    pub name: String,
    pub arity: u8,
}

impl ObjNative {
    pub fn new(function: NativeFn, name: String, arity: u8) -> Self {
        Self { function, name, arity }
    }

    pub fn call(&self, args: &[Value]) -> Result<Value, String> {
        (self.function)(args)
    }
}

/// VM-aware 原生函数类型（VM 指针在 runtime 层解析）
pub type NativeVmFn = fn(*mut (), &[Value]) -> Result<Value, String>;

#[derive(Debug)]
pub struct ObjNativeVm {
    pub function: NativeVmFn,
    pub name: String,
    pub arity: u8,
}

impl ObjNativeVm {
    pub fn new(function: NativeVmFn, name: String, arity: u8) -> Self {
        Self { function, name, arity }
    }
}

// ==================== Shape & Struct ====================

/// Shape 描述符
#[derive(Debug, Clone)]
pub struct ObjShape {
    pub shape_id: u16,
    pub name: String,
    pub field_names: Vec<String>,
    pub field_types: Vec<String>,  // 字段类型（与 field_names 对应）
    pub methods: Vec<*mut ObjFunction>,
    pub method_names: HashMap<String, u8>,
    pub operators: HashMap<Operator, *mut ObjClosure>,
}

impl ObjShape {
    pub fn new(shape_id: u16, name: String, field_names: Vec<String>) -> Self {
        Self {
            shape_id,
            name,
            field_names,
            field_types: Vec::new(),
            methods: Vec::new(),
            method_names: HashMap::new(),
            operators: HashMap::new(),
        }
    }

    /// 创建带字段类型的 Shape
    pub fn new_with_types(shape_id: u16, name: String, field_names: Vec<String>, field_types: Vec<String>) -> Self {
        Self {
            shape_id,
            name,
            field_names,
            field_types,
            methods: Vec::new(),
            method_names: HashMap::new(),
            operators: HashMap::new(),
        }
    }

    pub fn register_method(&mut self, name: String, method: *mut ObjFunction) -> u8 {
        let idx = self.methods.len() as u8;
        self.methods.push(method);
        self.method_names.insert(name, idx);
        idx
    }

    pub fn get_method(&self, idx: u8) -> Option<*mut ObjFunction> {
        self.methods.get(idx as usize).copied()
    }

    pub fn get_method_index(&self, name: &str) -> Option<u8> {
        self.method_names.get(name).copied()
    }

    pub fn get_field_index(&self, name: &str) -> Option<u8> {
        self.field_names.iter().position(|n| n == name).map(|i| i as u8)
    }

    pub fn field_count(&self) -> usize {
        self.field_names.len()
    }

    pub fn register_operator(&mut self, op: Operator, closure: *mut ObjClosure) {
        self.operators.insert(op, closure);
    }

    pub fn get_operator(&self, op: Operator) -> Option<*mut ObjClosure> {
        self.operators.get(&op).copied()
    }

    pub fn has_operator(&self, op: Operator) -> bool {
        self.operators.contains_key(&op)
    }
}

/// Struct 实例
#[derive(Debug)]
pub struct ObjStruct {
    pub shape: *const ObjShape,
    pub fields: Vec<Value>,
}

impl ObjStruct {
    pub fn new(shape: *const ObjShape, fields: Vec<Value>) -> Self {
        Self { shape, fields }
    }

    pub fn get_field(&self, idx: usize) -> Option<Value> {
        self.fields.get(idx).copied()
    }

    pub fn set_field(&mut self, idx: usize, value: Value) {
        if idx < self.fields.len() {
            self.fields[idx] = value;
        }
    }

    pub fn shape_id(&self) -> u16 {
        unsafe { (*self.shape).shape_id }
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}
