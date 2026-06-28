# 事件与日志系统

目标读者：维护编译器、运行时、以及所有需要调试输出的 Stage 的开发者。

## 设计原则

**Stage 不感知输出，只发结构化事件；编排层监听事件后统一输出。**

```
┌────────────────────────────────────────────────────────────────┐
│                    Orchestration Layer                          │
│  Driver 组装 CompositeHandler，同时分发到多个输出              │
│                                                                │
│  CompositeHandler (in kaubo-driver)                            │
│    ├── ConsoleHandler (stderr / browser console)               │
│    ├── FileHandler (用户实现)                                   │
│    └── LocalStorageHandler (用户实现)                           │
│                                                                │
│  Stage 只发 ToolchainEvent（结构化），不碰输出，不知道有几个   │
└────────────────────────────────────────────────────────────────┘
         ↑  events via EventHandler trait (单入口)
  ┌──────┴──────┐  ┌──────────┐  ┌──────────┐
  │  CPS Build  │  │    VM    │  │  Passes  │
  └─────────────┘  └──────────┘  └──────────┘
```

- **Stage** 只感知自己的结构化输入/输出，运行时通过 `EventHandler` trait 发事件
- **编排层 (Driver)** 组装一个或多个 `EventHandler` 实例并注入。原生可组合 `ConsoleHandler` + `FileHandler`；WASM 可组合 `ConsoleHandler` + `LocalStorageHandler`；测试用 Mock
- **跨平台**：同一套 trait + 事件类型 + Stage 代码，原生和 WASM 无差异。只有编排层组装哪些 handler 的决策不同

## 业界对齐

本设计与以下编译器/VM 项目的做法一致：

| 系统 | 机制 | 与本设计的关系 |
|------|------|---------------|
| **LLVM** | `PassInstrumentation` 虚接口 + `DEBUG_TYPE` 按 pass 隔离 | `EventHandler` trait ≈ `PassInstrumentation`；嵌套 event enum ≈ `DEBUG_TYPE` |
| **.NET CLR** | `EventSource.WriteEvent()` + `EventListener.OnEventWritten()` | 同构的事件发射/消费分离；`IsEnabled()` 预检查 → `filter()` |
| **JVM JFR** | `Event.shouldCommit()` 让 JIT 消掉整个记录块 | `filter()` 的极端优化版本 |
| **V8** | `--trace-*` flags + `PrintF` | `KAUBO_LOG` 环境变量 ≈ V8 的 `--trace-turbo` 等 flag 模式 |
| **Rust `log` / `tracing`** | `log` trait (抽象) + `env_logger` (实现) 分离 | `kaubo-log` (trait only) + `kaubo-driver` (handler impls) |
| **Wasmtime** | `log` crate + 各模块随意写 | 我们没有采纳——Stage 耦合输出难以测试 |

## Crate 职责

### `kaubo-log` —— 只提供抽象（零平台代码）

```
kaubo-log (leaf, 零外部依赖，零平台代码)
  ↑
kaubo-ir ── kaubo-vm ── kaubo-driver (只依赖抽象)
                ↑              ↑              ↑
                kaubo-log-handlers (ConsoleHandler, CompositeHandler)
                             ↑              ↑
                     kaubo2-cli          kaubo-wasm
```

`kaubo-log` 只包含三样东西，不包含任何具体 handler：

| 内容 | 文件 | 说明 |
|------|------|------|
| `ToolchainEvent` / `VmEvent` / `CpsEvent` / `PassEvent` | `event.rs` | 结构化事件类型 |
| `EventHandler` trait + `NoopHandler` | `handler.rs` | 抽象接口 |
| `emit!` 宏 | `macros.rs` | 编译期零开销的发射入口 |

**不在 `kaubo-log` 中**：`ConsoleHandler`、`CompositeHandler`、`parse_env()`。这些都进 `kaubo-log-handlers`。

设计理由：对齐 Rust 生态的 `log`（trait only） vs `env_logger`（实现）分离模式。`kaubo-log` 纯抽象、不依赖任何平台特性，确保所有 Stage crate（`kaubo-ir`、`kaubo-vm`）和 `kaubo-driver` 不引入平台代码。

### `kaubo-log-handlers` —— 提供具体 handler（新建 crate）

```
kaubo-log-handlers/
  Cargo.toml      — 依赖 kaubo-log + web-sys (wasm32 only) + wasm-bindgen (wasm32 only)
  src/
    lib.rs         — re-export ConsoleHandler, CompositeHandler
    console.rs     — ConsoleHandler (eprintln! / web_sys::console)
    composite.rs   — CompositeHandler (广播语义)
    env.rs         — parse_env(), init_from_env()
```

所有具体 handler 实现在 `kaubo-log-handlers` 中，与 `kaubo-driver` 平级。

**`kaubo-driver` 不依赖 `kaubo-log-handlers`**。Driver 只持有 `Option<&dyn kaubo_log::EventHandler>`，由调用方（CLI / WASM / 测试）构造具体 handler 后注入。

## 事件类型（按 Stage 嵌套拆分）

按子系统拆分的理由：避免中心 enum 成为所有 Stage 改动的瓶颈（业界先例：.NET `EventSource` 按组件 / LLVM `DEBUG_TYPE` 按 pass）。

```rust
// 在 kaubo-log/src/event.rs 中

/// VM 执行阶段事件
pub enum VmEvent {
    /// 每条指令执行（仅 trace 级别，量极大）
    Instruction { func: usize, ip: usize, opcode: u8, inst: u32 },
    /// 循环迭代计数
    LoopIteration { func_idx: usize, block_id: usize, count: u64 },
    /// 接近循环上限（≥80%），预警用
    LoopNearLimit { func_idx: usize, block_id: usize, count: u64, limit: u64 },
}

/// CPS / IR 构建阶段事件
pub enum CpsEvent {
    /// while/for 循环降级为 CPS blocks
    WhileLowered { header: usize, body: usize, exit: usize },
    /// 新 CPS block 创建
    BlockCreated { id: usize, param_count: usize },
}

/// Pass 优化阶段事件
pub enum PassEvent {
    Started { name: &'static str },
    Finished { name: &'static str },
}

/// 顶层事件——所有 Stage 通过此类型发事件
pub enum ToolchainEvent {
    Vm(VmEvent),
    Cps(CpsEvent),
    Pass(PassEvent),
    /// 通用诊断（任意 Stage 可用，作为未分类事件的兜底）
    Diagnostic { level: Severity, stage: &'static str, message: String },
}

pub enum Severity {
    Trace = 0,
    Debug = 1,
    Info  = 2,
    Warn  = 3,
    Error = 4,
}
```

## 双层架构：宏（编译层）+ trait（运行层）

事件系统分两层，各司其职：

```
编译层  →  #[cfg(feature = "kaubo-debug-log")] 控制代码是否存在
               │
运行层  →  EventHandler trait 控制事件是否被处理
```

**编译层**（`emit!` 宏）负责零开销保证。**运行层**（`EventHandler` trait）负责灵活的路由和格式化。

### EventHandler trait（运行层，在 `kaubo-log` 中）

```rust
// kaubo-log/src/handler.rs

pub trait EventHandler {
    /// 廉价预检查。返回 false 时跳过格式化。
    /// 业界先例：.NET `IsEnabled()`, JVM `shouldCommit()`, Rust `log_enabled!()`
    fn filter(&self, event: &ToolchainEvent) -> bool;

    /// 处理事件。仅当 `filter()` 返回 true 时被调用。
    fn handle(&self, event: &ToolchainEvent);
}

/// 关闭日志时的空实现
pub struct NoopHandler;
impl EventHandler for NoopHandler {
    fn filter(&self, _: &ToolchainEvent) -> bool { false }
    fn handle(&self, _: &ToolchainEvent) {}
}
```

### `emit!` 宏（编译层，在 `kaubo-log` 中——Stage 的唯一发射入口）

```rust
// kaubo-log/src/macros.rs

// ── feature ON ──
#[cfg(feature = "kaubo-debug-log")]
macro_rules! emit {
    ($events:expr, $event_type:ident, $variant:ident { $($field:ident: $value:expr),* $(,)? }) => {
        if let Some(h) = $events {
            let evt = $crate::ToolchainEvent::$event_type(
                $crate::$event_type::$variant { $($field: $value),* }
            );
            if h.filter(&evt) {
                h.handle(&evt);
            }
        }
    };
}

// ── feature OFF (release) ──
#[cfg(not(feature = "kaubo-debug-log"))]
macro_rules! emit {
    ($events:expr, $($rest:tt)*) => {
        // 编译期展开为空。不读取 $events，不求值任何参数。
        // 等价于 LLVM 的 `((void)0)` / C 的 `#ifndef NDEBUG ... #endif`
        let _ = &$events; // 抑制 unused variable 告警，编译器优化后零指令
    };
}
```

### Stage 端使用模式

```rust
// 编译阶段：FuncCtx 持有 events 引用
fn build_while(&mut self, cond: &Expr, body: &Expr) -> Result<...> {
    // ...
    emit!(self.ctx.events, Cps, WhileLowered { header, body, exit });
    // ...
}

// 运行时：events 作为 execute() 参数传入（不存 VM struct，避免生命周期复杂化）
pub fn execute(&mut self, entry_func: usize, reg_count: usize,
               events: Option<&dyn kaubo_log::EventHandler>) -> Result<i64, RuntimeError>
```

`EventHandler` 作为函数参数传入而非存 VM struct，生命周期在函数签名上最清晰。LLVM new pass manager 的 `AnalysisManager` 也采用同模式。

## 内置 Handler（在 `kaubo-driver` 中，不在 `kaubo-log` 中）

`kaubo-log` 只提供 trait。所有具体的 `EventHandler` 实现在 `kaubo-driver` 中。

### CompositeHandler（多输出组合——纯广播）

将事件分发到多个下游 handler。对 Stage 完全透明——Stage 只看到 `&dyn EventHandler`，不知道后面是 1 个 handler 还是 5 个。

**广播语义**：`CompositeHandler` **不重写 `filter` 方法**（永远返回 `true`）。`handle` 方法直接遍历所有子 handler，由每个子 handler 自身的 `filter` + `handle` 决定是否处理和输出。Release 下 `emit!` 宏完全消除导致整个路径不存在；Debug 下空 Vec 遍历开销可忽略。

```rust
// kaubo-log-handlers/src/composite.rs

pub struct CompositeHandler {
    handlers: Vec<Box<dyn kaubo_log::EventHandler>>,
}

impl EventHandler for CompositeHandler {
    fn filter(&self, _event: &ToolchainEvent) -> bool {
        true  // 广播：永远返回 true，让子 handler 各自决策
    }
    fn handle(&self, event: &ToolchainEvent) {
        for h in &self.handlers {
            if h.filter(event) {
                h.handle(event);
            }
        }
    }
}

impl CompositeHandler {
    pub fn new() -> Self { Self { handlers: vec![] } }
    pub fn with(mut self, handler: Box<dyn EventHandler>) -> Self {
        self.handlers.push(handler); self
    }
}
```

### ConsoleHandler（文本输出到诊断流）

**显式处理原生和 WASM 两条路径**，不依赖隐式映射：

```rust
// kaubo-log-handlers/src/console.rs

pub struct ConsoleHandler {
    pub min_level: Severity,
}

impl EventHandler for ConsoleHandler {
    fn filter(&self, event: &ToolchainEvent) -> bool {
        match event {
            ToolchainEvent::Vm(VmEvent::Instruction { .. }) =>
                self.min_level <= Severity::Trace,
            ToolchainEvent::Vm(_) =>
                self.min_level <= Severity::Debug,
            ToolchainEvent::Cps(_) | ToolchainEvent::Pass(_) =>
                self.min_level <= Severity::Debug,
            ToolchainEvent::Diagnostic { level, .. } =>
                *level >= self.min_level,
        }
    }

    fn handle(&self, event: &ToolchainEvent) {
        let formatted = match event {
            ToolchainEvent::Vm(e) => format_vm(e),
            ToolchainEvent::Cps(e) => format_cps(e),
            ToolchainEvent::Pass(e) => format_pass(e),
            ToolchainEvent::Diagnostic { level, stage, message } => {
                format!("[{stage} {level:?}] {message}")
            }
        };

        // ── 两条显式路径，不依赖 wasm-bindgen 的 eprintln! → console.error 隐式映射 ──
        #[cfg(not(target_arch = "wasm32"))]
        { eprintln!("{formatted}"); }

        #[cfg(target_arch = "wasm32")]
        { web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&formatted)); }
    }
}
```

**命名理由**：不叫 `StderrHandler`——不假设输出是 stderr。WASM 上没有 stderr 概念。

**设计理由**：不依赖 wasm-bindgen 对 `eprintln!` 的隐式 remap。那条 remap 是 wasm-bindgen 的实现细节，不在 Kaubo 的控制范围内。显式 `#[cfg]` 分支让两条路径在源码中可见、可 grep、可审计。

## KAUBO_LOG 环境变量（在 `kaubo-driver` 中）

沿用 `RUST_LOG` 格式（Rust 生态事实标准）：

```
KAUBO_LOG=debug              # 所有 stage 默认 debug 级别
KAUBO_LOG=vm=trace           # 只 vm 开 trace
KAUBO_LOG=debug,vm=trace     # 默认 debug，vm 额外开到 trace
```

**优先级**：CLI `--log-level` > `KAUBO_LOG` 环境变量 > 默认（不输出）。

`parse_env()` 和 `init_from_env()` 在 `kaubo-driver` 中实现（它们依赖 `std::env::var`，属于平台感知代码）。

## WASM 程序化初始化

WASM 中 `std::env::var` 不可用，走程序化 API：

```rust
// kaubo-log-handlers 中
pub fn make_handler(level: Severity) -> CompositeHandler {
    CompositeHandler::new()
        .with(Box::new(ConsoleHandler { min_level: level }))
}

// kaubo-wasm 暴露给 JS：
#[wasm_bindgen]
pub fn set_log_level(level: u8) {
    let severity = kaubo_log::Severity::from(level);
    let handler = kaubo_log_handlers::make_handler(severity);
    // 存储到全局状态，供后续 compile/run 使用
}
```

JS 侧：`kaubo.set_log_level(1)` 开 Debug 日志，事件通过 `ConsoleHandler` 显式写入 `console.error`。

## 三通道输出分类

不同目的的输出走不同通道，互不干扰：

| 通道 | 用途 | 输出目标 | 受 EventHandler 控制 | 业界先例 |
|------|------|---------|---------------------|---------|
| **事件通道** | 编译/运行时诊断 | 由 handler 决定（stderr / console / 文件...） | ✅ 由 handler 格式化 | LLVM `dbgs()` |
| **测试通道** | `#[cfg(test)]` 调试 dump | stderr | ❌ 保留 `eprintln!` | LLVM `ASSERT_*` |
| **用户通道** | CLI 正常输出 | stdout | ❌ 保留 `println!` | LLVM `outs()` |

## 零开销保证

### 保证模型：编译期消除，不是运行时判空

| 方案 | 机制 | 可证明性 | 二进制代价 |
|------|------|---------|-----------|
| `Option<&dyn EventHandler>` 判空 | 运行时分支，靠 CPU 预测器 | ❌ 依赖优化器 | 一条 `test` + `jz`（但有 dead code） |
| **`emit!` 宏 + feature gate**（我们） | 条件编译，宏体展开为空 | ✅ 编译期保证 | **0 条指令，0 字节** |

业界做法一致：

| 项目 | 编译期消除机制 |
|------|---------------|
| **LLVM** | `LLVM_DEBUG(X)` → `#ifndef NDEBUG ... #endif` → 零指令 |
| **V8** | `DCHECK(condition)` → release 展开为 `((void)0)` |
| **Rust `log`** | `log::debug!()` + `release_max_level_off` → 宏展开为空 |
| **.NET** | `[Conditional("DEBUG")]` → 整个调用在 IL link 时移除 |
| **Kaubo** | `emit!()` + `#[cfg(not(feature = "kaubo-debug-log"))]` → 展开为空 |

### Feature 行为

```
kaubo2-cli → kaubo-driver → kaubo-vm → kaubo-ir → kaubo-log
                ↓               ↓           ↓           ↓
           kaubo-debug-log  kaubo-debug-log  ...  kaubo-debug-log (canonical)
```

| Feature | `${events}` | `emit!` 展开 | 二进制代价 | 适用 |
|---------|------------|-------------|-----------|------|
| **OFF** (release) | 参数保留但宏不读 | 空 | 0 字节 | 生产环境 |
| **ON** (debug/CI) | `Some(&handler)` | `if let Some(h) { let evt = ...; if h.filter(&evt) { h.handle(&evt); } }` | 检查 + vtable + 格式化 | 开发 / CI |
| **ON + WASM** | `Some(&handler)` | 同上（`ConsoleHandler` 显式 `#[cfg]` 分支） | 同上 | Web Playground 调试面板 |

### 验证方法

**方法 1**：`cargo-asm` 对比

```bash
cargo asm --lib "kaubo_vm::VM::execute" --release > release.s        # feature OFF
cargo asm --lib "kaubo_vm::VM::execute" --release --features kaubo-debug-log > debug.s
diff release.s debug.s  # debug build 多了 emit 展开的代码
```

**方法 2**：参数不求值测试——feature OFF 时 `emit!` 不触碰参数

```rust
let events: Option<&dyn kaubo_log::EventHandler> = None;
emit!(events, Vm, LoopIteration { func_idx: nonexistent_field, block_id: 0, count: 0 });
// feature OFF → 编译通过（参数不求值）
// feature ON  → 编译失败（nonexistent_field 不存在）—— 正确！
```

**方法 3**：`strings` 检查二进制

```bash
strings target/release/kaubo2 | grep -i "LoopIteration\|WhileLowered\|StderrHandler"
# 期望：空输出
```

## 死循环检测

在 VM 层实现，通过 backward jump IP 比较检测循环迭代。

### 机制

CPS flatten 后的 block 按拓扑顺序排列，循环头的 IP 一定 ≤ 循环体的 IP。因此 `target_ip <= ip` 等价于 backward jump = 循环迭代。

```rust
// VM 内部
loop_iter_counts: HashMap<(func_idx, block_id), u64>

// Opcode::Branch handler 中
let target_ip = self.block_ip(block_id);
if target_ip <= ip {                              // backward jump
    let key = (self.current_func, block_id);
    let count = self.loop_iter_counts.entry(key).or_insert(0);
    *count += 1;

    emit!(events, Vm, LoopIteration { func_idx: self.current_func, block_id, count: *count });

    if *count > self.max_loop_iterations {
        return Err(RuntimeError::LoopExceeded { block_id, limit: self.max_loop_iterations });
    }

    if *count >= self.max_loop_iterations * 8 / 10 {
        emit!(events, Vm, LoopNearLimit {
            func_idx: self.current_func, block_id, count: *count, limit: self.max_loop_iterations,
        });
    }
}
```

### 配置

- 默认上限：`1_000_000`
- CLI 覆盖：`--max-loop-iterations <N>`
- 错误类型：`RuntimeError::LoopExceeded { block_id, limit }`
- **计数器重置时机**：`VM::load` 方法加载新 CPS 模块时清空 `loop_iter_counts`。确保单次执行隔离，避免跨多次 `run_module` 调用累计计数。终态 DAG 下每次 `build(VMExec)` 新建 VM 并 load，计数器天然隔离。

### 为什么 Key 用 `(func_idx, block_id)`

跨函数调用时需要循环计数隔离。一个函数里的 while 循环不应该和它调用的函数里的 while 循环共享计数器。

## 执行路径（含事件流）

```
源码
  │
  ▼
Parser.parse()                         ← 无事件
  │
  ▼
infer_module()                         ← 无事件（后续可加类型推导事件）
  │
  ▼
build_module(..., events)              ← CpsEvent::WhileLowered, CpsEvent::BlockCreated
  │
  ▼
flatten_module()                       ← 无事件
  │
  ▼
run_passes(..., events)                ← PassEvent::Started, PassEvent::Finished
  │
  ▼
VM.load()                              ← 无事件
  │
  ▼
VM.execute(..., events)                ← VmEvent::Instruction, VmEvent::LoopIteration
  │                                        VmEvent::LoopNearLimit
  ▼
LoopExceeded Error / RunOutcome        ← RuntimeError 直接返回，不通过事件系统
```

## WASM 兼容性

整个事件系统从类型层到 trait 层都与 WASM 兼容。`kaubo-log` 零平台代码，所有平台感知的逻辑在 `kaubo-driver` 的 `#[cfg]` 分支中。

### 约束与设计应对

| WASM 不可用 | 对事件系统的影响 | 设计应对 |
|-------------|-----------------|---------|
| `std::env::var` | `init_from_env()` 无法读取 `KAUBO_LOG` | WASM 走 `make_handler(level)` 程序化 API；`init_from_env()` 仅在原生 CLI 使用 |
| `std::thread` | 无多线程 | 同步 `&dyn EventHandler` 回调天然单线程 |
| 文件 IO | 无法写日志文件 | 默认输出 `ConsoleHandler` 显式 `#[cfg]` 分支：原生 `eprintln!`、WASM `web_sys::console::error_1` |
| `std::sync::Mutex` | 全局状态需要替代方案 | 当前设计不需要全局状态；handler 作为函数参数传递 |

### 跨平台架构

```
                    kaubo-log (trait + 事件类型 + emit! 宏)
                   /                        \
        kaubo-ir + kaubo-vm           kaubo-log-handlers (ConsoleHandler, CompositeHandler)
                   \                        /
                kaubo-driver (只依赖 kaubo-log 抽象)
                   /                        \
        kaubo2-cli (原生)              kaubo-wasm (WASM)

原生 CLI:                                        WASM:
  CompositeHandler                                CompositeHandler
    ├── ConsoleHandler → eprintln!                  ├── ConsoleHandler → web_sys::console
    └── FileHandler (用户实现)                      └── LocalStorageHandler (用户实现)

  init_from_env()                                 make_handler(level)
```

Stage 代码（`kaubo-ir`、`kaubo-vm`）**完全相同**，不感知平台、不感知输出目标、不感知下游有几个 handler。

### 软件工程收益

因为 `EventHandler` trait 隔离了 Stage 和输出，同一套 Stage 逻辑在以下场景零改动：

- 原生 CLI 调试（`--log-level debug` → stderr）
- WASM Web Playground 调试（显式 `web_sys::console::error_1` → 浏览器 DevTools console）
- 单元测试（Mock handler → 断言事件序列）
- CI（环境变量一键开 trace）
- 后续可能的 JSON 输出 / 文件输出 / 网络上报

每个新输出目标只需实现一个 `EventHandler`，不改 Stage 代码。跨平台逻辑在 `ConsoleHandler` 内部显式 `#[cfg]` 分支，可 grep、可审计。

## 关键设计决策

| 决策 | 理由 | 业界先例 |
|------|------|---------|
| **`kaubo-log` 只提供 trait + 事件类型 + 宏** | 纯抽象层，零平台代码；实现层在 `kaubo-driver` | Rust `log` (trait only) + `env_logger` (impl) |
| **`emit!` 宏 + feature gate = 编译期消除** | 可证明的零开销，不依赖分支预测器 | LLVM `LLVM_DEBUG`, V8 `DCHECK`, .NET `[Conditional]` |
| 宏（编译层）+ trait（运行层）双层架构 | 编译期保证零开销 + 运行时灵活路由 | Rust `log` 宏 + `Log` trait |
| `EventHandler` trait 不含输出假设 | 日志系统可组合到任意输出 | — |
| `CompositeHandler` 组合模式 | 同时分发到多个输出，Stage 不感知 | .NET `EventSource` + 多 `EventListener` |
| `ConsoleHandler` 显式 `#[cfg]` 跨平台 | 不依赖 wasm-bindgen 隐式 remap | — |
| EventHandler trait 含 `filter()` | 运行层廉价预检查 | .NET `IsEnabled()`, JVM `shouldCommit()` |
| 事件按 Stage 嵌套拆分 | 避免中心 enum 成为改动瓶颈 | .NET `EventSource`, LLVM `DEBUG_TYPE` |
| KAUBO_LOG 用 RUST_LOG 格式 | Rust 生态事实标准 | `RUST_LOG=debug,hyper=info` |
| EventHandler 作为函数参数传入 | 生命周期清晰、测试友好 | LLVM new pass manager |
| 死循环检测 `(func_idx, block_id)` key | 跨函数调用隔离 | — |
| VM.debug 删除，VmInstruction 替代 | 统一由 ConsoleHandler.min_level 控制 | V8 `--trace-*` flags |
| 三通道输出分类 | 事件/测试/用户分离 | LLVM `dbgs()` / `ASSERT_*` / `outs()` |

## 决策记录（ADR）

以下为本文档所有关键决策的明确答案。

| # | 决策点 | 结论 | 理由 |
|---|--------|------|------|
| 1 | CompositeHandler filter 语义 | **纯广播**：不重写 `filter`（永远返回 `true`），`handle` 遍历子 handler 各自决策 | Release 下 `emit!` 消除整个路径；Debug 下空 Vec 开销可忽略 |
| 2 | Handler 实现放在哪 | **新建 `kaubo-log-handlers` crate** | `kaubo-log` 纯抽象；`kaubo-driver` 不依赖具体 handler；CLI/WASM/测试各自注入 |
| 3 | WASM console 按级别映射 API | **架构支持，本次不强制实现** | `ConsoleHandler::handle` 已有 `Severity`；`#[cfg(wasm32)]` 分支内 `match severity` 随时可加；不改 trait/Driver/公共接口 |
| 4 | Release 下 unused variable 警告 | **`emit!` 关闭臂 `let _ = &$events;`** | 编译器优化后零指令；不掩盖其他 unused lint |
| 5 | KAUBO_LOG 细粒度过滤 (`vm=trace`) | **架构支持，本次不强制实现** | `filter(&self, event)` 可匹配变体提取 target；`ConsoleHandler` 可扩展 `HashMap<String, Severity>`；解析在 CLI 层 |
| 6 | 循环计数器重置时机 | **`VM::load` 时清空 `loop_iter_counts`** | 单次执行隔离；终态 DAG 下每次 `build(VMExec)` 新建 VM 并 load，天然隔离 |
| 7 | 编排层终态 | **DAG 调度器（惰性求值 + 缓存）** | 所有编译单元注册为 Rule；`driver.build(ArtifactKey)` 递归求值依赖、命中缓存、拓扑排序 |
| 8 | 语义事实层角色 | **DAG 一等节点 `ArtifactKey::Semantic`** | InferRule 生成；LSP 只求值到此节点；CPS Lowering 显式依赖它；跨模块类型检查递归 `build(ImportedSemantic)` |
| 9 | 日志在 DAG + 模块下的传递契约 | **`&dyn EventHandler` 沿调用链透传，无全局状态** | 通过 `BuildContext` 获取；递归求值依赖时原样透传；未来并行用 `Arc<dyn EventHandler + Sync>` |
| 10 | 模块缓存失效策略 | **传递闭包哈希** | 模块 Key 哈希 = 源码哈希 + 所有传递依赖模块 Key 哈希；依赖变化 → 父 Key 变化 → 自动重编译 |

## 当前迭代范围

### 已实现

| 交付物 | 所在 crate | 状态 |
|--------|-----------|------|
| `ToolchainEvent` / `VmEvent` / `CpsEvent` / `PassEvent` | `kaubo-log` | ✅ |
| `EventHandler` trait + `NoopHandler` | `kaubo-log` | ✅ |
| `emit!` 宏 | `kaubo-log` | ✅ |
| `ConsoleHandler` + `CompositeHandler` | `kaubo-log-handlers` | ✅ |
| `KAUBO_LOG` env 解析 + `init_from_env()` | `kaubo-log-handlers` | ✅ |
| VM `LoopExceeded` 错误 + backward jump 检测 | `kaubo-vm` | ✅ |
| `RunConfig` + handler 透传 | `kaubo-driver` | ✅ |
| CLI `--log-level` / `--max-loop-iterations` | `kaubo2-cli` | ✅ |
| WASM `set_log_level()` | `kaubo-wasm` | ✅ |

### 已实现（原"明确延后"）

| 项目 | 实现位置 |
|------|---------|
| DAG 调度器（Coordinator + Stage/Pipeline/Cache） | `kaubo-driver/src/` (Phase 2b ✅) |
| 语义事实层（SemanticArtifact） | `kaubo-infer` + `kaubo-driver` (Phase 2b ✅) |
| 模块系统（ModuleGraph/ModuleCompiler/LinkStage） | `kaubo-driver/src/` (Phase 3b ✅) |

### 仍延后（架构已预留）

| 延后项 | 当前预留 | 后续实现位置 |
|--------|---------|-------------|
| WASM 按 `Severity` 映射 `console.error` / `console.warn` / `console.log` | `ConsoleHandler` `#[cfg(wasm32)]` 分支内 `match severity` 预留 | `kaubo-log-handlers/src/console.rs` |
| per-target 细粒度过滤（`vm=trace,cps=info`） | `EventHandler::filter(&self, event)` 接收完整事件可匹配 target；`ConsoleHandler` 字段可扩展 | `kaubo-log-handlers/src/console.rs` + CLI 解析 |

## 未来方向

- **per-stage 级别过滤**：当前 `ConsoleHandler` 只有一个全局 `min_level`，后续可加 `HashMap<&str, Severity>` 支持 `vm=trace,cps=info` 的细粒度
- **事件总线**：当前同步 `&dyn EventHandler` 回调，后续可替换为 channel-based 实现不改 Stage 代码（`emit!` 宏不变）
- **WASM 专用 handler**：`LocalStorageHandler`、`IndexedDBHandler` 等持久化输出（用户在 `kaubo-wasm` 层实现，不进 `kaubo-log` 库）
- **类型推导事件**：`kaubo-infer` 当前无事件，后续可加 `InferEvent` 用于调试类型错误
- **JSON 输出 handler**：结构化 JSON lines 输出（用户实现 `EventHandler` 即可，不进库）
