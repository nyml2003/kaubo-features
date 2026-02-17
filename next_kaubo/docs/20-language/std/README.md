# Kaubo 标准库（Std）

> Kaubo 内置的标准库函数和模块。

---

## 标准库结构

当前标准库通过 `std` 模块提供：

```kaubo
import std;

std.sqrt(16.0);
std.print("hello");
```

---

## 核心函数

### 数学函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `std.sqrt` | `\|float\| -> float` | 平方根 |
| `std.sin` | `\|float\| -> float` | 正弦 |
| `std.cos` | `\|float\| -> float` | 余弦 |
| `std.floor` | `\|float\| -> float` | 向下取整 |
| `std.ceil` | `\|float\| -> float` | 向上取整 |

### 实用函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `print` | `\|any\| -> void` | 打印输出 |
| `assert` | `\|bool, string?\| -> void` | 断言 |
| `type` | `\|any\| -> string` | 获取类型名 |
| `len` | `\|any\| -> int` | 获取长度 |
| `range` | `\|int, int\| -> List<int>` | 生成范围 |

---

## 未来规划

标准库将改造为插件化架构（独立 `kaubo-std` crate），详见 [设计文档](../../30-implementation/design/module-system.md)。

---

*最后更新：2026-02-17*
