# Web 和 VSCode

目标读者：维护编辑器适配层或 WASM 边界的开发者。

## 当前状态

Web app 和 VSCode 扩展都消费 WASM，但暴露的能力还不一致。

- Web Playground：CodeMirror editor、diagnostics、semantic highlighting、completions、hover。
- VSCode 扩展：TextMate grammar、snippets、language configuration，以及 WASM-backed diagnostics。

VSCode 当前还没有注册基于 `kaubo-language-service` 的 semantic token provider 或 completion provider。

## WASM 边界

`kaubo-wasm` 导出给适配层消费的函数：

- `lex(source)`
- `diagnose(source)`
- `compile(source)`
- `run(bytes)`
- `hover(source, offset)`
- `semantic_tokens(source)`
- `complete(source, offset)`

Web app 使用 `semantic_tokens` 做 CodeMirror decorations，使用 `complete` 做成员补全。

## Web Playground

重要 Web package：

- `next_kaubo/gui/packages/app`：Solid/Vite app。
- `next_kaubo/gui/packages/wasm`：生成的 WASM package，以 `@kaubo/wasm` 被消费。
- `next_kaubo/gui/packages/types`：共享 TypeScript package。

重要 editor 文件：

- `packages/app/src/editor/kauboLang.ts`：CodeMirror language 集成。
- `packages/app/src/editor/kauboAutocomplete.ts`：completion 集成。
- `packages/app/src/components/Editor/Editor.module.css`：editor token 样式。

Web 不应该自己分类语言语义。它应该消费 language service 产出的 token roles。

## VSCode 扩展

重要文件：

- `vscode-extension/src/extension.js`：activation 和 diagnostics wiring。
- `vscode-extension/syntaxes/kaubo.tmLanguage.json`：TextMate grammar。
- `vscode-extension/snippets/kaubo.json`：snippets。
- `vscode-extension/build-wasm.sh`：本地 WASM packaging helper。

当前 VSCode diagnostics 流程：

```text
VSCode document -> wasm.diagnose(source) -> JSON diagnostics -> DiagnosticCollection
```

目标未来流程：

```text
VSCode document -> wasm language service exports -> diagnostics / semantic tokens / completions
```

## 样式

共享 semantic roles 由 language service 定义。适配层把 role 映射到自己的样式系统：

- Web 把 role 映射到 CodeMirror CSS classes。
- VSCode 应把 role 映射到 semantic token types 和 theme colors。

Web 默认颜色应该偏浅色、可读性优先。不要把颜色策略编码进编译器 crate。

## 兼容性说明

WASM API 当前返回 JSON strings。如果 DTO 字段变化，需要在同一次改动中更新 Rust serialization、TypeScript parsing、Web tests 和 VSCode adapter tests。
