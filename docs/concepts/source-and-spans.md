# 源码与 Span

源码身份和文本位置是编译器概念，不是展示便利字段。编译器内核必须从词法分析一直到运行时诊断都保留稳定位置。展示层可以转换这些位置，但不能发明这些位置。

## 概念

- `SourceText`：一次编译输入对应的不可变源码内容。
- `SourceId`：一次编译会话中某个 `SourceText` 的稳定标识。它由编译会话分配，用来跟踪同一份源码，不表示 lexer 的内部状态。
- `ModuleId`：import 解析之后，一个逻辑 Kaubo 模块的稳定标识。
- `SourceMap`：从 `SourceId` 映射到源码文本、文件路径或虚拟 URI、行起点，以及编码转换辅助信息。
- `TextRange`：源码中的规范半开区间，使用字节偏移 `[start, end)` 存储。
- `ByteSpan`：绑定到某个 `SourceId` 的 `TextRange`。
- `LineCol`：面向人类展示的派生坐标，不是规范位置。
- `Utf16Range`：面向 UTF-16 文本 API 的派生坐标。

## 规则

- 编译器内核使用 `SourceId + TextRange` 作为规范位置。
- `LineCol` 和 `Utf16Range` 只在展示边界通过 `SourceMap` 派生。
- token、AST 节点、类型错误、IR 诊断和运行时 trap 只要有源码位置，就必须携带 source span。
- 允许空区间表示插入点诊断，但展示层在 UI 展示时可以按需要扩展。
- 缺失 span 表示“没有已知源码位置”，不能偷换成“第 1 行第 1 列”。
- 字符串 token 的文本值和源码范围必须分开。处理后的字符串值不能用于计算源码 offset。

## 当前差距

当前 lexer 在每个 token 上存储 `line`、`col` 和处理后的 `lexeme`。展示相关代码又在 lexer 外部重算 UTF-16 offset。这很脆弱：带引号的字符串、转义、注释、多字节文本、语法错误恢复，都需要不依赖“从处理后 token 值反推源码长度”的 source range。

目标形态：

```text
SourceText
  -> Token { kind, source_range, text/value }
  -> AST node { source_range, children/value }
  -> Diagnostic { source_id, range, phase, code, message }
  -> 展示边界通过 SourceMap 转换 range
```

`SourceId` 由编译任务或源码管理边界附着到产物和诊断上，不是 lexer 的内部状态。

## 坐标转换

规范 range 可以在展示边界转换成不同坐标系统：

- 行列号和源码片段；
- UTF-16 offset；
- 外部编辑器或传输协议需要的 range。

传输数据可以同时包含规范字节 range 和预计算派生 range，以降低展示层复杂度；但规范 range 仍然是唯一事实来源。
