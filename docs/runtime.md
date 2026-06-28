# 运行时

目标读者：维护执行正确性、lowering、bytecode 或 VM 行为的开发者。

## 当前状态

运行时路径由 `kaubo-driver` 编排。它把源码编译为 `CpsModule`，可选地进行编码/解码，然后用 `kaubo-vm` 执行。

VM 是寄存器 VM，包含独立的整数寄存器和浮点寄存器，并通过堆保存字符串、列表、结构体和闭包。部分值目前以原始 `i64` payload 表示，包括堆 handle 和浮点 bit pattern。

## 编译和执行流程

```text
source
  -> Parser::parse
  -> infer_module
  -> build_module
  -> flatten_module
  -> ConstantFold
  -> VM::load
  -> VM::execute
```

当前 CLI 和 WASM 导出都通过 `kaubo-driver` 使用这条直接路径。

## 重要运行时类型

- `kaubo_driver::RunOutcome`：返回值和捕获到的 `print()` 输出。
- `kaubo_driver::DriverError`：parse、infer、build、decode、load、runtime 错误分类。
- `kaubo_cps::CpsModule`：传给 VM 的可执行 module。
- `kaubo_vm::VM`：包含寄存器、堆、native 函数和输出的执行器。
- `kaubo_vm::HeapObj`：字符串、列表、结构体和闭包的堆表示。

## 当前 Value Model

VM 当前包含：

- 统一寄存器组（`regs: Vec<u64>`，JVM 风格）：操作码决定值的解释方式（整数、浮点 bit pattern、堆 handle、布尔值）。
- heap slots：用于字符串、列表、结构体、闭包和 interface obj。

这个模型已替代了早期的 `ints[]/floats[]` 双数组方案。

## 已知风险区域

以下是当前工程风险，不是期望语义：

- 部分 lowering 路径历史上会在 symbol、struct 或 field 无法解析时使用 `0` 之类的 fallback。
- VM 操作应优先返回显式 runtime error，而不是 silent no-op。
- Native 函数返回原始 `i64` payload，因此参数和结果编码必须与 lowering 保持一致。
- 浮点操作和转换需要仔细同步 int bits 与 float registers。
- 结构体字段访问应该从对象类型解析，而不是全局按字段名搜索。

## Runtime 测试策略

执行行为应该根据层级放到 driver 或 VM 测试里：

- 源码到输出的回归测试放 driver。
- lowering 形状测试放 IR/CPS。
- 指令级行为测试放 VM。

修 bug 时应先增加一个失败回归，用它描述可观察行为。例如 `Point.dis` 回归应该放在 driver 层，因为它同时覆盖 parser、infer、lowering、native `sqrt`、VM 浮点、方法调用和字符串转换。
