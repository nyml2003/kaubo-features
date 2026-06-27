# 类型语法

## 基础类型

当前类型推断和常见示例覆盖这些基础类型名：

```text
Int64
Float64
String
Bool
Null
```

示例：

```kaubo
const x: Int64 = 42;
const y: Float64 = 1.5;
const s: String = "hi";
const ok: Bool = true;
```

## Struct 类型

struct 声明会把类型名注册到推断和 lowering 路径：

```kaubo
struct Point {
  x: Int64,
  y: Int64,
};

const p: Point = Point { x: 1, y: 2 };
```

struct 名当前由普通标识符承载，大小写不是 parser 的硬性要求。

## Enum 类型（规划中，未实现）

`enum` 声明的设计目标为代数数据类型，变体可以是单元变体或带字段变体：

```kaubo
// 以下为设计语法，当前不可运行：
enum Color { Red, Green };
enum Option { Some(value: Int64), None };
```

当前状态：lexer 未识别 `enum` 关键字，parser/infer/CPS/VM 均无对应实现。
详见 [扩展规划](xx-extensions.md) 中的 Enum/ADT 条目。

旧设计说明（保留供参考）：
- 单元变体（无字段）作为常量使用：`const c = Red;`
- 带字段变体作为构造器调用：`const x = Some(42);`
- 变体在运行时表示为带 tag 的堆对象，不同变体可区分
- 变体名称在同一个程序中必须唯一（当前不支持跨 enum 的同名变体）

## List<T>

类型语法支持 `List<T>`：

```kaubo
var xs: List<Int64>;
```

但 list literal 和 index 的 runtime 主路径不完整，不能把 `List<T>` 当作完整集合能力使用。详见 [部分实现的语法表面](10-partial-features.md)。

## 函数类型

AST 中存在函数类型表达式，内部字符串形态类似：

```text
|Int64, String| -> Bool
```

当前源码 parser 的显式类型解析主要覆盖命名类型和 `List<T>`；lambda 的参数和返回值可以直接写类型标注：

```kaubo
const add = |a: Int64, b: Int64| -> Int64 {
  return a + b;
};
```

## 类型标注位置

当前常用标注位置：

```kaubo
const x: Int64 = 42;
var y: Float64 = 1.0;
const f = |x: Int64| -> Int64 { x + 1 };
```
