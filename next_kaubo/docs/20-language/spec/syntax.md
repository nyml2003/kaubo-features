# Kaubo 语法参考（v2026.02）

> 本文档描述当前实际实现的语法。状态：稳定，随时可能扩展。

## 核心设计理念

> **默认静态，显式动态。**
> 
> 类似 Rust `mut` 默认不可变：  
> - `let x = 5;`      // 不可变  
> - `let mut x = 5;`  // 可变  
>
> Kaubo 默认编译期确定：  
> - `var x = 5;`           // 编译期常量  
> - `runtime var x = 5;`   // 运行时变量

---

## 词法元素

### 关键字

```
var, val, runtime, if, else, elif, while, for, return, in, yield
break, continue, struct, impl, import, as, from
module, pub, json, cfg
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
// 默认编译期变量（值必须在编译期可确定）
var x = 1;                    // x 是编译期常量
var y: int = 2;               // 显式类型标注
val z = 3.14;                 // val 也是编译期常量

// 运行时变量（显式标记）
runtime var user_input = read_line();   // 运行时才能确定
runtime var timestamp = std.now();      // 运行时获取时间

// 编译期变量可以用编译期表达式初始化
var size = 100;
var double_size = size * 2;     // 编译期计算

// 编译期变量不能用运行时值初始化
runtime var dynamic = get_random();  // OK
var static_val = dynamic;            // ERROR: 不能用运行时值初始化编译期变量
```

**规则：**
- `var` / `val`：编译期变量，值必须在编译期可确定
- `runtime var` / `runtime val`：运行时变量，值在运行时确定
- 习惯上编译期常量用大写（`MAX_SIZE`），运行时变量用小写（`user_input`）

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

// 编译期条件（条件是编译期常量，分支可能完全消除）
if (cfg.DEBUG) {
    // 只在 debug 模式编译的代码
}

// 运行时条件（显式标记）
runtime if (user_input == "yes") {
    // 运行时决定的代码路径
}

// if 表达式（返回值）
var log_level = if (cfg.DEBUG) { "verbose" } else { "error" };
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
// 编译期结构体（默认）
// 所有字段必须是编译期可确定的
struct Point {
    x: float,
    y: float
}

struct Config {
    max_size: int,
    debug: bool
}

// 使用编译期常量定义数组大小
val SIZE = cfg.BUFFER_SIZE;
struct Buffer {
    data: [int; SIZE],    // 编译期确定的数组大小
    len: int
}

// 运行时结构体（显式标记）
runtime struct UserSession {
    id: string,                 // 字段可以是运行时值
    created_at: int,
    token: string
}

// 编译期结构体不能包含运行时字段
struct BadConfig {
    value: int,
    timestamp: std.now()        // ERROR：std.now() 是运行时函数
}
```

### 方法实现

```kaubo
// 编译期 impl（默认）
impl Point {
    // 方法必须是编译期函数
    distance: |self: Point, other: Point| -> float {
        var dx = self.x - other.x;
        var dy = self.y - other.y;
        return std.sqrt(dx * dx + dy * dy);
    }
}

// 运行时 impl（显式标记）
runtime impl HttpServer {
    handle: |self: HttpServer, req: HttpRequest| -> HttpResponse {
        runtime var result = process(req);   // 可以调用运行时函数
        return result;
    }
}
```

### 模块系统

```kaubo
// 编译期模块（默认）
module math {
    pub val pi = 3.14159;
    
    pub val add = |a: int, b: int| -> int {
        return a + b;
    }
}

// 运行时模块（显式标记）
runtime module io {
    pub runtime val read_file = |path: string| -> string {
        return std.read_file(path);     // 可以 IO
    };
}

// 导入
import math;
var x = math.pi;

// 条件导入（编译期 if）
if (cfg.ENABLE_NETWORKING) {
    import http;
    
    runtime val fetch = |url: string| -> string {
        return http.get(url);
    };
}
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

// 使用编译期常量定义大小
val SIZE = cfg.MAX_ITEMS;
var arr = [0; SIZE];          // SIZE 个 0
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

// 配置访问（编译期常量）
cfg.DEBUG;        // 访问编译期配置
cfg.PLATFORM;
cfg.MAX_SIZE;
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
// 编译期 lambda（默认）
// 约束：函数体必须是纯计算，不能 IO、不能网络
val add = |a: int, b: int| -> int {
    return a + b;
};

val get_config = | | -> int {
    return cfg.MAX_SIZE;    // OK：读取编译期配置
};

// 运行时 lambda（显式标记）
runtime val fetch_url = |url: string| -> string {
    return http.get(url);       // OK：可以网络请求
};

runtime val read_file = |path: string| -> string {
    return std.read_file(path); // OK：可以 IO
};

// 编译期 lambda 不能调用运行时函数
val bad_fn = |url: string| -> string {
    return fetch_url(url);  // ERROR：编译期函数不能调用运行时函数
};

// 运行时 lambda 可以调用编译期函数
runtime val good_fn = |a: int, b: int| -> int {
    return add(a, b);       // OK：运行时函数可以调用编译期函数
};

// 多行函数体
val average = |a: float, b: float| -> float {
    var sum = a + b;
    return sum / 2.0;
};

// 条件 lambda
val log = if (cfg.DEBUG) {
    |msg: string| -> void {
        print(msg);
    }
} else {
    |msg: string| -> void {
        // 空实现
    }
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

// 配置嵌套访问
cfg.FEATURES.networking;
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
// 编译期生成器
val counter = | | -> int {
    var i = 0;
    while true {
        yield i;
        i = i + 1;
    }
};

// 运行时生成器
runtime val async_counter = | | -> int {
    while true {
        yield std.now();    // 运行时获取时间
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

// 编译期数组大小
val SIZE = 100;
[int; SIZE]                   // 编译期确定大小的数组
```

### 类型标注位置

```kaubo
// 变量声明
var x: int = 42;

// 函数参数
|x: int, y: float| -> string { ... }

// 返回类型
val add: |int, int| -> int = |a: int, b: int| -> int {
    return a + b;
};

// Struct 字段
struct Point { x: float, y: float }

// 编译期常量
val MAX: int = 100;
```

---

## 编译期计算

### 编译期表达式

以下表达式在**编译期求值**：

```kaubo
// 1. 字面量
42
3.14
"hello"
true

// 2. val 绑定
val MAX = 100;
MAX

// 3. cfg 访问
cfg.DEBUG
cfg.MAX_SIZE
cfg.PLATFORM

// 4. 由以上构成的表达式
val SIZE = cfg.MAX_SIZE * 2;
val IS_BIG = cfg.MAX_SIZE > 1000;
val PLATFORM_IS_UNIX = cfg.PLATFORM == "linux" or cfg.PLATFORM == "macos";
```

### 编译期条件分支

```kaubo
// 如果条件是编译期常量，整个分支可能完全消除

// 编译期条件（代码可能完全消除）
if (cfg.DEBUG) {
    val x = expensive_operation();  // release 模式下不存在
}

// 运行时条件（代码始终存在）
runtime if (user_input == "yes") {
    runtime var x = expensive_operation();  // 始终编译，运行时决定
}
```

---

## 标准库（std）

### 数学函数（编译期）

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
print(value: any) -> void         // 打印（运行时）
assert(cond: bool, msg?: string)  // 断言（编译期）
type(value: any) -> string        // 获取类型名（编译期）
to_string(value: any) -> string   // 转为字符串
len(container: any) -> int        // 获取长度（编译期）
range(start: int, end: int) -> List<int>  // 生成范围（编译期）
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

### 文件操作（运行时）

```kaubo
runtime std.read_file(path: string) -> string
runtime std.write_file(path: string, content: string) -> void
runtime std.exists(path: string) -> bool
runtime std.is_file(path: string) -> bool
runtime std.is_dir(path: string) -> bool
```

### 环境访问（运行时）

```kaubo
runtime std.env(name: string) -> string   // 获取环境变量
runtime std.now() -> int                  // 获取当前时间戳
```

---

## 完整示例

```kaubo
// 编译期常量
val DEBUG = cfg.DEBUG;
val PLATFORM = cfg.PLATFORM;
val MAX_SIZE = cfg.MAX_SIZE;

// 平台适配（编译期）
val get_config_path = if (PLATFORM == "linux") {
    | | -> string {
        return "/home/user/.config/myapp";
    }
} elif (PLATFORM == "windows") {
    | | -> string {
        return "C:\\Users\\user\\AppData\\myapp";
    }
} else {
    | | -> string {
        return "/tmp/myapp";
    }
};

// 计算两点距离
struct Point {
    x: float,
    y: float
}

impl Point {
    distance: |self: Point, other: Point| -> float {
        var dx = self.x - other.x;
        var dy = self.y - other.y;
        return std.sqrt(dx * dx + dy * dy);
    }
}

var p1 = Point { x: 0.0, y: 0.0 };
var p2 = Point { x: 3.0, y: 4.0 };

print(p1.distance(p2));  // 5.0

// 调试日志（编译期条件）
val debug_log = if (DEBUG) {
    |msg: string| -> void {
        print("[DEBUG] " + msg);
    }
} else {
    |msg: string| -> void {
        // 空实现，release 模式下编译期消除
    }
};

debug_log("starting app");

// 使用编译期常量定义数组
val BUFFER_SIZE = MAX_SIZE * 2;
var buffer = [0; BUFFER_SIZE];

// 列表操作
var numbers = [1, 2, 3, 4, 5];
var doubled = numbers.map(|n: int| -> int {
    return n * 2;
});

// 类型转换
var avg = (1 + 2 + 3) as float / 3.0;
print("Average: " + avg as string);

// 运行时文件读取
runtime var config_content = std.read_file("app.config");
runtime var port = parse_port(config_content);
print("Server will start on port: " + port as string);

// 协程生成器（编译期）
val fib = | | -> int {
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
| 2026-02-19 | 添加 `runtime` 关键字，默认静态、显式动态的设计 |
