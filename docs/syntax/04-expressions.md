# 表达式

## 字面量

主路径支持：

```kaubo
42
1.5
"hello"
'hello'
true
false
null
```

`true` lowering 为整数 `1`，`false` 和 `null` lowering 为整数 `0`。

## 变量引用

```kaubo
const x = 42;
x;
```

未绑定变量会在 infer 或 build 阶段返回明确错误。

## 一元运算

```kaubo
-x
not ok
```

`-` 用于数值取负。`not` 返回布尔语义结果。

## 二元运算

主路径覆盖：

```kaubo
a + b
a - b
a * b
a / b
a % b
a == b
a != b
a < b
a <= b
a > b
a >= b
```

整数和浮点比较都应产生可用于分支的布尔值。

当前 parser 也接受 `and`、`or`、`|>`、`>>`，但 lowering 不完整。详见 [部分实现的语法表面](10-partial-features.md)。

## 赋值

变量赋值：

```kaubo
var n = 0;
n = n + 1;
```

字段赋值和 index 赋值属于更弱路径，使用前需要补测试确认。

## 调用

```kaubo
print("hi");
add(1, 2);
```

函数值主要来自绑定到名称的 lambda，或 impl 方法调用。

## Member

字段访问：

```kaubo
p.x
```

内建 member 方法：

```kaubo
42.to_string()
1.5.to_string()
42.to_float()
```

struct 方法调用也使用 member syntax：

```kaubo
p1.dis(p2)
```

## 块表达式

块是表达式，值来自最后一个表达式语句：

```kaubo
const result = {
  var x = 10;
  x + 1
};
```

块内支持 `const`、`var` 和表达式语句。

## Struct literal

```kaubo
struct Pair { left: Int64, right: Int64 };
const p = Pair { right: 20, left: 10 };
```

lowering 会按 struct 声明顺序写入字段，而不是按 literal 中出现的顺序。
