//! API 类型定义
//!
//! 编译和执行的输入输出类型。

use kaubo_core::{Chunk, ObjShape, Value};

/// 编译输出
#[derive(Debug)]
pub struct CompileOutput {
    /// 字节码块
    pub chunk: Chunk,
    /// 局部变量数量
    pub local_count: usize,
    /// 生成的 struct shapes（需要注册到 VM）
    pub shapes: Vec<ObjShape>,
}

/// 执行输出
#[derive(Debug)]
pub struct ExecuteOutput {
    /// 返回值
    pub value: Option<Value>,
    /// 标准输出捕获
    pub stdout: String,
}
