# 代码示例

Playground 内置 8 个示例，位于侧边栏 Examples 面板。所有示例均已通过 WASM `diagnose()` 验证。

## 1. Hello World

```kaubo
print "Hello, World!";
```

最简单的 Kaubo 程序。`print` 可以不带括号直接打印简单值。

## 2. Variables & Types

```kaubo
var age = 25;
var pi = 3.14159;
var name = "Kaubo";
var is_valid = true;
var nothing = null;

print name;
print age;
print pi;
print is_valid;
```

演示五种基本类型：`int`、`float`、`string`、`bool`、`null`。变量声明用 `var`，支持类型推导。

## 3. Control Flow

```kaubo
var age = 18;
if age >= 18 {
    print "Adult";
} else {
    print "Minor";
}

var counter = 0;
var sum = 0;
while counter < 5 {
    sum = sum + counter;
    counter = counter + 1;
}
print sum;

var items = [1, 2, 3, 4, 5];
var total = 0;
for var item in items {
    total = total + item;
}
print total;
```

演示 `if/else`、`while`、`for-in` 三种控制流。`for var item in items` 是标准遍历写法。

## 4. Functions & Closures

```kaubo
var add = |a, b| -> int {
    return a + b;
};
print add(3, 5);

var greet = || -> string {
    return "Hello!";
};
print greet();

var square = |x| -> int {
    return x * x;
};
print square(4);

var make_counter = || {
    var count = 0;
    return || {
        count = count + 1;
        return count;
    };
};
var counter = make_counter();
print counter();   // 1
print counter();   // 2
```

演示 Lambda 定义（带/不带参数和类型标注）、闭包捕获外部变量。

## 5. Structs

```kaubo
struct Point {
    x: int,
    y: int,
}

var p = Point { x: 100, y: 200 };
print p.x;
print p.y;
```

演示 struct 定义和实例化。字段访问用 `.field` 语法。

## 6. Lists

```kaubo
var numbers = [1, 2, 3, 4, 5];
var fruits = ["apple", "banana", "cherry"];

print numbers[0];
print fruits[1];

numbers[0] = 10;
print numbers[0];

var sum = 0;
for var n in numbers {
    sum = sum + n;
}
print sum;
```

演示列表创建、索引访问（`list[idx]`）、元素修改、for-in 遍历。

## 7. List Methods

```kaubo
var list = [1, 2, 3];
list.push(4);
list.push(5);
print(list.len());

var doubled = list.map(|x| { return x * 2; });
print(doubled.len());

var evens = list.filter(|x| { return x % 2 == 0; });
print(evens.len());
```

演示列表方法：`.push()`（可链式）、`.len()`、`.map()`、`.filter()`。Lambda 表达式必须用 `|x| { return expr; }` 花括号形式。

## 8. JSON Literals

```kaubo
var obj = json {
    "name": "Kaubo",
    "age": 1
};
print obj.name;
obj.age = 2;
print obj.age;
```

演示 JSON 字面量创建和字段读写。JSON 对象的字段访问也使用 `.field` 语法。

## 语法要点

| 规则 | 说明 |
|------|------|
| 语句以 `;` 结尾 | `print "hello";` |
| `print` 两种写法 | `print val` 或 `print(expr)`（复杂表达式用括号） |
| `for` 用 `for var` | `for var item in list { ... }` |
| Lambda 必须有 `{ }` | `|x| { return x * 2; }`，不支持 `|x| x * 2` |
| 类型标注用 `:` 和 `->` | `var x: int = 1;` / `|a: int| -> int { ... }` |
