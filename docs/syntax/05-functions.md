# 函数和 Lambda

## Lambda 语法

当前函数主要通过 lambda 表达式表示。body 为单个表达式时可省略花括号：

```kaubo
|x| x + 1
|a, b| a + b
```

带参数类型和返回类型：

```kaubo
|x: Int64, y: Int64| -> Int64 {
  return x + y;
}

// 单表达式也可标注类型
|x: Int64| -> Int64 x + 1
```

## 顶层函数绑定

常见写法是把 lambda 绑定到 `const` 或 `var`：

```kaubo
const add = |a, b| {
  return a + b;
};

add(1, 2);
```

lowering 会把顶层 lambda 注册成独立函数。

## return

`return` 是表达式表面：

```kaubo
const id = |x| {
  return x;
};
```

当前 parser 的 `return` 必须跟一个表达式；AST 支持空 return，但源码 parser 未提供 `return;` 形态。

## 闭包限制

当前 lambda lowering 更接近函数注册路径，不应假设已经支持完整闭包捕获语义。需要捕获外部变量的行为应先补回归测试。

## 方法函数

impl 方法体也是 lambda：

```kaubo
impl Point {
  dis: |self: Point, other: Point| -> Float64 {
    return sqrt((self.x - other.x).to_float());
  }
};
```

方法调用时，`self` 由 member 调用路径提供：

```kaubo
p1.dis(p2);
```
