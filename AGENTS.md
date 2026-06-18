# Kaubo 仓库指南

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
- Rust 核心：`cd next_kaubo && cargo check --workspace --all-targets`
- Rust 测试：`cd next_kaubo && cargo test --workspace`
- Web 应用测试：`cd next_kaubo/gui && pnpm --filter @kaubo/app test`
- Web 应用构建：`cd next_kaubo/gui && pnpm --filter @kaubo/app build`
- VSCode 扩展测试：`cd vscode-extension && npm test`
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
