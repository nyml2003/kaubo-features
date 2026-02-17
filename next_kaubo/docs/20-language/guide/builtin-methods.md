# Kaubo 内置类型方法 (Builtin Type Methods)

## 概述

Kaubo 支持为内置类型（List、String、Json）定义方法，语法与 Struct 方法一致：

```kaubo
var list = [1, 2, 3];
list.push(4);              // 调用方法
list.len();                // 返回值
```

## List 方法

### 基础方法

#### `push(item: any) -> List`
向列表末尾添加一个元素，返回 receiver 支持链式调用。

```kaubo
var list: List<int> = [1, 2];
list.push(3);                    // [1, 2, 3]
list.push(4).push(5);            // [1, 2, 3, 4, 5]
```

#### `len() -> int`
返回列表元素个数。

```kaubo
var list = [1, 2, 3];
var n: int = list.len();         // n = 3
```

#### `remove(index: int) -> any`
移除并返回指定索引处的元素。

```kaubo
var list = [10, 20, 30];
var removed: int = list.remove(1);   // removed = 20, list = [10, 30]
```

#### `clear() -> List`
清空列表所有元素，返回 receiver 支持链式调用。

```kaubo
var list = [1, 2, 3];
list.clear();                    // list = []
```

#### `is_empty() -> bool`
检查列表是否为空。

```kaubo
var list = [];
var empty: bool = list.is_empty();   // true
```

### 函数式方法

#### `foreach(callback: |item| -> void) -> List`
遍历列表，对每个元素调用回调函数，返回 receiver 支持链式调用。

```kaubo
var list = [1, 2, 3];
var sum: int = 0;
list.foreach(|x| {
    sum = sum + x;
});
// sum = 6, list = [1, 2, 3]

// 链式调用
list.foreach(|x| print(x))
    .push(4);
```

#### `map(callback: |item| -> any) -> List`
遍历列表，对每个元素调用回调函数，返回新列表。

```kaubo
var list = [1, 2, 3];
var doubled: List<int> = list.map(|x| x * 2);
// doubled = [2, 4, 6], list 不变
```

#### `filter(predicate: |item| -> bool) -> List`
遍历列表，返回满足条件的元素组成的新列表。

```kaubo
var list = [1, 2, 3, 4, 5];
var evens: List<int> = list.filter(|x| x % 2 == 0);
// evens = [2, 4]
```

#### `reduce(callback: |acc, item| -> any, initial: any) -> any`
遍历列表，累积计算单一结果。

```kaubo
var list = [1, 2, 3, 4, 5];
var sum: int = list.reduce(|acc, x| acc + x, 0);
// sum = 15

var product: int = list.reduce(|acc, x| acc * x, 1);
// product = 120
```

#### `find(predicate: |item| -> bool) -> any | null`
返回第一个满足条件的元素，未找到返回 null。

```kaubo
var list = [1, 2, 3, 4, 5];
var firstEven: int = list.find(|x| x % 2 == 0);
// firstEven = 2

var firstNegative: int = list.find(|x| x < 0);
// firstNegative = null
```

#### `any(predicate: |item| -> bool) -> bool`
检查是否有任意元素满足条件。

```kaubo
var list = [1, 2, 3];
var hasEven: bool = list.any(|x| x % 2 == 0);
// hasEven = true

var hasNegative: bool = list.any(|x| x < 0);
// hasNegative = false
```

#### `all(predicate: |item| -> bool) -> bool`
检查是否所有元素都满足条件。

```kaubo
var list = [1, 2, 3];
var allPositive: bool = list.all(|x| x > 0);
// allPositive = true

var allEven: bool = list.all(|x| x % 2 == 0);
// allEven = false
```

### 链式调用示例

```kaubo
var result: List<int> = [1, 2, 3, 4, 5, 6]
    .filter(|x| x > 2)          // [3, 4, 5, 6]
    .map(|x| x * 10)            // [30, 40, 50, 60]
    .filter(|x| x < 55);        // [30, 40, 50]
```

## String 方法

### `len() -> int`
返回字符串字符数。

```kaubo
var s: string = "hello";
var n: int = s.len();        // n = 5
```

### `is_empty() -> bool`
检查字符串是否为空。

```kaubo
var s: string = "";
var empty: bool = s.is_empty();   // true
```

## Json 方法

### `len() -> int`
返回 JSON 对象的键值对数量。

```kaubo
var j: json = json {a: 1, b: 2, c: 3};
var n: int = j.len();        // n = 3
```

### `is_empty() -> bool`
检查 JSON 对象是否为空。

```kaubo
var j: json = json {};
var empty: bool = j.is_empty();   // true
```

## 实现说明

### 架构设计

1. **编译期解析**：方法名在编译期解析为索引（0=push, 1=len...）
2. **CallBuiltin 指令**：使用专用指令 `0xDC` 调用内置方法
3. **VM 参数传递**：函数式方法通过 VM 实例调用闭包
4. **零堆分配**：基础方法无额外内存分配

### 方法签名

```rust
pub type BuiltinMethodFn = fn(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String>;
```

### 类型常量

```rust
pub const LIST: u8 = 0;
pub const STRING: u8 = 1;
pub const JSON: u8 = 2;
```

## 与 Struct 方法的对比

| 特性 | 内置类型方法 | Struct 方法 |
|------|-------------|-------------|
| 语法 | `list.push(1)` | `obj.method()` |
| 实现 | 原生 Rust 函数 | Kaubo 字节码 |
| 性能 | 零开销 | 需要 CallFrame |
| 链式调用 | 支持 | 支持 |

## 未来扩展

- **String**: `contains()`, `starts_with()`, `ends_with()`, `split()`, `trim()`
- **List**: `slice()`, `reverse()`, `sort()`, `join()`, `flat_map()`
- **Json**: `keys()`, `values()`, `has_key()`
