# 语言参考

目标读者：需要了解当前实现语言子集的维护者和使用者。

## 当前状态

这是一份当前实现参考，不是最终语言规范。标记为受限或实验性的能力可能已经能 parse，但 runtime、inference 或 editor 支持仍不完整。

## 文件和语句

源码文件会被解析成包含多条语句的 module。常见顶层形式：

```kaubo
const answer = 42;
var count = 0;
```

声明和表达式语句之间通常使用分号。

## 基础值

当前已覆盖的基础值包括：

```kaubo
42
1.5
"hello"
true
false
null
```

常见内建类型名包括：

```kaubo
Int64
Float64
String
Bool
Null
List
```

## 变量和常量

```kaubo
const x = 40 + 2;
var n = 0;
n = n + 1;
```

常见声明和参数位置支持类型标注：

```kaubo
const x: Int64 = 42;
```

## 函数和 Lambda

当前函数主要通过绑定到名称的 lambda 表达式表示：

```kaubo
const add_one = |x: Int64| -> Int64 {
  return x + 1;
};

add_one(41);
```

当前 driver 已经有 lambda call 回归覆盖。

## 控制流

条件表达式：

```kaubo
const x = if true { 42 } else { 0 };
```

循环：

```kaubo
var n = 0;
while n < 3 {
  n = n + 1;
};
n;
```

`break` 和 `continue` 存在于 AST/lowering 表面，但扩展高级循环行为前，需要先补足回归测试。

## Struct

Struct 声明：

```kaubo
struct Point {
  x: Int64,
  y: Int64,
};
```

Struct literal：

```kaubo
const p = Point { x: 200, y: 300 };
```

字段访问：

```kaubo
p.x;
```

字段解析是重要正确性区域。长期行为应该由类型驱动，而不是基于字段名 heuristic。

## Impl 方法

方法可以声明在 `impl` block 中：

```kaubo
impl Point {
  dis: |self: Point, other: Point| -> Float64 {
    const dx = self.x - other.x;
    const dy = self.y - other.y;
    return sqrt((dx * dx + dy * dy).to_float());
  }
};
```

方法调用使用 member syntax：

```kaubo
p1.dis(p2);
```

## Built-ins

当前 runtime 覆盖的 built-ins 包括：

- `print`
- `assert`
- `sqrt`
- `sin`
- `cos`
- `floor`
- `ceil`

部分编辑器补全可能列出更多规划中的 built-ins。在 runtime 和 inference 覆盖完成前，应把那些名字视作 UI hint。

## 示例

```kaubo
struct Point {
  x: Int64,
  y: Int64,
};

impl Point {
  dis: |self: Point, other: Point| -> Float64 {
    const dx = self.x - other.x;
    const dy = self.y - other.y;
    return sqrt((dx * dx + dy * dy).to_float()) + 1.0;
  }
};

const p1 = Point { x: 200, y: 300 };
const p2 = Point { x: 300, y: 400 };
print(p1.dis(p2).to_string());
```

当前期望打印值以以下内容开头：

```text
142.421
```
