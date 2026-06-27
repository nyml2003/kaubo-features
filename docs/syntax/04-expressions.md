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

## 模板字符串

反引号 `` ` `` 界定，`{expr}` 内嵌表达式：

```kaubo
const msg = `hello {name}, age {age}`;
const calc = `{a} + {b} = {a + b}`;
```

脱糖为 `.to_string()` 调用 + `+` 拼接。内嵌表达式可以是任意 kaubo 表达式。

## Null 合并 `??`

```kaubo
const name = input ?? "default";
const deep = a ?? b ?? c;    // 左结合: (a ?? b) ?? c
```

脱糖为 `if left != null { left } else { right }`。
注意 kaubo 中 `null` 和 `0` 在 VM 中同值（均为 `i64(0)`），`??` 对字面量 `0` 无法正确判别，
需等 nullable 类型系统（enum `Option<T>`）落地后修复。

## 可选链 `?.` `?[`

```kaubo
const name = user?.profile?.name;
const item = list?[0];
```

脱糖为临时变量 + null 检查 + 字段/index 访问的链式 if/else。

## 字符串拼接

`+` 用于字符串拼接时走 CPS `SAdd` 指令，字符串拼接在 VM 中为堆分配+拼接：

```kaubo
const g = "hello, " + name;
```

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
