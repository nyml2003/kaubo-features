//! 运行时对象定义

use crate::core::runtime::bytecode::chunk::Chunk;
use crate::core::runtime::Value;

/// Upvalue 对象 - 表示对外部变量的引用
/// 采用 Lua 风格：按引用捕获，变量离开栈时转储到 closed
#[derive(Debug)]
pub struct ObjUpvalue {
    /// 指向外部变量的指针（栈上位置）
    pub location: *mut Value,
    /// 如果变量离开栈，转储到这里
    pub closed: Option<Value>,
}

impl ObjUpvalue {
    /// 创建新的 upvalue，指向栈上的位置
    pub fn new(location: *mut Value) -> Self {
        Self {
            location,
            closed: None,
        }
    }

    /// 获取当前值（优先使用 closed，否则使用 location）
    pub fn get(&self) -> Value {
        match self.closed {
            Some(value) => value,
            None => unsafe { *self.location },
        }
    }

    /// 设置值（优先写入 closed，否则写入 location）
    pub fn set(&mut self, value: Value) {
        match self.closed {
            Some(_) => self.closed = Some(value),
            None => unsafe { *self.location = value; }
        }
    }

    /// 关闭 upvalue：将栈上的值复制到 closed
    pub fn close(&mut self) {
        if self.closed.is_none() {
            self.closed = Some(unsafe { *self.location });
            // location 不再使用，设为 null
            self.location = std::ptr::null_mut();
        }
    }
}

/// 闭包对象 - 包含函数和捕获的 upvalues
#[derive(Debug)]
pub struct ObjClosure {
    /// 原始函数
    pub function: *mut ObjFunction,
    /// 捕获的 upvalues
    pub upvalues: Vec<*mut ObjUpvalue>,
}

impl ObjClosure {
    /// 创建新的闭包
    pub fn new(function: *mut ObjFunction) -> Self {
        Self {
            function,
            upvalues: Vec::new(),
        }
    }

    /// 添加 upvalue
    pub fn add_upvalue(&mut self, upvalue: *mut ObjUpvalue) {
        self.upvalues.push(upvalue);
    }

    /// 获取 upvalue
    pub fn get_upvalue(&self, index: usize) -> Option<*mut ObjUpvalue> {
        self.upvalues.get(index).copied()
    }
}

/// 函数对象
#[derive(Debug)]
pub struct ObjFunction {
    /// 函数的字节码
    pub chunk: Chunk,
    /// 参数数量
    pub arity: u8,
    /// 函数名（用于调试）
    pub name: Option<String>,
}

impl ObjFunction {
    /// 创建新的函数对象
    pub fn new(chunk: Chunk, arity: u8, name: Option<String>) -> Self {
        Self { chunk, arity, name }
    }
}

/// 字符串对象
#[derive(Debug)]
pub struct ObjString {
    /// 字符串内容
    pub chars: String,
}

impl ObjString {
    /// 创建新的字符串对象
    pub fn new(chars: String) -> Self {
        Self { chars }
    }
}

/// 列表对象
#[derive(Debug)]
pub struct ObjList {
    /// 列表元素
    pub elements: Vec<Value>,
}

impl ObjList {
    /// 创建新的空列表
    pub fn new() -> Self {
        Self { elements: Vec::new() }
    }

    /// 从 Vec 创建列表
    pub fn from_vec(elements: Vec<Value>) -> Self {
        Self { elements }
    }

    /// 获取长度
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// 索引获取
    pub fn get(&self, index: usize) -> Option<Value> {
        self.elements.get(index).copied()
    }
}

impl Default for ObjList {
    fn default() -> Self {
        Self::new()
    }
}

/// 迭代器源（支持多种可迭代类型）
#[derive(Debug)]
pub enum IteratorSource {
    /// 列表迭代器
    List {
        list: *mut ObjList,
        index: usize,
        done: bool,
    },
    /// 协程迭代器（通过 resume 获取值）
    Coroutine {
        coroutine: *mut ObjCoroutine,
        done: bool,
    },
}

/// 迭代器对象
#[derive(Debug)]
pub struct ObjIterator {
    /// 迭代器源
    pub source: IteratorSource,
}

impl ObjIterator {
    /// 从列表创建迭代器
    pub fn from_list(list: *mut ObjList) -> Self {
        Self {
            source: IteratorSource::List {
                list,
                index: 0,
                done: false,
            },
        }
    }

    /// 从协程创建迭代器
    pub fn from_coroutine(coroutine: *mut ObjCoroutine) -> Self {
        Self {
            source: IteratorSource::Coroutine {
                coroutine,
                done: false,
            },
        }
    }

    /// 获取下一个元素
    /// 对于列表：返回元素或 None
    /// 对于协程：resume 协程，返回 yield 值；协程死亡时返回 None
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
                    // 协程迭代器暂不直接处理 resume
                    // 实际 resume 由 VM 特殊处理
                    // 这里返回一个标记值表示需要 resume
                    Some(Value::coroutine(*coroutine))
                }
            }
        }
    }

    /// 检查是否结束
    pub fn is_done(&self) -> bool {
        match &self.source {
            IteratorSource::List { done, .. } => *done,
            IteratorSource::Coroutine { done, .. } => *done,
        }
    }

    /// 获取协程指针（如果是协程迭代器）
    pub fn as_coroutine(&self) -> Option<*mut ObjCoroutine> {
        match &self.source {
            IteratorSource::Coroutine { coroutine, .. } => Some(*coroutine),
            _ => None,
        }
    }
}

/// 调用帧（协程需要，所以定义在这里）
#[derive(Debug)]
pub struct CallFrame {
    /// 当前执行的闭包
    pub closure: *mut ObjClosure,
    /// 指令指针在该帧中的偏移
    pub ip: *const u8,
    /// 该帧的局部变量数组
    pub locals: Vec<Value>,
    /// 该帧在值栈中的起始位置
    pub stack_base: usize,
}

impl CallFrame {
    /// 获取当前 chunk
    pub fn chunk(&self) -> &Chunk {
        unsafe { &(*(*self.closure).function).chunk }
    }
}

/// 协程状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoroutineState {
    /// 已挂起，可以恢复执行
    Suspended,
    /// 正在运行
    Running,
    /// 已死亡（执行完毕）
    Dead,
}

/// 协程对象 - 包含独立的执行上下文
#[derive(Debug)]
pub struct ObjCoroutine {
    /// 协程状态
    pub state: CoroutineState,
    /// 调用栈（独立的）
    pub frames: Vec<CallFrame>,
    /// 值栈（独立的）
    pub stack: Vec<Value>,
    /// 打开的 upvalues
    pub open_upvalues: Vec<*mut ObjUpvalue>,
    /// 入口闭包（用于初始化）
    pub entry_closure: *mut ObjClosure,
}

impl ObjCoroutine {
    /// 创建新的协程（初始状态为 Suspended）
    pub fn new(entry_closure: *mut ObjClosure) -> Self {
        Self {
            state: CoroutineState::Suspended,
            frames: Vec::with_capacity(64),
            stack: Vec::with_capacity(256),
            open_upvalues: Vec::new(),
            entry_closure,
        }
    }

    /// 检查协程是否可以恢复
    pub fn is_resumable(&self) -> bool {
        self.state == CoroutineState::Suspended
    }

    /// 检查协程是否已死亡
    pub fn is_dead(&self) -> bool {
        self.state == CoroutineState::Dead
    }
}

// ==================== Result 和 Option 类型 ====================

/// Result 类型变体
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultVariant {
    Ok,
    Err,
}

/// Result 对象 - 表示可能失败的操作
#[derive(Debug)]
pub struct ObjResult {
    /// Ok 或 Err
    pub variant: ResultVariant,
    /// 存储的值
    pub value: Value,
}

impl ObjResult {
    /// 创建 Ok 结果
    pub fn ok(value: Value) -> Self {
        Self {
            variant: ResultVariant::Ok,
            value,
        }
    }

    /// 创建 Err 结果
    pub fn err(value: Value) -> Self {
        Self {
            variant: ResultVariant::Err,
            value,
        }
    }

    /// 是否为 Ok
    pub fn is_ok(&self) -> bool {
        self.variant == ResultVariant::Ok
    }

    /// 是否为 Err
    pub fn is_err(&self) -> bool {
        self.variant == ResultVariant::Err
    }

    /// 获取 Ok 值（如果是 Ok）
    pub fn ok_value(&self) -> Option<Value> {
        if self.is_ok() {
            Some(self.value)
        } else {
            None
        }
    }

    /// 获取 Err 值（如果是 Err）
    pub fn err_value(&self) -> Option<Value> {
        if self.is_err() {
            Some(self.value)
        } else {
            None
        }
    }
}

/// Option 类型变体
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionVariant {
    Some,
    None,
}

/// Option 对象 - 表示可能不存在的值
#[derive(Debug)]
pub struct ObjOption {
    /// Some 或 None
    pub variant: OptionVariant,
    /// Some 时存储的值
    pub value: Value,
}

impl ObjOption {
    /// 创建 Some
    pub fn some(value: Value) -> Self {
        Self {
            variant: OptionVariant::Some,
            value,
        }
    }

    /// 创建 None
    pub fn none() -> Self {
        Self {
            variant: OptionVariant::None,
            value: Value::NULL,
        }
    }

    /// 是否为 Some
    pub fn is_some(&self) -> bool {
        self.variant == OptionVariant::Some
    }

    /// 是否为 None
    pub fn is_none(&self) -> bool {
        self.variant == OptionVariant::None
    }

    /// 获取值（如果是 Some）
    pub fn value(&self) -> Option<Value> {
        if self.is_some() {
            Some(self.value)
        } else {
            None
        }
    }
}

/// 协程状态值（用于在 Kaubo 代码中表示协程状态）
pub const COROUTINE_STATE_SUSPENDED: i64 = 0;
pub const COROUTINE_STATE_RUNNING: i64 = 1;
pub const COROUTINE_STATE_DEAD: i64 = 2;

/// JSON 对象 - 用于 JSON 字面量声明
/// 功能丰富的 JSON 数据类型，支持对象、数组、嵌套结构
#[derive(Debug)]
pub struct ObjJson {
    /// 键值对存储（使用 HashMap）
    pub entries: std::collections::HashMap<String, Value>,
}

impl ObjJson {
    /// 创建空 JSON 对象
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// 从 HashMap 创建
    pub fn from_hashmap(entries: std::collections::HashMap<String, Value>) -> Self {
        Self { entries }
    }

    /// 获取值
    pub fn get(&self, key: &str) -> Option<Value> {
        self.entries.get(key).copied()
    }

    /// 设置值
    pub fn set(&mut self, key: String, value: Value) {
        self.entries.insert(key, value);
    }

    /// 检查是否包含键
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// 获取长度
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ObjJson {
    fn default() -> Self {
        Self::new()
    }
}

/// 模块对象 - 用于存储模块导出项
/// 导出项按 ShapeID 索引，编译期完全确定，运行时静态布局
#[derive(Debug)]
pub struct ObjModule {
    /// 模块名
    pub name: String,
    /// 导出项数组（固定长度，按 ShapeID 索引）
    pub exports: Box<[Value]>,
    /// 名称到 ShapeID 的映射（编译期/调试用）
    pub name_to_index: std::collections::HashMap<String, u16>,
}

impl ObjModule {
    /// 创建模块对象（编译期构建）
    pub fn new(name: String, exports: Vec<Value>, name_to_index: std::collections::HashMap<String, u16>) -> Self {
        Self {
            name,
            exports: exports.into_boxed_slice(),
            name_to_index,
        }
    }

    /// 通过 ShapeID 获取导出项（O(1)）
    pub fn get_by_shape_id(&self, shape_id: u16) -> Option<Value> {
        self.exports.get(shape_id as usize).copied()
    }

    /// 通过名称获取 ShapeID（编译期/调试用）
    pub fn get_shape_id(&self, name: &str) -> Option<u16> {
        self.name_to_index.get(name).copied()
    }

    /// 获取导出项数量
    pub fn len(&self) -> usize {
        self.exports.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.exports.is_empty()
    }
}

impl Default for ObjModule {
    fn default() -> Self {
        Self::new(String::new(), Vec::new(), std::collections::HashMap::new())
    }
}

/// 原生函数类型
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

/// 原生函数对象 - 包装 Rust 函数
#[derive(Debug)]
pub struct ObjNative {
    /// 函数指针
    pub function: NativeFn,
    /// 函数名（用于调试）
    pub name: String,
    /// 参数数量（用于校验）
    pub arity: u8,
}

impl ObjNative {
    /// 创建新的原生函数对象
    pub fn new(function: NativeFn, name: String, arity: u8) -> Self {
        Self {
            function,
            name,
            arity,
        }
    }

    /// 调用原生函数
    pub fn call(&self, args: &[Value]) -> Result<Value, String> {
        (self.function)(args)
    }
}
