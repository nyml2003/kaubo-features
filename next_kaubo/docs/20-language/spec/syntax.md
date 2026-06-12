# Kaubo 语法参考 (MVP)

> 本文档描述当前实际实现的语法。版本：v0.1.0

---

## 关键字

```
var, if, else, elif, while, for, return, in, yield
break, continue, struct, impl, import, as, from, pass
module, pub, json
true, false, null
and, or, not
```

---

## 字面量

| 类型 | 示例 | 说明 |
|------|------|------|
| 整数 | `42`, `-10`, `0` | 32 位有符号整数 |
| 浮点数 | `3.14`, `-0.5` | 64 位浮点数 |
| 字符串 | `"hello"` | 双引号包裹 |
| 布尔 | `true`, `false` | 布尔值 |
| null | `null` | 空值 |

---

## 运算符

| 优先级 | 运算符 | 说明 | 结合性 |
|--------|--------|------|--------|
| 1 | `()` `[]` `.` | 调用、索引、成员访问 | 左 |
| 2 | `-` `not` | 负号、逻辑非 | 右 |
| 3 | `*` `/` `%` | 乘除模 | 左 |
| 4 | `+` `-` | 加减 | 左 |
| 5 | `<` `>` `<=` `>=` | 比较 | 左 |
| 6 | `==` `!=` | 等于、不等于 | 左 |
| 7 | `and` | 逻辑与 | 左 |
| 8 | `or` | 逻辑或 | 左 |
| 9 | `as` | 类型转换 | 左 |
| 10 | `=` | 赋值 | 右 |

### 分隔符

```
( )     圆括号
{ }     花括号（代码块、struct/impl 体）
[ ]     方括号（列表、索引）
;       语句结束
:       类型标注分隔符
,       列表分隔符
| |     lambda 参数包裹
->      返回类型箭头
```

---

## 语句

### 变量声明

```kaubo
var x = 1;
var y: int = 2;
pub var z = 42;         // 导出到模块
```

### 表达式语句

```kaubo
1 + 2;
print("hello");
```

### 占位语句

```kaubo
pass;                   // 空操作
```

### 代码块

```kaubo
{
    var x = 1;
    var y = 2;
    x + y;
}
```

### 条件语句

```kaubo
if condition {
    // ...
}

if condition {
    // ...
} else {
    // ...
}

if a > 0 {
    // ...
} elif a == 0 {
    // ...
} else {
    // ...
}
```

### 循环语句

```kaubo
// while
while condition {
    if done { break; }
    if skip { continue; }
}

// for-in
for item in list {
    print(item);
}

// for with range
for i in range(0, 10) {
    print(i);
}
```

### 返回语句

```kaubo
return;
return value;
```

### 结构体定义

```kaubo
struct Point {
    x: float,
    y: float
}

var p = Point { x: 1.0, y: 2.0 };
print(p.x);  // 1.0
```

### 方法实现

```kaubo
impl Point {
    distance: |self: Point, other: Point| -> float {
        var dx = self.x - other.x;
        var dy = self.y - other.y;
        return std.sqrt(dx * dx + dy * dy);
    }
}

p.distance(p2);
```

### 运算符重载

```kaubo
impl Counter {
    operator add: |self: Counter, other: Counter| -> Counter {
        return Counter { value: self.value + other.value };
    }
}
```

### 模块导入

```kaubo
import math;
var x = math.pi;

import math as m;
var y = m.sqrt(16.0);

from math import add;
from math import add, sub;
```

### Print

```kaubo
print("hello");
print(x);
print("x = " + x as string);
```

---

## 表达式

### 列表

```kaubo
[];
[1, 2, 3];
["a", "b", "c"];
[1, "mixed", true];     // List[any]
```

### JSON

```kaubo
json { "name": "Alice", "age": 30 }
json { "nested": { "x": 1 }, "list": [1, 2] }
```

### 结构体实例化

```kaubo
Point { x: 1.0, y: 2.0 }
Person { name: "Bob", age: 25 }
```

### 二元运算

```kaubo
a + b; a - b; a * b; a / b; a % b;
a == b; a != b; a < b; a > b; a <= b; a >= b;
a and b; a or b;
"hello " + "world";     // 字符串拼接
```

### 一元运算

```kaubo
-x;
not x;
```

### 函数调用

```kaubo
print("hello");
add(1, 2);
list.len();
std.sqrt(16.0);
```

### Lambda（匿名函数）

```kaubo
|x| { return x + 1; };
|x, y| { return x + y; };
|x: int, y: int| -> int { return x + y; };

var average = |a: float, b: float| -> float {
    var sum = a + b;
    return sum / 2.0;
};
```

### 成员访问

```kaubo
point.x;
std.sqrt(16.0);
```

### 索引访问

```kaubo
list[0];
list[i];
list[len - 1];
```

### 类型转换

```kaubo
42 as float;             // int -> float
3.14 as int;             // float -> int
42 as string;            // int -> string
true as string;          // bool -> string
```

### Yield（协程）

```kaubo
|x| {
    var i = 0;
    while true {
        yield i;
        i = i + 1;
    }
};
```

---

## 类型系统

### 基础类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `int` | 32 位整数 | `42` |
| `float` | 64 位浮点数 | `3.14` |
| `bool` | 布尔值 | `true` |
| `string` | 字符串 | `"hello"` |
| `any` | 顶层类型 | - |

### 复合类型

```kaubo
List[int]
List[string]
List[any]
|int, int| -> int       // 函数类型
|int| -> void           // 无返回值
```

### 类型标注

```kaubo
var x: int = 42;
|x: int, y: float| -> string { ... }
struct Point { x: float, y: float }
```

---

## 标准库

### 核心函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `print` | `(any) -> void` | 打印到标准输出 |
| `assert` | `(bool, string?) -> void` | 断言 |
| `type` | `(any) -> string` | 获取类型名 |
| `to_string` | `(any) -> string` | 转为字符串 |

### 数学

| 函数 | 签名 | 说明 |
|------|------|------|
| `sqrt` | `(float) -> float` | 平方根 |
| `sin` | `(float) -> float` | 正弦 |
| `cos` | `(float) -> float` | 余弦 |
| `floor` | `(float) -> float` | 向下取整 |
| `ceil` | `(float) -> float` | 向上取整 |
| `PI` | `float` | 圆周率常量 |
| `E` | `float` | 自然对数底 |

### 列表

| 函数 | 签名 | 说明 |
|------|------|------|
| `len` | `(any) -> int` | 列表/字符串/JSON 长度 |
| `push` | `(list, any) -> list` | 追加元素（返回新列表） |
| `is_empty` | `(any) -> bool` | 是否为空 |

| 方法 | 说明 |
|------|------|
| `list.push(x)` | 追加 |
| `list.len()` | 长度 |
| `list.remove(i)` | 移除 |
| `list.clear()` | 清空 |
| `list.is_empty()` | 判空 |
| `list.foreach(f)` | 遍历 |
| `list.map(f)` | 映射 |
| `list.filter(f)` | 过滤 |
| `list.reduce(f, init)` | 归约 |
| `list.find(f)` | 查找 |
| `list.any(f)` | 任一满足 |
| `list.all(f)` | 全部满足 |

### 字符串方法

| 方法 | 说明 |
|------|------|
| `str.len()` | 长度 |
| `str.is_empty()` | 判空 |

### 字符串函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `substring` | `(string, int, int) -> string` | 子串截取 |
| `contains` | `(string, string) -> bool` | 是否包含 |
| `starts_with` | `(string, string) -> bool` | 前缀匹配 |
| `ends_with` | `(string, string) -> bool` | 后缀匹配 |

### 环境与时间

| 函数 | 签名 | 说明 |
|------|------|------|
| `env` | `(string) -> string` | 获取环境变量 |
| `now` | `() -> float` | Unix 时间戳（秒） |

### 工具

| 函数 | 签名 | 说明 |
|------|------|------|
| `range` | `(int, int?, int?) -> List[int]` | 生成整数范围 |
| `clone` | `(any) -> any` | 浅拷贝 |

### 文件 I/O

| 函数 | 签名 | 说明 |
|------|------|------|
| `read_file` | `(string) -> string` | 读取文件 |
| `write_file` | `(string, string) -> void` | 写入文件 |
| `exists` | `(string) -> bool` | 路径是否存在 |
| `is_file` | `(string) -> bool` | 是否为文件 |
| `is_dir` | `(string) -> bool` | 是否为目录 |

### 协程

| 函数 | 签名 | 说明 |
|------|------|------|
| `create_coroutine` | `(closure) -> coroutine` | 创建协程 |
| `resume` | `(coroutine, ...args) -> any` | 恢复协程 |
| `coroutine_status` | `(coroutine) -> int` | 状态 (0/1/2) |

### JSON 方法

| 方法 | 说明 |
|------|------|
| `json.len()` | 属性数量 |
| `json.is_empty()` | 判空 |

---

## 语法变更记录

| 日期 | 变更 |
|------|------|
| 2025-02-14 | 初始语法设计 |
| 2026-02-14 | 添加 `as` 类型转换、impl 方法、yield 协程、json 字面量 |
| 2026-06-11 | MVP 收敛：删除 `interface`/`async`/`await`/`val`/`runtime`/`cfg`，精简为实际实现的语法 |
| 2026-06-12 | 实现 `break`/`continue`/`pass`；新增 std: `substring`/`contains`/`starts_with`/`ends_with`/`env`/`now`；修复闭包捕获 |
