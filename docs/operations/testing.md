# 测试

目标读者：添加功能或修复 bug 的维护者。

## 当前标准命令

所有测试入口统一收口到 `python kaubo-ops`。在 `packages/kaubo-features/` 目录下运行。

标准验证：

```bash
python kaubo-ops ci        # 标准 CI：check + lint + fmt-check + test + build
python kaubo-ops ci-full   # ci + Web e2e
```

分层任务：

```bash
python kaubo-ops check          # 快速类型检查（Rust + Web，无测试）
python kaubo-ops test           # 全部测试（Rust + Web + VSCode）
python kaubo-ops test-rust      # Rust 测试
python kaubo-ops test-web       # Web 单元测试
python kaubo-ops test-web-e2e   # Web e2e 测试
python kaubo-ops test-vscode    # VSCode 扩展测试
python kaubo-ops lint           # 全部 lint（clippy + eslint）
python kaubo-ops lint-rust      # Rust clippy
python kaubo-ops lint-web       # Web eslint
python kaubo-ops fmt            # 全部格式化（写入模式）
python kaubo-ops fmt-check      # 全部格式检查（dry-run）
python kaubo-ops build          # 构建所有产物
python kaubo-ops build-wasm     # 仅构建 WASM
python kaubo-ops build-cli      # 仅构建 CLI
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
