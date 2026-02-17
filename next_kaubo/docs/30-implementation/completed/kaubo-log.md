# kaubo-log 设计文档

> 结构化日志系统，支持崩溃转储和编译器调试
> 
> **核心设计**：平台无关，通过 feature flag 支持 `std` / `no_std+alloc` / `wasm`

## 平台支持

| Feature | 依赖 | 说明 | 适用场景 |
|---------|------|------|----------|
| `std` (默认) | `thiserror` | 完整标准库支持 | 桌面/服务器 |
| `alloc` | - | 仅分配器，无 std | 嵌入式 |
| `wasm` | `web-sys` | Web 平台支持 | 浏览器 |

## 快速开始

### 标准平台

```toml
[dependencies]
kaubo-log = "0.1"
```

```rust
use kaubo_log::{LogConfig, debug};

let (logger, ring) = LogConfig::dev().init();
debug!(logger, "应用启动成功");
```

### no_std + alloc 平台

```toml
[dependencies]
kaubo-log = { version = "0.1", default-features = false, features = ["alloc"] }
```

```rust
use kaubo_log::{Logger, Level, LogRingBuffer, debug};

// 仅支持环形缓冲区（无 stdout/stderr）
let ring = LogRingBuffer::new(1000);
let logger = Logger::new(Level::Debug).with_sink(ring);
debug!(logger, "嵌入式日志");
```

### WASM 平台

```toml
[dependencies]
kaubo-log = { version = "0.1", default-features = false, features = ["wasm"] }
```

```rust
use kaubo_log::{WasmConfig, wasm_debug};

let logger = WasmConfig::dev().init();
wasm_debug!(logger, "WASM 启动成功");
```

## 设计目标

- **平台无关**：核心逻辑不依赖 std，支持嵌入式和 WASM
- **显式传递**：无全局 logger，配置通过代码传入
- **非阻塞**：日志不卡主线程，满了覆盖旧数据
- **崩溃恢复**：环形缓冲区保留最后N条日志
- **简洁API**：宏支持，使用方式接近 tracing

## 非目标

- 复杂过滤器（运行时级别检查足够）
- 网络日志收集
- 异步/后台线程（单线程同步足够）

## 核心数据结构

### Record（所有平台）

```rust
pub struct Record {
    pub timestamp_ms: u64,
    pub level: Level,           // Trace/Debug/Info/Warn/Error
    pub target: &'static str,   // 模块路径
    pub message: String,        // 需要 alloc feature
    pub span_id: Option<u64>,
}
```

### Logger（需要 alloc）

```rust
pub struct Logger {
    level: AtomicU8,
    sinks: Mutex<Vec<Box<dyn LogSink>>>,  // 使用自旋锁
    span_stack: Mutex<Vec<Span>>,
    next_span_id: AtomicU64,
}
```

### LogRingBuffer（需要 alloc）

```rust
pub struct LogRingBuffer {
    inner: spin::Mutex<VecDeque<Record>>,  // no_std 自旋锁
    capacity: usize,
    dropped: AtomicUsize,
}
```

## 关键决策

### 1. no_std 支持

- **选择**：核心逻辑使用 `no_std` + `alloc`，通过 feature flag 控制平台代码
- **理由**：支持嵌入式和 WASM 场景，避免平台锁定

### 2. 自旋锁 vs 标准 Mutex

- **选择**：no_std 下使用自旋锁（简单实现）
- **理由**：标准库的 `std::sync::Mutex` 不可用，自旋锁在单线程/低竞争场景足够

### 3. 时间戳处理

- **std 平台**：使用 `SystemTime::now()`
- **no_std 平台**：使用单调递增计数器（实际项目应接入硬件时钟）

### 4. 宏设计

- **选择**：`#[macro_export]` 宏
- **理由**：惰性求值，no_std 兼容

## 性能预期

- 日志级别检查：~1ns（原子 load）
- 记录创建（含 format）：~100ns-1μs
- 写入环形缓冲：~50ns（自旋锁竞争）
- 预期日志量：编译10k行代码 < 10000条日志 < 1ms开销

## 测试覆盖率

> 目标：Branch Coverage ≥ 90%

### 当前状态（68 个测试）

| 指标 | 覆盖率 | 状态 |
|------|--------|------|
| **Branch Coverage** | **96.15%** | ✅ |
| **Line Coverage** | **96.75%** | ✅ |
| **Function Coverage** | **96.32%** | ✅ |

### 各文件覆盖详情

| 文件 | Branch | Line | Function | 说明 |
|------|--------|------|----------|------|
| `logger.rs` | 100% | 100% | 100% | 完美覆盖 |
| `ring_buffer.rs` | 100% | 98.62% | 96.88% | 核心模块 |
| `record.rs` | 100% | 98.91% | 100% | 数据结构 |
| `config.rs` | 87.50% | 100% | 100% | 1个平台特定分支 |
| `macros.rs` | - | 100% | 100% | 宏代码 |
| `span.rs` | - | 94.74% | 92.31% | 简单结构 |
| `wasm.rs` | - | 70.33% | 75.00% | 需浏览器环境 |

### 关键测试场景

1. **正常路径**：各级别日志写入、span 跟踪、多 sink 输出
2. **边界条件**：缓冲区溢出、禁用级别过滤、空 span 栈
3. **错误处理**：Mutex poison、文件打开失败、无效路径
4. **并发测试**：多线程竞争自旋锁
5. **平台特性**：std/alloc/wasm 条件编译代码

### 运行测试

```bash
# 运行所有测试
cargo test -p kaubo-log --all-features

# 查看覆盖率（终端）
cargo make cov-log

# 生成 HTML 报告
cargo make cov-log-html
# 报告位置: target/llvm-cov/kaubo-log/html/index.html
```

## 测试策略

1. **单元测试**：Logger 级别过滤、RingBuffer 覆盖行为
2. **平台测试**：
   - `cargo test`（std 平台）
   - `cargo test --no-default-features --features alloc`（no_std+alloc）
3. **集成测试**：崩溃时 dump 文件完整性

## 平台特定注意事项

### 标准平台
- 使用 `std::sync::Mutex`（已实现为自旋锁包装，可优化）
- 支持 stdout/stderr/文件输出

### no_std + alloc 平台
- 使用自旋锁（忙等待）
- 仅支持 `LogRingBuffer` 输出（需外部消费）
- 时间戳是单调计数器，非真实时间

### WASM 平台
- 使用 `web_sys::console` API
- 日志级别映射到 console.debug/info/warn/error
- 不支持崩溃转储文件（可用 IndexedDB 替代）

## 项目集成

### 与 tracing 的关系

本项目替换了原有的 `tracing` 日志系统。迁移原因：

1. **显式传递**：`tracing` 依赖全局 subscriber，`kaubo-log` 显式传递 Logger
2. **平台无关**：`tracing` 不支持 no_std，`kaubo-log` 原生支持
3. **测试友好**：`tracing` 难以断言日志，`kaubo-log` 的 RingBuffer 可直接读取

完整迁移记录：[tracing-to-kaubo-log-migration.md](../../30-lessons/tracing-to-kaubo-log-migration.md)

## 命名规范

⚠️ **重要**：代码中禁止使用 `_` 开头的变量名。

```rust
// ❌ 错误
let _temp = 42;
let _unused = "hello";

// ✅ 正确
let temp = 42;
drop(temp); // 显式标记不使用

#[allow(unused)]
let unused_variable = 42;
```
