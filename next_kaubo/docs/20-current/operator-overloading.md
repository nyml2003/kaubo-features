# Kaubo 运算符重载设计

## 设计目标

1. **无下划线命名**：遵循原则 #8，使用 `operator add` 而非 `__add__`
2. **性能优先**：内置类型（int/float）编译期特化，自定义类型运行时扩展
3. **灵活统一**：索引访问、算术运算使用同一套机制

## 语法

```kaubo
struct Vector {
    data: List<float>
}

impl Vector {
    // 构造函数（普通方法）
    new: |size: int| -> Vector { ... }
    
    // 运算符重载
    operator add: |self, other: Vector| -> Vector {
        var result = Vector::new(len(self));
        for i in range(0, len(self)) {
            result[i] = self[i] + other[i];
        }
        return result;
    }
    
    operator mul: |self, scalar: float| -> Vector { ... }
    operator neg: |self| -> Vector { ... }
    operator eq:  |self, other| -> bool { ... }
    operator get: |self, index: int| -> float { ... }
    operator set: |self, index: int, value: float| { ... }
    operator str: |self| -> string { ... }
    operator len: |self| -> int { ... }
    
    // 反向乘法（scalar * vector）
    operator rmul: |self, scalar: float| -> Vector { ... }
}

// 使用
var v1 = Vector::new(3);
var v2 = Vector::new(3);
var v3 = v1 + v2;           // operator add
var v4 = v1 * 2.0;          // operator mul
var v5 = 2.0 * v1;          // operator rmul
var v6 = -v1;               // operator neg
var b = v1 == v2;           // operator eq
var x = v1[0];              // operator get
v1[1] = 5.0;                // operator set
var s = v1 as string;       // operator str
var n = len(v1);            // operator len
```

## 四级分发策略

### Level 1: 编译期特化（5ns）

编译器根据操作数类型生成特化字节码：

```kaubo
var a = 1 + 1;        // AddInt（纯整数加法）
var b = 1.0 + 2.0;    // AddFloat（纯浮点加法）
var c = 1 + 1.0;      // AddMixed（混合类型）
```

**字节码设计：**

```rust
AddInt, AddFloat, AddMixed    // 加法特化
SubInt, SubFloat, SubMixed    // 减法特化
MulInt, MulFloat, MulMixed    // 乘法特化
// ... 其他算术运算符
```

**适用场景：** 字面量表达式、类型已知的编译期常量

### Level 2: 内联缓存（15ns）

变量表达式使用 Shape ID 缓存：

```kaubo
var a = 1;
var b = 2;
var c = a + b;        // 缓存 (Int, Int) → add_int_int 函数
```

**实现：** 缓存 `(ShapeA, ShapeB) → 函数指针`，后续直接调用

**适用场景：** 变量表达式，类型稳定

### Level 3: 元表查找（30ns+）

自定义类型通过 Shape 查找元方法：

```kaubo
var v1 = Vector::new(3);
var v2 = Vector::new(3);
var v3 = v1 + v2;     // 查找 Vector 的 operator add
```

**实现：** `shape.operators.get(Operator::Add)`

**适用场景：** 自定义类型、缓存未命中

### Level 4: 错误处理

找不到匹配的运算符时返回**运行时错误**（panic）。

```kaubo
struct Point { x: float, y: float }

var p1 = Point { x: 1, y: 2 };
var p2 = Point { x: 3, y: 4 };

// Point 没有实现 operator add
var p3 = p1 + p2;   // 运行时错误：类型 Point 不支持运算符 '+'
```

**错误信息格式：**
```
OperatorError: 类型 'Point' 不支持运算符 '+'
  左侧类型: Point
  右侧类型: Point
  建议: 为 Point 实现 'operator add' 方法
```

### 错误类型分类

| 场景 | 错误类型 | 说明 |
|------|----------|------|
| 未实现运算符 | `OperatorError` | 类型没有对应的 operator xxx |
| 参数类型不匹配 | `TypeError` | 实现了运算符但参数类型不对 |
| 反向运算符也失败 | `OperatorError` | 左操作数和右操作数都不支持 |
| 元方法执行出错 | `RuntimeError` | operator 内部逻辑出错 |

## 支持的运算符

### 二元运算符（12 个）

| 元方法 | 语法 | 签名 | 分发级别 | 说明 |
|--------|------|------|----------|------|
| `add` | `a + b` | `\|self, other\| -> T` | L1/L2/L3 | 加法/拼接 |
| `sub` | `a - b` | `\|self, other\| -> T` | L1/L2/L3 | 减法 |
| `mul` | `a * b` | `\|self, other\| -> T` | L1/L2/L3 | 乘法 |
| `div` | `a / b` | `\|self, other\| -> T` | L1/L2/L3 | 除法（返回 float） |
| `mod` | `a % b` | `\|self, other\| -> T` | L1/L2/L3 | 取模 |
| `eq` | `a == b` | `\|self, other\| -> bool` | L1/L3 | 相等比较 |
| `lt` | `a < b` | `\|self, other\| -> bool` | L1/L3 | 小于比较 |
| `le` | `a <= b` | `\|self, other\| -> bool` | L3 | 小于等于 |
| `radd` | `b + a` | `\|self, other\| -> T` | L3 | 反向加法 |
| `rmul` | `b * a` | `\|self, other\| -> T` | L3 | 反向乘法 |
| `get` | `a[i]` | `\|self, index\| -> T` | L3 | 索引读取 |
| `set` | `a[i]=v` | `\|self, index, value\|` | L3 | 索引赋值 |

### 一元运算符（4 个）

| 元方法 | 语法 | 签名 | 分发级别 | 说明 |
|--------|------|------|----------|------|
| `neg` | `-a` | `\|self\| -> T` | L1/L3 | 一元负号 |
| `str` | `a as string` | `\|self\| -> string` | L3 | 字符串转换 |
| `len` | `len(a)` | `\|self\| -> int` | L3 | 长度/大小 |
| `call` | `a(args)` | `\|self, ...args\| -> T` | L3 | 可调用对象（变长参数） |

### 不支持重载（9 个）

| 运算符 | 原因 |
|--------|------|
| `and` / `or` | 短路求效特性无法重载 |
| `not` | 通常只返回 bool |
| `!=` / `>` / `>=` | 由 `==` / `<` / `<=` 推导 |
| `.field` | 编译期偏移，性能优先，**唯一字段访问方式** |
| `as bool` | 真值性判断专用 |

### Struct 字段访问规范

```kaubo
struct Point { x: float, y: float };
var p = Point { x: 1.0, y: 2.0 };

// ✅ 正确：使用 . 直接访问（编译期偏移，O(1)）
var x = p.x;
var y = p.y;

// ❌ 错误：整数索引访问字段（已移除）
// var x = p[0];  // 不再支持！使用 operator get 代替

// ❌ 错误：字符串键访问字段（将在 release 版移除）
// var x = p["x"];  // 不推荐！仅用于 JSON
```

**设计原则**：
1. **性能优先**：`.field` 是编译期偏移，~3ns，最快
2. **语义明确**：`obj.field` 一看就知道是字段访问
3. **简化实现**：移除字符串键查找，减少复杂度
4. **统一习惯**：不允许多种方式做同一件事

**operator get/set 用途**：
- 用于自定义容器的**动态索引**（如 `vector[0]`）
- 用于**计算属性**（如 `matrix[i, j]`）
- **不用于**直接暴露 struct 字段

### 构造函数约定

Kaubo **不强制**构造函数命名，依赖编码习惯：

```kaubo
impl Vector {
    // 推荐命名（但不强制）
    new: |size: int| -> Vector { ... }           // 默认构造
    from_list: |list: List<float>| -> Vector { ... }  // 从其他类型转换
    zero: || -> Vector { ... }                   // 特殊构造
}

// 使用
var v1 = Vector::new(3);
var v2 = Vector::from_list([1, 2, 3]);
var v3 = Vector::zero();

// 简单构造可直接用字面量
var v4 = Vector { data: [1, 2, 3] };
```

**命名建议**（团队/项目内统一即可）：
- `new` - 默认构造函数
- `from_xxx` - 从其他类型转换
- `with_xxx` - 带特定配置的构造
- `default`, `zero`, `empty` - 特殊值构造

## 内置类型的默认实现

内置类型（int/float/string/list）提供硬编码实现，走 Level 1/2：

```kaubo
// Int（硬编码）
impl int {
    operator add[int]: |self, other: int| -> int
    operator add[float]: |self, other: float| -> float
    operator sub, mul, div, mod, neg
    operator eq, lt, str
}

// String（硬编码）
impl string {
    operator add: |self, other: string| -> string    // 拼接
    operator get: |self, index: int| -> string       // 取字符
    operator mul: |self, count: int| -> string       // "a" * 3 = "aaa"
    operator eq, len, str
}

// List（硬编码）
impl List<T> {
    operator add: |self, other: List<T>| -> List<T>   // 拼接
    operator get, set, len, eq
}
```

## 性能对比

| 场景 | 分发级别 | 延迟 |
|------|----------|------|
| `1 + 1` | Level 1 | ~5 ns |
| `1 + 1.0` | Level 1 | ~10 ns |
| `a + b` (int) | Level 2 | ~15 ns |
| `v1 + v2` (Vector) | Level 3 | ~30-100 ns |
| 属性访问 `obj.x` | 直接偏移 | ~3 ns（最快，不重载） |

## 反向运算符

当左操作数不支持某运算符时，尝试右操作数的反向运算符：

```kaubo
var v = Vector::new(3);
var r = 2.0 * v;      // float 的 operator mul 不认识 Vector
                      // → 尝试 Vector 的 operator rmul
```

## 可调用对象（operator call）

让实例可以像函数一样被调用：

```kaubo
struct Adder {
    offset: int
}

impl Adder {
    new: |offset: int| -> Adder {
        return Adder { offset: offset };
    },
    
    // 一元 + 变长参数
    operator call: |self, x: int| -> int {
        return x + self.offset;
    }
}

var add5 = Adder::new(5);
var result = add5(10);   // 调用 operator call，返回 15
```

## 一元运算符示例

```kaubo
impl Vector {
    // 一元负号
    operator neg: |self| -> Vector {
        return self * -1.0;
    }
    
    // 长度（一元）
    operator len: |self| -> int {
        return len(self.data);
    }
    
    // 字符串转换（一元）
    operator str: |self| -> string {
        return "Vector(" + len(self) as string + ")";
    }
}

var v = Vector::new(3);
var neg = -v;            // operator neg（一元）
var n = len(v);          // operator len（一元）
var s = v as string;     // operator str（一元）
```

## 二元运算符示例

```kaubo
impl Vector {
    // 二元加法
    operator add: |self, other: Vector| -> Vector {
        var result = Vector::new(len(self));
        for i in range(0, len(self)) {
            result[i] = self[i] + other[i];
        }
        return result;
    }
    
    // 二元乘法（数乘）
    operator mul: |self, scalar: float| -> Vector {
        var result = Vector::new(len(self));
        for i in range(0, len(self)) {
            result[i] = self[i] * scalar;
        }
        return result;
    }
    
    // 反向乘法（scalar * vector）
    operator rmul: |self, scalar: float| -> Vector {
        return self * scalar;   // 调用上面的 operator mul
    }
    
    // 二元相等
    operator eq: |self, other: Vector| -> bool {
        if len(self) != len(other) { return false; }
        for i in range(0, len(self)) {
            if self[i] != other[i] { return false; }
        }
        return true;
    }
    
    // 二元索引读取
    operator get: |self, index: int| -> float {
        return self.data[index];
    }
    
    // 二元索引赋值（注意是 3 个参数）
    operator set: |self, index: int, value: float| {
        self.data[index] = value;
    }
}

var v1 = Vector::new(3);
var v2 = Vector::new(3);
var v3 = v1 + v2;           // operator add（二元）
var v4 = v1 * 2.0;          // operator mul（二元）
var v5 = 2.0 * v1;          // operator rmul（二元，反向）
var eq = v1 == v2;          // operator eq（二元）
var x = v1[0];              // operator get（二元）
v1[1] = 5.0;                // operator set（三元，但算二元运算）
```

## 实现计划

### Phase 1: 基础设施（3 天）
- 扩展 `ObjShape` 添加 `operators: HashMap<Operator, Value>`
- 定义 `Operator` 枚举
- `impl` 块支持 `operator xxx` 语法

### Phase 2: 核心运算符（3 天）
- `add`, `sub`, `mul`, `div`, `neg`
- `eq`, `lt`
- `get`, `set`
- Level 1 特化字节码

### Phase 3: 增强运算符（2 天）
- `mod`, `str`, `len`
- `radd`, `rmul`
- `call`（可调用对象）
- Level 2 内联缓存

### Phase 4: 内置类型（2 天）
- int/float 的特化实现
- string/list 的默认元方法

---

## 错误处理详细策略

### 场景 1：未实现运算符（运行时错误）

```kaubo
struct Point { x: float, y: float }

impl Point {
    // 只实现了加法，没实现减法
    operator add: |self, other: Point| -> Point {
        return Point { x: self.x + other.x, y: self.y + other.y };
    }
}

var p1 = Point { x: 1, y: 2 };
var p2 = Point { x: 3, y: 4 };

var p3 = p1 + p2;   // ✅ 正常：operator add 存在
var p4 = p1 - p2;   // ❌ 错误：Point 没有 operator sub
```

**错误信息：**
```
OperatorError: 类型 'Point' 不支持运算符 '-'
  --> test.kaubo:15:10
   |
15 | var p4 = p1 - p2;
   |          ^^^^^^^
   |
   = 左侧类型: Point
   = 右侧类型: Point
   = 建议: 为 Point 实现 'operator sub' 方法
```

### 场景 2：反向运算符尝试

```kaubo
impl Vector {
    operator mul: |self, scalar: float| -> Vector { ... }
    // 没有实现 rmul
}

var v = Vector::new(3);
var r1 = v * 2.0;       // ✅ 正常：operator mul 匹配
var r2 = 2.0 * v;       // ❌ 错误：
                        // 1. float 的 operator mul 不认识 Vector
                        // 2. Vector 的 operator rmul 不存在
```

**错误信息：**
```
OperatorError: 无法对类型 'float' 和 'Vector' 执行乘法
  = float 的 'operator mul' 不支持 Vector 类型
  = Vector 的 'operator rmul' 未实现
  = 建议: 为 Vector 实现 'operator rmul: |self, scalar: float| -> Vector'
```

### 场景 3：元方法内部出错

```kaubo
impl Vector {
    operator get: |self, index: int| -> float {
        // 主动抛出错误
        if index < 0 or index >= len(self) {
            panic("索引越界: " + index as string);
        }
        return self.data[index];
    }
}

var v = Vector::new(3);
var x = v[10];   // ❌ RuntimeError: 索引越界: 10
```

**错误信息：**
```
RuntimeError: 索引越界: 10
  --> test.kaubo:12:9
   |
12 | var x = v[10];
   |         ^^^^^
   |
   = 发生在 Vector 的 'operator get' 方法中
```

### 场景 4：类型不匹配（编译期警告）

```kaubo
impl Vector {
    // 期望 float 标量
    operator mul: |self, scalar: float| -> Vector { ... }
}

var v = Vector::new(3);
var r = v * "hello";   // ⚠️ 编译期警告：类型不匹配
                       // 虽然语法允许，但可能运行时错误
```

**编译期警告：**
```
Warning: 运算符 'mul' 的参数类型可能不匹配
  --> test.kaubo:8:9
   |
8  | var r = v * "hello";
   |         ^^^^^^^^^^^
   |
   = Vector::operator mul 期望参数类型: float
   = 实际传入类型: string
   = 这将在运行时导致 OperatorError
```

### 场景 5：链式运算中的错误

```kaubo
var result = v1 + v2 * v3;   // 如果 v2 * v3 失败，v1 + ... 不会执行
```

**执行顺序：**
1. 计算 `v2 * v3` → 如果失败，抛出错误，停止执行
2. 计算 `v1 + (结果)` → 如果失败，抛出错误

### 默认行为 vs 显式错误

**问题：** 是否应该提供默认实现？

```kaubo
// 选项 1：默认报错（当前设计）
Point1 + Point2  // 如果没有 operator add → 报错

// 选项 2：默认行为（如字段逐个相加）
Point1 + Point2  // 默认: Point { x: p1.x+p2.x, y: p1.y+p2.y }
```

**决策：** 采用选项 1（默认报错），因为：
- 显式优于隐式
- 避免意外的默认行为导致 bug
- 强制用户思考运算符的语义

## 设计决策记录

### 为什么用 `operator add` 而非 `operator+`？

- 避免解析器特殊处理符号
- 符合 "显式命名" 原则
- 易于扩展新运算符

### 为什么 `.field` 不支持重载？

```
obj.field     → GetField(index)    // 直接偏移，O(1)，~3ns
obj["field"]  → operator get       // 哈希查找，O(n)，~30ns
```

### 为什么 `and`/`or` 不支持重载？

```kaubo
a and b   // 如果 a 为 false，不计算 b（短路）

// 如果重载为函数：
operator_and(a, b)  // b 会被提前计算，失去短路特性
```
