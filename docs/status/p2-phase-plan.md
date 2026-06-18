# P2 Phase Plan: Pipeline Core

本文档定义 P2 当前范围。P2 只做 pipeline 编排核心，不接入具体 compiler stage，不迁移 CLI/WASM 默认路径。

## Scope

- 新增独立 crate：`kaubo-pipeline`。
- 覆盖编排能力：plan、policy、cache、event、cancellation、scheduler。
- 通过抽象 `StageAdapter` 执行 node，测试使用 fake adapter。
- 不依赖 `kaubo-syntax`、`kaubo-infer`、`kaubo-ir`、`kaubo-vm` 或 adapter crate。

## Public Contracts

- `PipelinePlan`：确定性 DAG，维护 node、edge、policy 和 requested outputs。
- `StageNode`：抽象 stage node，只保存 stage 名称、输入 artifact id 和输出 artifact id。
- `PipelinePolicy`：当前包含 fail-fast 和 cache 开关。
- `Scheduler`：按 plan 执行抽象 stage，处理依赖、缓存、取消、失败和事件。
- `StageAdapter`：具体 stage 的唯一接入 trait；P2 不提供真实 stage adapter。
- `EventHub`：发布带 sequence、task id、source version 的 pipeline event。
- `ArtifactCache`：按 profile、source version、stage 和 output artifact 隔离缓存。

## Test Acceptance

P2 以 `kaubo-pipeline` crate-local 测试为主：

- `PipelinePlan` 输出稳定拓扑顺序。
- plan 拒绝重复 node、缺失 node edge 和 dependency cycle。
- `Scheduler` 按依赖顺序执行 node。
- cache 命中时不重新执行 stage。
- fail-fast 失败后跳过未完成 node。
- cancellation 阻止后续未开始 node。
- event sequence 稳定，并可按 task/source version 过滤。
- cache 按 source version 隔离。
- `kaubo-pipeline` 行覆盖率不低于 80%。

## Boundary Gates

- `cargo test -p kaubo-pipeline`
- `cargo llvm-cov --package kaubo-pipeline --summary-only`
- `cargo tree -p kaubo-pipeline --edges normal,dev,no-proc-macro`
- `cargo test -p kaubo-workspace --test crate_boundaries`
- `cargo check --workspace --all-targets`

## Explicit Non-Goals

- 不把 syntax/infer/IR/VM 接入 `kaubo-pipeline`。
- 不删除旧 CLI/WASM pipeline。
- 不定义 SourceText、Diagnostic、DTO 或 schema version 的最终形态。
- 不改变 Web Playground 或 VSCode 扩展路径。

## Current Status

- `kaubo-pipeline` 已加入 workspace。
- crate-local orchestration 测试已覆盖 plan、scheduler、event 和 cache。
- `cargo llvm-cov --package kaubo-pipeline --summary-only` 当前行覆盖率为 96.94%。
- stage crate 边界测试已把 `kaubo-pipeline` 纳入禁止互相依赖集合。
- 旧直连 pipeline 仍由 P4/P5 接管。
