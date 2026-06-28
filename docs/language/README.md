# 语言参考

目标读者：使用 Kaubo 语言写代码的开发者。

语言参考按语法特性组织，每篇文档区分三类状态：

- **主路径可运行**：parser、infer、lowering 和 VM/driver 主路径已有覆盖。
- **部分实现**：能 parse 或能 infer 一部分，但 lowering/runtime 不完整。
- **语法表面**：AST、编辑器或 snippet 中出现，但不能当作可运行语言能力。

VSCode TextMate grammar 和 snippets 只用于编辑器体验，不是语法规范。遇到差异时，以 `kaubo-token`、`kaubo-syntax`、`kaubo-infer`、`kaubo-ir` 和 `kaubo-vm` 的主路径为准。

## 阅读顺序

1. [词法](01-lexical.md)
2. [文件和语句](02-statements.md)
3. [类型语法](03-types.md)
4. [表达式](04-expressions.md)
5. [函数和 Lambda](05-functions.md)
6. [控制流](06-control-flow.md)
7. [Struct 和 Impl](07-structs-and-impls.md)
8. [模块语法](08-modules.md)
9. [标准库和内建方法](09-builtins.md)
10. [部分实现的语法表面](10-partial-features.md)
11. [边界例子](11-edge-cases.md)
12. [扩展特性状态](xx-extensions.md)

## 当前主路径能力

当前相对稳定的可运行路径包括：

- 基础字面量：整数、浮点数、字符串、布尔值、`null`。
- 元组字面量和类型：`()`、`(1,)`、`(1, "a")`，类型 `(Int64, String)`。
- 模板字符串：`` `hello {name}` ``。
- lambda（含单表达式简写）、函数调用、`return`。
- `if/else`、`while`、`break`、`continue`。
- `match` 表达式（常量匹配 + 通配符）。
- `??` null 合并、`?.` `?[` 可选链。
- 基础算术和比较、字符串拼接。
- struct 声明、struct literal（含简写属性 `{x, y}` 和 spread `{...p, y:3}`）。
- impl 方法和 member 方法调用。
- interface 定义 + operator 重载 + dyn Trait。
- 模块系统（import/export、跨文件编译）。
- enum 声明、单元变体构造、带字段变体构造和 match 解构。
- `print`、`assert`、math built-ins、`to_string`、`to_float`。

## 当前需要谨慎的区域

- list/index、`for`、`and/or`、pipe、`>>`、async/await 等不应写成完整可运行能力。
- `type_of` 有类型层占位，但 lowering/runtime 明确未实现。
- 比较运算（`== != < > <= >=`）要求操作数类型统一，`1 == 2.0` 会报错。
- VM 采用统一寄存器组（`Vec<u64>`，JVM 风格），操作码决定值的解释方式。
- 模块系统已完整实现（import/export、跨文件类型推断、CPS 链接），详见 [08-modules](08-modules.md)。

## 其他文档

- [路线图](../roadmap.md)
- [架构设计](../architecture/README.md)
- [运维指南](../operations/README.md)
