# Kaubo 语法参考（v2026.02）

> 本文档描述当前实际实现的语法。状态：稳定，随时可能扩展。

## 词法元素

### 关键字

```
var, if, else, elif, while, for, return, in, yield
break, continue, struct, impl, import, as, from
module, pub, json
true, false, null
and, or, not
```

### 字面量

| 类型 | 示例 | 说明 |
|------|------|------|
| 整数 | `42`, `-10`, `0` | 32位有符号整数 |
| 浮点数 | `3.14`, `-0.5` | 64位浮点数 |
| 字符串 | `"hello"`, `'world'` | 支持 `"` 或 `'` 包裹 |
| 布尔 | `true`, `false` | 布尔值 |
| null | `null` | 空值 |

### 运算符

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
( )      // 圆括号
{ }      // 花括号（代码块、struct/impl 体）
[ ]      // 方括号（列表、索引）
;        // 语句结束
:        // 类型标注分隔符
,        // 列表分隔符
| |      // lambda 参数包裹
->       // 返回类型箭头
=>       // （保留）
```

---

## 语句（Statements）

### 变量声明

```kaubo
var x = 1;                    // 自动推导类型
var y: int = 2;               // 显式类型标注
var z: float = 1.0;           // 浮点数
var name: string = "kaubo";   // 字符串
var flag: bool = true;        // 布尔值

// pub 导出
pub var public_var = 42;
```

### 表达式语句

```kaubo
1 + 2;                        // 表达式作为语句
print("hello");               // 函数调用
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
// if 语句
if condition {
    // ...
}

// if-else
if condition {
    // ...
} else {
    // ...
}

// if-elif-else 链
if a > 0 {
    // ...
} elif a == 0 {
    // ...
} elif a < 0 {
    // ...
} else {
    // ...
}
```

### 循环语句

```kaubo
// while 循环
while condition {
    // ...
}

// for 循环（迭代列表）
for item in list {
    // ...
}

// for 循环（range）
for i in range(0, 10) {
    // ...
}

// break / continue
while true {
    if should_stop {
        break;
    }
    if should_skip {
        continue;
    }
}
```

### 返回语句

```kaubo
return;           // 无返回值
return value;     // 返回表达式值
```

### 结构体定义

```kaubo
struct Point {
    x: float,
    y: float
}

struct Person {
    name: string,
    age: int,
    tags: List<string>
}
```

### 方法实现

```kaubo
impl Point {
    // 方法：第一个参数是 self
    distance: |self, other: Point| -> float {
        var dx: float = self.x - other.x;
        var dy: float = self.y - other.y;
        return std.sqrt(dx * dx + dy * dy);
    },
    
    // 无参数方法
    to_string: |self| -> string {
        return "(" + self.x as string + ", " + self.y as string + ")";
    }
}
```

### 模块系统

```kaubo
// 定义模块
module math {
    pub var pi = 3.14159;
    
    pub add = |a: int, b: int| -> int {
        return a + b;
    }
}

// 导入整个模块
import math;
var x = math.pi;

// 导入并起别名
import math as m;
var y = m.add(1, 2);

// 从模块导入特定项
from std import sqrt, sin, cos;
var z = sqrt(16.0);
```

---

## 表达式（Expressions）

### 字面量表达式

```kaubo
42;               // 整数
3.14;             // 浮点数
"hello";          // 字符串
true;             // 布尔真
false;            // 布尔假
null;             // 空值
```

### 列表字面量

```kaubo
[];                           // 空列表
[1, 2, 3];                    // 整数列表
["a", "b", "c"];              // 字符串列表
[1, "mixed", true];           // 混合类型（推导为 List<any>）
```

### JSON 字面量

```kaubo
json { "name": "Alice", "age": 30 }
json { 
    "nested": { "x": 1, "y": 2 },
    "list": [1, 2, 3]
}
```

### 结构体实例化

```kaubo
var p = Point { x: 1.0, y: 2.0 };
var person = Person { 
    name: "Bob", 
    age: 25, 
    tags: ["dev", "rust"] 
};
```

### 变量引用

```kaubo
x;                // 变量 x
std;              // 标准库模块
```

### 二元运算

```kaubo
// 算术
a + b;
a - b;
a * b;
a / b;
a % b;

// 比较
a == b;
a != b;
a < b;
a > b;
a <= b;
a >= b;

// 逻辑
a and b;
a or b;

// 字符串拼接
"hello " + "world";  // "hello world"
```

### 一元运算

```kaubo
-x;               // 负号
not x;            // 逻辑非
```

### 括号表达式

```kaubo
(a + b) * c;      // 改变优先级
```

### 函数调用

```kaubo
// 普通调用
print("hello");
add(1, 2);

// 链式调用
list.len();

// 嵌套调用
std.sqrt(add(9, 16) as float);
```

### Lambda（匿名函数）

```kaubo
// 基本形式
|x| x + 1;

// 多参数
|a, b| a + b;

// 带参数类型标注
|x: int, y: int| x + y;

// 带返回类型
|x: int| -> int { x * 2 };

// 多行函数体
|a: float, b: float| -> float {
    var sum = a + b;
    return sum / 2.0;
};

// 赋值给变量
var add = |a: int, b: int| -> int {
    return a + b;
};
```

### 成员访问

```kaubo
// 对象属性
point.x;
point.y;

// 模块函数
std.sqrt(16.0);
std.sin(angle);
```

### 索引访问

```kaubo
list[0];          // 第一个元素
list[i];          // 变量索引
list[len - 1];    // 表达式索引
```

### 类型转换

```kaubo
42 as float;              // int -> float
3.14 as int;              // float -> int（截断）
42 as string;             // int -> string
3.14 as string;           // float -> string
true as string;           // bool -> string
```

### Yield（协程）

```kaubo
// 生成器函数
var counter = || {
    var i = 0;
    while true {
        yield i;
        i = i + 1;
    }
};

// 使用
var gen = counter();
var value = gen.next();  // 0
```

---

## 类型系统

### 基础类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `int` | 32位整数 | `42` |
| `float` | 64位浮点数 | `3.14` |
| `bool` | 布尔值 | `true` |
| `string` | 字符串 | `"hello"` |
| `any` | 顶层类型 | - |
| `void` | 无返回值 | - |

### 复合类型

```kaubo
List<int>                     // 整数列表
List<string>                  // 字符串列表
List<any>                     // 任意类型列表
Tuple<int, string>            // 元组
|int, int| -> int             // 函数类型（参数 -> 返回）
|int| -> void                 // 无返回值函数
```

### 类型标注位置

```kaubo
// 变量声明
var x: int = 42;

// 函数参数
|x: int, y: float| -> string { ... }

// 返回类型
var add: |int, int| -> int = |a, b| a + b;

// Struct 字段
struct Point { x: float, y: float }
```

---

## 标准库（std）

### 数学函数

```kaubo
std.sqrt(x: float) -> float       // 平方根
std.sin(x: float) -> float        // 正弦
std.cos(x: float) -> float        // 余弦
std.floor(x: float) -> float      // 向下取整
std.ceil(x: float) -> float       // 向上取整
std.pi                            // 圆周率常量
std.e                             // 自然对数底
```

### 实用函数

```kaubo
print(value: any) -> void         // 打印
assert(cond: bool, msg?: string)  // 断言
type(value: any) -> string        // 获取类型名
to_string(value: any) -> string   // 转为字符串
len(container: any) -> int        // 获取长度
range(start: int, end: int) -> List<int>  // 生成范围
clone(value: any) -> any          // 深拷贝
```

### 列表方法

```kaubo
list.len() -> int
list.append(item: any) -> void
list.remove(index: int) -> any
list.clear() -> void
list.is_empty() -> bool
```

### 字符串方法

```kaubo
str.len() -> int
str.substring(start: int, end: int) -> string
str.contains(substr: string) -> bool
str.starts_with(prefix: string) -> bool
str.ends_with(suffix: string) -> bool
```

### 文件操作

```kaubo
std.read_file(path: string) -> string
std.write_file(path: string, content: string) -> void
std.exists(path: string) -> bool
std.is_file(path: string) -> bool
std.is_dir(path: string) -> bool
```

---

## 完整示例

```kaubo
// 计算两点距离
struct Point {
    x: float,
    y: float
}

impl Point {
    distance: |self, other: Point| -> float {
        var dx = self.x - other.x;
        var dy = self.y - other.y;
        return std.sqrt(dx * dx + dy * dy);
    }
}

var p1 = Point { x: 0.0, y: 0.0 };
var p2 = Point { x: 3.0, y: 4.0 };

print(p1.distance(p2));  // 5.0

// 列表操作
var numbers = [1, 2, 3, 4, 5];
var doubled = numbers.map(|n| n * 2);

// 类型转换
var avg = (1 + 2 + 3) as float / 3.0;
print("Average: " + avg as string);

// 协程生成器
var fib = || {
    var a = 0;
    var b = 1;
    while true {
        yield a;
        var temp = a + b;
        a = b;
        b = temp;
    }
};

var fib_gen = fib();
print(fib_gen.next());  // 0
print(fib_gen.next());  // 1
print(fib_gen.next());  // 1
print(fib_gen.next());  // 2
```

---

## 语法变更记录

| 日期 | 变更 |
|------|------|
| 2025-02-14 | 初始语法设计 |
| 2026-02-14 | 添加 `as` 类型转换、impl 方法、yield 协程、json 字面量 |
