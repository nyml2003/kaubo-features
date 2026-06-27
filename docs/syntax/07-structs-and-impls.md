# Struct 和 Impl

## Struct 声明

```kaubo
struct Point {
  x: Int64,
  y: Int64,
};
```

字段使用 `name: Type` 形式，字段之间可以用逗号分隔。

## Struct literal

```kaubo
const p = Point { x: 200, y: 300 };
```

literal 必须提供声明中的所有字段，未知字段会返回错误。

### 简写属性

当字段名与变量名相同时可省略 `: value`：

```kaubo
const x = 200;
const y = 300;
const p = Point { x, y };        // 等价于 Point { x: x, y: y }
const q = Point { x, y: 400 };   // 混合使用
```

### 结构体 spread

使用 `...` 从已有 struct 复制字段：

```kaubo
const p1 = Point { x: 1, y: 2 };
const p2 = Point { ...p1, y: 3 };   // Point { x: 1, y: 3 }  显式字段覆盖 spread
```

spread 按 struct 声明字段序展开，后续显式字段覆盖同名展开字段。

### 字段顺序

字段写入按 struct 声明顺序进行：

```kaubo
struct Pair { left: Int64, right: Int64 };
const p = Pair { right: 20, left: 10 };
p.left + p.right; // 30
```

## 字段访问

```kaubo
p.x
```

当前字段解析由类型和 struct 定义驱动。访问未知字段会在 infer/build 阶段返回明确错误。

## Impl 方法

```kaubo
impl Point {
  sum: |self: Point| -> Int64 {
    return self.x + self.y;
  }
};
```

方法名通过 `StructName.method` 注册到 lowering 的函数表。

## 方法调用

```kaubo
p.sum();
```

带参数：

```kaubo
impl Point {
  add: |self: Point, other: Point| -> Int64 {
    return self.x + other.x;
  }
};

p1.add(p2);
```

## 当前限制

- 方法体建议写成 lambda。
- 不应假设已有继承、trait、可见性或泛型方法。
- `self` 是普通关键字 token，方法签名中需要显式写出。
