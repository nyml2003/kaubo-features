# Kaubo 运算符重载

> 四级分发策略：编译期特化 → 内联缓存 → 元表查找 → 错误

---

## 设计决策

### 为什么用 `operator add` 而非 `__add__`？

遵循 [原则 #8](../../../00-principles/README.md)：显式命名优于隐式约定。

- ✅ `operator add` - 清晰、可读、易搜索
- ❌ `__add__` - 隐式约定，AI易误用

### 为什么 `.field` 不支持重载？

性能优先：

| 方式 | 机制 | 延迟 |
|------|------|------|
| `obj.field` | 编译期偏移 | ~3ns |
| `obj[key]` | 哈希查找 | ~30ns |

`.field` 是**唯一**的 struct 字段访问方式，不重载。

---

## 语法

```kaubo
struct Vector {
    data: List<float>
}

impl Vector {
    // 构造函数（普通方法）
    new: |size: int| -> Vector { ... }
    
    // 运算符重载
    operator add: |self, other: Vector| -> Vector { ... }
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

---

## 四级分发策略

### Level 1: 编译期特化（~5ns）

编译器根据操作数类型生成特化字节码：

```kaubo
var a = 1 + 1;        // AddInt（纯整数加法）
var b = 1.0 + 2.0;    // AddFloat（纯浮点加法）
var c = 1 + 1.0;      // AddMixed（混合类型）
```

**字节码**：`AddInt`, `AddFloat`, `AddMixed`, `SubInt`, ...

### Level 2: 内联缓存（~15ns）

变量表达式使用 Shape ID 缓存：

```kaubo
var a = 1;
var b = 2;
var c = a + b;        // 缓存 (Int, Int) → add_int_int 函数
```

实现：缓存 `(ShapeA, ShapeB) → 函数指针`

**状态**：🚧 基础设施就绪，待集成到指令

### Level 3: 元表查找（~30-100ns）

自定义类型通过 Shape 查找元方法：

```kaubo
var v1 = Vector::new(3);
var v2 = Vector::new(3);
var v3 = v1 + v2;     // 查找 Vector 的 operator add
```

实现：`shape.operators.get(Operator::Add)`

### Level 4: 错误处理

找不到匹配运算符时返回**运行时错误**：

```kaubo
struct Point { x: float, y: float }
var p1 = Point { x: 1, y: 2 };
var p2 = Point { x: 3, y: 4 };
var p3 = p1 + p2;   // ❌ OperatorError: 类型 'Point' 不支持运算符 '+'
```

---

## 支持的运算符

### 二元运算符（12个）

| 元方法 | 语法 | 签名 | 级别 |
|--------|------|------|------|
| `add` | `a + b` | `\|self, other\| -> T` | L1/L2/L3 |
| `sub` | `a - b` | `\|self, other\| -> T` | L1/L2/L3 |
| `mul` | `a * b` | `\|self, other\| -> T` | L1/L2/L3 |
| `div` | `a / b` | `\|self, other\| -> T` | L1/L2/L3 |
| `mod` | `a % b` | `\|self, other\| -> T` | L3 |
| `eq` | `a == b` | `\|self, other\| -> bool` | L1/L3 |
| `lt` | `a < b` | `\|self, other\| -> bool` | L1/L3 |
| `le` | `a <= b` | `\|self, other\| -> bool` | L3 |
| `radd` | `b + a` | `\|self, other\| -> T` | L3 |
| `rmul` | `b * a` | `\|self, other\| -> T` | L3 |
| `get` | `a[i]` | `\|self, index\| -> T` | L3 |
| `set` | `a[i]=v` | `\|self, index, value\|` | L3 |

### 一元运算符（4个）

| 元方法 | 语法 | 签名 | 级别 |
|--------|------|------|------|
| `neg` | `-a` | `\|self\| -> T` | L1/L3 |
| `str` | `a as string` | `\|self\| -> string` | L3 |
| `len` | `len(a)` | `\|self\| -> int` | L3 |
| `call` | `a(args)` | `\|self, ...args\| -> T` | L3 |

### 不支持重载（9个）

| 运算符 | 原因 |
|--------|------|
| `and` / `or` | 短路求值特性无法重载 |
| `not` | 通常只返回 bool |
| `!=` / `>` / `>=` | 由 `==` / `<` / `<=` 推导 |
| `.field` | 编译期偏移，性能优先 |
| `as bool` | 真值性判断专用 |

---

## 内置类型实现

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

---

## 反向运算符

当左操作数不支持某运算符时，尝试右操作数的反向运算符：

```kaubo
var v = Vector::new(3);
var r = 2.0 * v;      // float 的 operator mul 不认识 Vector
                      // → 尝试 Vector 的 operator rmul
```

---

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
    
    operator call: |self, x: int| -> int {
        return x + self.offset;
    }
}

var add5 = Adder::new(5);
var result = add5(10);   // 调用 operator call，返回 15
```

---

## 实现路线图

| 阶段 | 功能 | 状态 |
|------|------|------|
| Phase 1 | 基础设施（Shape扩展、Operator枚举）| ✅ |
| Phase 2 | 核心运算符（add/sub/mul/div/neg/eq/lt/get/set）| ✅ |
| Phase 3 | 增强运算符（mod/str/len/radd/rmul/call）| ✅ |
| Phase 3 | Level 2 内联缓存 | 🚧 基础设施就绪 |
| Phase 4 | 内置类型特化实现 | ✅ |

---

## 参考

- [调研报告](../../../30-lessons/operator-overloading-research.md) - Python/Rust/Lua/JS/C++/Ruby方案对比
- [核心原则](../../../00-principles/README.md) - 显式命名原则
- [VM技术债务](../../tech-debt/README.md) - 内联缓存待办
