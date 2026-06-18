# 诊断模型

诊断是描述编译器和运行时问题的结构化契约。它必须足够稳定，让不同展示和传输边界能一致展示同一个问题，而不重复实现编译器逻辑。

## 诊断类别

- `CompilerDiagnostic`：由 lexer、parser、类型推断、lowering 或优化阶段产生。
- `RuntimeDiagnostic`：由 VM 加载或执行阶段产生。
- `InfrastructureError`：由文件系统、模块解析、序列化或传输边界产生。
- `InternalBug`：内部不变量被破坏。它必须显式、可审计，不能静默伪装成普通用户错误。

## 必需字段

每个 diagnostic 必须定义这些字段：

- `severity`：`error`、`warning`、`info` 或 `hint`。
- `code`：稳定、机器可读的错误码，例如 `syntax.expected-token`。
- `phase`：稳定的问题来源标识，例如 `lexer`、`parser`、`infer`、`cps`、`optimizer`、`vm-load`、`vm-execute`。
- `message`：简洁的人类可读摘要。
- `source_id`：只有当诊断没有源码位置时才可以为空。
- `range`：可选规范 `TextRange`；存在时是半开区间。
- `related`：可选的关联源码范围和消息列表。
- `notes`：面向富展示器的可选补充说明。

展示层或传输层可以添加行列号、源码片段、UTF-16 range 等展示字段，但不能删除或重新解释内核字段。

## 流程

```text
发生问题
  -> 产生结构化 diagnostic
  -> 按展示环境转换坐标和格式
```

diagnostic 是数据模型，不决定编译是否继续，也不决定具体 UI 怎么渲染。控制流策略和展示方式都属于诊断模型之外的职责。

## 运行时诊断

运行时失败在存在源码信息时也使用同一套诊断模型。例如：

- 除零在 IR 保留 origin data 时映射到对应表达式 span；
- index out of bounds 映射到索引表达式 span；
- 没有源码 origin 的 VM load 错误使用 `source_id = none` 和 phase `vm-load`；
- opcode 或 register 内部不变量错误使用 `InternalBug`。

## 展示边界

展示边界可以把诊断转换成终端文本、编辑器标记、问题列表或传输数据。转换只能增加展示字段，不能改变 `severity`、`code`、`phase`、`source_id`、`range` 或 `related` 的语义。

测试优先断言结构化字段，其次断言渲染字符串。

## 当前差距

当前展示相关 diagnostic 路径存在手工拼接传输数据、混用行号、offset 和消息的问题。目标是所有应用边界共享同一个结构化 diagnostic 模型。
