# Kaubo 类型系统

## 概述

Kaubo 采用**严格类型系统**，不允许隐式类型转换。所有类型转换必须通过显式的 `as` 表达式。

## 基础类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `int` | 32位整数 | `42` |
| `float` | 64位浮点数 | `3.14` |
| `bool` | 布尔值 | `true`, `false` |
| `string` | 字符串 | `"hello"` |
| `any` | 顶层类型，接受任何类型 | - |
| `List<T>` | 列表 | `[1, 2, 3]` |
| `Function` | 函数/闭包 | `\|x\| x + 1` |

## 严格类型检查

### 无隐式转换

```kaubo
// ❌ 错误：int 不能隐式转为 float
var x: float = 1;

// ✅ 正确：使用显式转换
var x: float = 1 as float;

// ✅ 正确：使用 float 字面量
var x: float = 1.0;
```

### 函数参数检查

```kaubo
// sqrt: |float| -> float
std.sqrt(16);      // ❌ 错误：期望 float，得到 int
std.sqrt(16.0);    // ✅ 正确
std.sqrt(16 as float);  // ✅ 正确
```

## `as` 类型转换

### 支持的转换

| 从 | 到 | 说明 |
|----|----|----|
| `int` | `float` | 整数转浮点数 |
| `float` | `int` | 截断小数部分 |
| `int` | `string` | 数字转字符串 |
| `float` | `string` | 浮点转字符串 |
| `bool` | `string` | "true" / "false" |

### 示例

```kaubo
// 基本转换
var f: float = 42 as float;      // 42.0
var s: string = 123 as string;   // "123"

// 在表达式中使用
var result = std.sqrt((a * a + b * b) as float);

// 列表元素转换
var nums = [1, 2, 3];
var floats = nums.map(|n| n as float);
```

## `any` 顶层类型

`any` 是类型系统的顶层类型，可以接受任何值。用于：

1. **泛型容器**
```kaubo
// len: |List<any>| -> int
len([1, "hello", true]);  // ✅ 可以，List<any> 接受任何元素
```

2. **通用函数参数**
```kaubo
// print: |any| -> void
print("hello");   // ✅
print(42);        // ✅
print([1, 2, 3]); // ✅
```

3. **未知类型**
```kaubo
// 无类型标注时推导为 any
var x = get_unknown_value();  // x: any
```

## Struct 类型推导

### 字段类型检查

```kaubo
struct Point {
    x: float,
    y: float
};

var p = Point { x: 0.0, y: 0.0 };
var x = p.x;  // 推导为 float
```

### 方法中的成员访问

```kaubo
impl Point {
    distance: |self, other: Point| -> float {
        var dx: float = self.x - other.x;  // self.x 推导为 float
        var dy: float = self.y - other.y;  // other.y 推导为 float
        return std.sqrt(dx * dx + dy * dy);
    }
};
```

## 函数类型

### 标注语法

```kaubo
// 无参数，无返回值
var f: || -> void = || { ... };

// 单参数，有返回值
var g: |int| -> int = |x| x * 2;

// 多参数
var h: |int, int| -> int = |a, b| a + b;
```

### Lambda 类型推导

```kaubo
// 自动推导返回类型
var add = |a: int, b: int| a + b;  // 推导为 |int, int| -> int

// 显式返回类型
var div = |a: float, b: float| -> float {
    return a / b;
};
```

## 类型检查模式

### 严格模式

在严格模式下：
- 所有变量必须有可推导或标注的类型
- 函数参数类型必须匹配
- 条件表达式必须是 `bool`
- 返回类型必须匹配

## 限制与注意事项

1. **无隐式数字转换**：`int` 和 `float` 不能自动转换
2. **无空值**：`null` 是单独的类型，与 `any` 不同
3. **无联合类型**：暂不支持 `int | string` 这样的联合类型
4. **泛型有限**：仅支持 `List<T>`，不支持用户定义泛型

## 实现细节

### 类型检查器架构

- `TypeChecker`: 主检查器，维护类型环境
- `TypeEnv`: 类型环境（作用域链）
- `struct_types`: struct 类型定义表
- `is_compatible(from, to)`: 类型兼容性检查

### 关键函数

```rust
// 检查表达式类型
fn check_expression(&mut self, expr: &Expr) -> TypeCheckResult<Option<TypeExpr>>;

// 检查类型兼容性（严格模式）
fn is_compatible(&self, from: &TypeExpr, to: &TypeExpr) -> bool;

// 检查类型转换合法性
fn is_valid_cast(&self, from: &TypeExpr, to: &TypeExpr) -> bool;
```
