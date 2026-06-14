//! 虚拟机定义 (Core 层)
//!
//! 纯类型定义，执行逻辑在 runtime/vm_impl.rs 中

use crate::builtin_methods::BuiltinMethodTable;
use crate::object::{CallFrame, ObjShape, ObjUpvalue};
use crate::operators::InlineCacheEntry;
use crate::value::Value;
use crate::error::RuntimeError;
use crate::interfaces::ErrorReporter;
use std::collections::HashMap;
use std::sync::Arc;
use kaubo_log::Logger;

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
    RuntimeError(RuntimeError),
}

impl InterpretResult {
    pub fn runtime_error(msg: impl Into<String>) -> Self {
        InterpretResult::RuntimeError(RuntimeError::other(msg))
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, InterpretResult::Ok)
    }
}

impl From<String> for InterpretResult {
    fn from(s: String) -> Self {
        InterpretResult::RuntimeError(RuntimeError::other(s))
    }
}

impl From<RuntimeError> for InterpretResult {
    fn from(e: RuntimeError) -> Self {
        InterpretResult::RuntimeError(e)
    }
}

/// 输出回调类型
pub type OutputCallback = Box<dyn Fn(&str) + Send + Sync>;

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
    /// Logger（用于执行追踪）
    pub logger: Arc<Logger>,
    /// 内置类型方法表
    pub builtin_methods: BuiltinMethodTable,
    /// 输出回调（用于 print 语句等）
    output_callback: Option<OutputCallback>,
    /// 错误回调（用于 CLI/WASM 错误报告）
    pub error_reporter: Option<Box<dyn ErrorReporter>>,
}

impl VM {
    /// 创建新的虚拟机（使用默认配置）
    pub fn new() -> Self {
        Self::with_config(VMConfig::default())
    }

    /// 创建新的虚拟机（带配置）
    pub fn with_config(config: VMConfig) -> Self {
        Self::with_config_and_logger(config, Logger::noop())
    }

    /// 创建新的虚拟机（带配置和 logger）
    pub fn with_config_and_logger(config: VMConfig, logger: Arc<Logger>) -> Self {
        Self {
            stack: Vec::with_capacity(config.initial_stack_size),
            frames: Vec::with_capacity(config.initial_frames_capacity),
            open_upvalues: Vec::new(),
            globals: HashMap::new(),
            shapes: HashMap::new(),
            inline_caches: Vec::with_capacity(config.inline_cache_capacity),
            logger,
            builtin_methods: BuiltinMethodTable::new(),
            output_callback: None,
            error_reporter: None,
        }
    }
    
    /// 设置输出回调
    pub fn set_output_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.output_callback = Some(Box::new(callback));
    }

    /// 设置错误报告器
    pub fn set_error_reporter(&mut self, reporter: Box<dyn ErrorReporter>) {
        self.error_reporter = Some(reporter);
    }
    
    /// 输出消息（通过回调或默认到 stdout）
    pub fn output(&self, message: &str) {
        if let Some(ref callback) = self.output_callback {
            callback(message);
        } else {
            println!("{}", message);
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
