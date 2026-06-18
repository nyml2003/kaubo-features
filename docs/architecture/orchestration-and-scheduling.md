# 编排组件与调度

orchestration 不是一个应该无限膨胀的模块。它是用例层的一组协作组件，负责把用户任务转换成可执行计划，并在不污染 stage 和领域模型的前提下处理策略、缓存、事件、取消和并行调度。

## 目标

- 避免所有 stage 组装、mapper、缓存、任务策略都堆在一个超大模块里；
- 让 check、compile、run、inspect 等命令共享同一套编排能力；
- 为多核并行解析、并行类型推导和多任务并发隔离预留稳定契约；
- 保持 stage 无状态、单输入、单成功产物的边界不变。

## 子组件

orchestration 应拆成以下子组件，而不是一个万能服务：

- `TaskService`：接收 `CompilationTask`，校验 profile/options，创建任务作用域；
- `PipelinePlanner`：把 `CommandProfile` 转成 `PipelinePlan`；
- `Connector`：根据上游 artifact 和显式 mapper 组装下游 request；
- `MapperRegistry`：注册纯 mapper，禁止 mapper 承载新语言语义；
- `Scheduler`：执行 `PipelinePlan`，处理依赖、并行、取消和失败传播；
- `PolicyEngine`：决定 fail-fast、recover、warning-as-error、产物导出等策略；
- `ArtifactCache`：按源码版本、profile、options 和依赖版本缓存产物；
- `EventHub`：统一发布诊断、进度、产物、日志和任务生命周期事件；
- `CancellationCoordinator`：管理任务取消和旧版本结果过期。

这些组件可以先在一个 crate 中实现，但代码边界和测试边界必须按组件划分。后续如果模块变大，可以把组件拆到独立 crate，而不改变外部用例语义。

## PipelinePlan

`PipelinePlan` 是用例层的执行计划，不是领域概念。

目标形态是确定性 DAG：

```text
PipelinePlan
  nodes: StageInvocation[]
  edges: ArtifactDependency[]
  policy: PipelinePolicy
  outputs: RequestedArtifacts
```

- node 表示一次 stage 调用；
- edge 表示 artifact 依赖；
- policy 表示失败、恢复、缓存和导出策略；
- outputs 表示本次任务需要返回或发布的产物。

线性 pipeline 是 DAG 的特例。当前可以先按线性执行，但契约必须允许未来调度器并行执行没有依赖冲突的 node。

## 并行调度规则

并行执行必须保持确定性：

- 同一 `PipelinePlan`、同一源码版本、同一 options，最终 outcome 必须一致；
- 并行 node 不能共享可变 stage 状态；
- request 和 artifact 要么拥有数据，要么只共享不可变数据；
- diagnostics 和 events 可以并发产生，但对外发布必须带 `sequence` 或由 `EventHub` 进行稳定排序；
- 多个 source/module 可并行解析；
- 没有依赖关系的类型推导单元可以并行，但跨模块约束合并必须有确定顺序；
- artifact cache 必须按 task/source/profile/options 隔离，不能让旧任务覆盖新任务结果；
- 任务取消后，scheduler 应停止启动新 node，并让已运行 node 尽快退出。

并行能力属于 scheduler 和 use case 层。stage 只需要满足无状态、可重入、输入不可变、输出显式。

## 多任务隔离

IDE 和在线编辑器会同时存在多个任务：

- 旧源码版本的 diagnose；
- 新源码版本的 diagnose；
- 用户手动触发的 run；
- 后台 semantic tokens；
- inspect/debug 请求。

每个任务必须有独立 `task_id`、source version、event stream 和 cancellation token。应用层可以根据 `task_id` 和 source version 丢弃过期事件。

共享缓存只能保存可复用 artifact，不能保存任务私有状态。任务私有状态包括 event sequence、diagnostic buffer、progress、cancellation 和 requested outputs。

## 测试要求

orchestration 测试不应该只测端到端 happy path。至少要覆盖：

- planner 能为同一 profile 生成稳定 plan；
- connector 在上下游 artifact 不对齐时调用显式 mapper；
- scheduler 不执行依赖未满足的 node；
- scheduler 能取消未启动的后续 node；
- 并发事件通过 `EventHub` 稳定排序；
- cache 命中不会改变 observable outcome；
- 两个不同 source version 的任务不会互相覆盖 diagnostics 或 artifact。

