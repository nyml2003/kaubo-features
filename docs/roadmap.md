# Kaubo Roadmap

本文档定义当前架构清理的实施顺序。架构文档描述目标边界；roadmap 描述每个阶段做什么、做完项目处于什么状态，以及旧代码什么时候退场。

核心策略是 crate-first：先让每个 crate 的当前行为可测试、无 warning、边界清楚，再把 `SourceText`、span、diagnostic、stage、orchestration 和 adapter DTO 等概念按文档落到代码里。

## P1 Crate-Local 收敛

目标：每个 Rust workspace crate 先在自己的边界内变干净。这个阶段不追求整体 pipeline、Web 或 VSCode 全部跑通。

工作内容：

- 按 crate 独立清理所有 Rust workspace crate：`kaubo-syntax`、`kaubo-infer`、`kaubo-ir`、`kaubo-vm`、`kaubo-log`、`kaubo-vfs`、`kaubo-web-api`、`kaubo-module`、`kaubo-wasm`、`kaubo2-cli` 和 workspace root。
- 每个 crate 至少满足 `cargo check -p <crate> --all-targets` 无 warning，`cargo test -p <crate>` 通过。
- 为公共行为补局部回归测试，优先测试本 crate 输入和输出。
- IR 和 VM 测试优先直接构造 IR / runtime fixture，不通过 parser 证明执行语义。
- 记录仍然失败的跨 crate、CLI、Web 或 VSCode 路径，以及失败原因，写入 `docs/status/p1-known-reds.md`。
- 逐 crate 的起始状态和 P1 任务清单维护在 `docs/status/p1-crate-baseline.md`。

阶段完成状态：

- 各 crate 内部行为可被信任。
- 整体 pipeline 允许仍然不通，但失败范围已知。
- 仓库不再在一堆 warning 和 crate-local 红测之上继续堆架构改动。
- P1 结束时，`docs/status/p1-known-reds.md` 仍可保留全链路红项，但必须逐条可追踪。

旧代码退场：

- 只删除 crate 内已经被测试覆盖的 dead code、未使用 helper 和明显冗余分支。
- 不删除跨 crate 兼容入口；这些入口要等新 stage 或 orchestration 路径接管后再退场。

## P2 Pipeline Core 落地

目标：先把 pipeline 编排能力做成独立 crate，只验证 plan、cache、event、cancel 和 scheduler 的编排语义。这个阶段不接入具体 compiler stage，也不迁移 CLI/WASM 默认路径。

工作内容：

- 新增 `kaubo-pipeline`，承载 `PipelinePlan`、`PipelinePolicy`、`StageAdapter`、`Scheduler`、`EventHub`、`ArtifactCache` 和 cancellation token。
- `PipelinePlan` 使用确定性 DAG；线性 pipeline 只是 DAG 的特例。
- `Scheduler` 只通过 `StageAdapter` trait 调用抽象 stage，不依赖 `kaubo-syntax`、`kaubo-infer`、`kaubo-ir` 或 `kaubo-vm`。
- 测试重点覆盖稳定拓扑顺序、依赖跳过、fail-fast、取消、缓存命中、事件顺序和 task/source version 隔离。
- `kaubo-pipeline` crate 行覆盖率必须达到 80% 以上。

阶段完成状态：

- 编排核心可以被独立单测，不需要真实 parser/infer/lowering/VM 参与。
- `kaubo-pipeline` 不依赖任何 stage crate；stage crate 也不互相依赖。
- 现有 CLI/WASM/adapter 仍可短期继续走旧 pipeline，后续接入由单独阶段完成。

旧代码退场：

- P2 不删除旧 CLI/WASM pipeline。
- 旧直连 pipeline 继续作为 P4/P5 红项追踪。
- 具体 stage 接入前，不把 parser/infer/lowering/VM 依赖加进 `kaubo-pipeline`。

## P2b Core Concepts 落地

目标：把源码身份、规范 range、诊断和 schema version 变成内核共享契约，停止从字符串、line/col 或 cooked token 反推编译器事实。

工作内容：

- 新增或明确内层基础契约位置，用于承载 `SourceText`、`SourceId`、`SourceVersion`、`TextRange`、`ByteSpan`、`Diagnostic`、`Severity`、`Phase` 和 `SchemaVersion`。
- `kaubo-syntax` token 开始携带规范 byte range；AST 节点按风险逐步补 range。
- parser、infer、lowering 和 runtime 错误逐步映射为结构化 diagnostic。
- 展示层需要的 line/col 和 UTF-16 range 只能由 source map 或 adapter 从规范 range 派生。

阶段完成状态：

- 内核开始有唯一源码位置和诊断事实来源。
- Web、VSCode 和 CLI 可以短期继续走旧接口，但旧接口只能包裹新 diagnostic，不再重新发明位置语义。
- 字符串转义、多字节字符、注释和语法错误恢复不再依赖 token cooked value 计算范围。

旧代码退场：

- 旧 line/col、字符串错误和手工 offset 逻辑先降级为 adapter 兼容层。
- 新 diagnostic 能覆盖旧行为后，删除对应的字符串拼接错误路径。
- 兼容 wrapper 最多允许跨到 P3，不能继续成为新调用点。

## P3 Stage Contracts 收窄

目标：让每个 stage 变成单 request 输入、单成功产物输出的无状态执行单元。

工作内容：

- 为 lex、parse、infer、lower、optimize、runtime build 和 run 增加 request envelope。
- stage 成功路径只返回 artifact；diagnostics、logs 和 events 通过调用时注入的 observer 或 sink 报告。
- 旧 API 保留为 wrapper，内部调用新 request API。
- 相邻 stage 的 request 组装不写进 stage；后续交给 connector 和 mapper。

阶段完成状态：

- stage 边界符合文档，可以独立测试、替换和组合。
- 现有 CLI/WASM 路径仍可运行旧入口，但新代码有明确 canonical stage API。
- 后续 orchestration 不需要猜测各 stage 的参数和副作用。

旧代码退场：

- 旧多参数入口、返回字符串错误入口和 stage 间直连 helper 变成 wrapper 后，不再新增调用点。
- P3 末统计旧 stage API 调用点；能改为新 request API 的全部迁走。
- 仍需保留的 wrapper 必须标记迁移原因，并在 P4 接管后删除。

## P4 Orchestration v1

目标：让 check、compile 和 run 由统一用例层接管，而不是 CLI 和 WASM 各自编排 pipeline。

工作内容：

- 新增 orchestration 层，提供 `CompilationTask`、`CommandProfile`、`PipelinePlan`、`Connector`、`PolicyEngine`、`EventHub` 和线性 `Scheduler`。
- v1 可以线性执行，但 `PipelinePlan` 数据模型必须允许未来扩展为确定性 DAG。
- 将 CLI 和 WASM 中重复的 parse -> infer -> lower -> flatten -> optimize -> run 逻辑迁入 orchestration。
- 事件和 diagnostic 通过统一出口发布，adapter 只做展示和传输转换。

阶段完成状态：

- check、compile 和 run 有同一条内核任务路径。
- CLI 和 WASM 不再拥有编译顺序、失败策略或产物选择语义。
- Rust integration 路径开始成为主要验收对象。

旧代码退场：

- 删除 CLI 和 WASM 里的直连 pipeline 实现。
- 删除只服务旧 pipeline 的 compile/run 缓存状态和临时 glue。
- 若某个旧入口仍要保留，只能作为调用 orchestration 的 adapter facade。

## P5 Adapter DTO 收口

目标：Web Playground 和 VSCode 扩展共享同一套稳定 JSON / DTO，不再各自推导 diagnostic、range、token class 或 pipeline 结果。

工作内容：

- `kaubo-web-api` 输出稳定 DTO：`VersionResponse`、`DiagnosticResponse`、`LexResponse`、`CompileResponse`、`RunResponse` 和 `HoverResponse`。
- `next_kaubo/gui/packages/types` 镜像这些 DTO，并作为 Web app 的类型入口。
- VSCode 扩展消费同一 diagnostic DTO，映射为 `vscode.Diagnostic`。
- Web app 消费同一 lex/hover/diagnostic DTO，映射为 CodeMirror state。

阶段完成状态：

- 应用层只做框架映射和渲染。
- Web 和 VSCode 的诊断、range、schema 和 token 语义一致。
- adapter 不再重新解析编译器消息，也不从源码重新推导编译器状态。

旧代码退场：

- 删除 Web 和 VSCode 中 ad hoc JSON 解析分支、offset 重算、重复 keyword list 和分叉 diagnostic shape。
- 删除旧 DTO 类型，保留的 TypeScript 类型必须对应 Rust DTO。
- 删除旧 WASM 返回字符串格式，除非该字符串本身就是稳定 DTO 序列化结果。

## P6 全链路门禁与残余退场

目标：仓库只保留一条默认 canonical path，并把全量门禁固定下来。

工作内容：

- 固定全量命令：`cargo check --workspace --all-targets`、`cargo test --workspace`、`pnpm --filter @kaubo/app test`、`pnpm --filter @kaubo/app build` 和 `npm test`。
- 引入 per-crate 覆盖率目标，逐步收敛到 80% 行覆盖率。
- 清掉只为迁移存在的 shim、dead branches、实验路径和废弃实现。
- 文档更新为当前路径，不保留旧路径作为默认方案。

阶段完成状态：

- 内核 crate、orchestration、CLI、WASM、Web 和 VSCode 全部走同一套分层契约。
- 默认构建和默认运行路径不再经过旧实现。
- 后续新增语言特性可以按 crate -> stage -> orchestration -> adapter 的稳定路径推进。

旧代码退场：

- 删除所有超过一个 phase 仍未迁走的 wrapper，除非有明确兼容承诺和测试。
- 删除旧代码后必须跑对应局部测试；跨层删除必须跑全链路门禁。
- 如果某个旧实现仍被保留，必须在文档中说明它不是默认路径，并列出移除条件。

## Old-Code Retire Rule

- 新路径成为默认入口、同类测试通过、旧调用点归零后，立刻删除旧实现。
- 兼容 wrapper 最多跨一个 phase。
- 删除旧代码前，先补能证明新路径覆盖旧行为的回归测试。
- 超过一个 phase 还保留的旧代码，必须记录为风险项。
- 旧代码、实验代码不能重新成为新工作的默认路径。

## 验收原则

- 每个阶段先补失败测试，再实现或删除。
- 局部改动先跑最窄 crate 或 adapter 测试。
- 阶段末记录剩余红项和下一阶段归属。
- P1 不要求整体产品可用；P4 之后开始要求核心 check/compile/run integration 稳定；P6 才要求全量门禁稳定。
- P2 及后续阶段开始前必须补单独 phase plan，明确新增类型、接口、迁移顺序和测试验收。
