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

Rust 核心：

```bash
cd next_kaubo
cargo check --workspace --all-targets
cargo test --workspace
```

Web app：

```bash
cd next_kaubo/gui
pnpm --filter @kaubo/app test
pnpm --filter @kaubo/app build
pnpm test:e2e
```

VSCode 扩展：

```bash
cd vscode-extension
npm test
```

Ops 工具：

```bash
cd next_kaubo
python3 ops/benchmark/runner.py bench --release
python3 ops/quality/coverage.py --help
python3 ops/release/publish.py --help
python3 ops/deploy/deploy.py --help
```

## 文档

- [架构](docs/architecture.md)
- [运行时](docs/runtime.md)
- [Language Service](docs/language-service.md)
- [语言参考](docs/language-reference.md)
- [Web 和 VSCode](docs/web-and-vscode.md)
- [测试](docs/testing.md)
- [发布和部署](docs/deploy.md)
- [路线图](docs/roadmap.md)

## 维护规则

仓库级工作规则在 [AGENTS.md](AGENTS.md)。简要原则是：保持改动小而局部，尊重 crate 边界，修 bug 优先 TDD，不要让 Web 或 VSCode 适配层重新实现编译器逻辑。
