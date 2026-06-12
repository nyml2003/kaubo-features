//! 领域端口定义 — 纯 trait，无平台依赖
//!
//! 所有 I/O、内存分配、平台操作都通过 trait 抽象，
//! 领域逻辑不直接接触 std::fs、println!、Box::into_raw 等。

use crate::Value;

// ============================================================
// Platform — 操作系统/运行时抽象
// ============================================================

/// 平台抽象：将 stdlib 中的 I/O 操作隔离到 infrastructure 层。
///
/// 不同平台提供不同实现：
/// - `NativePlatform` (Windows/Mac/Linux)
/// - `WasmPlatform` (WebAssembly — 未来)
pub trait Platform: Send + Sync {
    /// 读取文件内容
    fn read_file(&self, path: &str) -> Result<String, String>;

    /// 写入文件内容
    fn write_file(&self, path: &str, content: &str) -> Result<(), String>;

    /// 检查路径是否存在
    fn path_exists(&self, path: &str) -> bool;

    /// 检查是否为文件
    fn is_file(&self, path: &str) -> bool;

    /// 检查是否为目录
    fn is_dir(&self, path: &str) -> bool;

    /// 获取当前 Unix 时间戳（秒，浮点精度）
    fn now_secs(&self) -> f64;

    /// 获取环境变量
    fn env_var(&self, name: &str) -> Option<String>;

    /// 打印到标准输出
    fn stdout_write(&self, text: &str);
}

// ============================================================
// Allocator — 内存管理抽象
// ============================================================

/// 内存分配器：将 heap object 的创建和销毁集中管理。
///
/// 当前实现使用 `Box::into_raw`，未来可替换为 GC。
pub trait Allocator: Send + Sync {
    /// 分配字符串对象
    fn alloc_string(&self, s: String) -> Value;

    /// 分配列表对象
    fn alloc_list(&self, elements: Vec<Value>) -> Value;

    /// 释放 VM 对象占用的内存
    fn free_value(&self, value: Value);
}

// ============================================================
// SourceRepo — 源码仓库抽象
// ============================================================

/// 源码加载接口。
///
/// 不同来源有不同实现：
/// - `FileSourceRepo` — 从文件系统读取
/// - `MemorySourceRepo` — 从内存中读取（测试用）
/// - `StdinSourceRepo` — 从标准输入读取
pub trait SourceRepo: Send + Sync {
    /// 判断指定路径是否存在
    fn exists(&self, path: &str) -> bool;

    /// 读取源码内容
    fn read(&self, path: &str) -> Result<String, String>;

    /// 读取二进制内容
    fn read_binary(&self, path: &str) -> Result<Vec<u8>, String>;

    /// 列出目录下的文件（用于模块解析）
    fn list_dir(&self, dir: &str) -> Result<Vec<String>, String>;

    /// 获取根目录（用于相对路径解析）
    fn root_dir(&self) -> String;
}

// ============================================================
// OutputPort — 输出端口抽象
// ============================================================

/// 编译器/VM 输出接口。
///
/// 实现：
/// - `StdoutOutput` — 打印到控制台
/// - `MemoryOutput` — 收集到 Vec（用于测试）
/// - `CallbackOutput` — 回调函数（用于嵌入）
pub trait OutputPort: Send + Sync {
    /// 输出一行文本
    fn writeln(&self, text: &str);

    /// 输出调试/字节码信息
    fn write_raw(&self, data: &[u8]);

    /// 写入编译产物到磁盘
    fn emit_file(&self, path: &str, data: &[u8]) -> Result<(), String>;

    /// 获取所有已收集的输出（仅 MemoryOutput）
    fn collected(&self) -> Vec<String> {
        Vec::new()
    }
}

// ============================================================
// DomainLogger — 领域日志抽象
// ============================================================

/// 日志接口：纯 trait，不依赖具体 logger 实现。
///
/// 默认实现可对接 `kaubo_log::Logger`。
pub trait DomainLogger: Send + Sync {
    fn trace(&self, msg: &str);
    fn debug(&self, msg: &str);
    fn info(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn error(&self, msg: &str);
}
