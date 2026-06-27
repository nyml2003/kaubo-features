# 架构

目标读者：维护编译器、运行时、Web app 或编辑器集成的开发者。

## 当前状态

Kaubo 是一个 monorepo。核心实现位于 Rust workspace，外层有两个主要 UI 适配层：Web Playground 和 VSCode 扩展。

当前执行链路是线性的，由 `kaubo-driver` 负责。历史上未接入主路径的 pipeline/module/vfs/log 实验 crate 已移除，避免 workspace 同时维护多套入口。

## 分层

期望的依赖方向是：

```text
source text
  -> token/syntax
  -> AST
  -> infer/semantic facts
  -> CPS/IR
  -> VM
  -> adapters
```

适配层应该调用稳定 API 和 DTO，不应该重复实现编译器逻辑。

## 核心 Crate

- `kaubo-log`：结构化事件类型、`EventHandler` trait、`emit!` 宏。**纯抽象层，零平台代码，零外部依赖。** 详见 [事件与日志系统](events-and-logging.md)。
- `kaubo-log-handlers`：具体 `EventHandler` 实现——`ConsoleHandler`、`CompositeHandler`、`KAUBO_LOG` 环境变量解析。详见 [事件与日志系统](events-and-logging.md)。
- `kaubo-token`：token kind 和 token 级数据。
- `kaubo-ast`：源码级语法树数据结构。
- `kaubo-syntax`：lexer 和 parser。
- `kaubo-infer`：类型推断和类型错误。
- `kaubo-cps`：CPS module/function/block/instruction 定义。
- `kaubo-ir`：AST 到 CPS lowering、flatten、二进制编码和 pass。
- `kaubo-vm`：寄存器 VM、堆对象、native 函数和执行逻辑。
- `kaubo-driver`：compile/run 编排层。依赖 `kaubo-log` 抽象，不依赖 `kaubo-log-handlers`。接收外部注入的 `EventHandler` 并透传到各 Stage。
- `kaubo-language-service`：编辑器侧 semantic tokens 和 completion。
- `kaubo-web-api`：与 WASM-facing 代码共享的 JSON/DTO 辅助逻辑。
- `kaubo-wasm`：wasm-bindgen 导出。

### 依赖关系图（含 kaubo-log 体系）

```
                              kaubo-log (trait + event types + emit! 宏)
                              /        \
              kaubo-ir ── kaubo-vm    kaubo-log-handlers (ConsoleHandler, CompositeHandler)
                   \         /              |
                    kaubo-driver ───────────┘
                   /           \
           kaubo2-cli         kaubo-wasm
```

- `kaubo-ir`、`kaubo-vm`：只依赖 `kaubo-log` 的类型和 trait。不碰输出。
- `kaubo-driver`：只依赖 `kaubo-log` 抽象。具体 handler 由调用方注入。
- `kaubo2-cli`：依赖 `kaubo-log-handlers`，构建 `ConsoleHandler` 等注入 Driver。
- `kaubo-wasm`：依赖 `kaubo-log-handlers`，构建 WASM 输出 handler 注入 Driver。

## Ops 工具

发布、部署、覆盖率和 benchmark 统一放在 `next_kaubo/ops/`：

- `ops/release/publish.py`：构建 Web app、打包并发布 GitHub Release。
- `ops/deploy/deploy.py`：从 GitHub Release 下载产物并部署到 nginx。
- `ops/quality/coverage.py`：运行 Rust workspace 覆盖率报告。
- `ops/benchmark/runner.py`：运行 Kaubo/Python/Rust benchmark。

## 适配层边界

Web 和 VSCode 不应该自己解析编译器内部状态。目标形态是：

```text
adapter -> kaubo-wasm -> language service / driver -> compiler/runtime
```

目前 Web app 已经通过 WASM 消费 `semantic_tokens` 和 `complete`。VSCode 当前消费 WASM diagnostics，但还没有暴露与 Web app 相同的 semantic token provider。

## 当前执行路径

`kaubo-driver::compile_source` 当前执行编译路径：

1. 使用 `kaubo_syntax::parser::Parser` 解析源码。
2. 运行 `kaubo_infer::infer_module`。
3. 使用 `kaubo_ir::cps_build::build_module` 构建 CPS。
4. 使用 `kaubo_ir::flatten::flatten_module` 展平 CPS。
5. 通过 `kaubo_ir::pass` 运行常量折叠。

`kaubo-driver::run_module` 随后把 CPS module 加载进 `kaubo-vm`，并把最后一个函数作为入口执行。

所有 Stage（CPS build、pass、VM execute）通过 `kaubo-log::EventHandler` trait 发射结构化事件；Driver 将外部注入的 handler 沿调用链原样透传。遵循 "Stage 不感知输出，只发事件" 原则。详见 [事件与日志系统](events-and-logging.md)。

## 当前迭代计划

本次迭代（日志与死循环检测）的目标和边界：

### 目标

| 目标                         | 说明                                                           |
| ---------------------------- | -------------------------------------------------------------- |
| `kaubo-log` crate          | trait + 事件类型 +`emit!` 宏                                 |
| `kaubo-log-handlers` crate | `ConsoleHandler` + `CompositeHandler` + `KAUBO_LOG` 解析 |
| VM 死循环检测                | `LoopExceeded` 错误 + backward jump 计数                     |
| Driver 透传 handler          | `RunConfig` 携带 `EventHandler`，注入到各 Stage            |
| CLI / WASM 接入              | 各自依赖`kaubo-log-handlers` 构建 handler                    |

### 对终态的兼容

本次所有改动在以下终态假设下仍然成立：

| 终态假设               | 对当前改动的约束                                                               | 满足方式                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Driver 变为 DAG 调度器 | `compile_source` / `run_module` 签名会变，但 `EventHandler` 透传模式不变 | `&dyn EventHandler` 沿调用链透传，未来升级为 `Arc<dyn EventHandler + Sync>` 不改 Stage 代码 |
| 新增语义事实层         | `InferRule` 生成 `Semantic` artifact                                       | 语义事实层不影响 VM 层的事件系统                                                                |
| 模块系统（传递哈希）   | 循环计数器在`load` 时重置                                                    | 每次`build(VMExec)` 新建 VM 并 load，计数器天然隔离                                           |
| Parallel build         | `emit!` 宏和 handler 都不持全局状态                                          | 未来线程安全只需`Arc<dyn EventHandler + Sync>`                                                |
| Composite 不变         | `CompositeHandler` 的广播语义（不重写 filter）                               | 已按此实现                                                                                      |

### 不在本次范围

- DAG 调度器
- 语义事实层
- 模块系统 / 传递闭包哈希
- per-target 细粒度日志过滤（架构预留）
- WASM 按级别映射不同 console API（架构预留）

## 终态架构

本节描述当前迭代之后的长期架构目标，作为设计参考。

### DAG 调度器

所有编译单元注册为独立规则（Rule），每个规则声明输入依赖和计算逻辑：

```rust
trait Rule {
    type Key: ArtifactKey;
    type Artifact;
    fn dependencies(&self, key: &Self::Key) -> Vec<Box<dyn ArtifactKey>>;
    fn build(&self, key: &Self::Key, ctx: &BuildContext) -> Result<Self::Artifact>;
}

struct BuildContext<'a> {
    events: Option<&'a dyn kaubo_log::EventHandler>,
    cache: &'a mut dyn Cache,
}
```

调用方通过 `driver.build(ArtifactKey)` 声明目标产物，调度器自动：

1. 递归求值依赖（拓扑排序）
2. 命中缓存（传递闭包哈希）
3. 并行执行无依赖规则

`compile_and_run(source)` 仅作为 `driver.build(RunResult::key(source))` 的便捷别名保留。

### ArtifactKey 体系

```rust
enum ArtifactKey {
    Parse(SourceHash),
    Infer(ParseHash),
    Semantic(SemanticHash),       // 语义事实：类型、符号表、作用域
    CpsModule(SemanticHash),
    OptimizedModule(CpsHash),
    VmExec(ModuleHash),
    RunResult(ModuleHash),
}
```

### 语义事实层

由 `InferRule` 生成，作为类型事实、符号表、作用域的唯一来源：

- LSP 语言服务只求值到 `ArtifactKey::Semantic`（不触发 Lowering 和 VM）
- CPS Lowering 规则显式声明依赖 `Semantic`
- 跨模块类型检查通过递归 `build(ImportedSemantic(key))` 获取依赖模块的事实

### 模块系统：传递闭包哈希

每个 `ArtifactKey` 包含内容哈希。模块 Key 的哈希计算包含：

- 源码内容哈希
- **所有传递依赖模块的 Key 哈希**

依赖模块内容变化 → 其 Key 变化 → 父 Key 变化 → 自动触发父模块重编译。Driver 的 `cache: HashMap<Key, Artifact>` 自动处理命中与失效，不引入版本号或时间戳。循环依赖在规则注册或构建时由 Driver 检测并报错（沿用 Rust 模块系统禁止循环依赖的限制）。

### 日志系统在 DAG 下的传递契约

- 同步 `&dyn EventHandler` 沿调用链透传，无全局状态
- 所有规则通过 `BuildContext` 获取 `Option<&dyn EventHandler>`
- 递归求值依赖时原样透传
- 未来并行场景下，由调用方构造 `Arc<dyn EventHandler + Sync>` 传入，Driver 核心逻辑不改动，`emit!` 宏也不变

### 从当前到终态的演进路径

```
Phase 1 (本次): 日志 + 死循环检测 + handler crate
  ├── kaubo-log (trait + 事件类型 + emit! 宏)
  ├── kaubo-log-handlers (ConsoleHandler + CompositeHandler)
  ├── VM LoopExceeded 错误
  └── Driver RunConfig + handler 透传

Phase 2: DAG 调度器 + 语义事实
  ├── Driver 重构为 DAG 调度器
  ├── Rule trait + BuildContext
  ├── 语义事实层 (InferRule → Semantic artifact)
  ├── LSP 切到 Semantic artifact
  └── cache: HashMap<Key, Artifact>

Phase 3: 模块系统
  ├── 传递闭包哈希
  ├── 跨模块类型检查
  ├── 循环依赖检测
  └── Parallel build (Arc<dyn EventHandler + Sync>)
```

## 日志系统与语言特性的正交性

日志/事件系统是横向基础设施，语言特性是纵向演进。两者正交：

```
日志/事件系统（横向基础设施）         语言特性演进（纵向语义）

emit!(events, Vm, Instruction {..})   Interface → CallIndirect opcode
emit!(events, Pass, Started {..})     Generics → Monomorphization pass
emit!(events, Cps, WhileLowered {..}) Modules → ModuleGraph
emit!(events, Vm, LoopIteration {..}) Effects → Suspend handler table
```

**日志不关心执行什么语义——只关心"有个事件发生了"。** 新特性会新增事件变体（加几个 enum variant），但 `EventHandler` trait、`emit!` 宏、透传模式全都不变。

唯一需要预留的是 Driver 的编排职责：`RunConfig`（Phase 1）→ `BuildContext`（DAG 阶段）。`EventHandler` 从 `Option<Box<dyn EventHandler>>` 变为 `Option<&dyn EventHandler>`（透传模式一致），未来并行场景升级为 `Arc<dyn EventHandler + Sync>`。

## 全局路线图

### 特性全景与依赖关系

```
Phase 1: 可观测性 + 死循环防护 ─────────────────────────────
  │  解锁了调试和 profiling 能力                              │
  │                                                          │
  ├── Phase 2a: VM 运行时性能 ────────────────────────────── │
  │    profile 热点 → 优化指令分派 / 寄存器 / GC              │
  │    不依赖编排层改动，可独立推进                            │
  │                                                          │
  ├── Phase 2b: 编排解耦 + 语义事实 ──────────────────────── │
  │    DAG 调度器 + ArtifactKey + Semantic                    │
  │    解锁 LSP 基础 + 模块系统的架构前提                      │
  │    │                                                     │
  │    ├── Phase 3a: LSP 完善 ─────────────────────────────── │
  │    │    go-to-def, hover, find-refs, completion           │
  │    │    消费 Semantic artifact，不改编译器核心             │
  │    │                                                     │
  │    ├── Phase 3b: 模块系统 ─────────────────────────────── │
  │    │    import/export, 传递闭包哈希, 循环依赖检测          │
  │    │    依赖 DAG 框架 + 跨模块名称解析                     │
  │    │    │                                                │
  │    │    ├── Phase 4a: Interface ───────────────────────── │
  │    │    │    动态分派, vtable, CallIndirect                │
  │    │    │    ~500 行核心改动                               │
  │    │    │    │                                            │
  │    │    │    └── Phase 4b: 内置模块化 ──────────────────── │
  │    │    │         prelude.kb, 编译器去硬编码                │
  │    │    │         ~600 行, 依赖 Interface                   │
  │    │    │                                                 │
  │    │    ├── Phase 5a: 显式泛型 ─────────────────────────── │
  │    │    │    Monomorphization, 函数体复制+类型替换          │
  │    │    │    ~1200 行, 可与 4a 并行                         │
  │    │    │                                                 │
  │    │    └── Phase 5b: 效应系统 ─────────────────────────── │
  │    │         行多态, Suspend 语义化, handler 表             │
  │    │         ~2000 行, 可与 5a 并行                         │
  │    │                                                      │
  │    └── (更多语法糖: 区间、列表推导、match 解构...)           │
  │                                                          │
  └── (日志系统贯穿全程，所有 Phase 受益)                       │
```

### 各 Phase 详解

#### Phase 1：可观测性 + 死循环防护（本次）

| 痛点   | #5 没有日志 + while 死循环                                                       |
| ------ | -------------------------------------------------------------------------------- |
| 前置   | 无                                                                               |
| 交付   | `kaubo-log`、`kaubo-log-handlers`、VM 死循环检测、Driver 透传、CLI/WASM 接入 |
| 改什么 | 不碰 Driver 架构、不碰 VM 执行逻辑核心、不碰 parser                              |
| 解锁   | 所有后续 Phase 的调试和 profiling                                                |

#### Phase 2a：VM 运行时性能

| 痛点 | #1 性能太差——运行时执行慢                                      |
| ---- | ---------------------------------------------------------------- |
| 前置 | Phase 1（日志用于 profiling）                                    |
| 不改 | Driver、parser、type inference、CPS lowering                     |
| 方向 | profile 热点 → 优化指令分派 / 寄存器文件访问 / GC / native call |

#### Phase 2b：编排解耦 + 语义事实

| 痛点 | #3 架构死板 + #4 LSP 基础                                                                              |
| ---- | ------------------------------------------------------------------------------------------------------ |
| 前置 | Phase 1                                                                                                |
| 交付 | DAG 调度器（`Rule` trait + `BuildContext` + `ArtifactKey`）、语义事实层（`Semantic` artifact） |
| 不改 | VM、parser、token/ast/syntax                                                                           |
| 解锁 | LSP 在`Semantic` 节点停下（不再跑完整编译）、模块系统架构前提                                        |

#### Phase 3a：LSP 完善

| 痛点 | #4 语言服务器                                                       |
| ---- | ------------------------------------------------------------------- |
| 前置 | Phase 2b（语义事实层）                                              |
| 交付 | go-to-definition, hover type info, find references, completion 增强 |
| 不改 | 编译器核心                                                          |
| 模式 | 只消费`Semantic` artifact，不触发 Lowering/VM                     |

#### Phase 3b：模块系统

| 痛点 | #2 特性不够——多文件                                                     |
| ---- | ------------------------------------------------------------------------- |
| 前置 | Phase 2b（DAG 调度器作为模块图构建框架）                                  |
| 交付 | `import`/`export` 语义、模块图构建、传递闭包哈希缓存、跨模块名称解析  |
| 改动 | AST import/export 语义化、Driver 模块图、Infer 跨模块查询、CPS 多模块链接 |
| 规模 | ~1500 行                                                                  |

#### Phase 4a：Interface

| 痛点 | #2 特性不够——动态分派                                                           |
| ---- | --------------------------------------------------------------------------------- |
| 前置 | Phase 3b（模块系统，Interface 需要跨模块 impl 可见性）                            |
| 交付 | `interface`/`impl`、胖指针 `(vtable, data)`、`CallIndirect` opcode        |
| 改动 | AST`InterfaceDef`、Infer 接口匹配、CPS `LoadVtable` 指令、VM `CallIndirect` |
| 规模 | ~500 行核心                                                                       |

#### Phase 4b：内置模块化

| 痛点 | #2 特性不够——编译器硬编码过多                                          |
| ---- | ------------------------------------------------------------------------ |
| 前置 | Phase 4a（Interface）+ Phase 3b（模块系统）                              |
| 交付 | `@builtins` 原子操作层 + `interface Add/Display/Eq` + `prelude.kb` |
| 规模 | ~600 行                                                                  |

#### Phase 5a：显式泛型

| 痛点 | #2 特性不够——类型参数                                               |
| ---- | --------------------------------------------------------------------- |
| 前置 | 无硬依赖（可与 Phase 4 并行推进）                                     |
| 交付 | `struct Container<T>`、泛型函数、CPS Monomorphization               |
| 改动 | AST 泛型参数、Type 参数化、Infer 绑定+实例化、CPS 函数体复制+类型替换 |
| 规模 | ~1200 行                                                              |

#### Phase 5b：效应系统

| 痛点 | #2 特性不够——副作用追踪                                                                                               |
| ---- | ----------------------------------------------------------------------------------------------------------------------- |
| 前置 | 无硬依赖（可与 Phase 5a 并行推进）                                                                                      |
| 交付 | `effect io`、`handle ... with`、Suspend 语义化                                                                      |
| 改动 | AST`EffectDecl`/`Do`/`Handle`、Type `EffectRow`、Infer 效应传播、CPS Suspend + handler 表、VM 调度 continuation |
| 规模 | ~2000 行                                                                                                                |

### 并行度

```
Phase 1 ────────────────（当前）
  │
  ├── Phase 2a ────────（VM 性能，独立）
  │
  └── Phase 2b ────────（DAG + Semantic）
        │
        ├── Phase 3a ──（LSP，独立）
        │
        └── Phase 3b ──（模块系统）
              │
              ├── Phase 4a ── Phase 4b ──（Interface → 内置模块化，串行）
              │
              ├── Phase 5a ──（泛型，可与 4a/4b 并行）
              │
              └── Phase 5b ──（效应系统，可与 5a 并行）
```

2a 和 2b 可并行；3a 不依赖 3b；4a/4b 与 5a/5b 可并行。

### Phase 1 对终态的兼容性总结

| 终态架构   | Phase 1 动作                           | 兼容性                             |
| ---------- | -------------------------------------- | ---------------------------------- |
| DAG 调度器 | `RunConfig` → `EventHandler` 透传 | ✅ 升级为`BuildContext` 的字段   |
| 语义事实层 | 不涉及                                 | ✅ 语义层是新增节点，不影响日志    |
| 模块系统   | 循环计数器在`load` 重置              | ✅ 每次`build(VMExec)` 新建 VM   |
| Interface  | 不改 VM 核心执行循环                   | ✅ 只加 opcode，不改 Branch/Jump   |
| 泛型       | 不改 CPS 构建逻辑                      | ✅ Monomorphization 是新增 Pass    |
| 效应系统   | 不改 Suspend 语义                      | ✅ 效应是 Suspend 的上层封装       |
| 并行 build | `emit!` 无全局状态                   | ✅`Arc<dyn EventHandler + Sync>` |
