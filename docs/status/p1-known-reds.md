# P1 Known Reds

本文档记录 P1 阶段允许暂时保留的全链路红项。P1 的目标是 crate-local 收敛，不要求这些命令全部通过。逐 crate 基线见 [P1 Crate Baseline](p1-crate-baseline.md)。

## 当前状态

| Command | Result | Reason | Next Phase | Notes |
| --- | --- | --- | --- | --- |
| `cd next_kaubo && cargo check --workspace --all-targets` | pass | Rust crate-local warning 已清理 | P1 | workspace profile warning 已移到 root |
| `cd next_kaubo && cargo test --workspace` | pass | `kaubo-log` 单测已修复 | P1 | crate-local Rust tests 已通过 |
| `cd next_kaubo && cargo test -p kaubo-token` | pass | token contract 有局部单测 | P1 | 作为 syntax/web-api 共享 contract |
| `cd next_kaubo && cargo tree -p kaubo-infer --edges normal,dev,no-proc-macro` | pass | infer 只依赖 AST contract | P1 | 不再依赖 syntax parser |
| `cd next_kaubo && cargo tree -p kaubo-ir --edges normal,dev,no-proc-macro` | pass | IR 只依赖 AST/CPS contract 和 serde | P1 | IR tests 不再依赖 parser |
| `cd next_kaubo && cargo tree -p kaubo-vm --edges normal,dev,no-proc-macro` | pass | VM 只依赖 CPS contract | P1 | 不再依赖 kaubo-ir |
| `cd next_kaubo && cargo tree --workspace --edges normal,no-proc-macro` | red | adapter/orchestration crates still compose pipeline directly | P4/P5 | `kaubo-module`、`kaubo-wasm`、`kaubo2-cli`、`kaubo-web-api` 仍依赖多个 stage crate |
| `cd next_kaubo/gui && pnpm --filter @kaubo/app test` | fail | app store 单测期望与当前实现不一致 | P1, P5 | 4 个失败：WASM not loaded、compile/run 返回 null、clearError |
| `cd vscode-extension && npm test` | fail | grammar 规则与测试期望不一致 | P1, P5 | 5 个失败：string escape、keyword group / coverage |
| `cd next_kaubo && cargo metadata --format-version 1 --no-deps` | pass | workspace profile 定义已收敛到 root | P1 | 不再提示 profile warning |

## 说明

- P1 不要求 Web app test/build 或 VSCode extension test 全绿。
- P1 当前已完成核心 stage 依赖切分；外层 adapter/orchestration 的 pipeline 依赖继续作为红项追踪。
- P1 结束后，这些红项必须仍然能按命令、crate 和原因追踪。
- 当某个红项被新路径接管后，才从这里移除。
- P2 及之后的工作开始前，应单独写 phase plan；本文件不定义 core concepts、stage request 或 DTO 具体 schema。
