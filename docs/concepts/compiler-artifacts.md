# 编译产物

编译产物是编译过程中产生的类型化数据。它们定义了每一步语义转换的结果，以及后续语义处理可以依赖的数据形态。

## 产物序列

```text
SourceText
  -> TokenStream
  -> AST
  -> TypeTable or TypedAST
  -> CPS IR
  -> Optimized CPS IR
  -> Runtime Program
  -> RunResult
```

不是每个命令都必须物化所有产物，但每个编译边界都应该能映射到这个成功路径。这里描述的是编译成功路径；diagnostics、日志、事件和 trace 不是核心 artifact 的一部分。

## SourceText

`SourceText` 在一次编译会话中不可变。GUI 中的编辑会产生新的 source input。未来增量编译可以复用缓存产物，但复用必须显式，并通过源码身份和内容失效。

## TokenStream

`TokenStream` 是 lexer 输出。它把 token kind、source range、raw/cooked value 作为不同字段保存。parser 可以忽略 trivia，而工具可以请求 trivia 用于高亮和格式化。

## ParseEvents 或 CST

项目应该区分 parser 机制和 AST 语义。CST 或 parse-event stream 可以为工具保留注释、错误和不完整语法，但它不应该作为语义 parser 的第二个成功输出。需要这类信息时，应设计独立 tooling 流程或 parser 模式。AST 仍然是后续语义处理的输入。

## AST

`AST` 表示解析后的语言结构。它包含 source range，但不包含类型推断决策。它可以包含语法层面的类型标注，因为那是源码语法的一部分。

## 类型信息

类型推断输出应该是独立产物：

- `TypeTable`：把 AST node id 或 symbol id 映射到推断类型；
- `TypedAST`：把 AST 结构和类型标注组合起来。

后续语义处理不能从语法重新计算类型决策。

## CPS IR

`CPS IR` 是核心编译器 IR。它应该足够稳定，让测试可以直接构造小模块，而不必调用 lexer/parser/infer。这能避免 IR 和 VM 测试依赖前序编译步骤。

## Optimized CPS IR

优化是 `CPS IR in -> CPS IR out`。optimizer 内部可以使用结构共享，但公共契约必须返回新产物。这会让 pass 更容易测试、比较和审计。

## Runtime Program

`Runtime Program` 是面向 VM 的产物。它和 CPS 不是同一个概念。它可以是解释执行的结构化程序、编码后的 bytecode stream，或者两者都有；但编码格式和 VM 执行语义必须分别命名、分别测试。

## RunResult

`RunResult` 包含进程级输出、返回值、diagnostics 或 trap，以及可选 timing/debug 数据。展示层不应该为了拿运行结果而检查 VM 内部字段，例如 register 或 output buffer。
