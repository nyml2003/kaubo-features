# 语言总览

Kaubo 是一门静态类型的编译型脚本语言，设计目标是清晰的语法和可控的性能。

## 核心特性

| 特性 | 说明 |
|------|------|
| 静态类型 | 变量可以有类型标注，编译器执行类型检查 |
| 编译执行 | 源码 → 字节码 → 栈式 VM 解释执行 |
| Lambda/闭包 | 一等函数，支持 upvalue 捕获 |
| 结构体 | struct 定义 + impl 方法 + 运算符重载 |
| 列表 | 动态数组，内置 map/filter/reduce 方法 |
| 协程 | yield/resume 协作式多任务 |
| 模块系统 | 多文件 import/export，二进制格式 |
| JSON 字面量 | 内置 JSON 对象表示法 |

## 文件格式

| 扩展名 | 格式 |
|--------|------|
| `.kaubo` | 源代码 |
| `.kaubod` | 调试版二进制（无压缩） |
| `.kaubor` | 发布版二进制（可选压缩） |

## 编译流水线

```
源码.kaubo
  → Lexer（词法分析）
    → Token 流（39 种 token 类型）
  → Parser（语法分析）
    → AST Module（19 种表达式 + 14 种语句）
  → TypeChecker（类型检查）
    → 带类型的 AST
  → Codegen（代码生成）
    → Chunk（字节码，146 种 OpCode）
  → VM Runtime（解释执行）
    → 输出
```

## 文档导航

| 文档 | 内容 |
|------|------|
| [语法参考](./syntax.md) | 完整关键字、表达式、语句、注释语法 |
| [类型系统](./types.md) | int/float/string/bool/null/struct/json/list |
| [内置函数](./builtins.md) | print/sqrt/len/list.map 等 25+ 个内置函数 |
| [代码示例](./examples.md) | 8 个示例程序逐行解释 |
| [编程指南](./programming-guide.md) | 变量、函数、闭包、协程、模块 |
