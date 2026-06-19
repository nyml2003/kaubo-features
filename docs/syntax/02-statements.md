# 文件和语句

## Module

源码文件会被解析成一个 module，module 是多条顶层语句的列表：

```kaubo
const answer = 42;
var count = 0;
answer + count;
```

当前 driver 处理单文件源码，不做多文件模块加载。

## 分号

顶层声明和表达式语句通常需要分号：

```kaubo
const x = 1;
var y = 2;
x + y;
```

parser 会跳过多余分号和语句边界处的注释。

struct 和 impl 声明结束后可以带分号，也可以不带：

```kaubo
struct Point { x: Int64 };

impl Point {
  value: |self: Point| { self.x }
};
```

## const

`const` 声明必须有初始化表达式：

```kaubo
const x = 42;
const y: Int64 = x + 1;
```

如果值是 lambda，lowering 会把它注册成可调用函数。

## var

`var` 可以有初始化表达式，也可以只有类型标注：

```kaubo
var count = 0;
var pending: Int64;
```

当前未初始化 `var` 会分配寄存器，但实际默认值语义应谨慎依赖。

## 表达式语句

表达式可以作为语句出现：

```kaubo
print("hi");
40 + 2;
```

driver 返回当前入口执行后的整数结果，并捕获 `print` 输出。

## export 和 import

`export` 和 `import` 有 AST 表面，但当前主路径不做模块链接。详见 [模块语法](08-modules.md)。
