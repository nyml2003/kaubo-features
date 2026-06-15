# 语法参考

## 关键字（22 个）

```
var       if        else      elif      while
for       return    in        yield     break
continue  struct    impl      import    as
from      pass      module    pub       json
and       or        not
```

## 字面量

| 类型 | 示例 |
|------|------|
| 整数 | `42`, `-10`, `0` |
| 浮点 | `3.14159`, `19.99`, `-0.5` |
| 字符串 | `"Hello"`, `"Kaubo"` |
| 布尔 | `true`, `false` |
| 空值 | `null` |

字符串转义：

| 转义 | 含义 |
|------|------|
| `\n` | 换行 |
| `\r` | 回车 |
| `\t` | 制表 |
| `\\` | 反斜杠 |
| `\"` | 双引号 |
| `\'` | 单引号 |

## 注释

```kaubo
// 行注释

/* 块注释，可以
   跨多行 */
```

## 运算符

### 算术

| 运算符 | 含义 |
|--------|------|
| `+` | 加法 / 字符串连接 |
| `-` | 减法 / 负号 |
| `*` | 乘法 |
| `/` | 除法 |
| `%` | 取模 |

### 比较

| 运算符 | 含义 |
|--------|------|
| `==` | 等于 |
| `!=` | 不等于 |
| `<` | 小于 |
| `>` | 大于 |
| `<=` | 小于等于 |
| `>=` | 大于等于 |

### 逻辑

| 运算符 | 含义 |
|--------|------|
| `and` | 逻辑与（短路） |
| `or` | 逻辑或（短路） |
| `not` | 逻辑非 |

## 变量声明

```kaubo
var name = "Kaubo";
var age = 25;
var price = 19.99;

// 带类型标注
var count: int = 42;
var active: bool = true;
```

## 控制流

### if / elif / else

```kaubo
if score >= 90 {
    print "A";
} elif score >= 80 {
    print "B";
} else {
    print "F";
}
```

### while

```kaubo
var counter = 0;
while counter < 5 {
    print counter;
    counter = counter + 1;
}
```

### for-in

```kaubo
var items = [1, 2, 3, 4, 5];
for var item in items {
    print item;
}
```

### break / continue / pass

```kaubo
while true {
    if condition {
        break;       // 退出循环
    }
    if skip {
        continue;    // 跳过本次迭代
    }
    pass;            // 空语句
}
```

## 函数（Lambda）

```kaubo
// 带参数和返回类型
var add = |a, b| -> int {
    return a + b;
};

// 无参数
var greet = || -> string {
    return "Hello!";
};

// 单参数
var square = |x| -> int {
    return x * x;
};

// 调用
var result = add(3, 5);
```

## 闭包

```kaubo
var make_counter = || {
    var count = 0;
    return || {
        count = count + 1;
        return count;
    };
};

var counter = make_counter();
print counter();  // 1
print counter();  // 2
```

## 结构体

```kaubo
struct Point {
    x: int,
    y: int,
}

var p = Point { x: 100, y: 200 };
print p.x;
print p.y;
```

### impl 方法

```kaubo
impl Point {
    distance: |self, other: Point| -> float {
        return std.sqrt(
            (self.x - other.x) * (self.x - other.x) +
            (self.y - other.y) * (self.y - other.y)
        );
    }
}

var p1 = Point { x: 0, y: 0 };
var p2 = Point { x: 3, y: 4 };
print p1.distance(p2);  // 5.0
```

## 列表

```kaubo
// 创建
var numbers = [1, 2, 3, 4, 5];
var fruits = ["apple", "banana"];

// 索引访问
print numbers[0];  // 1

// 修改
numbers[0] = 10;

// 遍历
for var n in numbers {
    print n;
}
```

## JSON 字面量

```kaubo
var obj = json {
    "name": "Kaubo",
    "age": 1
};
print obj.name;
obj.age = 2;
```

## 模块导入

```kaubo
// 简单导入
import "module_name";

// 带别名
import "module_name" as alias;

// 命名导入
from "module_name" import { export1, export2 };
```

## 协程

```kaubo
var coro = create_coroutine(|| {
    print "start";
    yield 1;
    print "middle";
    yield 2;
});

var r1 = resume(coro);  // 1
var r2 = resume(coro);  // 2
```

## print 语句

```kaubo
print "hello";              // 简单打印
print some_variable;        // 打印变量
print(complex_expression);  // 复杂表达式用括号
```
