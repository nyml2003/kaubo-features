# 分层与 crate

crate 边界就是架构边界。目标不是为了拆分而拆分，而是让 DDD 的概念边界和整洁架构的依赖方向在代码中可见。

## 分层

```text
frameworks
  -> interface adapters
    -> use cases / orchestration
      -> stage execution mechanisms
        -> domain contracts + support primitives
```

依赖方向只能向内。外层可以实现内层定义的 ports；内层不能依赖外层框架。

## Domain 与 Support Primitives 层

最内层应该区分两类契约：

- domain contracts：语言、源码、诊断、产物和运行时领域模型；
- support primitives：schema version、纯坐标转换、结构化错误码等跨领域基础能力。

这些契约可以暂时由一个 common crate 承载，但文档和 API 必须区分它们的概念归属。

domain contracts 负责：

- source identity 和 source map；
- span 和 range；
- diagnostics；
- artifact identity 和产物结构；
- 被多个 stage 共享的运行时 value 和 program 契约。

support primitives 可以包含纯工具能力，例如坐标转换、结构化诊断构造和 schema version 定义。

这两类契约都不能解析、推断、lower、优化或执行语言语义，也不能依赖 Web、VSCode、CLI 或 WASM 框架。

## Stage 层

每个 stage crate 只负责一个编译职责：

- lexer：source text -> token stream；
- parser：token stream -> AST；
- infer：AST + type environment -> type artifact；
- lowering：AST/type artifact -> CPS；
- optimizer：CPS -> optimized CPS；
- runtime loader：runtime program preparation；
- VM：runtime program execution。

stage 是用例执行机制，不是领域概念。stage crate 不能依赖另一个 stage crate。它接收一个单一 request，产出一个成功路径 artifact。diagnostics、日志和事件通过调用时注入的 observer 或 sink 报告，不作为多返回值。

stage 必须无状态。它不能持有 session、cache、source map、diagnostic buffer、事件观察者或全局编译状态。

## Orchestration 层

use case / orchestration 负责组合 stage：

- 构建 compilation task；
- 选择 command profile；
- 定义或消费用例端口；
- 发布诊断、进度、产物、日志和任务事件；
- 解析 module 和 source；
- 生成 pipeline plan；
- 选择 pipeline policy；
- 在 stage 之间传递 artifact；
- 在相邻 stage 的输出和输入不对齐时，调用显式 mapper 组装下游 request；
- 汇总或转发 diagnostics/events；
- 处理取消、缓存、按需产物和失败策略；
- 向 adapter 暴露命令级 outcome 或事件流。

orchestration 可以依赖 stage crate。stage crate 不能依赖 orchestration。
stage crate 不知道上游或下游是谁；它只声明自己的 request 和 artifact。

orchestration 不能实现为单一超大模块。用例层至少应该按 `TaskService`、`PipelinePlanner`、`Connector`、`MapperRegistry`、`Scheduler`、`PolicyEngine`、`ArtifactCache`、`EventHub` 和 `CancellationCoordinator` 划分职责。详见 [编排组件与调度](orchestration-and-scheduling.md)。

## Adapter 层

interface adapter 把 use case 结果和事件暴露给具体环境：

- CLI 解析进程参数、读写文件、渲染终端输出；
- WASM 序列化 DTO，并向 JavaScript 暴露稳定函数；
- Web GUI 渲染编辑器状态、diagnostics、hover、output 和 settings；
- VSCode 把 DTO 映射到 extension APIs。

adapter 不能实现编译器语义。它们不能从 source text 推断 token class、diagnostic position 或 pipeline order。

## Frameworks 层

frameworks 是最外层，包括：

- `wasm-bindgen`；
- VSCode extension API；
- CodeMirror 和前端框架；
- 文件系统；
- 终端；
- 测试 runner 和构建工具。

框架依赖不能向内泄漏。领域和用例代码不应该出现 VSCode、CodeMirror、浏览器或终端专用类型。

## 当前 crate 的迁移说明

当前仓库已有一些有用基础，但依赖形态还不是目标形态：

- syntax 当前混合了 token、lexer、parser 和 AST；
- infer 直接依赖 syntax AST；
- IR 依赖 syntax，并同时拥有 CPS build 和 optimization；
- VM 依赖 IR，并把 encoding 和 execution 放在一起；
- WASM 和 CLI 重复编排 pipeline；
- Web-facing utilities 在 source model 外部计算展示 range。

迁移应该渐进完成，但每一步都必须让代码更接近上述分层规则。优先收敛公共 API 和跨层依赖，再拆内部实现。
