# Phase 2b：编排解耦 + 语义事实层 — 设计文档

> **实现状态**：✅ 已完成。本设计文档保留为架构参考。最终实现约 320 行，与设计一致。
> 代码位于 `kaubo-driver/src/`（protocol.rs, event.rs, coordinator.rs, stages.rs）。

## 1. 分层架构

```
┌──────────────────────────────────────────────────────────┐
│  编排层 (Coordinator)                                     │
│  接线 + 扇出——Stage 串成管线，事件路由到 Sink                │
│                                                          │
│  Source ─→ Frontend ─→ Semantic ─→ CPS ─→ Passes ─→ VM   │
│                │          │         │        │            │
│                ▼          ▼         ▼        ▼            │
│           ┌──────────────────────────────────────┐        │
│           │          Event Bus (扇出)             │        │
│           │  Terminal · File · Web · Remote · …  │        │
│           └──────────────────────────────────────┘        │
│                                                          │
│  LSP: Coordinator.semantic_at(&module) ←── 只到 Semantic  │
├──────────────────────────────────────────────────────────┤
│  协议层 (Protocol)                                        │
│                                                          │
│  Stage<I, O>  ·  Pass  ·  Pipeline  ·  EventSink         │
│  Cache  ·  EventRouter                                   │
└──────────────────────────────────────────────────────────┘
```

**协议层**定义三段契约：
- **计算协议**：`Stage<I, O>`、`Pass`、`Pipeline`
- **存储协议**：`Cache`
- **事件协议**：`EventSink`、`EventRouter`

**编排层**做两件事：
- 把 Stage 串成编译管线（接线）
- 把事件路由到 Sink（扇出）

Stage 只管 `emit!()`，完全不感知事件最终去了终端、文件还是 Web。

## 2. 协议层

### 2.1 Stage<I, O> — 一段可缓存的计算

```rust
/// 一段编译计算：输入 I，输出 O。
/// 不规定 I 和 O 的结构——每个 Stage 自由定义自己的输入输出类型。
trait Stage<I, O> {
    /// 标识（用于日志和缓存键前缀）
    fn name(&self) -> &str;

    /// 执行计算
    fn execute(&self, input: I, ctx: &BuildContext) -> Result<O, BuildError>;
}
```

**特点**：
- `I` 和 `O` 是泛型参数，不强约束——每个 Stage 有自己的具体类型
- `execute` 接收具体类型的 input，不经过 `Box<dyn Any>` 拆装箱
- 不要求 `I` 实现任何特殊 trait——Stage 自己知道需要什么

**示例**：

```rust
struct FrontendStage;
impl Stage<&str, Module> for FrontendStage {
    fn name(&self) -> &str { "frontend" }
    fn execute(&self, source: &str, _ctx: &BuildContext) -> Result<Module> {
        Parser::new(source).parse()
    }
}

struct SemanticStage;
impl Stage<Module, SemanticArtifact> for SemanticStage {
    fn name(&self) -> &str { "semantic" }
    fn execute(&self, module: Module, _ctx: &BuildContext) -> Result<SemanticArtifact> {
        let (type_env, struct_fields) = infer_module(&module)?;
        let symbols = collect_symbols(&module, &type_env, &struct_fields);
        let references = collect_references(&module);
        Ok(SemanticArtifact { type_env, struct_fields, symbols, references })
    }
}
```

### 2.2 Pass — CpsModule 的变换

```rust
/// Pass 操作在 CpsModule 上，是 Stage 的一个特化子协议
trait Pass {
    fn name(&self) -> &str;

    /// 原地修改 CpsModule
    fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>);
}
```

Pass 不同于 Stage——它总是 `CpsModule → CpsModule`，输入输出类型一致，可以链式组合。

### 2.3 QueryPass（预留）— 只读不写，可并行

```rust
/// QueryPass：只读查询，不修改 CpsModule。
/// 未来可与 Pass 并行执行（Pass 串行，同一批 QueryPass 并行）。
/// 本期不实现，仅预留 trait 定义。
trait QueryPass {
    fn name(&self) -> &str;
    fn query(&self, module: &CpsModule) -> Result<QueryResult>;
}
```

### 2.4 Pipeline — Pass 的有序组合

```rust
struct Pipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl Pipeline {
    fn new() -> Self { Self { passes: vec![] } }

    fn add(mut self, pass: impl Pass + 'static) -> Self {
        self.passes.push(Box::new(pass));
        self
    }

    fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>) {
        for pass in &self.passes {
            pass.run(module, events);
        }
    }
}
```

### 2.4 Cache — 键值存储

```rust
trait Cache {
    /// 取缓存。T 必须与 put 时的类型一致，否则 panic（debug 断言给出明确消息）。
    fn get<T: Clone + 'static>(&self, key: &str) -> Option<T>;

    fn put<T: 'static + Send + Sync>(&mut self, key: String, value: T);
}

struct MemoryCache {
    store: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl MemoryCache {
    fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let any = self.store.get(key)?;
        Some(any.downcast_ref::<T>()
            .expect("cache: type mismatch — did a Stage store a different type under this key?")
            .clone())
    }
}
```

**为什么不用泛型约束消除 panic**：当前目标是快速落地，不做编译期类型安全的 Cache（如 `salsa` 风格）。但 `expect` 消息让类型混乱时 debug 有明确指向。

### 2.5 EventSink — 事件的输出目标

EventSink 是 EventHandler 的超集，增加生命周期和标识能力。通过 **blanket impl** 实现零成本兼容——所有现有的 EventHandler 自动成为 EventSink：

```rust
/// EventSink：事件的输出目标。
trait EventSink {
    /// 唯一标识（用于配置和日志）
    fn name(&self) -> &str;

    /// 处理事件（继承自 EventHandler 的语义）
    fn handle(&self, event: &ToolchainEvent);

    /// 可选：Sink 初始化（打开文件、建立连接等）
    fn open(&mut self) {}

    /// 可选：Sink 清理（关闭文件、断开连接等）
    fn close(&mut self) {}

    /// 可选：刷新缓冲区
    fn flush(&mut self) {}
}

/// RAII 守卫：保证 Coordinator 析构时所有 Sink 的 close() 被调用
struct SinkGuard<'a> {
    sinks: &'a mut Vec<Box<dyn EventSink>>,
}

impl Drop for SinkGuard<'_> {
    fn drop(&mut self) {
        for sink in self.sinks.iter_mut() {
            sink.flush();
            sink.close();
        }
    }
}

/// ★ 关键：所有现有 EventHandler 自动成为 EventSink，零迁移成本
impl<T: EventHandler> EventSink for T {
    fn name(&self) -> &str { "handler" }
    fn handle(&self, event: &ToolchainEvent) {
        if self.filter(event) {
            EventHandler::handle(self, event);
        }
    }
}
```

**兼容性保证**：
- 现有的 `ConsoleHandler`、`CompositeHandler` 不需要任何改动
- `emit!` 宏保持原样，新增 `emit_to!` 宏用于显式 EventSink 路由
- `RunConfig.events: Option<Box<dyn EventHandler>>` 保持兼容，内部自动包装为 EventSink

**内置 Sink**（Phase 2b 新增）：

| Sink | name | 输出位置 | 生命周期动作 |
|------|------|---------|-------------|
| `TerminalSink` | `"terminal"` | stderr / WASM console | open=noop, close=flush |
| `FileSink` | `"file:path"` | 文件写入 | open=创建文件, close=关闭 |
| `BufferSink` | `"buffer"` | 内存 `Vec<String>` | open/close=noop |
| `WebConsoleSink` | `"web"` | WASM `console.*` | open=noop |
| `NullSink` | `"null"` | /dev/null | open/close=noop |

### 2.6 EventRouter — 多路扇出

```rust
/// 将事件扇出到多个 Sink。可运行时增减 Sink。
struct EventRouter {
    sinks: Vec<Box<dyn EventSink>>,
}

impl EventRouter {
    fn add(&mut self, sink: Box<dyn EventSink>) {
        self.sinks.push(sink);
    }

    fn remove(&mut self, name: &str) {
        self.sinks.retain(|s| s.name() != name);
    }
}

impl EventHandler for EventRouter {
    fn handle(&self, event: &ToolchainEvent) {
        for sink in &self.sinks {
            sink.handle(event);
        }
    }
}
```

**Stage 不感知路由**——Stage 只接收 `&dyn EventHandler`，不知道也不关心它是一个 Sink 还是一百个 Sink 的扇出。

### 2.7 事件类型扩展

Phase 1 的 `ToolchainEvent` 已有 `Vm`、`Loop` 等变体。Phase 2b 新增"中间产物"类事件：

```rust
enum ToolchainEvent {
    // Phase 1 已有
    Vm(VmEvent),
    LoopIteration { /* ... */ },

    // Phase 2b 新增：Stage 生命周期
    StageStart { stage: &'static str },
    StageDone { stage: &'static str, duration_ms: u64 },

    // Phase 2b 新增：中间产物快照
    AstDump { json: String },        // Frontend 输出
    SemanticDump { json: String },   // Semantic 输出
    CpsDump { dot: String },         // CPS IR 图
    PassDump { pass: &'static str, before: bool, dot: String },

    // 诊断（类型错误等）
    Diagnostic { severity: Severity, message: String, span: Span },
}
```

每个 Stage 调用 `emit!(events, StageStart { stage: "semantic" })` 和 `emit!(events, StageDone { stage: "semantic", duration_ms: 12 })`。Sink 可以选择性消费——`TerminalSink` 可以只打印 `StageDone` 的耗时摘要，`FileSink` 可以完整记录所有 `Dump`。

## 3. 编排层（Coordinator）

```rust
struct Coordinator {
    cache: MemoryCache,
    pipeline: Pipeline,
    router: EventRouter,  // ★ 替代原来的单个 EventHandler
}

impl Coordinator {
    /// 添加一个事件输出目标
    fn add_sink(&mut self, sink: Box<dyn EventSink>) {
        self.router.add(sink);
    }

    /// 构建时注入配置
    fn with_sink(mut self, sink: Box<dyn EventSink>) -> Self {
        self.router.add(sink);
        self
    }

    // ── 公开 API ──

    /// 完整编译+执行
    fn run(&mut self, source: &str) -> Result<RunOutcome> {
        let cps = self.build_cps(source)?;
        self.execute(cps)
    }

    /// LSP：只构建到 Semantic。接收已解析的 Module 避免重复解析。
    fn semantic_at(&mut self, module: &Module) -> Result<SemanticArtifact> {
        let key = cache_key_module(module, "semantic");
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached);
        }
        let semantic = SemanticStage.execute(module.clone(), &self.build_ctx())?;
        self.cache.put(key, semantic.clone());
        Ok(semantic)
    }

    // ── 内部接线（编排层特有的胶水逻辑） ──

    fn frontend(&self, source: &str) -> Result<Module> {
        // Frontend 足够快，不缓存
        FrontendStage.execute(source, &self.build_ctx())
    }

    // ── 缓存键：命名空间 + 阶段 + 内容哈希 ──
    //
    // `namespace` 隔离不同模块的缓存空间。单模块下固定为 `"local"`，
    // Phase 3b 自然替换为 `ModuleId` 的字符串表示，Cache 存储层一行不动。

    fn cache_key(&self, namespace: &str, content_hash: &str, stage: &str) -> String {
        format!("{}/{}/{}", namespace, stage, content_hash)
    }

    fn cache_key_source(&self, source: &str, stage: &str) -> String {
        let hash = sha256(source.as_bytes());
        self.cache_key("local", &hex::encode(hash), stage)
    }

    fn cache_key_module(&self, module: &Module, stage: &str) -> String {
        let hash = module.source_hash();
        self.cache_key("local", &hex::encode(hash), stage)
    }

    fn build_cps(&mut self, source: &str) -> Result<CpsModule> {
        let key = cache_key_source(source, "cps");
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached);
        }

        let module = self.frontend(source)?;
        let semantic = self.semantic_at(&module)?;  // 复用同一个 Module，不重复解析
        let mut cps = CpsBuildStage.execute(&semantic, &self.build_ctx())?;

        // Pass pipeline（Flatten 在 pipeline 外，必须最先执行）
        flatten_module(&mut cps);
        self.pipeline.run(&mut cps, self.events.as_deref());

        self.cache.put(key, cps.clone());
        Ok(cps)
    }

    fn execute(&mut self, cps: CpsModule) -> Result<RunOutcome> {
        let mut vm = VM::new();
        vm.load(&cps)?;
        let result = vm.execute(/* ... */)?;
        Ok(RunOutcome { result, output: vm.output })
    }

    fn build_ctx(&self) -> BuildContext {
        BuildContext {
            events: Some(&self.router as &dyn EventHandler),
        }
    }
}
```

**使用示例**：

```rust
// CLI：终端 + 文件双写
let coordinator = Coordinator::new()
    .with_sink(Box::new(TerminalSink::stderr()))
    .with_sink(Box::new(FileSink::new("build.log")))
    .with_pipeline(Pipeline::new()
        .add(EmptyBlockElim)
        .add(ConstantFold));

coordinator.run(source)?;

// WASM Playground：写到浏览器 console
let coordinator = Coordinator::new()
    .with_sink(Box::new(WebConsoleSink::new()))
    .with_pipeline(Pipeline::new());  // 空管线，快速响应

// 调试：AST dump 到文件，其他到终端
let coordinator = Coordinator::new()
    .with_sink(Box::new(TerminalSink::stderr()))
    .with_sink(Box::new(FileSink::new("ast_dump.json")
        .filter(|e| matches!(e, ToolchainEvent::AstDump { .. }))));
```
```

**编排层做的事**（协议层不做）：
- 决定 Frontend 不缓存（够快），Semantic 和 CPS 缓存
- 知道 `semantic_at` 可以被 `build_cps` 复用（调用链共享缓存）
- 知道 Flatten 必须在 Pipeline 之前执行
- 提供 LSP 专用的 `semantic_at` 入口

## 4. Semantic 语义事实层

```rust
/// 模块内唯一的符号 ID。按声明顺序递增。
/// 全局唯一标识是 (ModuleId, SymbolId) 对。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SymbolId(u32);

/// 全局唯一的符号引用键。模块 A 的 SymbolId(42) 和模块 B 的 SymbolId(42) 是不同的符号。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SymbolKey {
    module: ModuleId,
    symbol: SymbolId,
}

struct SemanticArtifact {
    /// 本模块的 ID（由 Coordinator 分配或从缓存键派生）
    module_id: ModuleId,

    type_env: TypeEnv,
    struct_fields: HashMap<usize, Vec<(String, Type)>>,

    /// ★ 新增——LSP 数据源
    /// 符号表：本模块内的 SymbolId → 定义
    symbols: HashMap<SymbolId, SymbolDef>,
    /// 仅用于 LSP 展示和名称查找
    symbol_names: HashMap<SymbolId, String>,
    /// 标识符引用位置 → 它的定义（跨模块时为 (other_module, other_symbol)）
    references: HashMap<Span, SymbolKey>,
}

struct SymbolDef {
    kind: SymbolKind,
    ty: Type,
    span: Span,
}

enum SymbolKind { Const, Var, Function, Struct, Interface }
```

**为什么用 (ModuleId, SymbolId) 而不是 String**：`symbol_names` 存展示名，`symbols` 以模块内整数索引。模块 A 导出 `add` 是 `(ModuleA, SymbolId(0))`，模块 B 局部变量 `add` 是 `(ModuleB, SymbolId(3))`——键对永不冲突，引用直接指向定义所在模块。

`SemanticStage` 包装现有 `infer_module`，额外收集符号表和引用图。不修改 `infer_module` 的核心逻辑。

## 5. Pass 管线使用

```rust
// CLI：全量优化
let coordinator = Coordinator::new()
    .with_pipeline(Pipeline::new()
        .add(EmptyBlockElim)
        .add(MoveFold)
        .add(ConstantFold));

// LSP：跳过优化，只要错误检查
let lsp_coordinator = Coordinator::new()
    .with_pipeline(Pipeline::new());  // 空管线

// 调试：注入日志
let debug_coordinator = Coordinator::new()
    .with_pipeline(Pipeline::new()
        .add(DumpCps::new("before"))
        .add(EmptyBlockElim)
        .add(DumpCps::new("after")));
```

## 6. LSP 集成

```rust
impl Coordinator {
    fn hover(&mut self, source: &str, offset: usize) -> Result<HoverInfo> {
        let module = self.frontend(source)?;
        let sem = self.semantic_at(&module)?;
        // 从 sem.references 查光标处的符号
        // 从 sem.symbols 返回类型信息
    }

    fn goto_def(&mut self, source: &str, offset: usize) -> Option<Span> {
        let module = self.frontend(source).ok()?;
        let sem = self.semantic_at(&module).ok()?;
        // 查 references → 找到定义的 span
    }
}
```

## 7. 与现有代码的关系

| 模块 | 变化 |
|------|------|
| `kaubo-driver/src/protocol.rs` (新) | Stage, Pass, Pipeline, Cache trait |
| `kaubo-driver/src/event.rs` (新) | EventSink, EventRouter, 内置 Sink |
| `kaubo-driver/src/coordinator.rs` (新) | Coordinator 编排 + 事件路由 |
| `kaubo-driver/src/stages/` (新) | FrontendStage, SemanticStage, CpsBuildStage |
| `kaubo-driver/src/lib.rs` | 保留 compile_source/run_source 便捷别名 |
| `kaubo-infer/src/infer.rs` | collect_symbols/collect_references |
| `kaubo-log/src/lib.rs` | ToolchainEvent 新增 StageStart/StageDone/Dump 变体 |
| `kaubo-language-service` | 改用 Coordinator::semantic_at |
| VM / Parser / CPS | **不改** |

## 8. 与 Phase 1 日志系统的关系

**零冲突，渐进升级**。Phase 1 的三个核心组件全部保留并自然演进：

| Phase 1 组件 | Phase 2b 变化 |
|-------------|--------------|
| `EventHandler` trait | 保留不变；通过 blanket impl 自动获得 EventSink 能力 |
| `emit!` 宏 | 保留不变；新增 `emit_to!` 宏用于显式路由 |
| `CompositeHandler` | 保留不变；EventRouter 内部用它做扇出引擎 |
| `ToolchainEvent` | 加 `#[non_exhaustive]`，新增 StageStart/StageDone/Dump 变体 |
| `ConsoleHandler` | 保留不变；自动成为 EventSink |
| `RunConfig.events` | 保留兼容；内部自动包装进 EventRouter |

**迁移路径**：

```
Phase 1（当前）:
  RunConfig.events: Option<Box<dyn EventHandler>>
  → 透传给 build_module / run_passes / VM.execute
  → emit! 宏直接调用 handler.handle()

Phase 2b:
  RunConfig.events 内部包装进 EventRouter
  → EventRouter 默认包含原来的 EventHandler 作为 Sink
  → Coordinator.add_sink() 可追加更多 Sink
  → emit! 行为不变（Stage 代码零改动）
  → emit_to! 可选使用（需要精细路由时）

Phase 3b:
  Cache 命名空间从 "local" 改为 ModuleId
  → cache_key 格式不变，只有 namespace 字段的值变化
  → Cache 存储层零改动
```

**不改的**：所有 6 处现有 `emit!` 调用点（cps_build 1 处、pass 2 处、VM 3 处）——代码一行不动。

## 9. 改动规模

| 组件 | 预估行数 |
|------|---------|
| `protocol.rs` — Stage, Pass, Pipeline, Cache | ~50 |
| `event.rs` — EventSink, EventRouter, 内置 Sink | ~80 |
| `coordinator.rs` — 编排逻辑 + 事件路由配置 | ~70 |
| `stages/*.rs` — 各 Stage 实现 | ~50 |
| `kaubo-log` — ToolchainEvent 扩展 | ~30 |
| `infer.rs` — 符号收集 | ~60 |
| LSP 适配 | ~30 |
| 测试 | ~50 |

**总计：已实现 ~320 行**（协议层 50 + 事件层 80 + Coordinator 70 + stages 50 + 适配 70）

## 10. Phase 3a：LSP 编排层独立化

### 设计原则

语言服务器**不应该依赖编译器 Coordinator**。两者共享同一套协议（Stage、Cache、EventSink），但 Coordinator 各自独立：

```
kaubo-driver (协议层)                    kaubo-language-service (LSP 编排层)
├── protocol.rs ← 共享 ──────────────→  ├── lsp_coordinator.rs
├── event.rs    ← 共享 ──────────────→  │   LspCoordinator (Frontend→Semantic)
├── coordinator.rs (编译器 Coordinator) │   不到 CPS/VM
└── stages.rs                           ├── hover.rs (基于 SemanticArtifact)
                                        ├── goto_def.rs
                                        ├── completion.rs
                                        └── diagnostics.rs
```

**编译器 Coordinator**：Source → Frontend → Semantic → CPS → Passes → VM

**LSP Coordinator**：Source → Frontend → Semantic（断点停下）

### LspCoordinator 设计

```rust
struct LspCoordinator {
    cache: MemoryCache,
    current: Option<(Module, SemanticArtifact)>,
}

impl LspCoordinator {
    fn on_change(&mut self, source: &str) -> Result<(), BuildError> {
        let module = FrontendStage.execute(source, &self.build_ctx())?;
        let semantic = SemanticStage.execute(module.clone(), &self.build_ctx())?;
        self.current = Some((module, semantic));
        Ok(())
    }

    fn hover(&self, offset: usize) -> Option<HoverInfo> { /* ... */ }
    fn goto_def(&self, offset: usize) -> Option<Span> { /* ... */ }
    fn complete(&self, offset: usize) -> Vec<CompletionItem> { /* ... */ }
    fn semantic_tokens(&self, source: &str) -> Vec<SemanticToken> { /* ... */ }
}
```

### 与当前 token-based LSP 的关系

当前 `kaubo-language-service` 是纯 token 扫描。Phase 3a 改为：
- **主路径**：基于 `SemanticArtifact`（AST + 类型信息）提供 hover/goto-def/completion
- **Fallback**：原有的 token 扫描保留，用于 `SemanticArtifact` 不可用时的降级

### 改动规模

| 组件 | 预估行数 |
|------|---------|
| `lsp_coordinator.rs` (新) | ~60 |
| `hover.rs` (新) | ~30 |
| `goto_def.rs` (新) | ~30 |
| `completion.rs` (重写) | ~40 |
| `semantic_tokens.rs` (重写) | ~40 |
| WASM 适配 | ~20 |
| 测试 | ~40 |

**总计：~260 行**
