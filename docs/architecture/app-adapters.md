# 应用适配层

CLI、WASM、Web GUI 和 VSCode 是编译器内核外围的应用。它们共享稳定 DTO；它们不拥有编译器语义。

## CLI

CLI 负责：

- 参数解析；
- 文件 IO；
- 选择 check、compile、run、inspect 等命令；
- 在终端渲染 diagnostics 和 run results；
- 进程 exit code。

CLI 不拥有 pipeline 内部逻辑。它调用编译服务或 orchestration 暴露的命令接口。

## WASM 桥接层

WASM crate 负责：

- 向 JavaScript 暴露稳定函数；
- 序列化和反序列化稳定 DTO；
- 只有在失败不属于普通 diagnostic flow 时，才把 Rust error 转成 adapter-level transport error；
- 保留 panic hook 等浏览器相关设置。

WASM 不独立计算 semantic token class、compiler diagnostic 或编译顺序。

## Web GUI

Web Playground 负责：

- editor state 和 CodeMirror 集成；
- 渲染 syntax highlighting、hover、diagnostics、output、settings 和 examples；
- 调度用户触发的操作，例如 debounce diagnostics。

GUI 消费来自 WASM 的 DTO。它把 range 映射到 CodeMirror 坐标并渲染 UI。它不解析编译器消息，也不从源码推断编译器状态。

## VSCode 扩展

VSCode 扩展负责：

- extension activation；
- document event wiring；
- 把 diagnostic DTO 映射到 `vscode.Diagnostic`；
- 在 semantic tokens 可用之前维护 language configuration、snippets 和 TextMate grammar。

扩展应该使用和 Web GUI 相同的 diagnostic DTO。semantic token DTO 稳定后，VSCode 应该消费它，而不是维护一份分叉的 keyword list。

## 坐标映射

规范位置来自内核的 `SourceId + TextRange`。adapter 可以派生展示坐标：

- CLI 转成行列号和源码片段；
- Web GUI 转成 CodeMirror 位置；
- VSCode 转成 `vscode.Range`；
- WASM DTO 可以同时携带规范字节 range 和预计算 UTF-16 range。

这些派生坐标不能反向覆盖规范 range。

## 共享 adapter DTO

稳定 adapter surface 应包括：

- `LexResponse` or `SemanticTokensResponse`;
- `DiagnosticResponse`;
- `CompileResponse`;
- `RunResponse`;
- `HoverResponse`;
- 用于兼容性检查的 `VersionResponse`。

每个 response 都应包含足够元数据，让 adapter 能检测 schema version mismatch。

## 错误路径示例

```text
GUI 中 source edit
  -> WASM diagnose(source)
  -> 编译服务执行诊断流程
  -> 带 SourceId + TextRange 的 diagnostics
  -> WASM 序列化 DiagnosticResponse
  -> GUI 把 range 转成 CodeMirror diagnostics
```

同一个内核 diagnostic 也应该流经 CLI 和 VSCode：

```text
CLI check file
  -> 编译服务
  -> diagnostic
  -> terminal renderer

VSCode document change
  -> WASM 或 native diagnostic provider
  -> diagnostic
  -> vscode.Diagnostic
```
