# 测试

目标读者：添加功能或修复 bug 的维护者。

## 当前标准命令

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

覆盖率：

```bash
cd next_kaubo
python3 ops/quality/coverage.py --html
```

Benchmark：

```bash
cd next_kaubo
python3 ops/benchmark/runner.py bench --release
python3 ops/benchmark/runner.py bench --suite fib --iterations 1 --no-warmup --lang kaubo
```

## 分层测试归属

测试应放在能证明行为的最窄层：

- Lexer/parser/span/diagnostics：`kaubo-syntax`。
- 类型规则：`kaubo-infer`。
- Lowering 和 CPS shape：`kaubo-ir` / `kaubo-cps`。
- 指令执行：`kaubo-vm`。
- 源码到输出行为：`kaubo-driver`。
- WASM DTO 行为：`kaubo-wasm` 或 adapter tests。
- CodeMirror glue：`@kaubo/app` tests。
- 浏览器行为：Playwright e2e。
- VSCode adapter 行为：`vscode-extension` tests。

## Bug 修复策略

Bug 修复优先使用 TDD：

1. 在最窄的有意义层级添加失败回归。
2. 实现最小修复。
3. 运行目标测试。
4. 如果修复跨 crate 或 adapter 边界，再运行更大范围的测试。

不要为了测试方便新增跨层依赖。如果一个测试需要另一层才能证明行为，它大概率应该放到更高的集成层。

## 推荐回归形态

Lexer/parser bug：断言 token kind、lexeme 和 range。

Semantic highlighting bug：先断言 language service 输出的 semantic token role 和 range，再按需单独断言 Web class mapping。

Runtime bug：如果 bug 涉及多个 stage，优先使用 driver-level 源码样例。只有当重点是指令语义时，才使用 VM-level 测试。

WASM/Web bug：分别断言序列化 DTO 和 adapter mapping。

## 生成产物

不要提交构建输出、测试结果、安装包或生成产物，除非仓库明确把该产物作为开发流程的一部分追踪。

`next_kaubo/gui/packages/wasm/pkg` 下的 Web WASM 生成 package 当前存在于 working tree。修改这里要谨慎，并确认任务是否明确要求重新生成 WASM。

Benchmark 的 Rust 对照实现会在 `ops/benchmark/suites/rust/target/` 生成构建产物；不要提交该目录。
