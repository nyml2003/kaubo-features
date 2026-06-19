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

这足够支持基础 type、field、method、function 高亮，但仍然是 heuristic。它还没有完整 AST spans、词法作用域、definition/reference tracking，也没有来自 inference 的完整 type facts。

## 期望定位

service 应该成为 adapter-facing 的编排层：

```text
source
  -> syntax/infer/semantic facts
  -> language service DTOs
  -> Web / VSCode / CLI
```

它的职责是把编译器事实翻译成稳定 DTO，而不是发明语言语义。

## 未来语义事实层

下一层很可能是新的 `kaubo-semantics` crate，或者一个职责等价的内部模块。它最终应该提供：

- 重要名称和表达式的 AST spans；
- structs、fields、methods、functions、consts、vars、params 的 symbols；
- scope-aware name resolution；
- definition/reference 映射；
- expression type facts；
- `expr.field` 和 `expr.method(...)` 的 member resolution；
- 带准确源码范围的 diagnostics。

这层存在后，language service 和 lowering 都应该消费同一套事实，而不是重复推断或重复实现名称解析 heuristic。

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
