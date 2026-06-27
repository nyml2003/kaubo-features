# 边界例子

本页收集当前语法和主路径行为的边界例子。它们暂时只作为文档案例，不等同于已经落地的自动化测试。

标记说明：

- **应可运行**：当前 parser、infer、lowering、VM/driver 主路径应该接受。
- **应明确报错**：当前应返回 parse/infer/build/runtime 错误，不应该 panic。
- **parse-only**：parser 可以接受，但不代表主路径完整支持。

## 词法和字面量

### 字符串转义

**应可运行**：

```kaubo
print("a\nb\tc\\d\"e");
print('single quote string');
```

说明：核心 lexer 支持双引号和单引号字符串，支持 `\n`、`\r`、`\t`、`\\`、`\"`、`\'`。

### 未闭合字符串

**应明确报错**：

```kaubo
print("unterminated);
```

期望：词法错误最终表现为 parse 失败，而不是 panic。

### 浮点和 member 点号

**应可运行**：

```kaubo
print(42.to_float().to_string());
print(42.0.to_string());
```

说明：`42.0` 是浮点；`42.to_float()` 是整数 `42` 后跟 member 访问，不应被 lexer 误识别成非法浮点。

## 语句和分号

### 顶层表达式必须以分号结束

**应可运行**：

```kaubo
const x = 41;
x + 1;
```

**应明确报错**：

```kaubo
const x = 41;
x + 1
```

期望：顶层表达式语句缺少分号时报 parse 错误。

### struct / impl 尾部分号

**应可运行**：

```kaubo
struct Point { x: Int64 }
struct Pair { left: Int64, right: Int64 };
```

说明：struct / impl 解析后会跳过可选分号。

### 未初始化 var

**谨慎使用**：

```kaubo
var x: Int64;
x;
```

说明：当前 lowering 会为未初始化 `var` 分配寄存器，但默认值语义不应作为稳定语言能力依赖。

## 类型和推断

### 未知类型

**应明确报错**：

```kaubo
const f = |x: MissingType| {
  x
};
```

期望：infer 阶段报告 unknown type。

### if 分支类型不一致

**应明确报错**：

```kaubo
const x = if true { 1 } else { "one" };
```

期望：infer 阶段报告分支类型无法统一。

### List 类型标注不等于 list runtime

**parse/infer 表面存在，runtime 不完整**：

```kaubo
var xs: List<Int64>;
```

**应明确报错**：

```kaubo
const xs = [1, 2, 3];
```

期望：非空 list literal 在 build 阶段返回 `list literals are not implemented`。

## 运算和转换

### 整数和浮点不能混算混比

**应明确报错**：

```kaubo
1 + 2.0;     // 算术：类型不匹配
1 == 2.0;    // 比较：类型不匹配（v2.4+）
1 < 2.0;     // 比较：类型不匹配（v2.4+）
```

**应可运行**：

```kaubo
1.to_float() + 2.0;              // 显式转换后算术
1.to_float() == 2.0;             // 显式转换后比较
```

说明：算术运算和比较运算都要求操作数类型统一，不隐式提升。
`Null` 是例外——可与任何类型进行比较（用于 `??` 等场景）。

### logical and/or

**parse-only / build 未完整实现**：

```kaubo
true and false;
true or false;
```

期望：当前 lowering 返回 logical binary operators 未实现，而不是 panic。

## 块、return 和函数

### 块值来自最后一个表达式

**应可运行**：

```kaubo
const result = {
  var x = 10;
  x + 1
};
result;
```

### return 必须带表达式

**应可运行**：

```kaubo
const id = |x| {
  return x;
};
id(42);
```

**应明确报错**：

```kaubo
const f = || {
  return;
};
```

期望：源码 parser 当前不支持空 `return;`。

### 闭包捕获不要视为稳定能力

**谨慎使用**：

```kaubo
const base = 10;
const add_base = |x| { x + base };
add_base(1);
```

说明：当前 lambda 更接近顶层函数注册路径，不应假设完整闭包捕获已经稳定。

## 控制流

### while 修改外部变量

**应可运行**：

```kaubo
var n = 0;
while n < 3 {
  n = n + 1;
};
n;
```

### break / continue 在循环外

**应明确报错**：

```kaubo
break;
```

```kaubo
continue;
```

期望：build 阶段返回 outside loop 错误。

### for 当前不可运行

**parse-only / build 未实现**：

```kaubo
for x in xs {
  print(x.to_string());
}
```

期望：lowering 返回 `for loops are not implemented in lowering`。

## Struct 和 Impl

### literal 字段顺序不影响存储顺序

**应可运行**：

```kaubo
struct Pair { left: Int64, right: Int64 };
const p = Pair { right: 20, left: 10 };
p.left + p.right;
```

期望结果：`30`。

### 缺字段

**应明确报错**：

```kaubo
struct Point { x: Int64, y: Int64 };
const p = Point { x: 1 };
```

期望：build 阶段报告 missing field。

### 未知字段

**应明确报错**：

```kaubo
struct Point { x: Int64 };
const p = Point { x: 1, y: 2 };
```

```kaubo
struct Point { x: Int64 };
const p = Point { x: 1 };
p.y;
```

期望：infer 或 build 阶段报告 field not found。

### 方法调用会隐式传入 self

**应可运行**：

```kaubo
struct Point { x: Int64 };

impl Point {
  value: |self: Point| -> Int64 {
    return self.x;
  }
};

const p = Point { x: 42 };
p.value();
```

## 模块语法

### import/export 是 parse-only

**parse-only**：

```kaubo
import "std/prelude";
import "math" as math;
import { sqrt, sin } from "std/math";
export const answer = 42;
```

说明：当前不会加载文件、解析命名空间或生成导出表。

### VSCode snippet 的 from-import 形态不是核心 parser 形态

**应明确报错或不要使用**：

```kaubo
from "std/math" import { sqrt };
```

核心 parser 当前接受的是：

```kaubo
import { sqrt } from "std/math";
```

## Built-ins

### print 稳定写法

**应可运行**：

```kaubo
print("hi");
print(42.to_string());
print(1.5.to_string());
```

说明：`print` 输出由 VM 捕获。为了避开类型签名和 runtime 表示差异，非字符串建议显式 `.to_string()`。

### math 函数参数

**应可运行**：

```kaubo
sqrt(25.0);
sqrt(25.to_float());
```

**应明确报错**：

```kaubo
sqrt();
```

期望：native 返回缺少参数错误。

### assert

**应可运行**：

```kaubo
assert(1 < 2);
```

**应返回 runtime 错误**：

```kaubo
assert(false);
```

### type_of

**应明确报错**：

```kaubo
type_of(42);
```

期望：当前 lowering/runtime 返回 `type_of is not implemented`。

## 编辑器提示边界

### grammar/snippets 中的规划词

**不要当作核心语法**：

```kaubo
elif
pass
yield
module
pub
operator
json
```

说明：这些词可能出现在 VSCode grammar 或 snippets 中，但当前核心 lexer/parser/driver 主路径并不把它们都作为语言能力实现。
