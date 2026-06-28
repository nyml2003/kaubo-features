# Kaubo

Kaubo 是一个实验性编程语言实现。本仓库包含 Rust 编译器/运行时工作区、Web Playground，以及 VSCode 扩展。

## 当前状态

当前可运行主路径是 `kaubo-driver` 提供的直接驱动：

```text
source -> parser -> infer -> CPS build -> flatten -> constant fold -> VM
```

仓库已经收敛到当前主路径。判断“这个程序现在能不能跑”，应优先看 `kaubo-driver`。

编辑器侧的公共服务是 `kaubo-language-service`。它当前通过 WASM 给 Web app 提供 semantic tokens 和成员补全，但仍是轻量服务，还没有完整的 span-aware 语义模型。

## 仓库结构

- `next_kaubo/`：主 Rust workspace。
- `next_kaubo/crates/kaubo-token`：token 定义。
- `next_kaubo/crates/kaubo-ast`：AST 定义。
- `next_kaubo/crates/kaubo-syntax`：lexer 和 parser。
- `next_kaubo/crates/kaubo-infer`：类型推断和类型诊断。
- `next_kaubo/crates/kaubo-cps`：CPS IR 数据模型。
- `next_kaubo/crates/kaubo-ir`：CPS 构建、优化 pass、二进制编码。
- `next_kaubo/crates/kaubo-vm`：寄存器 VM 和 native stdlib。
- `next_kaubo/crates/kaubo-driver`：当前直接 compile/run 编排层。
- `next_kaubo/crates/kaubo-language-service`：编辑器语义能力。
- `next_kaubo/crates/kaubo-web-api`：Web/WASM 共享 DTO 辅助逻辑。
- `next_kaubo/crates/kaubo-wasm`：wasm-bindgen 导出。
- `next_kaubo/kaubo2-cli`：当前基于 `kaubo-driver` 的 CLI。
- `next_kaubo/ops/`：发布、部署、覆盖率和 benchmark 工具。
- `next_kaubo/gui/`：Web Playground。
- `vscode-extension/`：VSCode 扩展。
- `docs/`：维护者文档。

## 快速验证

项目统一使用 `cargo-make` 编排验证和运维任务。确认本机已安装 `cargo-make` 后，在仓库根目录运行：

```bash
cargo make ci
```

如果只想确认入口工具：

```bash
cargo make --version
```

常用任务：

```bash
cargo make rust-check
cargo make rust-test
cargo make web-test
cargo make web-build
cargo make vscode-test
cargo make ci-full
```

Ops 工具：

```bash
cargo make ops
cargo make ops-coverage-html
cargo make ops-bench
cargo make ops-bench-check
cargo make ops-release -- --bump patch
cargo make ops-deploy -- 0.5.0 --repo owner/repo
```

## 文档

- [文档入口](docs/README.md) — 按读者分层的索引入口
- [架构：编译管线](docs/architecture/README.md) — crate 地图 + "要改X看这里"
- [语言参考](docs/language/README.md) — 语法特性、标准库、已实现能力
- [运维指南](docs/operations/README.md) — 测试、发布、部署
- [路线图](docs/roadmap.md) — 迭代计划和 Phase 状态

## 维护规则

仓库级工作规则在 [AGENTS.md](AGENTS.md)。简要原则是：保持改动小而局部，尊重 crate 边界，修 bug 优先 TDD，不要让 Web 或 VSCode 适配层重新实现编译器逻辑。
