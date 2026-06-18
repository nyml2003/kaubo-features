# 事件模型

事件模型用于支撑 IDE、在线编辑器和 CLI 工具的可观测性。它描述用例执行期间“发生了什么”，不描述 UI 应该“怎么显示”。

## 为什么需要事件

应用优先的编译器不能只返回最终结果：

- 编辑器需要实时诊断；
- Web Playground 需要展示进度和运行输出；
- CLI 需要稳定 exit code 和可读日志；
- VSCode 需要丢弃过期源码版本的结果；
- 长任务需要能被取消。

因此，编译任务应该通过事件流暴露过程事实，同时仍然保留清晰的成功路径产物。

## 事件类型

### DiagnosticEvent

表示诊断变化。

建议语义：

- `publish`：发布当前源码版本的一组诊断；
- `clear`：清理某个范围、phase 或 task 的旧诊断；
- `stale`：标记旧源码版本的诊断已过期。

诊断事件承载结构化 `Diagnostic`，不承载展示格式。

### ProgressEvent

表示任务进度。

建议字段：

- `task_id`；
- `label`；
- `phase`；
- `current`；
- `total`；
- `message`。

`current` 和 `total` 可以为空，因为有些步骤无法精确计数。

### ArtifactEvent

表示某个产物已经可用。

它用于 inspect、调试面板、增量缓存或工具查询。默认业务路径不应该要求应用消费所有中间产物事件。

### LogEvent

表示调试、性能或审计日志。

日志不是诊断。用户源码错误应该用 `DiagnosticEvent`，内部可观测信息才用 `LogEvent`。

### TaskEvent

表示任务生命周期：

- `started`；
- `completed`；
- `failed`；
- `cancelled`。

任务失败和用户源码诊断不是同一个概念。语法错误可能让 `check` 成功完成并发布 error diagnostic；基础设施错误才可能让任务失败。

## 通用元数据

事件应该尽量携带：

- `task_id`；
- `source_id`；
- `source_version`；
- `profile`；
- `phase`；
- `timestamp`；
- `sequence`。

这些字段用于排序、去重、过期清理和 UI 状态一致性。

并行调度下，多个 stage invocation 可能同时产生事件。`EventHub` 必须为对外事件提供稳定顺序：可以按生成顺序分配单调 `sequence`，也可以按 task、source version、phase 和局部序号排序。adapter 不应该自己解决并发事件排序。

## 与 Stage 的关系

stage 可以在调用时接收事件观察者，但不能持有观察者，也不能把事件观察者保存到全局状态。

事件观察者是执行时依赖，不是 stage 状态。stage 仍然必须满足：

- 单一输入；
- 单一成功产物；
- 无状态；
- 不知道上游和下游；
- 不构造 CLI/Web/VSCode 专用数据。

## 与 Adapter 的关系

adapter 订阅或接收事件，并把事件映射到具体展示环境：

- CLI 渲染为终端行、摘要和 exit code；
- Web GUI 渲染为 editor diagnostics、progress bar、output panel；
- VSCode 映射到 Problems、status item、semantic tokens 或 output channel。

adapter 不应该重新解释诊断语义，也不应该从源码重新推导事件。
