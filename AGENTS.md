# Kaubo 仓库指南

## 构建工具

本仓库用 **Ops2** (`python kaubo-ops <cmd>`) 统一编排所有任务。**优先用 `python kaubo-ops <cmd>`，不要手写 `cd xxx && cargo check ...` 这类裸命令。**

旧入口 `cargo make <task>` 仍可用（向后兼容），但新功能只在 Ops2 中添加。

### 所有命令统一在 `kaubo-ops/` 目录

**单一发现点**：`ls kaubo-ops/app/` 查看所有用例，`python kaubo-ops --help` 查看所有命令。

常用任务速查：

| 任务 | 用途 |
|------|------|
| `python kaubo-ops ci` | 标准 CI（check + clippy + fmt + test + WASM + Web + VSCode） |
| `python kaubo-ops ci-full` | CI + e2e |
| `python kaubo-ops check` | 快速类型检查（Rust + Web，无测试） |
| `python kaubo-ops build` | 构建所有产物（WASM + CLI + Web + VSCode） |
| `python kaubo-ops build-wasm` | 仅构建 WASM 双目标（web + nodejs） |
| `python kaubo-ops build-cli` | 仅构建 CLI 二进制（release） |
| `python kaubo-ops test` | 全部测试（Rust + Web + VSCode） |
| `python kaubo-ops test-rust` | Rust 测试 |
| `python kaubo-ops test-web` | Web 单元测试 |
| `python kaubo-ops test-web-e2e` | Web e2e 测试 |
| `python kaubo-ops test-vscode` | VSCode 扩展测试 |
| `python kaubo-ops lint` | 全部 lint（clippy + eslint） |
| `python kaubo-ops lint-rust` | Rust clippy |
| `python kaubo-ops lint-web` | Web eslint |
| `python kaubo-ops fmt` | 全部格式化（rustfmt + prettier，写入模式） |
| `python kaubo-ops fmt-check` | 全部格式检查（dry-run，不写入） |
| `python kaubo-ops dev` | 启动 Web 开发服务器（长驻进程，Ctrl-C 停止） |
| `python kaubo-ops release --bump patch` | 发布到 GitHub Release |
| `python kaubo-ops deploy 0.5.0` | 部署到 nginx |
| `python kaubo-ops bench --lang python,node` | 运行跨语言性能对比 |
| `python kaubo-ops coverage --html` | 生成覆盖率报告 |

### 架构速查

```
kaubo-ops/
├── cli/main.py         ← 表示层：argparse + 命令路由
├── app/                 ← 应用层：用例编排（CI/Build/Test/Lint/Fmt/Dev/Bench/Release/Deploy）
├── domain/              ← 领域层：KauboProject 聚合根 + 值对象（WasmArtifact/RustWorkspace/...）
├── infra/               ← 基础设施层：CommandRunner/ProcessRunner/FileSystem/EventBus 抽象
└── config.json          ← 集中配置：路径映射、WASM 目标、工具版本要求
```

**依赖方向：表示层 → 应用层 → 领域层 ← 基础设施层**

- 改子项目路径 → 只改 `config.json`
- 加新命令 → `app/` 下写用例 + `cli/main.py` 注册路由
- 改构建逻辑 → 改对应的领域对象（`domain/rust.py`、`domain/gui.py` 等）

## 范围

- 这是一个 monorepo。
- 主要 Rust workspace 在 `next_kaubo/`。
- Web Playground 在 `next_kaubo/gui/`。
- VSCode 扩展在 `vscode-extension/`。

## 工作规则

- 默认用中文回复用户。
- 优先走 TDD：先写失败测试，再实现，再重构。
- 优先做小而局部的修改，保留当前可运行路径。
- 把 crate 边界当作架构边界。
- 不要为了测试方便新增跨层依赖。
- 删除或简化代码前，先补测试或更新测试，保留当前行为的证据。
- 只要改动没有明确要求破坏兼容，就尽量保持公共接口稳定。
- 你引入的新 Rust、TypeScript 或 lint warning 要自己处理掉，不要留给后续。
- 一次改动如果会碰到多层，只做最窄能完成目标的那一层。

## 分层约束

- 词法、语法前端只负责把源码变成结构化 token、AST、span 和诊断信息。
- 类型推断、CPS/IR、优化和 VM 不应该依赖 Web 或 VSCode 适配层。
- VM 只消费已编译的 IR，不应该知道源码解析细节。
- Web 和 VSCode 适配层应该共享稳定的 JSON / DTO 结构，不要各自重新推导编译器逻辑。
- 旧代码、实验代码不要变成新工作的默认路径。

## 测试

- 优先用 Ops2 命令（见上方构建工具表格）：
  - Rust 核心：`python kaubo-ops test-rust`
  - Web 应用测试：`python kaubo-ops test-web`
  - Web e2e：`python kaubo-ops test-web-e2e`
  - VSCode 扩展测试：`python kaubo-ops test-vscode`
  - 标准 CI：`python kaubo-ops ci`
- 能拆开测就拆开测：
  - 词法 / 语法 / span / diagnostics 测 syntax
  - lowering 和 optimization 测 IR / CPS
  - 执行行为测 VM
  - 适配层和 UI glue 测 app
- 修 bug 时，优先补回归测试，最好和修复一起提交。

## 仓库卫生

- 不要提交生成产物、构建输出、测试结果或安装包目录。
- 除非任务明确要求，不要碰历史文档和归档代码。
- 如果你重命名或移动某个子系统，要把相关文档和测试一起更新。

## 默认流程

1. 先看相关代码和文档。
2. 只做能解决当前问题的最小修改。
3. 用最窄、最有用的测试集验证。
4. 只有当修改真正跨过边界时，才扩大测试范围。
