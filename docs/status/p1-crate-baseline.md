# P1 Crate Baseline

本文档记录 P1 阶段每个 Rust workspace crate 的局部状态。P1 的重点不是只跑通测试，而是让 stage crate 不再互相依赖，把共享数据结构下沉到 contract crate，并让 crate-local 测试直接构造本 crate 边界输入。

## Baseline

| Crate | `cargo check -p <crate> --all-targets` | `cargo test -p <crate>` | P1 work |
| --- | --- | --- | --- |
| `kaubo-ast` | pass | pass | 新增 AST contract；无 parser/infer/lowering 逻辑 |
| `kaubo-cps` | pass | pass | 新增 CPS contract；无 lowering/optimizer/VM 逻辑 |
| `kaubo-token` | pass | pass | 新增 token contract；无 lexer/parser 逻辑 |
| `kaubo-vfs` | pass | pass | 独立 crate；P1 未继续拆分 |
| `kaubo-log` | pass | pass | 独立 crate；修复 file sink 错误分支测试 |
| `kaubo-syntax` | pass | pass | 依赖 `kaubo-ast`；只负责 lexer/parser 输出 AST |
| `kaubo-infer` | pass | pass | 已从 `kaubo-syntax` 切到 `kaubo-ast`；测试直接构造 AST fixture |
| `kaubo-ir` | pass | pass | 已从 `kaubo-syntax` 切到 `kaubo-ast` + `kaubo-cps`；测试不再依赖 parser |
| `kaubo-vm` | pass | pass | 已从 `kaubo-ir` 切到 `kaubo-cps`；测试直接构造 CPS fixture |
| `kaubo-module` | pass | pass | 仍是外层 pipeline 编排，依赖 syntax/infer/IR；P1 剩余边界项 |
| `kaubo-web-api` | pass | pass | 仍混合 adapter 与诊断/语义调用；P1/P5 剩余边界项 |
| `kaubo-wasm` | pass | pass | 仍直接编排 pipeline；P4/P5 剩余边界项 |
| `kaubo2-cli` | pass | pass | 仍直接编排 pipeline；P4 剩余边界项 |
| `kaubo-workspace` | pass | pass | root package 无业务逻辑 |

## Warning Ownership

- `kaubo-wasm` non-root profile warning 已移到 workspace root。
- 目前不再保留 Rust crate-local warning。
- stage crate 的普通依赖已经切到 contract crate：`kaubo-infer -> kaubo-ast`，`kaubo-ir -> kaubo-ast + kaubo-cps`，`kaubo-vm -> kaubo-cps`。
- token contract 已从 `kaubo-syntax` 下沉到 `kaubo-token`，`kaubo-web-api` 的 token 展示逻辑直接依赖该 contract。
- `kaubo-module`、`kaubo-wasm`、`kaubo2-cli` 的 pipeline 编排依赖属于外层 orchestration/adapter 问题，不应再被当作 stage crate 内部依赖。
- `kaubo-web-api` 的 DTO/diagnostic 边界仍未收口，留到 P5，但当前依赖关系需继续记录。

## P1 Completion Checklist

- 所有 crate 满足 `cargo check -p <crate> --all-targets` 无 warning。
- 所有 crate 满足 `cargo test -p <crate>` 通过。
- stage crate 不直接依赖其他 stage crate，只依赖 contract/support crate。
- stage crate 测试优先直接构造本 crate 输入，不通过前序 stage 证明行为。
- `docs/status/p1-known-reds.md` 更新为 P1 结束时仍允许保留的全链路红项。
- 未引入 `kaubo-core`、stage request、orchestration 或新 DTO 迁移。
