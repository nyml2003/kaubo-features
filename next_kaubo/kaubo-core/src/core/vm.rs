//! 虚拟机定义 (Core 层)
//!
//! 纯类型定义，执行逻辑在 runtime/vm_impl.rs 中

use super::object::{CallFrame, ObjShape, ObjUpvalue};
use super::operators::InlineCacheEntry;
use super::value::Value;
use std::collections::HashMap;

/// 虚拟机配置
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// 初始栈容量
    pub initial_stack_size: usize,
    /// 初始调用帧容量
    pub initial_frames_capacity: usize,
    /// 内联缓存容量
    pub inline_cache_capacity: usize,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            initial_stack_size: 256,
            initial_frames_capacity: 64,
            inline_cache_capacity: 64,
        }
    }
}

/// 解释执行结果
#[derive(Debug, Clone, PartialEq)]
pub enum InterpretResult {
    Ok,
    CompileError(String),
    RuntimeError(String),
}

/// 虚拟机
///
/// 注意：执行逻辑在 runtime/vm_impl.rs 中通过 impl VM 添加
pub struct VM {
    /// 操作数栈
    pub stack: Vec<Value>,
    /// 调用栈
    pub frames: Vec<CallFrame>,
    /// 打开的 upvalues
    pub open_upvalues: Vec<*mut ObjUpvalue>,
    /// 全局变量表
    pub globals: HashMap<String, Value>,
    /// Shape 表
    pub shapes: HashMap<u16, *const ObjShape>,
    /// 内联缓存表
    pub inline_caches: Vec<InlineCacheEntry>,
}

impl VM {
    /// 创建新的虚拟机（使用默认配置）
    pub fn new() -> Self {
        Self::with_config(VMConfig::default())
    }

    /// 创建新的虚拟机（带配置）
    pub fn with_config(config: VMConfig) -> Self {
        Self {
            stack: Vec::with_capacity(config.initial_stack_size),
            frames: Vec::with_capacity(config.initial_frames_capacity),
            open_upvalues: Vec::new(),
            globals: HashMap::new(),
            shapes: HashMap::new(),
            inline_caches: Vec::with_capacity(config.inline_cache_capacity),
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
