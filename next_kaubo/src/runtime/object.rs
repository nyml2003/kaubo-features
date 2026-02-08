//! 运行时对象定义

use crate::runtime::bytecode::chunk::Chunk;
use crate::runtime::Value;

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
