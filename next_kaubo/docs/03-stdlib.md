# Kaubo 标准库文档

> 标准库 `std` 模块的完整 API 参考

## 目录

1. [概述](#1-概述)
2. [核心函数](#2-核心函数)
3. [数学函数](#3-数学函数)
4. [数学常量](#4-数学常量)
5. [类型系统](#5-类型系统)
6. [实现细节](#6-实现细节)

---

## 1. 概述

### 1.1 设计原则

- **扁平化设计**：所有功能直接放在 `std` 下，无子模块
- **原生实现**：核心函数用 Rust 实现，通过 `ObjNative` 暴露
- **显式导入**：必须通过 `import std;` 才能使用

### 1.2 使用方式

```kaubo
import std;

// 输出
std.print("Hello, World!");

// 数学计算
var area = std.PI * r * r;
var dist = std.sqrt(x * x + y * y);

// 类型检查
if (std.type(x) == "int") {
    // ...
}
```

### 1.3 ShapeID 映射

编译期确定的字段索引：

| 名称 | ShapeID | 类型 |
|------|---------|------|
| `print` | 0 | 函数 |
| `assert` | 1 | 函数 |
| `type` | 2 | 函数 |
| `to_string` | 3 | 函数 |
| `sqrt` | 4 | 函数 |
| `sin` | 5 | 函数 |
| `cos` | 6 | 函数 |
| `floor` | 7 | 函数 |
| `ceil` | 8 | 函数 |
| `PI` | 9 | 常量 |
| `E` | 10 | 常量 |
| `create_coroutine` | 11 | 函数 |
| `resume` | 12 | 函数 |
| `coroutine_status` | 13 | 函数 |
| `len` | 14 | 函数 |
| `push` | 15 | 函数 |
| `is_empty` | 16 | 函数 |
| `range` | 17 | 函数 |
| `clone` | 18 | 函数 |
| `read_file` | 19 | 函数 |
| `write_file` | 20 | 函数 |
| `exists` | 21 | 函数 |
| `is_file` | 22 | 函数 |
| `is_dir` | 23 | 函数 |

---

## 2. 核心函数

### 2.1 print

**签名**: `print(value)`

**描述**: 将值输出到标准输出并换行

**参数**:
- `value`: 任意类型的值

**返回值**: `null`

**示例**:
```kaubo
import std;

std.print("Hello");     // Hello
std.print(42);          // 42
std.print(true);        // true
std.print(null);        // null
```

---

### 2.2 assert

**签名**: `assert(condition)` 或 `assert(condition, message)`

**描述**: 断言条件为真，否则抛出运行时错误

**参数**:
- `condition`: 布尔表达式
- `message` (可选): 错误消息字符串

**返回值**: `null`

**错误**: 条件为假时抛出运行时错误

**示例**:
```kaubo
import std;

std.assert(x > 0);
std.assert(y != null, "y should not be null");
```

---

### 2.3 type

**签名**: `type(value)`

**描述**: 返回值的类型名称字符串

**参数**:
- `value`: 任意类型的值

**返回值**: 类型名字符串

**可能的返回值**:
- `"int"` - 整数
- `"float"` - 浮点数
- `"string"` - 字符串
- `"bool"` - 布尔值
- `"null"` - 空值
- `"list"` - 列表
- `"function"` - 函数/闭包
- `"module"` - 模块
- `"json"` - JSON 对象
- `"coroutine"` - 协程
- `"unknown"` - 未知类型

**示例**:
```kaubo
import std;

std.type(123);        // "int"
std.type(3.14);       // "float"
std.type("hello");    // "string"
std.type(true);       // "bool"
std.type(null);       // "null"
std.type([1,2,3]);    // "list"
std.type(|| {});      // "function"
```

---

### 2.4 to_string

**签名**: `to_string(value)`

**描述**: 将值转换为字符串

**参数**:
- `value`: 任意类型的值

**返回值**: 字符串

**示例**:
```kaubo
import std;

std.to_string(123);     // "123"
std.to_string(true);    // "true"
std.to_string(null);    // "null"
```

---

## 3. 数学函数

### 3.1 sqrt

**签名**: `sqrt(x)`

**描述**: 计算平方根

**参数**:
- `x`: 数值（整数或浮点数）

**返回值**: 浮点数

**示例**:
```kaubo
import std;

std.sqrt(16);     // 4.0
std.sqrt(2);      // 1.414...
```

---

### 3.2 sin

**签名**: `sin(x)`

**描述**: 计算正弦值（弧度）

**参数**:
- `x`: 弧度值

**返回值**: 浮点数（-1.0 ~ 1.0）

**示例**:
```kaubo
import std;

std.sin(0);               // 0.0
std.sin(std.PI / 2);      // 1.0
```

---

### 3.3 cos

**签名**: `cos(x)`

**描述**: 计算余弦值（弧度）

**参数**:
- `x`: 弧度值

**返回值**: 浮点数（-1.0 ~ 1.0）

**示例**:
```kaubo
import std;

std.cos(0);               // 1.0
std.cos(std.PI);          // -1.0
```

---

### 3.4 floor

**签名**: `floor(x)`

**描述**: 向下取整

**参数**:
- `x`: 数值

**返回值**: 浮点数

**示例**:
```kaubo
import std;

std.floor(3.7);   // 3.0
std.floor(-1.5);  // -2.0
```

---

### 3.5 ceil

**签名**: `ceil(x)`

**描述**: 向上取整

**参数**:
- `x`: 数值

**返回值**: 浮点数

**示例**:
```kaubo
import std;

std.ceil(3.2);    // 4.0
std.ceil(-1.5);   // -1.0
```

---

## 4. 数学常量

### 4.1 PI

**描述**: 圆周率 π

**值**: 3.141592653589793...

**示例**:
```kaubo
import std;

var radius = 5;
var circumference = 2 * std.PI * radius;
var area = std.PI * radius * radius;
```

---

### 4.2 E

**描述**: 自然常数 e

**值**: 2.718281828459045...

**示例**:
```kaubo
import std;

// 计算 e^x
var x = 2;
var result = std.pow(std.E, x);  // 假设有 pow 函数
```

---

## 5. 类型系统

### 5.1 类型判断示例

```kaubo
import std;

fn describe(value) {
    var t = std.type(value);
    if (t == "int" or t == "float") {
        return "number";
    } elif (t == "string") {
        return "text";
    } elif (t == "bool") {
        return "boolean";
    } elif (t == "null") {
        return "nothing";
    } else {
        return "complex";
    }
}
```

---

## 6. 实现细节

### 6.1 Rust 实现

**位置**: `src/runtime/stdlib/mod.rs`

**原生函数类型**:
```rust
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;
```

**包装为 Value**:
```rust
fn create_native_value(func: NativeFn, name: &str, arity: u8) -> Value {
    let native = Box::new(ObjNative::new(func, name.to_string(), arity));
    Value::native_fn(Box::into_raw(native))
}
```

### 6.2 变参函数

`assert` 使用 arity=255 表示可变参数：

```rust
// VM 中的参数检查
if native.arity != 255 && arg_count != native.arity {
    return InterpretResult::RuntimeError(
        format!("Expected {} arguments but got {}", native.arity, arg_count)
    );
}
```

### 6.3 添加新函数

1. 在 `src/runtime/stdlib/mod.rs` 实现函数
2. 在 `create_stdlib_modules()` 中注册
3. 在 `compiler.rs` 的 `find_std_module_shape_id()` 中添加映射
4. 更新本文档

## 7. 协程函数

### 7.1 create_coroutine

**签名**: `create_coroutine(closure)`

**描述**: 从闭包创建协程

**参数**:
- `closure`: 一个无参数的闭包

**返回值**: 协程对象

**示例**:
```kaubo
import std;

var producer = || {
    yield 1;
    yield 2;
    yield 3;
};

var coro = std.create_coroutine(producer);
std.resume(coro);  // 1
std.resume(coro);  // 2
```

---

### 7.2 resume

**签名**: `resume(coroutine, ...values)`

**描述**: 恢复协程执行，传入值作为 yield 的结果

**参数**:
- `coroutine`: 协程对象
- `values` (可选): 传递给协程的值

**返回值**: 协程 yield 的值或最终返回值

---

### 7.3 coroutine_status

**签名**: `coroutine_status(coroutine)`

**描述**: 获取协程状态

**返回值**: 
- `0` - Suspended（可恢复）
- `1` - Running（运行中）
- `2` - Dead（已结束）

---

## 8. 列表操作函数

### 8.1 len

**签名**: `len(list|string|json)`

**描述**: 获取长度

**示例**:
```kaubo
import std;

var list = [1, 2, 3];
std.len(list);        // 3
std.len("hello");     // 5
```

---

### 8.2 push

**签名**: `push(list, value)`

**描述**: 将元素添加到列表末尾（返回新列表）

**示例**:
```kaubo
import std;

var list = [1, 2];
var new_list = std.push(list, 3);  // [1, 2, 3]
```

---

### 8.3 is_empty

**签名**: `is_empty(list|string|json)`

**描述**: 检查是否为空

**返回值**: 布尔值

---

### 8.4 range

**签名**: `range(end)` 或 `range(start, end)` 或 `range(start, end, step)`

**描述**: 生成整数范围列表

**示例**:
```kaubo
import std;

std.range(5);           // [0, 1, 2, 3, 4]
std.range(1, 5);        // [1, 2, 3, 4]
std.range(0, 10, 2);    // [0, 2, 4, 6, 8]
std.range(5, 0, -1);    // [5, 4, 3, 2, 1]
```

---

### 8.5 clone

**签名**: `clone(value)`

**描述**: 创建值的浅拷贝

**示例**:
```kaubo
import std;

var list = [1, 2, 3];
var copy = std.clone(list);
```

---

## 9. 文件系统函数

### 9.1 read_file

**签名**: `read_file(path)`

**描述**: 读取文件内容为字符串

**参数**:
- `path`: 文件路径（字符串）

**返回值**: 文件内容字符串

**错误**: 文件不存在或读取失败时抛出错误

---

### 9.2 write_file

**签名**: `write_file(path, content)`

**描述**: 将字符串写入文件

**参数**:
- `path`: 文件路径（字符串）
- `content`: 文件内容（字符串）

**返回值**: `null`

---

### 9.3 exists

**签名**: `exists(path)`

**描述**: 检查路径是否存在

**返回值**: 布尔值

---

### 9.4 is_file

**签名**: `is_file(path)`

**描述**: 检查路径是否为文件

**返回值**: 布尔值

---

### 9.5 is_dir

**签名**: `is_dir(path)`

**描述**: 检查路径是否为目录

**返回值**: 布尔值

---

## 10. 已知限制

| 限制 | 说明 | 替代方案 |
|------|------|----------|
| 浮点数字面量 | 暂不支持 `3.14` 直接书写 | 使用 `std.sqrt` 等函数获得 |
| 字符串转义 | `\n` `\t` 等暂不支持 | 直接使用换行/制表符 |
| 垃圾回收 | 暂未实现，内存只增不减 | 避免大量对象创建 |

---

*最后更新: 2026-02-12*
