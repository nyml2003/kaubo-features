# Language Service

目标读者：维护语义高亮、补全、hover、diagnostics、Web editor 集成或 VSCode 集成的开发者。

## 当前状态

`kaubo-language-service` 是编辑器侧服务层。它消费源码并生成编辑器数据，例如 semantic tokens 和 completions。

它不是 compiler stage。VM 或 IR crate 不应该依赖它。

当前导出的能力：

- `semantic_tokens(source) -> Vec<SemanticToken>`
- `completions(source, offset) -> Vec<CompletionItem>`

Web app 通过 `kaubo-wasm` 消费这些能力。VSCode 还没有暴露同一套 semantic token provider。

## 当前模型

当前 service 从可见 lexer tokens 中构建一个小模型：

- 已知 structs；
- struct fields；
- impl methods；
- 简单 struct literal 对应的变量到 struct 绑定。

这足够支持基础 type、field、method、function 高亮，但仍然是 heuristic。它还没有完整 AST spans、词法作用域、definition/reference tracking。

## 已有基础设施：SemanticArtifact

Phase 2b 已在 `kaubo-driver` 中实现了 `SemanticArtifact`（symbols、type_env、references、symbol_names），由 `SemanticStage`（包装 `infer_module` + 符号收集）产出。编译器 Coordinator 通过 `semantic_at()` 可查询。

**当前 language-service 尚未消费 SemanticArtifact**。Phase 3a 将做这个对接。

## 目标定位（Phase 3a）

service 应该成为 adapter-facing 的编排层：

```text
source
  -> syntax/infer/semantic facts
  -> language service DTOs
  -> Web / VSCode / CLI
```

Phase 3a 的 `LspCoordinator` 将拥有自己的编排管线（Frontend→Semantic），和编译器 Coordinator 共享协议层。核心改动：

- **Go-to-definition**：基于 `SemanticArtifact.references`
- **Hover**：基于 `SemanticArtifact.symbols`
- **Completion 增强**：基于 `SemanticArtifact` + 原有 token 补全
- **Semantic tokens**：AST 节点类型 + 原有 token 分类 fallback

原有 token-based heuristic 保留作为 fallback。

## Semantic Token Roles

当前共享 role：

- `keyword`
- `number`
- `string`
- `comment`
- `identifier`
- `atom`
- `operator`
- `type`
- `field`
- `method`
- `function`

适配层可以用不同方式渲染这些 role，但不应该改变分类语义。

## DTO 兼容性

WASM 当前以 JSON array 形式序列化给 Web 消费。除非同一个 patch 同步修改适配层，否则保持 DTO 稳定。

未来支持 VSCode 时，应在扩展边界把 service token roles 映射到 VSCode semantic token types/modifiers，不要把 VSCode 概念放进编译器 crate。
