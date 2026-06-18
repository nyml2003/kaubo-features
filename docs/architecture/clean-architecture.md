# DDD 与整洁架构

Kaubo 的架构目标不是单纯把代码拆散，而是让概念有稳定归属。DDD 用来统一语言和边界，整洁架构用来约束依赖方向。

## 统一语言

同一个词只能在一个边界内拥有主要含义：

- `TokenStream`、`AST`、`CPS IR`、`Runtime Program` 是领域产物；
- `CompilationTask`、`CommandProfile`、`EventStream` 是用例概念；
- `Stage`、`Mapper`、`PipelinePlan` 是架构执行机制；
- `DiagnosticResponse`、`CodeMirrorRange`、`vscode.Diagnostic` 是展示或传输概念。

如果一个概念同时需要出现在多个边界，应该通过 DTO 或 adapter 显式转换，而不是让内部类型直接泄漏。

## 限界上下文

### Source Context

负责源码身份、源码文本、源码版本、字节范围、行列索引和坐标转换。

它不负责词法、语法或 UI 展示。

### Language Context

负责语言结构和语义产物，例如 token、AST、类型信息、CPS IR 和优化后的 IR。

它不负责任务取消、进度、缓存策略或应用展示。

### Diagnostics Context

负责结构化诊断的类别、严重级别、错误码、来源 phase、源码范围和关联信息。

它不决定 pipeline 是否继续，也不决定终端或编辑器如何渲染。

### Runtime Context

负责运行时程序、加载、执行、trap 和运行结果。

VM 只消费已编译的运行时产物，不应该知道源码解析细节。

### Compilation Task Context

负责应用触发的编译任务、命令 profile、取消、事件流、产物选择和缓存策略。

它可以编排语言领域能力，但不能把应用 UI 细节写回领域模型。

### Presentation Context

负责 CLI、WASM、Web GUI 和 VSCode 的展示、传输和框架 API 映射。

它不拥有编译器语义，不重新推导 token、类型、诊断位置或编译顺序。

## 整洁架构分层

```text
Frameworks
  -> Interface Adapters
    -> Use Cases
      -> Domain
```

依赖方向只能向内。外层可以依赖内层类型或端口，内层不能依赖外层框架。

## Domain

Domain 是最内层，包含稳定语言事实和运行时事实：

- `SourceText`、`SourceId`、`SourceVersion`、`TextRange`；
- `Token`、`TokenStream`、`AST`；
- `TypeTable`、`TypedAST`；
- `CPS IR`、`Optimized CPS IR`；
- `Runtime Program`、`RunResult`；
- `Diagnostic`。

Domain 不知道 CLI、WASM、Web、VSCode、文件系统、终端、CodeMirror 或 LSP。

## Use Cases

Use Cases 组织一次用户可见任务：

- `CompilationTask`；
- `CommandProfile`；
- `PipelinePlan`；
- `CancellationToken`；
- `EventStream`；
- `ArtifactCache`。

Use Cases 可以调用 stage、mapper 和 cache，但这些都是执行机制，不是领域概念。它们应该通过子编排组件协作，而不是集中到一个超大 orchestration 模块。

用例层的编排组件包括：

- `TaskService`；
- `PipelinePlanner`；
- `Connector`；
- `MapperRegistry`；
- `Scheduler`；
- `PolicyEngine`；
- `ArtifactCache`；
- `EventHub`；
- `CancellationCoordinator`。

其中 `Scheduler` 负责执行确定性 DAG 形式的 `PipelinePlan`，为多核并行解析、并行类型推导和多任务隔离提供统一入口。

## Ports

Ports 是用例层定义的接口，用来反转外部依赖：

- `SourceProvider`：提供源码；
- `ArtifactStore`：读取或保存产物；
- `DiagnosticObserver`：接收诊断事件；
- `ProgressObserver`：接收进度事件；
- `EventSink`：接收统一事件；
- `Clock` 或 `Timer`：提供时间信息。

端口由内层定义，外层实现。这样用例层不需要依赖 VSCode API、浏览器 API 或本地文件系统。

## Interface Adapters

Interface Adapters 把外部世界转换成用例层可理解的数据：

- CLI 参数转为 `CompilationTask`；
- WASM 函数参数转为稳定 DTO；
- Web GUI 把事件转成 editor state；
- VSCode 把诊断 DTO 转成 `vscode.Diagnostic`。

adapter 可以做格式转换和坐标转换，但不能做编译器语义判断。

## Frameworks

Frameworks 是最外层：

- `wasm-bindgen`；
- VSCode extension API；
- CodeMirror 或前端框架；
- 文件系统；
- 终端；
- 测试 runner 和构建工具。

框架可以替换，领域模型和用例语义不应该因此改变。

## Stage 的位置

`Stage` 是用例执行机制，不是领域概念。

一个 stage 封装某个语义转换能力，例如 lex、parse、infer、lower、optimize、build runtime program 或 execute VM。stage 必须无状态，并且只通过单一请求对象接收输入。

stage 的存在是为了让用例层可以组合、测试和替换能力；它不应该让领域产物知道 pipeline，也不应该让 UI 知道内部编译步骤。
