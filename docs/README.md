# Kaubo 文档基线

这组文档是清理 Kaubo 编译器架构和协作方式的共同基线。实施前先统一工作规则和概念；后续每次重构都应该能追溯到这里定义的边界。

## 阅读顺序

1. [工作规则](working-rules.md)
2. [Roadmap](roadmap.md)
3. [源码与 Span](concepts/source-and-spans.md)
4. [诊断模型](concepts/diagnostics.md)
5. [编译产物](concepts/compiler-artifacts.md)
6. [用例概念](concepts/use-cases.md)
7. [DDD 与整洁架构](architecture/clean-architecture.md)
8. [分层与 crate](architecture/layers-and-crates.md)
9. [编排组件与调度](architecture/orchestration-and-scheduling.md)
10. [Pipeline 契约](architecture/pipeline-contracts.md)
11. [事件模型](architecture/event-model.md)
12. [应用适配层](architecture/app-adapters.md)
13. [质量门禁](quality/gates.md)

Roadmap 描述实施顺序和阶段验收；其余文档描述目标架构、概念边界和质量规则。

## 仓库范围

这是一个 monorepo。

- 主要 Rust workspace：`next_kaubo/`
- Web Playground：`next_kaubo/gui/`
- VSCode 扩展：`vscode-extension/`

## 核心定位

Kaubo 是一个编译器项目。编译器内核由 Rust 编写，外围应用包括 Rust CLI、Web Playground 的 WASM 桥接层，以及 VSCode 扩展。编译器内核负责语言语义；应用层只负责展示、交互和传输。

项目优先交付的是应用体验：CLI 工具、在线编辑器和 IDE 集成。因此架构不能只服务离线批处理编译，还必须原生支持诊断流转、进度观察、任务取消、按需产物和后续增量能力。

概念分层遵循 DDD 和整洁架构：

- 领域概念定义语言和运行时事实，例如源码、span、token、AST、CPS IR、诊断和运行时程序；
- 用例概念定义用户想完成的任务，例如 check、compile、run、hover、semantic tokens、取消和事件流；
- 架构机制定义 stage、mapper、pipeline plan、ports、事件边界、子编排组件和调度器；
- 展示传输概念定义 CLI/WASM/Web/VSCode 的 DTO、坐标映射和渲染。

crate 边界必须反映这些概念边界。stage 解耦仍然重要，但它是服务用例编排的手段，不是领域模型本身。orchestration 也不能变成万能胶水层；任务规划、接线、mapper、缓存、策略、事件、取消和调度都应该有清晰子组件。

warning 和 error 不能跳过。一个 crate 在自身测试通过、warning 清零、覆盖率达标之前，不应该被继续接入更大的 pipeline。
