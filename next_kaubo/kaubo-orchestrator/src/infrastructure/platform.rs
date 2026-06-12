//! 基础设施层 — 平台相关实现
//!
//! 实现 domain/interfaces.rs 中定义的 trait。

use crate::domain::interfaces::{Allocator, OutputPort, Platform};
use crate::vm::core::object::{ObjList, ObjString};
use crate::vm::core::Value;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================
// NativePlatform
// ============================================================

/// 原生平台实现（Windows/Mac/Linux）
pub struct NativePlatform;

impl Platform for NativePlatform {
    fn read_file(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("read_file({path}): {e}"))
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        std::fs::write(path, content).map_err(|e| format!("write_file({path}): {e}"))
    }

    fn path_exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    fn is_file(&self, path: &str) -> bool {
        std::path::Path::new(path).is_file()
    }

    fn is_dir(&self, path: &str) -> bool {
        std::path::Path::new(path).is_dir()
    }

    fn now_secs(&self) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn stdout_write(&self, text: &str) {
        println!("{text}");
    }
}

// ============================================================
// NativeAllocator
// ============================================================

/// 原生内存分配器（基于 Box::into_raw）
pub struct NativeAllocator;

impl Allocator for NativeAllocator {
    fn alloc_string(&self, s: String) -> Value {
        let obj = Box::new(ObjString::new(s));
        Value::string(Box::into_raw(obj))
    }

    fn alloc_list(&self, elements: Vec<Value>) -> Value {
        let obj = Box::new(ObjList::from_vec(elements));
        Value::list(Box::into_raw(obj))
    }

    fn free_value(&self, value: Value) {
        if let Some(ptr) = value.as_string() {
            unsafe { drop(Box::from_raw(ptr)); }
        } else if let Some(ptr) = value.as_list() {
            unsafe { drop(Box::from_raw(ptr)); }
        }
    }
}

// ============================================================
// StdoutOutput
// ============================================================

/// 标准输出端口
pub struct StdoutOutput;

impl OutputPort for StdoutOutput {
    fn writeln(&self, text: &str) {
        println!("{text}");
    }

    fn write_raw(&self, _data: &[u8]) {
        // debug output, ignore for stdout
    }

    fn emit_file(&self, path: &str, data: &[u8]) -> Result<(), String> {
        std::fs::write(path, data).map_err(|e| format!("emit_file({path}): {e}"))
    }
}

// ============================================================
// MemoryOutput
// ============================================================

/// 内存输出端口（测试用）
pub struct MemoryOutput {
    lines: std::sync::Mutex<Vec<String>>,
}

impl MemoryOutput {
    pub fn new() -> Self {
        Self {
            lines: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl OutputPort for MemoryOutput {
    fn writeln(&self, text: &str) {
        self.lines.lock().unwrap().push(text.to_string());
    }

    fn write_raw(&self, _data: &[u8]) {}

    fn emit_file(&self, _path: &str, _data: &[u8]) -> Result<(), String> {
        Ok(())
    }

    fn collected(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
}
