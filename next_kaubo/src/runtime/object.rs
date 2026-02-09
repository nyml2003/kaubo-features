//! 运行时对象定义

use crate::runtime::bytecode::chunk::Chunk;
use crate::runtime::Value;

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

/// 列表迭代器
#[derive(Debug)]
pub struct ListIterator {
    /// 列表指针
    list: *mut ObjList,
    /// 当前索引
    index: usize,
    /// 是否结束
    done: bool,
}

impl ListIterator {
    /// 创建新的列表迭代器
    pub fn new(list: *mut ObjList) -> Self {
        Self { list, index: 0, done: false }
    }

    /// 获取下一个元素，None 表示结束
    pub fn next(&mut self) -> Option<Value> {
        if self.done {
            return None;
        }
        
        unsafe {
            let list = &*self.list;
            if self.index < list.len() {
                let value = list.get(self.index)?;
                self.index += 1;
                Some(value)
            } else {
                self.done = true;
                None
            }
        }
    }

    /// 检查是否结束
    pub fn is_done(&self) -> bool {
        self.done
    }
}

/// 迭代器对象（包装 Box<dyn Iterator>）
#[derive(Debug)]
pub struct ObjIterator {
    /// 迭代器具体实现
    pub inner: ListIterator, // 目前只支持列表迭代器
}

impl ObjIterator {
    /// 从列表创建迭代器
    pub fn from_list(list: *mut ObjList) -> Self {
        Self {
            inner: ListIterator::new(list),
        }
    }

    /// 获取下一个元素
    pub fn next(&mut self) -> Option<Value> {
        self.inner.next()
    }

    /// 检查是否结束
    pub fn is_done(&self) -> bool {
        self.inner.is_done()
    }
}
