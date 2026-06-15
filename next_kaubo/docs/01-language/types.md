# 类型系统

## 基本类型

| 类型 | 说明 | 示例值 |
|------|------|--------|
| `int` | 有符号整数 | `42`, `-10`, `0` |
| `float` | 浮点数 | `3.14`, `-0.5`, `2.0` |
| `string` | UTF-8 字符串 | `"hello"` |
| `bool` | 布尔值 | `true`, `false` |
| `null` | 空值 | `null` |

## 类型标注

变量可以标注类型：

```kaubo
var count: int = 42;
var name: string = "Kaubo";
var price: float = 19.99;
var active: bool = true;
```

Lambda 的参数和返回值可以标注类型：

```kaubo
var add: |int, int| -> int = |a, b| -> int {
    return a + b;
};
```

## 类型推导

未标注类型的变量由编译器推导：

```kaubo
var x = 42;        // 推导为 int
var y = 3.14;      // 推导为 float
var z = "hello";   // 推导为 string
var w = true;      // 推导为 bool
var v = null;      // 推导为 null
```

## 结构体类型

```kaubo
struct Point {
    x: int,
    y: int,
}

// 带字段类型的实例
var p: Point = Point { x: 10, y: 20 };
```

## 列表类型

```kaubo
var numbers = [1, 2, 3];           // List<int>
var mixed = [1, "two", true];      // List<any>
```

## 函数类型

```kaubo
var f: |int, int| -> int = |a, b| -> int {
    return a + b;
};
```

函数类型语法：`|参数类型列表| -> 返回类型`

## 类型转换

使用 `as` 关键字：

```kaubo
var x = 3.14;
var y = x as int;     // 3

var a = 42;
var b = a as float;   // 42.0

var c = true;
var d = c as string;  // "true"
```

## 类型检查

```kaubo
var x = 42;
print type(x);  // "int"
```

## 当前限制

- 无泛型支持（`struct Box[T]` 待实现）
- List 运行时类型擦除（混合类型返回 `List<any>`）
- 除法语义：TypeChecker 认为 `int/int→int`，VM 执行 `int/int→float`

## 参考

- [TypeChecker 实现现状](../02-architecture/compiler-pipeline.md)
- [类型标注语法](./syntax.md#变量声明)
