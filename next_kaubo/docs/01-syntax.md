# Kaubo 语法参考

> 完整的语言语法说明

## 目录

1. [基础语法](#1-基础语法)
2. [变量与类型](#2-变量与类型)
3. [运算符](#3-运算符)
4. [控制流](#4-控制流)
5. [函数](#5-函数)
6. [模块系统](#6-模块系统)
7. [标准库](#7-标准库)

---

## 1. 基础语法

### 1.1 语句结束

Kaubo 使用分号 `;` 表示语句结束：

```kaubo
var x = 5;
var y = 10;
```

### 1.2 注释

```kaubo
// 单行注释

/*
 * 多行注释（暂不支持）
 */
```

### 1.3 标识符规则

- 以字母或下划线开头
- 可包含字母、数字、下划线
- 区分大小写

```kaubo
var my_var = 1;
var _private = 2;
var test123 = 3;
```

---

## 2. 变量与类型

### 2.1 变量声明

使用 `var` 关键字：

```kaubo
var x = 5;           // 整数
var name = "Kaubo";  // 字符串
var pi = 3.14159;    // 浮点数（需标准库支持）
var flag = true;     // 布尔值
var empty = null;    // 空值
```

### 2.2 基本类型

| 类型 | 示例 | 说明 |
|------|------|------|
| `int` | `42`, `-10` | 32/64位整数 |
| `float` | `3.14`, `-0.5` | 64位浮点数（通过标准库创建） |
| `string` | `"hello"` | 字符串 |
| `bool` | `true`, `false` | 布尔值 |
| `null` | `null` | 空值 |
| `list` | `[1, 2, 3]` | 列表/数组 |
| `function` | `\|x\| { x }` | 函数/闭包 |

### 2.3 类型检查

```kaubo
import std;
std.type(123);       // "int"
std.type("hello");   // "string"
std.type(true);      // "bool"
std.type(null);      // "null"
```

---

## 3. 运算符

### 3.1 算术运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `+` | 加法 | `1 + 2` → `3` |
| `-` | 减法/取负 | `5 - 3`, `-5` |
| `*` | 乘法 | `4 * 5` → `20` |
| `/` | 除法 | `20 / 4` → `5.0` |

### 3.2 比较运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `==` | 等于 | `5 == 5` → `true` |
| `!=` | 不等于 | `5 != 3` → `true` |
| `>` | 大于 | `5 > 3` → `true` |
| `<` | 小于 | `3 < 5` → `true` |
| `>=` | 大于等于 | `5 >= 5` → `true` |
| `<=` | 小于等于 | `3 <= 5` → `true` |

### 3.3 逻辑运算符

| 运算符 | 说明 | 示例 |
|--------|------|------|
| `and` | 逻辑与 | `true and false` → `false` |
| `or` | 逻辑或 | `true or false` → `true` |
| `not` | 逻辑非 | `not true` → `false` |

**注意**：`and`/`or` 暂未实现短路求值。

### 3.4 运算符优先级

从高到低：

1. `()` - 括号
2. `not`, `-` (一元) - 逻辑非、取负
3. `*`, `/` - 乘除
4. `+`, `-` - 加减
5. `==`, `!=`, `>`, `<`, `>=`, `<=` - 比较
6. `and` - 逻辑与
7. `or` - 逻辑或

---

## 4. 控制流

### 4.1 条件语句

```kaubo
if (x > 0) {
    // x 是正数
} elif (x < 0) {
    // x 是负数
} else {
    // x 是零
}
```

### 4.2 While 循环

```kaubo
var i = 0;
while (i < 10) {
    std.print(i);
    i = i + 1;
}
```

### 4.3 For-In 循环

```kaubo
var items = [1, 2, 3];
for (item in items) {
    std.print(item);
}
```

### 4.4 Return 语句

```kaubo
var add = |a, b| {
    return a + b;  // 返回值
};

var greet = || {
    std.print("Hello");
    return;  // 无返回值
};
```

---

## 5. 函数

### 5.1 Lambda 表达式

Kaubo 使用 `|params| { body }` 语法定义函数：

```kaubo
// 无参数
var say_hello = || {
    std.print("Hello!");
};

// 单参数
var double = |x| {
    return x * 2;
};

// 多参数
var add = |a, b| {
    return a + b;
};
```

### 5.2 函数调用

```kaubo
say_hello();
var result = add(3, 4);  // 7
```

### 5.3 闭包

函数可以捕获外部变量：

```kaubo
var make_counter = || {
    var count = 0;
    return || {
        count = count + 1;
        return count;
    };
};

var counter = make_counter();
counter();  // 1
counter();  // 2
counter();  // 3
```

### 5.4 递归（限制）

当前 Lambda 无法直接引用自身，递归需要通过参数传递：

```kaubo
// 暂不支持直接递归
// var factorial = |n| {
//     if (n <= 1) { return 1; }
//     return n * factorial(n - 1);  // 错误！factorial 未定义
// };
```

---

## 6. 模块系统

### 6.1 导入标准库

```kaubo
import std;

std.print("Hello");
var x = std.sqrt(16);
```

### 6.2 模块特性

- **扁平化设计**：没有子模块
- **显式导入**：必须通过 `import` 导入才能使用
- **ShapeID**：编译期确定字段索引，运行时 O(1) 访问

### 6.3 模块路径

```kaubo
// 单层模块
import std;

// 暂不支持嵌套
// import std.math;  // 错误！
```

---

## 7. 标准库

### 7.1 核心函数

```kaubo
import std;

std.print(x);          // 输出并换行
std.assert(cond);      // 断言
std.assert(cond, msg); // 断言带消息
std.type(x);           // 获取类型名称
std.to_string(x);      // 转为字符串
```

### 7.2 数学函数

```kaubo
import std;

std.sqrt(x);    // 平方根
std.sin(x);     // 正弦
std.cos(x);     // 余弦
std.floor(x);   // 向下取整
std.ceil(x);    // 向上取整

// 常量
std.PI;  // 3.14159...
std.E;   // 2.71828...
```

### 7.3 使用示例

```kaubo
import std;

// 计算圆面积
var circle_area = |r| {
    return std.PI * r * r;
};

std.print(circle_area(5));  // 78.54...
```

---

## 附录：保留关键字

```
var, if, else, elif, while, for, return
in, yield, true, false, null, break, continue
struct, interface, import, as, from, pass
and, or, not, async, await, module, pub, json
```

---

*最后更新: 2026-02-10*
