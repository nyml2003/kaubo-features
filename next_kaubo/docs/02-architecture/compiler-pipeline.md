# 编译器流水线

## Lexer（词法分析）

```
源码文本 → Lexer::feed() → Token 流
```

| 项目 | 状态 |
|------|------|
| 39 种 Token 类型 | 完成 |
| UTF-8 支持 | 完成 |
| 行注释 `//` + 块注释 `/* */` | 完成 |
| 字符串转义（`\n` `\t` `\"` 等） | 完成 |
| 整数/浮点字面量 | 完成 |
| 位置追踪（line/column/offset） | 完成 |
| 错误恢复 | 完成 |
| 模板字符串 | ❌ |
| 十六进制/八进制字面量 | ❌ |
| Unicode 转义 `\uXXXX` | ❌ |
| 嵌套块注释 | ❌ |

**关键文件：**
- `crates/kaubo-compiler/src/lexer/token_kind.rs` — 39 种 token 定义
- `crates/kaubo-compiler/src/lexer/kaubo.rs` — Kaubo 特定扫描模式
- `crates/kaubo-compiler/src/lexer/error.rs` — `LexerError` 结构化错误

## Parser（语法分析）

```
Token 流 → Parser::parse() → AST Module
```

使用 **Pratt 解析框架**（优先级 + 结合性驱动）。

| 项目 | 状态 |
|------|------|
| 19 种表达式 | 完成 |
| 14 种语句 | 完成 |
| Pratt 优先级/结合性 | 完成 |
| Lambda `\|params\| -> Type { body }` | 完成 |
| struct/impl/operator 定义 | 完成 |
| import/from...import/as | 完成 |
| pub 导出 | 完成 |
| for-in 循环 | 完成 |
| 协程 yield | 完成 |
| 复合赋值 `+=` `-=` | ❌ |
| match 表达式 | ❌ |
| 模式匹配/解构 | ❌ |

**关键文件：**
- `crates/kaubo-compiler/src/parser/parser.rs` — 主解析器（~2247 行）
- `crates/kaubo-compiler/src/parser/error.rs` — `ParserError`（含位置信息）

## TypeChecker（类型检查）

| 项目 | 状态 |
|------|------|
| 字面量类型推导 | 完成 |
| 变量类型追踪 + 作用域 | 完成 |
| 类型兼容性检查 | 完成 |
| Lambda 类型检查 | 完成 |
| 函数调用参数检查 | 完成 |
| struct/impl 校验 | 完成 |
| Strict 模式 | 完成 |
| 接入编译主线 | ⚠️ 已实现但 CLI 默认不启用 |

**关键文件：**
- `crates/kaubo-compiler/src/parser/type_checker.rs` — TypeChecker（~968 行）

## Codegen（代码生成）

```
AST Module → Compiler::compile() → Chunk（字节码）
```

生成 146 种 OpCode 的字节码序列。

| 项目 | 状态 |
|------|------|
| 全部字面量编译 | 完成 |
| 全部二元/一元运算符 | 完成 |
| and/or 短路计算 | 完成 |
| if/elif/else | 完成 |
| while/for-in | 完成 |
| break/continue（含嵌套） | 完成 |
| Lambda/闭包 + upvalue | 完成 |
| 运算符重载 | 完成 |
| 内联缓存 | 完成 |
| import/模块导出 | 完成 |
| 协程 yield/resume | 完成 |

**关键文件：**
- `crates/kaubo-compiler/src/codegen/mod.rs` — 编译器结构 + 入口函数

## HIR（实验性）

AST → HIR → Chunk 的中间表示路径，用于优化。已实现框架但未接入主线。

**关键文件：**
- `crates/kaubo-compiler/src/hir/` — lowering, codegen, types
