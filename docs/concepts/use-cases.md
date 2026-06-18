# 用例概念

用例概念描述“用户或应用想完成什么事”。它们不属于语言领域模型，也不属于某个 UI 框架。CLI、Web Playground 和 VSCode 可以用不同交互方式触发同一个用例。

## 核心原则

- 用例服务应用体验，而不是反过来让应用暴露编译器内部步骤。
- 用例可以选择只物化部分编译产物，例如只做诊断、只做高亮、只运行程序。
- 用例必须支持可观测性：诊断、进度、日志和中间产物事件都要能被上层消费。
- 用例必须支持取消。IDE 和在线编辑器中，新输入通常应该让旧任务尽快停止。

## CompilationTask

`CompilationTask` 是一次由应用触发的编译相关任务。

它至少应该能表达：

- 任务身份，例如 `task_id`；
- 源码身份和源码版本；
- 命令类型；
- 用户可见 options；
- 取消信号；
- 事件观察出口；
- 需要返回或导出的产物集合。

`CompilationTask` 不是语言概念。它不能进入 AST、CPS IR 或 VM 程序模型。

## CommandProfile

`CommandProfile` 描述任务目标和允许物化的产物范围。

常见 profile 包括：

- `check`：产生诊断，不运行程序；
- `lex`：产生 token 相关工具信息；
- `parse`：产生 AST 或语法工具信息；
- `compile`：产生运行时程序；
- `run`：编译并执行；
- `hover`：面向工具的局部语义查询；
- `semantic-tokens`：面向编辑器的语义高亮数据；
- `inspect`：导出调试用中间产物。

profile 不应该硬编码 UI 行为。Web 中的按钮、VSCode 命令和 CLI 参数只是触发 profile 的不同入口。

## EventStream

`EventStream` 是用例执行期间向应用暴露事实的统一出口。它承载事件，不承载 UI 决策。

事件至少分为：

- `DiagnosticEvent`：新增、更新或清理诊断；
- `ProgressEvent`：任务、阶段或子步骤进度；
- `ArtifactEvent`：某个产物可用；
- `LogEvent`：调试、性能或审计日志；
- `TaskEvent`：任务开始、完成、取消或失败。

事件应该带上 `task_id`、源码身份和源码版本，便于应用丢弃过期结果。

## CancellationToken

`CancellationToken` 表达任务取消请求。它由用例执行入口创建，并在调用链中传递。

取消不是错误诊断。取消表示当前任务已经不再对用户有价值，例如编辑器收到了更新的源码版本。

## ArtifactCache

`ArtifactCache` 是用例层的性能机制，用来复用已经物化且未失效的产物。

缓存键必须显式包含源码身份、源码版本或内容 hash、profile、options 以及相关依赖版本。缓存不能改变领域产物的含义，也不能让执行单元隐式持有状态。

## PipelinePlan

`PipelinePlan` 描述一次任务要执行哪些能力，以及这些能力之间的 artifact 依赖。

它应该被视为确定性 DAG，而不是只能串行执行的列表。线性执行是合法实现，但计划本身必须能表达未来的并行调度。

`PipelinePlan` 不是领域概念。AST、CPS IR、Runtime Program 等产物不应该知道自己属于哪个 plan。

## 不是用例概念的东西

以下概念不应该写进用例定义：

- AST 节点具体语义；
- CPS 指令含义；
- VM opcode 编码；
- CodeMirror range；
- `vscode.Diagnostic`；
- 终端渲染格式；
- WASM 序列化细节。
