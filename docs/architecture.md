# 架构

目标读者：维护编译器、运行时、Web app 或编辑器集成的开发者。

## 当前状态

Kaubo 是一个 monorepo。核心实现位于 Rust workspace，外层有两个主要 UI 适配层：Web Playground 和 VSCode 扩展。

当前执行链路基于 DAG 编排层（Phase 2b），由 `kaubo-driver` 的 Coordinator 负责。模块系统（Phase 3b）已完整实现，支持跨文件 import/export 和 CPS 链接。

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
- `kaubo-vm`：寄存器 VM（统一寄存器组 `Vec<u64>`）、堆对象、native 函数和执行逻辑。
- `kaubo-driver`：compile/run 编排层。包含 Coordinator、模块系统（ModuleGraph/ModuleCompiler/LinkStage）、协议层。依赖 `kaubo-log` 抽象，不依赖 `kaubo-log-handlers`。接收外部注入的 `EventHandler` 并透传到各 Stage。
- `kaubo-language-service`：编辑器侧 semantic tokens 和 completion（当前为 token-based heuristic，Phase 3a 将升级为基于 SemanticArtifact）。
- `kaubo-web-api`：与 WASM-facing 代码共享的 JSON/DTO 辅助逻辑。
- `kaubo-wasm`：wasm-bindgen 导出。
- `kaubo-vfs`：虚拟文件系统抽象（`VirtualFileSystem` trait + `FsVfs` + `MemVfs`）。

### 依赖关系图

```
                              kaubo-log (trait + event types + emit! 宏)
                              /        \
              kaubo-ir ── kaubo-vm    kaubo-log-handlers (ConsoleHandler, CompositeHandler)
                   \         /              |
                    kaubo-driver ───────────┘
                   /    |      \
     kaubo-vfs ──┘     |       kaubo-wasm
                   kaubo2-cli
```

- `kaubo-ir`、`kaubo-vm`：只依赖 `kaubo-log` 的类型和 trait。不碰输出。
- `kaubo-driver`：只依赖 `kaubo-log` 抽象。具体 handler 由调用方注入。
- `kaubo2-cli`：依赖 `kaubo-log-handlers`，构建 `ConsoleHandler` 等注入 Driver。
- `kaubo-wasm`：依赖 `kaubo-log-handlers`，构建 WASM 输出 handler 注入 Driver。

## 适配层边界

Web 和 VSCode 不应该自己解析编译器内部状态。目标形态是：

```text
adapter -> kaubo-wasm -> language service / driver -> compiler/runtime
```

目前 Web app 已经通过 WASM 消费 `semantic_tokens` 和 `complete`。VSCode 当前消费 WASM diagnostics，但还没有暴露与 Web app 相同的 semantic token provider。

## 当前执行路径

`kaubo-driver` 通过 Coordinator 编排执行：

1. `FrontendStage`：Parser 解析源码 → `Module`
2. `SemanticStage`：`infer_module` 类型检查 + 符号收集 → `SemanticArtifact`
3. `CpsBuildStage`：`build_module` 构建 CPS IR
4. `flatten_module` + `PassPipeline` 优化
5. `VmExecStage`：VM 加载并执行

**多文件路径**（模块系统）：
1. `ModuleGraph::build`：DFS + 拓扑排序 → 模块依赖图
2. `ModuleCompiler::compile_all`：按拓扑序编译每个模块
3. `LinkStage::link`：多模块 CPS 链接（函数表合并、CallExternal 重映射）

Coordinator 负责接线 + 缓存 + 事件扇出。

所有 Stage 通过 `kaubo-log::EventHandler` 发射结构化事件。Coordinator 持有一个 `EventRouter` 将事件扇出到多个 `EventSink`（终端/文件/Web）。**Stage 不感知路由**。详见 [事件与日志系统](events-and-logging.md) 和 [DAG 设计文档](dag-design.md)。

## LSP 编排层（Phase 3a 计划）

语言服务器将有**自己的 Coordinator**（`LspCoordinator`），和编译器 Coordinator 共享协议层，但独立接线：

```
kaubo-driver (协议层)               kaubo-language-service (LSP 编排层)
├── protocol.rs ← 共享 ──────────→  ├── lsp_coordinator.rs
├── event.rs    ← 共享 ──────────→  │   LspCoordinator (Frontend→Semantic)
├── coordinator.rs                  │   不到 CPS/VM
├── module_graph.rs                 ├── hover / goto_def / completion
├── module_compiler.rs              └── diagnostics
├── module_loader.rs
├── link_stage.rs
├── export_table.rs
└── stages.rs
```

**当前状态**：`kaubo-language-service` 仍为 token-based heuristic。Phase 2b 已交付 `SemanticArtifact`（symbols、type_env、references），Phase 3a 将基于它实现 go-to-def、hover、completion 增强。详见 [DAG 设计文档 - Phase 3a 节](dag-design.md#10-phase-3alsp-编排层独立化)。

## 当前实现状态

| Phase | 状态 | 关键交付 |
|-------|------|---------|
| Phase 1 | ✅ | 日志 + 死循环防护 |
| Phase 4a | ✅ | Interface + operator 重载 + dyn Trait + 模板字符串 |
| Phase 4b | 🔶 | 虚拟 prelude（9 接口 + 40+ 方法），prelude.kb 待做 |
| Phase 2b | ✅ | DAG 编排层 + Semantic 语义事实层 + EventSink/EventRouter |
| Phase 3b | ✅ | 模块系统：import/export、ModuleGraph、ModuleCompiler、LinkStage、kaubo-vfs |
| Phase 2a | ⏸ | VM 性能（推迟，~1.5x CPython 可接受） |
| Phase 3a | ▶ 下一步 | LSP 编排层独立化（LspCoordinator 基于 SemanticArtifact）

| 终态假设               | 对当前改动的约束                                                               | 满足方式                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Driver 变为 DAG 调度器 | `compile_source` / `run_module` 签名会变，但 `EventHandler` 透传模式不变 | `&dyn EventHandler` 沿调用链透传，未来升级为 `Arc<dyn EventHandler + Sync>` 不改 Stage 代码 |
| 新增语义事实层         | `InferRule` 生成 `Semantic` artifact                                       | 语义事实层不影响 VM 层的事件系统                                                                |
| 模块系统（传递哈希）   | 循环计数器在`load` 时重置                                                    | 每次`build(VMExec)` 新建 VM 并 load，计数器天然隔离                                           |
| Parallel build         | `emit!` 宏和 handler 都不持全局状态                                          | 未来线程安全只需`Arc<dyn EventHandler + Sync>`                                                |
| Composite 不变         | `CompositeHandler` 的广播语义（不重写 filter）                               | 已按此实现                                                                                      |

## 路线图

详见 [roadmap.md](roadmap.md)。
