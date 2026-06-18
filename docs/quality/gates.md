# 质量门禁

清理策略是 test-first 和 crate-first。每个 stage 在拥有有意义测试和干净输出之前，不应该追求一次性大重写。

## 不可协商规则

- 不跳过 warning 或 error。
- 不引入新的 Rust、TypeScript 或 lint warning。
- 不为了测试方便新增跨层依赖。
- 不依赖 E2E 测试来证明 stage-local 行为。
- crate 自身门禁未绿之前，不接入更大的 pipeline。
- 不把 planner、connector、scheduler、cache、policy、event hub 全部塞进一个无边界 orchestration 模块。

## Crate 就绪门禁

一个 crate 只有满足以下条件，才算准备好进入 orchestration 工作：

- 独立单测通过；
- 公共行为有有意义的回归测试；
- 该 crate 行覆盖率至少 80%；
- `cargo check` 和相关 lint 命令没有 warning；
- 公共 API 已按分层规则审计；
- 除非明确标记为 integration test，否则测试应直接构造 stage 输入，而不是依赖无关前序 stage。

## 推荐顺序

1. 文档化当前行为和预期 stage 契约。
2. 补缺失测试，直到每个 crate 都有能暴露已知缺口的有效红测试。
3. 按 crate 修 warning 和明显失败测试。
4. 按 crate 达到 80% 覆盖率。
5. 按目标分层审计公共 API 和依赖。
6. 通过 orchestration 组合 crate。
7. 增加跨 crate integration 和 adapter 测试。

## 覆盖率

覆盖率按每个 crate 计算，而不是只看 workspace 总体覆盖率。高覆盖率 crate 不能掩盖 syntax、IR、VM 或 adapter 的薄弱覆盖。

当前仓库已经有基于 `cargo-llvm-cov` 的覆盖率工具。门禁应逐步收敛到类似下面的 per-package 命令：

```text
cargo llvm-cov -p <crate> --branch --all-features
```

具体 CI 命令可以调整，但策略固定：每个 crate 必须达到 80% 门槛，才算足够稳定，可以进入更大范围集成。

## 测试拆分

当修改是局部的，先只测试最窄、最有用的层。默认拆分如下：

- syntax: lexer, parser, spans, diagnostics;
- IR：lowering 和 optimization；
- VM：execution behavior 和 runtime traps；
- app：adapter glue、store logic 和 UI rendering；
- cross-crate integration 只在本地 crate 门禁变绿之后进行。

## 当前审计项

这些是声明项目干净前需要处理的已知问题：

- `cargo check --workspace --all-targets` 当前仍报告 Rust warnings。
- `cargo test --workspace` 当前在 `kaubo-log` 失败。
- Web app 单测当前有 store-state expectation 失败。
- VSCode 扩展测试当前失败。
- `kaubo-wasm` 的 release profile 定义在成员 crate 中，会被 Cargo workspace profile 规则忽略。
- VM opcode encoding 和 execution 集中在一个文件里，并包含大量 magic numeric opcodes。
- Web 和 VSCode diagnostics 需要共享稳定 DTO，而不是 ad hoc JSON 和 offset reconstruction。

## Review checklist

合并架构清理工作前，检查：

- 修改过的 crate 是否新增了禁止依赖；
- 新公共类型应该属于 domain、use case、stage execution mechanism、interface adapter 还是 framework；
- diagnostics 是否使用结构化字段和 source span；
- adapter 代码是在渲染，而不是推导编译器语义；
- orchestration 改动是否落在明确子组件中；
- scheduler 行为是否保持确定性，尤其是并行事件排序和 cache 命中；
- optimizer API 是否是 `input -> output`；
- runtime magic number 是否被命名、隔离并测试；
- 测试是否在最窄、最有用的层证明行为。
