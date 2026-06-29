# Kaubo 编译管线

## 全景

```
Source text
  │
  ▼
┌─────────────────────────────────────────────┐
│ 01 词法与语法分析  kaubo-syntax             │
│   Lexer::tokenize() → Parser::parse() → AST │
└─────────────────────────────────────────────┘
  │  Module (Stmt/Expr AST)
  ▼
┌─────────────────────────────────────────────┐
│ 02 类型推断        kaubo-infer              │
│   Algorithm W + interface 匹配 + vtable 生成 │
│   → SemanticArtifact (types + symbols)      │
└─────────────────────────────────────────────┘
  │  Module + 类型信息
  ▼
┌─────────────────────────────────────────────┐
│ 03 CPS IR          kaubo-ir                 │
│   build_module → CpsModule（分层 block）     │
│   flatten_module → 扁平化                    │
│   PassPipeline (EmptyBlockElim/MoveFold/     │
│                 ConstantFold) → 优化        │
└─────────────────────────────────────────────┘
  │  CpsModule（优化后）
  ▼
┌─────────────────────────────────────────────┐
│ 04 寄存器 VM       kaubo-vm                 │
│   VM::load(binary) → VM::execute()           │
│   44 opcodes, 统一寄存器组 Vec<u64>,         │
│   引用计数 GC, 死循环检测                     │
└─────────────────────────────────────────────┘
  │  RunOutcome { result, output }
  ▼
  适配层（Web / VSCode / CLI）
```

## Crate 地图

| Crate | 位置 | 行数 | 角色 |
|-------|------|------|------|
| `kaubo-token` | `next_kaubo/crates/kaubo-token` |    | Token 种类定义 |
| `kaubo-ast` | `next_kaubo/crates/kaubo-ast` |    | AST 节点定义 |
| `kaubo-syntax` | `next_kaubo/crates/kaubo-syntax` | ~3100 | Lexer + Parser |
| `kaubo-infer` | `next_kaubo/crates/kaubo-infer` | ~2700 | 类型推断 + interface |
| `kaubo-cps` | `next_kaubo/crates/kaubo-cps` | ~160 | CPS IR 数据结构 |
| `kaubo-ir` | `next_kaubo/crates/kaubo-ir` | ~3400 | CPS lowering + flatten + passes |
| `kaubo-vm` | `next_kaubo/crates/kaubo-vm` | ~2900 | 寄存器 VM |
| `kaubo-driver` | `next_kaubo/crates/kaubo-driver` | ~3500 | 编排层 + 模块系统 |
| `kaubo-log` | `next_kaubo/crates/kaubo-log` |    | 事件 trait + 类型（零依赖） |
| `kaubo-log-handlers` | `next_kaubo/crates/kaubo-log-handlers` |    | ConsoleHandler + CompositeHandler |
| `kaubo-vfs` | `next_kaubo/crates/kaubo-vfs` | ~140 | 虚拟文件系统 |
| `kaubo-language-service` | `next_kaubo/crates/kaubo-language-service` | ~610 | 编辑器集成 |
| `kaubo-web-api` | `next_kaubo/crates/kaubo-web-api` |    | WASM 共享 DTO |
| `kaubo-wasm` | `next_kaubo/crates/kaubo-wasm` |    | wasm-bindgen 导出 |

## 快速定位：要改 X → 看 Y

| 你想… | 文档 | 代码入口 |
|--------|------|---------|
| 加新语法 / 改 parser | [01 词法与语法分析](01-parser.md) | `kaubo-syntax/src/parser.rs` |
| 改类型系统 / 加 interface | [02 类型推断](02-type-inference.md) | `kaubo-infer/src/infer.rs` |
| 改 IR 指令 / 优化 pass | [03 CPS IR](03-cps-ir.md) | `kaubo-ir/src/cps_build.rs` |
| 改 VM 执行 / GC / opcode | [04 寄存器 VM](04-vm.md) | `kaubo-vm/src/execute.rs` |
| 改多文件编译 / 链接 | [05 模块系统](05-module-system.md) | `kaubo-driver/src/module_*.rs` |
| 加日志 / 调试输出 | [06 事件与日志](06-events-and-logging.md) | `kaubo-log/src/` |
| 改编辑器补全 / 高亮 | [07 Language Service](07-language-service.md) | `kaubo-language-service/src/lib.rs` |
| 改 WASM 边界 / 适配 | [08 Web 与 VSCode](08-web-vscode.md) | `kaubo-wasm/src/` |
| LSP / go-to-def / hover 方案 | [10 LSP 实施计划](10-lsp-implementation-plan.md) | — |
| 元组实施记录 | [09 元组实施记录](09-tuple-implementation.md) | — |
| 改 DAG 调度 / 编排层 | [11 DAG 调度器](11-dag-scheduler.md) | `kaubo-driver/src/dag/` |

## 阅读顺序

按管线编号：`01 → 02 → 03 → 04 → 05`（主管线），`06 → 07 → 08`（支撑层）。

- `01-05` 每章统一格式：**输入 → 做什么 → 输出 → 关键类型 → 代码在哪**
- `06` 是跨切面（所有 Stage 都发射事件）
- `07`, `08` 是适配层

## 实现状态

| Phase | 状态 | 内容 |
|-------|------|------|
| Phase 1 | ✅ | 日志 + 死循环防护 |
| Phase 4a | ✅ | Interface + operator + dyn Trait |
| Phase 4b | 🔶 | 虚拟 prelude，真实 prelude.kb 待做 |
| Phase 2b | ✅ | 编排层 + SemanticArtifact |
| Phase 3b | ✅ | 模块系统（ModuleGraph / ModuleCompiler / LinkStage） |
| Phase 2a | ⏸ | VM 性能（推迟） |
| Phase 3a | ▶ 下一步 | LspCoordinator + go-to-def + hover |

## 其他

- [路线图](../roadmap.md) — 完整迭代计划
- [运维指南](../operations/README.md) — 构建、测试、发布
- `kaubo-dev` skill — AI agent 开发上下文（DDD 约束、Ops2 入口）
