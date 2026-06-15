# WASM 绑定

## 概述

`kaubo-wasm` 将 Kaubo 编译器/运行时编译为 WebAssembly，供浏览器和 VSCode 扩展使用。

**关键文件：** `crates/kaubo-wasm/src/lib.rs`（427 行）

## 导出函数

### `lex(source: string) → string`

词法分析，返回 JSON token 数组。

```json
[{"kind":"keyword","from":0,"to":3}, {"kind":"identifier","from":4,"to":5}]
```

Token 位置为 UTF-16 偏移量（兼容 JavaScript/CodeMirror）。

**用途：** 语法高亮、补全上下文分析

### `diagnose(source: string) → string`

编译器诊断，运行 Lexer + Parser + TypeChecker，返回 JSON 错误数组。

```json
[{"severity":"error","line":1,"column":9,"from":8,"to":9,"message":"Unexpected token"}]
```

如果没有错误，返回 `"[]"`。

**用途：** Web 内联错误标记（lint gutter）、VSCode 实时诊断

### `hover(source: string, offset: number) → string`

光标悬停信息。在给定 UTF-16 偏移处查找 token，返回种类和描述。

```json
{"kind":"keyword","from":0,"to":3,"description":"variable declaration"}
```

如果该位置没有 token，返回 `"null"`。

**用途：** 编辑器悬停提示（type info tooltip）

### `compile(source: string) → number`

编译源码为字节码。返回字节码指令数。

将编译结果存入内部静态 `COMPILED` 变量，供 `run()` 使用。

**限制：** 静态共享，存在并发竞态风险。

### `run(bytes: Uint8Array) → string`

运行最近编译的字节码。返回 stdout 输出字符串。

内部调用 `catch_unwind` 捕获 panic，转换为 JS 错误。

## 构建

### Web Playground 用

```bash
wasm-pack build crates/kaubo-wasm \
  --target web \
  --out-dir gui/packages/wasm/pkg \
  --out-name kaubo_wasm
```

输出：ES 模块（`kaubo_wasm.js` + `kaubo_wasm_bg.wasm`）

### VSCode 扩展用

```bash
wasm-pack build crates/kaubo-wasm \
  --target nodejs \
  --out-dir vscode-extension/wasm \
  --out-name kaubo_wasm
```

输出：CommonJS 模块

## Token 种类映射

WASM 的 `lex()` 返回 7 种 token 显示种类：

| 显示种类 | 对应的 TokenKind | 用途 |
|----------|-----------------|------|
| `keyword` | Var, If, Else, While, For, ... (28 个) | 语法高亮（紫色） |
| `number` | LiteralInteger, LiteralFloat | 语法高亮（橙色） |
| `string` | LiteralString | 语法高亮（绿色） |
| `comment` | Comment | 语法高亮（灰色斜体） |
| `identifier` | Identifier | 语法高亮（浅蓝） |
| `atom` | True, False, Null | 语法高亮（红色） |
| `operator` | +, -, *, /, ==, !=, ... (20 个) | 语法高亮（青色） |

## TypeScript 接口

```typescript
// 从 @kaubo/wasm 导入
import init, { compile, run, lex, diagnose, hover } from "@kaubo/wasm";

await init();
const tokens = JSON.parse(lex("var x = 1;"));
const errors = JSON.parse(diagnose("var x = ;"));
const info = JSON.parse(hover("var x = 1;", 4));
const bytecodeLen = compile("print 42;");
const output = run(new Uint8Array());
```
