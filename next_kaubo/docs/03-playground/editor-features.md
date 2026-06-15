# 编辑器功能

## 语法高亮

通过 WASM `lex()` 执行词法分析，返回的 token 流映射为 CodeMirror `Decoration.mark()` 范围。

7 种 token 类别的配色由 CSS 变量控制（详见[主题配色](./themes.md)）：

| Token 种类 | CSS 类名 | CSS 变量 |
|-----------|---------|---------|
| 关键字 | `cm-kaubo-keyword` | `--kb-keyword` |
| 数字 | `cm-kaubo-number` | `--kb-number` |
| 字符串 | `cm-kaubo-string` | `--kb-string` |
| 注释 | `cm-kaubo-comment` | `--kb-comment` |
| 标识符 | `cm-kaubo-identifier` | `--kb-identifier` |
| 原子量 | `cm-kaubo-atom` | `--kb-atom` |
| 运算符 | `cm-kaubo-operator` | `--kb-operator` |

**实现文件：**
- `src/editor/kauboLang.ts` — CodeMirror StateField + linter + autocomplete
- `src/editor/kauboAutocomplete.ts` — 补全源
- `crates/kaubo-wasm/src/lib.rs` — WASM `lex()` 导出

## 内联错误诊断

WASM `diagnose()` 运行 Lexer + Parser + TypeChecker，返回结构化错误。

- 错误标红：gutter 红点 + 下划线波浪线
- 警告标橙：gutter 橙点 + 下划线波浪线
- 悬停显示：鼠标移到标记行显示错误详情（`.cm-tooltip`）

编辑时 400ms debounce 自动诊断，不弹窗。编译失败时额外弹 ErrorOverlay。

**实现文件：**
- `src/store/app.ts` — `runDiagnose()` + `scheduleDiagnose()`
- `src/editor/kauboLang.ts` — `setKauboDiagnostics()` + `linter()`

## 自动补全

Ctrl+Space 触发。22 关键字 + 25 内置函数 + 3 原子常量前缀匹配。

补全类型标签：

| 类型 | 图标 | 示例 |
|------|------|------|
| `keyword` | K | `var`, `if`, `while`, `struct` |
| `function` | F | `print`, `sqrt`, `len`, `list.map` |
| `constant` | C | `true`, `false`, `null` |

**实现文件：** `src/editor/kauboAutocomplete.ts`

当前限制：纯静态列表，无作用域感知补全（需要 WASM `complete()` 分析 AST 上下文）。

## 悬停类型提示

鼠标悬停在任意 token 上 → WASM `hover()` 返回 token 种类和描述。

```json
{"kind":"keyword","from":0,"to":3,"description":"variable declaration"}
```

**实现文件：** `src/editor/kauboLang.ts` — `hoverSource()`

## 代码折叠

基于缩进/括号的代码折叠，由 `@codemirror/language` 的 `foldGutter` 提供。

## 括号匹配

光标在括号上时高亮配对括号，由 `@codemirror/language` 的 `bracketMatching` 提供。

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| Ctrl+Enter | 运行代码 |
| Ctrl+Space | 触发补全 |
| Ctrl+/ | 切换行注释 |
| Ctrl+Z / Ctrl+Shift+Z | 撤销/重做 |
| Tab | 缩进（2/4 空格可配） |

## 配置

通过 Toolbar 齿轮图标打开 Settings 面板，可配置：

- **主题** — 5 种预设（Material Dark / Nord / Gruvbox Dark / Min Light / High Contrast）
- **缩进宽度** — 2 或 4 空格
- **字号** — 12 / 14 / 16px

所有配置通过 localStorage 持久化。

## 测试

| 测试文件 | 数量 | 覆盖 |
|----------|------|------|
| `kauboLang.test.ts` | 30 | 高亮/诊断/装饰纯函数 |
| `kauboAutocomplete.test.ts` | 14 | 补全匹配 |
| `store/app.test.ts` | 30 | 状态机/编译/运行/配置 |
