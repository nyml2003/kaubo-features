# 编程指南

## 变量与作用域

Kaubo 使用词法作用域。`{ }` 块创建新作用域。

```kaubo
var x = 10;

{
    var y = 20;
    print x;  // 10 — 可以访问外层
    print y;  // 20
}

// print y;  // 错误 — y 已超出作用域
```

## 函数

### 定义 Lambda

```kaubo
// 完整语法：参数 + 类型 + 返回类型 + 函数体
var add: |int, int| -> int = |a, b| -> int {
    return a + b;
};

// 简化：省略外层类型标注（编译器推导）
var add = |a, b| -> int {
    return a + b;
};

// 无参数
var hello = || {
    return "Hello";
};

// 单参数
var inc = |x| { return x + 1; };
```

### 高阶函数

```kaubo
var apply = |f, x| -> int {
    return f(x);
};

var result = apply(|n| { return n * 2; }, 5);
print result;  // 10
```

### 闭包

闭包捕获外层作用域的变量（upvalue），即使外层函数已返回也能访问。

```kaubo
var make_adder = |n| {
    return |x| {
        return x + n;
    };
};

var add5 = make_adder(5);
print add5(3);  // 8
print add5(7);  // 12
```

## 结构体

### 定义与实例化

```kaubo
struct Person {
    name: string,
    age: int,
}

var alice = Person { name: "Alice", age: 30 };
var bob = Person { name: "Bob", age: 25 };
```

### 添加方法

```kaubo
impl Person {
    greet: |self| {
        print "Hello, " + self.name;
    }

    is_older: |self, other: Person| -> bool {
        return self.age > other.age;
    }
}

alice.greet();
print alice.is_older(bob);  // true
```

## 列表

### 基本操作

```kaubo
var items = [10, 20, 30];

// 索引（从 0 开始）
print items[0];  // 10

// 修改
items[1] = 25;

// 长度
print items.len();  // 3
```

### 函数式操作

```kaubo
var nums = [1, 2, 3, 4, 5];

// map — 映射每个元素
var squares = nums.map(|x| { return x * x; });
// squares = [1, 4, 9, 16, 25]

// filter — 过滤
var evens = nums.filter(|x| { return x % 2 == 0; });
// evens = [2, 4]

// reduce — 归约
var sum = nums.reduce(|acc, x| { return acc + x; }, 0);
// sum = 15

// find — 查找第一个匹配
var first = nums.find(|x| { return x > 3; });
// first = 4
```

## 协程

协程是协作式多任务的轻量机制。使用 `yield` 暂停执行，`resume` 恢复。

```kaubo
var coro = create_coroutine(|| {
    print "Step 1";
    yield 10;

    print "Step 2";
    yield 20;

    print "Done";
    return 30;
});

var r1 = resume(coro);  // 打印 "Step 1"，返回 10
var r2 = resume(coro);  // 打印 "Step 2"，返回 20
var r3 = resume(coro);  // 打印 "Done"，返回 30
```

## 模块

### 单文件模块

Kaubo 文件即模块，默认所有顶级声明为私有。

```kaubo
// math.kaubo
pub var PI = 3.14159;
pub var add = |a, b| { return a + b; };
```

### 导入

```kaubo
import "math";
print math.PI;
print math.add(1, 2);
```

### 带别名

```kaubo
import "math" as m;
print m.PI;
```

### 命名导入

```kaubo
from "math" import { PI, add };
print PI;
print add(1, 2);
```

## 注意事项

1. **语句以分号结尾** — 漏写 `;` 会导致语法错误
2. **`print` 复杂表达式加括号** — `print(x + y)` 而不是 `print x + y`
3. **`for` 循环用 `var`** — `for var item in list` 而不是 `for item in list`
4. **Lambda 必须用花括号** — `|x| { return x * 2; }` 而不是 `|x| x * 2`
5. **布尔运算用 `and` / `or` / `not`** — 不用 `&&` / `||` / `!`
6. **无 `++` / `--`** — 用 `x = x + 1`
