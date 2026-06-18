# Pipeline 契约

每个 compiler stage 都应该有狭窄契约：stage 本身只描述成功路径的语义转换，diagnostics、日志和事件通过调用时注入的副通道报告，pipeline 策略由用例层决定。

`Stage` 是用例执行机制，不是领域概念。领域产物不应该知道自己来自哪个 stage，也不应该知道下一个 stage 是什么。

stage 必须是无状态的。它不持有 compilation session、缓存、source map、diagnostic buffer 或其他可变状态。

stage 的输入也必须是单一的。即使语义上需要多份数据，它们也应该先由 `Connector` 组装进一个 immutable request envelope，再交给 stage。stage 本身不感知“我原来有几个输入”，只感知“我收到一个请求对象”。

相邻 stage 的输出和输入不要求天然对齐。`Stage A` 的 artifact 如果不能直接作为
`Stage B` 的 request，这不是 `Stage A` 或 `Stage B` 的责任。接线、拆分、
聚合和转换由 `Connector` 和显式 mapper 负责。

## 通用形态

```text
Stage<Request>
  -> 通过 DiagnosticObserver / EventSink / LogSink 回调报告副通道信息
  -> 返回成功路径产物 Artifact 或失败状态
```

具体 Rust API 可以不同，但语义契约必须保持：

- `Lexer`、`Parser`、`Infer` 这类 stage 都只接收一个请求对象，并且只表达成功路径；
- request 是不可变数据，不是持有状态的 context；
- diagnostics、日志、恢复事件、trace 不应该被塞进核心输出；
- observer、sink 或 reporter 作为调用时依赖传入，stage 不持有它们；
- stage 不 panic、不直接打印、不构造应用层 JSON。
- stage 不负责把自己的输出适配成下一个 stage 的输入。

## Stage 定义

`Stage` 不是“某个函数的参数列表”，而是一个语义单元：

- 它接收一个单一的 request envelope；
- request envelope 里可以装载该 stage 需要的所有前置信息、options 和
  显式数据；
- request 的组装由 `Connector` 负责，不由 stage 负责；
- stage 只负责消费 request，并产出一个成功结果 artifact；
- errors、diagnostics、logs 和 events 通过副通道传出，不作为多返回值的一部分。
- stage 不保存 request，不保存 sink，不缓存结果，不读取全局编译状态。
- stage 不知道前一个 stage 是谁，也不知道下一个 stage 是谁。

换句话说，stage 可以看到很多数据，但它只看见一个对象，而不是多个独立输入。

## 接线与转换

pipeline 中的接线是 `Connector` 和 `MapperRegistry` 的职责：

- 从上游 artifact 组装下游 request；
- 在两个相邻 stage 的数据形态不一致时调用显式 mapper；
- 把 source identity、options、policy 等外层信息附着到 request；

mapper 是纯转换单元，不拥有编译语义。它只表达“如何从已有 artifact 组装某个
stage 的 request”。如果转换过程需要新的语言语义判断，就说明这个逻辑应该进入
某个明确 stage，而不是混在 mapper 或 connector 里。

## Pipeline 策略

pipeline 策略由 `PolicyEngine` 和 `Scheduler` 执行：

- 当后续 stage 无法有意义执行时 fail fast；
- 当可以恢复时继续收集 diagnostics；
- 决定带有 partial diagnostics 时是否返回 artifact；
- 决定某个命令下 warning 是否视为 fatal；
- 决定哪些 artifact 需要缓存或导出。
- 决定任务取消后如何停止后续工作；
- 决定哪些事件需要转发给 adapter。

stage 只报告事实，不决定全局控制流。

## PipelinePlan 与并行

`PipelinePlan` 应按确定性 DAG 建模。线性 pipeline 是 DAG 的特例。

```text
StageInvocation nodes
  + ArtifactDependency edges
  + PipelinePolicy
  + RequestedArtifacts
```

`Scheduler` 可以并行执行依赖已满足且互不冲突的 node，但必须保持 observable outcome 确定：

- 同一源码版本和 options 下，最终 artifact、diagnostics 和 task outcome 必须一致；
- stage 必须无状态、可重入，不共享可变内部状态；
- request 和 artifact 必须是不可变输入输出；
- 多个 source/module 可以并行 lex/parse；
- 类型推导可以按无依赖单元并行，但约束合并顺序必须稳定；
- cache 命中不能改变 diagnostics、events 或 artifact 的语义；
- 事件可以并发产生，但必须由 `EventHub` 增加 sequence 或稳定排序后再暴露给 adapter。

并行调度细则见 [编排组件与调度](orchestration-and-scheduling.md)。

## 事件与取消

事件和取消是用例执行能力，不是领域产物的一部分。

- `DiagnosticEvent`、`ProgressEvent`、`ArtifactEvent`、`LogEvent` 和 `TaskEvent` 通过事件模型定义；
- `CancellationToken` 由用例入口创建，并作为调用时依赖传递；
- stage 可以检查取消信号并尽快返回，但不能保存取消信号；
- stage 可以发布事件，但不能构造 CLI/Web/VSCode 专用展示数据。

事件模型见 [事件模型](event-model.md)。

## Lexer

- 输入：`SourceText`。
- 输出：`TokenStream`。
- diagnostics：非法字符、未闭合字符串、未闭合块注释、不支持的 escape。
- 边界：不解析、不知道类型、不产生 Web/VSCode-specific offset。

`SourceId` 由编译任务或 connector 在 stage 外层附着到 `TokenStream`
和 `Diagnostic` 上，用于多文件编译、缓存、跳转和错误回传；它不是
lexer 的内部状态。

## Parser

- 输入：`TokenStreamRequest`，其中包含单一 `TokenStream` 请求对象。
- 输出：AST。
- diagnostics：通过 stage 的 `DiagnosticSink` 报告 expected token、unexpected token、malformed declaration、malformed expression、recovery notes。
- 边界：不做类型推断、不做 CPS 决策、不产出 adapter JSON。

如果工具链需要 parse-event 或 CST，它应作为独立 tooling stage 或 parser
模式设计，不能让语义 parser 的成功路径变成多输出。

## 类型推断

- 输入：`InferRequest`，其中包含单一的 AST 请求对象和显式 type environment。
- 输出：`TypeTable` 或 `TypedAST`。
- diagnostics：unknown symbol、unification failure、invalid call、invalid member/index access、unsupported construct。
- 边界：不解析源码、不发射 IR、不做 VM 假设。

## CPS lowering

- 输入：`LowerRequest`，其中包含单一的语义输入对象。
- 输出：CPS module。
- diagnostics：unsupported lowering form、missing required type information、invalid control-flow construct。
- 边界：不解析、不推断类型、不做 optimization pass、不做 VM opcode encoding。

## 优化

- 输入：`OptimizeRequest`，其中包含单一的 CPS module。
- 输出：新的 optimized CPS module。
- diagnostics：optimizer internal invariant violations 或 unsupported IR shape。
- 边界：公共 API 不能是 in-place mutation。pass 内部可以 mutate local clone。

## Runtime program build

- 输入：`RuntimeBuildRequest`，其中包含单一的 optimized CPS module。
- 输出：runtime program 或 encoded bytecode。
- diagnostics：invalid register、invalid block target、unsupported operation、encoding overflow。
- 边界：encoding 和 VM execution semantics 必须分离。

## VM execution

- 输入：`RunRequest`，其中包含单一的 runtime program 请求对象。
- 输出：`RunResult`。
- diagnostics：runtime trap 和 internal bug。
- 边界：不解析源码、不做类型推断、不做 frontend recovery。

## 测试规则

stage 测试应直接构造最接近的输入 artifact。parser 测试可以使用 token stream。IR 测试可以构造 AST/type fixture 或 CPS fixture。VM 测试应该直接构造 runtime program 或 CPS fixture；除非明确标记为 integration test，否则不应该依赖 lexer/parser 行为。
