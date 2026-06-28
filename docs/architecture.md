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

`kaubo-driver::compile_source` 执行路径（Phase 2b 后）：

1. `FrontendStage`：Parser 解析源码 → `Module`
2. `SemanticStage`：`infer_module` 类型检查 + 符号收集 → `SemanticArtifact`
3. `CpsBuildStage`：`build_module` 构建 CPS IR
4. `flatten_module` + `PassPipeline` 优化
5. `VmExecStage`：VM 加载并执行

Coordinator 负责接线 + 缓存 + 事件扇出。LSP 查询（hover/go-to-def）通过 Coordinator 的 `semantic_at()` 方法只跑到 Semantic，不触发 CPS/VM。

所有 Stage 通过 `kaubo-log::EventHandler` 发射结构化事件。Coordinator 持有一个 `EventRouter` 将事件扇出到多个 `EventSink`（终端/文件/Web）。**Stage 不感知路由**。详见 [事件与日志系统](events-and-logging.md) 和 [DAG 设计文档](dag-design.md)。

## 当前实现状态

| Phase | 状态 | 关键交付 |
|-------|------|---------|
| Phase 1 | ✅ | 日志 + 死循环防护 |
| Phase 4a | ✅ | Interface + operator 重载 + dyn Trait + 模板字符串 |
| Phase 4b | 🔶 | 虚拟 prelude（9 接口 + 40+ 方法），prelude.kb 待做 |
| Phase 2b | ✅ | DAG 编排层 + Semantic 语义事实层 + EventSink/EventRouter |
| Phase 2a | ⏸ | VM 性能（推迟，~1.5x CPython 可接受） |
| Phase 3a | ▶ 下一步 | LSP 编排层独立化（LspCoordinator 基于 SemanticArtifact）

| 终态假设               | 对当前改动的约束                                                               | 满足方式                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Driver 变为 DAG 调度器 | `compile_source` / `run_module` 签名会变，但 `EventHandler` 透传模式不变 | `&dyn EventHandler` 沿调用链透传，未来升级为 `Arc<dyn EventHandler + Sync>` 不改 Stage 代码 |
| 新增语义事实层         | `InferRule` 生成 `Semantic` artifact                                       | 语义事实层不影响 VM 层的事件系统                                                                |
| 模块系统（传递哈希）   | 循环计数器在`load` 时重置                                                    | 每次`build(VMExec)` 新建 VM 并 load，计数器天然隔离                                           |
| Parallel build         | `emit!` 宏和 handler 都不持全局状态                                          | 未来线程安全只需`Arc<dyn EventHandler + Sync>`                                                |
| Composite 不变         | `CompositeHandler` 的广播语义（不重写 filter）                               | 已按此实现                                                                                      |

## 当前架构（Phase 2b 后）

### 编排层 + 协议层

```
kaubo-driver
├── protocol.rs     Stage<I,O>, Pass, Pipeline, Cache
├── event.rs        EventSink, EventRouter, blanket impl
├── coordinator.rs  编译器 Coordinator (Frontend→Semantic→CPS→VM)
└── stages.rs       各 Stage 具体实现
```

**协议层**定义契约，**编排层**做具体接线。Coordinator 知道各 Stage 的具体签名，手动组合。缓存只在自然断点（Semantic、CPS）。详见 [DAG 设计文档](dag-design.md)。

### 语义事实层

`SemanticArtifact` 是 Infer 的完整输出，包含 type_env、struct_fields、symbols、references。LSP 查询通过 `Coordinator::semantic_at()` 只求值到 Semantic，不触发 Lowering/VM。

### 事件系统

Phase 1 的 `EventHandler` 通过 blanket impl 自动升级为 `EventSink`。`EventRouter` 扇出到多个 Sink（终端、文件、Web）。Stage 只持有 `&dyn EventHandler`，不感知路由。

### LSP 编排层（Phase 3a 计划）

语言服务器有**自己的 Coordinator**（`LspCoordinator`），和编译器 Coordinator 共享协议层，但独立接线：

```
kaubo-driver (协议层)               kaubo-language-service (LSP 编排层)
├── protocol.rs ← 共享 ─────────→  ├── lsp_coordinator.rs
├── event.rs    ← 共享 ─────────→  │   LspCoordinator (Frontend→Semantic)
├── coordinator.rs                 │   不到 CPS/VM
└── stages.rs                      ├── hover / goto_def / completion
```

详见 [DAG 设计文档 - Phase 3a 节](dag-design.md#10-phase-3alsp-编排层独立化)。

## 路线图

详见 [roadmap.md](roadmap.md)。
