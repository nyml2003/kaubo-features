# Kaubo 示例程序

本目录包含 Kaubo 编程语言的核心示例。

## 示例列表

| 目录 | 说明 | 主要特性 |
|------|------|----------|
| `01_hello_world` | Hello World | `print` 语句, `return` |
| `02_variables` | 变量和基本类型 | `var`, `int`, `float`, `string`, `bool`, `null` |
| `04_control_flow` | 控制流 | `if-elif-else`, `while`, `for` |
| `05_functions` | Lambda 函数 | 函数定义、调用、闭包 |
| `06_structs` | 结构体 | `struct`, `impl`, 操作符重载 |
| `07_lists` | 列表 | 列表字面量、索引访问、遍历 |

## 运行示例

```bash
# 运行示例
cargo run -p kaubo-cli -- examples/01_hello_world/package.json

# 详细模式
cargo run -p kaubo-cli -- examples/02_variables/package.json --verbose

# 只编译不执行
cargo run -p kaubo-cli -- examples/05_functions/package.json --compile-only

# 生成二进制文件
cargo run -p kaubo-cli -- examples/01_hello_world/package.json --emit-binary
```

## 语言特性速查

### 变量声明
```kaubo
var x = 10;           // 整数
var y = 3.14;         // 浮点数
var s = "hello";      // 字符串
var b = true;         // 布尔值
var n = null;         // null
```

### 控制流
```kaubo
if x > 0 {
    print "positive";
} elif x < 0 {
    print "negative";
} else {
    print "zero";
}

while i < 10 {
    i = i + 1;
}

for var item in list {
    print item;
}
```

### 函数
```kaubo
var add = |a, b| -> int {
    return a + b;
};
var result = add(1, 2);
```

### 结构体
```kaubo
struct Point {
    x: int,
    y: int
}

impl Point {
    operator add: |this, other| -> Point {
        return Point { x: this.x + other.x, y: this.y + other.y };
    }
};

var p = Point { x: 1, y: 2 };
```

### 列表
```kaubo
var list = [1, 2, 3];
var first = list[0];
list[0] = 10;
```
