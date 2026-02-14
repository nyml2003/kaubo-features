# 编程语言运算符重载设计方案调研报告

## 概述

本文档调研了主流编程语言中运算符重载的常见设计方案，分析各自的优缺点、性能考虑和扩展性，并为 Kaubo 脚本语言提供设计建议。

---

## 1. Python：Dunder Methods（双下划线方法）

### 设计原理

Python 使用特殊方法（magic methods/dunder methods）来实现运算符重载。当使用运算符时，Python 会自动调用对应的特殊方法：

```python
class Vector2D:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    
    def __add__(self, other):           # + 运算符
        return Vector2D(self.x + other.x, self.y + other.y)
    
    def __sub__(self, other):           # - 运算符
        return Vector2D(self.x - other.x, self.y - other.y)
    
    def __mul__(self, scalar):          # * 运算符
        return Vector2D(self.x * scalar, self.y * scalar)
    
    def __eq__(self, other):            # == 运算符
        return self.x == other.x and self.y == other.y
    
    def __repr__(self):                 # 字符串表示
        return f"Vector2D({self.x}, {self.y})"
```

### 完整运算符映射表

| 运算符 | 方法名 | 说明 |
|--------|--------|------|
| `+` | `__add__` | 加法 |
| `-` | `__sub__` | 减法 |
| `*` | `__mul__` | 乘法 |
| `/` | `__truediv__` | 真除法 |
| `//` | `__floordiv__` | 地板除 |
| `%` | `__mod__` | 取模 |
| `**` | `__pow__` | 幂运算 |
| `@` | `__matmul__` | 矩阵乘法 |
| `==` | `__eq__` | 等于 |
| `!=` | `__ne__` | 不等于 |
| `<` | `__lt__` | 小于 |
| `<=` | `__le__` | 小于等于 |
| `>` | `__gt__` | 大于 |
| `>=` | `__ge__` | 大于等于 |
| `+=` | `__iadd__` | 原地加法 |
| `-=` | `__isub__` | 原地减法 |
| `[]` | `__getitem__` | 索引获取 |
| `[]=` | `__setitem__` | 索引设置 |
| `len()` | `__len__` | 长度 |
| `str()` | `__str__` | 字符串转换 |
| `repr()` | `__repr__` | 表示形式 |

### 优点

1. **直观易懂**：方法名与运算符有明显对应关系，易于记忆
2. **一致性**：所有特殊方法都遵循 `__xxx__` 的命名约定
3. **灵活性高**：可以返回 `NotImplemented` 让 Python 尝试反向操作
4. **丰富的协议支持**：不仅支持运算符，还支持上下文管理器、迭代器、可调用对象等
5. **文档完善**：Python 社区有大量最佳实践文档

### 缺点

1. **命名冗长**：`__add__` 比简单的 `+` 要长得多
2. **运行时开销**：每个运算符调用都是方法调用，有一定开销
3. **不支持自定义运算符**：只能重载预定义的运算符
4. **左操作数优先**：如果左操作数不支持，才会尝试右操作数的 `__radd__` 等方法

### 性能考虑

- 每次运算符调用都是 Python 方法调用，有约 50-100ns 的开销
- 可以使用 `__slots__` 优化属性访问
- CPython 对常见类型有特殊优化（如整数、字符串）

### 扩展性

- 通过 `functools.total_ordering` 可以自动生成比较方法
- 可以与 `typing` 模块结合进行类型检查
- 支持反射运算（`__radd__`, `__rsub__` 等）处理左操作数不是自定义类型的情况

---

## 2. Rust：std::ops Trait 系统

### 设计原理

Rust 使用 Trait 系统来实现运算符重载，每个运算符对应一个 Trait：

```rust
use std::ops::{Add, Sub, Mul, Neg, Index};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector2D {
    x: f64,
    y: f64,
}

// 实现加法
impl Add for Vector2D {
    type Output = Self;
    
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

// 实现减法
impl Sub for Vector2D {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

// 实现与标量的乘法（非对称运算）
impl Mul<f64> for Vector2D {
    type Output = Self;
    
    fn mul(self, scalar: f64) -> Self::Output {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

// 实现一元取负
impl Neg for Vector2D {
    type Output = Self;
    
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

// 原地修改版本
impl std::ops::AddAssign for Vector2D {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}
```

### 核心 Trait 定义

```rust
pub trait Add<RHS = Self> {
    type Output;
    fn add(self, rhs: RHS) -> Self::Output;
}

pub trait Sub<RHS = Self> {
    type Output;
    fn sub(self, rhs: RHS) -> Self::Output;
}

pub trait Mul<RHS = Self> {
    type Output;
    fn mul(self, rhs: RHS) -> Self::Output;
}

pub trait Div<RHS = Self> {
    type Output;
    fn div(self, rhs: RHS) -> Self::Output;
}
```

### 优点

1. **类型安全**：编译时检查，无运行时开销
2. **明确的契约**：通过 Trait 定义清晰的接口
3. **灵活的非对称运算**：`Mul<f64>` 允许不同类型之间的运算
4. **关联类型控制返回值**：`type Output` 可以返回不同类型
5. **所有权语义**：`self` vs `&self` vs `&mut self` 明确资源管理
6. **零成本抽象**：运算符重载在编译期解析，无运行时开销

### 缺点

1. **学习曲线陡峭**：需要理解 Trait、关联类型、所有权等概念
2. **代码冗长**：每个运算符都需要完整的 impl 块
3. **不能重载所有运算符**：`&&`、`||`、`= `等不能重载
4. **对称性需要手动实现**：需要分别实现 `Add<A> for B` 和 `Add<B> for A`

### 性能考虑

- **零成本抽象**：运算符在编译期完全解析为函数调用
- **内联优化**：编译器可以内联小型运算符方法
- **无动态分派**：默认使用静态分派
- **Move 语义**：对于非 Copy 类型，运算符可以消耗左操作数以优化性能

### 扩展性

- 支持泛型约束：`fn process<T: Add<Output = T>>(a: T, b: T) -> T`
- 可以通过 derive 宏自动生成实现
- 与标准库无缝集成（`BTreeMap` 需要 `Ord` 等）

---

## 3. Lua：元表（Metatable）和元方法

### 设计原理

Lua 使用元表（metatable）机制，将运算符映射到元表中的特殊字段：

```lua
-- 创建向量类型
local Vector2D = {}
Vector2D.__index = Vector2D

function Vector2D.new(x, y)
    local self = setmetatable({}, Vector2D)
    self.x = x or 0
    self.y = y or 0
    return self
end

-- 定义元方法
function Vector2D.__add(a, b)
    return Vector2D.new(a.x + b.x, a.y + b.y)
end

function Vector2D.__sub(a, b)
    return Vector2D.new(a.x - b.x, a.y - b.y)
end

function Vector2D.__mul(a, b)
    -- 支持向量 * 标量
    if type(b) == "number" then
        return Vector2D.new(a.x * b, a.y * b)
    end
    return Vector2D.new(a.x * b.x, a.y * b.y)
end

function Vector2D.__eq(a, b)
    return a.x == b.x and a.y == b.y
end

function Vector2D.__tostring(v)
    return string.format("Vector2D(%f, %f)", v.x, v.y)
end

-- 使用
local v1 = Vector2D.new(1, 2)
local v2 = Vector2D.new(3, 4)
local v3 = v1 + v2  -- 调用 __add
print(v3)           -- 调用 __tostring
```

### 完整元方法列表

| 元方法 | 运算符/操作 | 说明 |
|--------|-------------|------|
| `__add` | `+` | 加法 |
| `__sub` | `-` | 减法 |
| `__mul` | `*` | 乘法 |
| `__div` | `/` | 除法 |
| `__mod` | `%` | 取模 |
| `__pow` | `^` | 幂运算 |
| `__unm` | `-`（一元） | 取负 |
| `__idiv` | `//` | 整除 |
| `__band` | `&` | 按位与 |
| `__bor` | `\|` | 按位或 |
| `__bxor` | `~` | 按位异或 |
| `__bnot` | `~`（一元） | 按位非 |
| `__shl` | `<<` | 左移 |
| `__shr` | `>>` | 右移 |
| `__concat` | `..` | 连接 |
| `__len` | `#` | 长度 |
| `__eq` | `==` | 等于 |
| `__lt` | `<` | 小于 |
| `__le` | `<=` | 小于等于 |
| `__index` | `table[key]` | 索引访问 |
| `__newindex` | `table[key] = value` | 索引赋值 |
| `__call` | `func(args)` | 调用 |
| `__tostring` | `tostring()` | 字符串转换 |

### 优点

1. **简洁灵活**：元表是普通的 Lua 表，可以动态修改
2. **运行时动态性**：可以在运行时添加/修改运算符行为
3. **统一机制**：所有对象类型（包括基础类型）都使用相同的元表机制
4. **非侵入式**：可以为任何表设置元表，包括库中的表
5. **左操作数优先**：先检查左操作数的元表，再检查右操作数

### 缺点

1. **运行时开销**：每次运算符操作都需要元表查找
2. **无编译期检查**：错误只能在运行时发现
3. **调试困难**：元方法调用栈可能难以理解
4. **性能波动**：元表查找增加了间接层

### 性能考虑

- 元表查找增加了一层间接访问
- LuaJIT 可以优化常见的元表模式
- 内建类型（number、string）有专门的快速路径
- 表和 userdata 的元表查找开销约为 5-10%

### 扩展性

- 可以创建元表的元表（多层元表）
- 支持 `__index` 为表或函数，实现继承或代理
- 弱表（weak table）可以与元表结合实现复杂数据结构

---

## 4. JavaScript：Symbol 和有限支持

### 设计原理

JavaScript 本身**不支持传统意义上的运算符重载**，但提供了一些 Symbol 和类型转换机制：

```javascript
class Vector2D {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    
    // 类型转换 - 用于数学运算时的自动转换
    valueOf() {
        // 返回一个数值，用于 + - * / 等运算
        // 但这会丢失对象结构，通常不实用
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
    
    // 字符串转换
    toString() {
        return `Vector2D(${this.x}, ${this.y})`;
    }
    
    // 使用 Symbol.toPrimitive 提供更精细的控制
    [Symbol.toPrimitive](hint) {
        if (hint === 'number') {
            return this.magnitude();
        }
        if (hint === 'string') {
            return this.toString();
        }
        return this;
    }
    
    magnitude() {
        return Math.sqrt(this.x ** 2 + this.y ** 2);
    }
    
    // 只能通过显式方法调用
    add(other) {
        return new Vector2D(this.x + other.x, this.y + other.y);
    }
    
    static add(a, b) {
        return new Vector2D(a.x + b.x, a.y + b.y);
    }
}

// 使用 - 必须使用显式方法调用
const v1 = new Vector2D(1, 2);
const v2 = new Vector2D(3, 4);
const v3 = v1.add(v2);  // 不能使用 v1 + v2
```

### ES6+ 提供的 Symbol 相关功能

```javascript
class CustomCollection {
    // 使对象可迭代
    *[Symbol.iterator]() {
        for (const item of this.data) {
            yield item;
        }
    }
    
    // instanceof 操作符行为
    static [Symbol.hasInstance](instance) {
        return instance && typeof instance.specialMethod === 'function';
    }
    
    // Object.prototype.toString 的返回值
    get [Symbol.toStringTag]() {
        return 'CustomCollection';
    }
}
```

### 有限运算符重载的变通方案

```javascript
// 方案1：使用 Proxy 和库级转换
// 需要外部工具如 Babel 插件或源码转换

// 方案2：链式方法调用
const v3 = v1.add(v2).sub(v4).mul(2);

// 方案3：函数式 API
const {add, sub, mul} = Vector2D;
const v3 = mul(sub(add(v1, v2), v4), 2);
```

### 优点

1. **简单安全**：避免了运算符重载可能带来的复杂性和滥用
2. **明确的语义**：所有操作都是显式方法调用，代码意图清晰
3. **类型转换控制**：`Symbol.toPrimitive` 提供有限的自定义能力

### 缺点

1. **无法真正的运算符重载**：`+ - * /` 等运算符不能自定义
2. **数学代码冗长**：`v1.add(v2)` 比 `v1 + v2` 更冗长
3. **与内建类型不一致**：数组有 `+` 的奇怪行为，但自定义类型没有
4. **库间互操作性差**：每个库有自己的方法命名约定

### 性能考虑

- 无额外的运算符重载开销（因为不支持）
- 方法调用有正常的 JavaScript 函数调用开销
- `valueOf` 和 `toString` 转换有轻微开销

### 扩展性

- 需要借助外部工具（如 Babel 插件）实现真正的运算符重载
- 社区有一些提案（如 operator overloading proposal），但尚未标准化

---

## 5. C++：函数重载方式

### 设计原理

C++ 使用 `operator` 关键字定义运算符函数，可以是成员函数或全局函数：

```cpp
#include <iostream>

class Vector2D {
public:
    double x, y;
    
    Vector2D(double x = 0, double y = 0) : x(x), y(y) {}
    
    // 成员函数方式重载二元运算符
    Vector2D operator+(const Vector2D& other) const {
        return Vector2D(x + other.x, y + other.y);
    }
    
    Vector2D operator-(const Vector2D& other) const {
        return Vector2D(x - other.x, y - other.y);
    }
    
    // 成员函数方式重载与标量的乘法
    Vector2D operator*(double scalar) const {
        return Vector2D(x * scalar, y * scalar);
    }
    
    // 成员函数方式重载一元运算符
    Vector2D operator-() const {
        return Vector2D(-x, -y);
    }
    
    // 比较运算符
    bool operator==(const Vector2D& other) const {
        return x == other.x && y == other.y;
    }
    
    // 复合赋值运算符（返回引用支持链式赋值）
    Vector2D& operator+=(const Vector2D& other) {
        x += other.x;
        y += other.y;
        return *this;
    }
    
    // 下标运算符
    double& operator[](size_t index) {
        return index == 0 ? x : y;
    }
    
    // 函数调用运算符（仿函数）
    double operator()(double t) const {
        return x * t + y * (1 - t);
    }
    
    // 输出流运算符（必须是友元或全局函数）
    friend std::ostream& operator<<(std::ostream& os, const Vector2D& v) {
        return os << "Vector2D(" << v.x << ", " << v.y << ")";
    }
};

// 全局函数方式（实现左操作数为内置类型的情况）
Vector2D operator*(double scalar, const Vector2D& vec) {
    return vec * scalar;  // 复用成员函数实现
}
```

### 可重载与不可重载的运算符

| 可重载 | 不可重载 |
|--------|----------|
| `+ - * / %` | `::`（作用域解析） |
| `++ --` | `.`（成员访问） |
| `== != < > <= >=` | `.*`（成员指针访问） |
| `= += -= *= /=` | `?:`（三元条件） |
| `<< >>` | `sizeof` |
| `[] () ->` | `typeid` |
| `new delete` | |
| `& \| ~ ^ !` | |
| `&& \|\|`（不建议）| |

### 优点

1. **极高的灵活性**：可以重载绝大多数运算符
2. **零开销**：运算符重载是编译期特性，无运行时开销
3. **与内建类型一致**：自定义类型可以使用与 `int` 相同的语法
4. **支持全局函数**：可以处理左操作数为内置类型的情况
5. **丰富的运算符集**：包括 `[]`, `()`, `->` 等特殊运算符

### 缺点

1. **容易被滥用**：`+` 可以定义为减法，导致代码难以理解
2. **优先级固定**：不能改变运算符的优先级和结合性
3. **逻辑运算符短路失效**：重载的 `&&` 和 `\|\|` 不再有短路特性
4. **隐式类型转换问题**：可能产生意外的临时对象和类型转换
5. **学习曲线**：需要理解成员 vs 全局函数、const 正确性等

### 性能考虑

- **零运行时开销**：运算符调用在编译期解析为函数调用
- **内联优化**：小型运算符函数可以被编译器内联
- **返回值优化（RVO）**：现代编译器可以优化返回值的拷贝
- **注意避免临时对象**：`a + b + c` 可能创建多个临时对象

### 扩展性

- 模板运算符：`template<typename T> Vector2D operator*(T scalar)`
- 与泛型编程无缝集成
- 表达式模板（Expression Templates）可以实现延迟求值

---

## 6. Ruby：方法名方式

### 设计原理

Ruby 中运算符本质上是方法调用，可以直接定义以运算符为名的方法：

```ruby
class Vector2D
    attr_accessor :x, :y
    
    def initialize(x, y)
        @x = x
        @y = y
    end
    
    # 重载 + 运算符
    def +(other)
        Vector2D.new(@x + other.x, @y + other.y)
    end
    
    # 重载 - 运算符
    def -(other)
        Vector2D.new(@x - other.x, @y - other.y)
    end
    
    # 重载 * 运算符（标量乘法）
    def *(scalar)
        if scalar.is_a?(Numeric)
            Vector2D.new(@x * scalar, @y * scalar)
        else
            raise TypeError, "Expected numeric, got #{scalar.class}"
        end
    end
    
    # 重载 == 运算符
    def ==(other)
        return false unless other.is_a?(Vector2D)
        @x == other.x && @y == other.y
    end
    
    # 重载 [] 运算符
    def [](index)
        case index
        when 0 then @x
        when 1 then @y
        else nil
        end
    end
    
    # 重载 []= 运算符
    def []=(index, value)
        case index
        when 0 then @x = value
        when 1 then @y = value
        end
    end
    
    # 字符串表示
    def to_s
        "Vector2D(#{@x}, #{@y})"
    end
    
    # 检查可比较性
    def <=>(other)
        # 返回 -1, 0, 或 1
        magnitude <=> other.magnitude
    end
    
    def magnitude
        Math.sqrt(@x**2 + @y**2)
    end
end

# 使用
v1 = Vector2D.new(1, 2)
v2 = Vector2D.new(3, 4)
v3 = v1 + v2  # 等同于 v1.+(v2)
puts v3       # 输出: Vector2D(4, 6)
```

### 可重载与不可重载的运算符

| 可重载 | 不可重载 |
|--------|----------|
| `+ - * / % **` | `&& \|\|`（逻辑运算） |
| `== != <=>` | `and or not`（关键字） |
| `< > <= >=` | `::`（常量查找） |
| `[] []=` | `=`（赋值） |
| `<< >>` | `?:`（无三元运算符） |
| `~ & \| ^` | |
| `+= -= *=`（通过 + - * 间接支持）| |

### 优点

1. **语法简洁**：直接使用方法名，无需特殊关键字
2. **语义清晰**：运算符就是方法，符合 Ruby 的"一切皆对象"哲学
3. **动态性**：运行时动态分派，支持鸭子类型
4. **比较模块集成**：`Comparable` 模块可以通过 `<=>` 自动生成其他比较方法

### 缺点

1. **非交换性**：`3 + obj` 不等于 `obj + 3`，需要额外处理
2. **有限的可重载运算符**：逻辑运算符不能重载
3. **运行时错误**：类型检查在运行时进行
4. **性能开销**：方法调用比 C++ 的运算符有更高开销

### 性能考虑

- 方法调用有约 100-200ns 的开销
- 可以使用 `method_cache` 优化重复调用
- MRI (CRuby) 有全局方法缓存
- JRuby/TruffleRuby 有更好的 JIT 优化

### 扩展性

- `Comparable` 模块可以自动生成比较方法
- `coerce` 方法可以处理混合类型运算
- 可以动态添加/修改运算符行为

---

## 方案对比总结

| 特性 | Python | Rust | Lua | JavaScript | C++ | Ruby |
|------|--------|------|-----|------------|-----|------|
| **实现方式** | 特殊方法 | Trait | 元表 | 不支持/有限 | 函数重载 | 方法名 |
| **类型安全** | 动态 | 静态 | 动态 | 动态 | 静态 | 动态 |
| **运行时开销** | 中 | 零 | 中 | 无（不支持）| 零 | 中 |
| **编译期检查** | 否 | 是 | 否 | 否 | 是 | 否 |
| **学习难度** | 低 | 高 | 低 | 低 | 中 | 低 |
| **扩展性** | 高 | 高 | 很高 | 低 | 高 | 高 |
| **自定义运算符** | 否 | 否 | 否 | 否 | 否 | 否 |
| **非对称运算** | 是 | 是 | 是 | 否 | 是 | 是 |

---

## 对 Kaubo 的建议

### Kaubo 的现状分析

Kaubo 是一个基于 Rust 实现的脚本语言，具有以下特点：

1. **NaN-boxed Value 表示**：高效的值表示方式
2. **Shape-based 对象系统**：用于 struct 类型
3. **字节码虚拟机**：解释执行
4. **模块系统**：支持代码组织
5. **协程支持**：协作式多任务

### 推荐方案：混合 Lua 元表 + Python Dunder 方法

基于 Kaubo 的设计目标和现有架构，建议采用**元表（Metatable）**方案，原因如下：

#### 1. 为什么选元表方案

| 考量因素 | 分析 |
|----------|------|
| **与现有架构匹配** | Kaubo 已有 `ObjShape` 和 `ObjStruct`，可以自然扩展为元表系统 |
| **动态语言特性** | Kaubo 是动态类型脚本语言，元表提供运行时灵活性 |
| **性能可控** | 可以在 VM 中实现快速路径（fast path）优化常见操作 |
| **用户友好** | 与 Lua 类似的 API 被广泛接受，学习成本低 |
| **扩展性** | 元表是普通的表，可以动态修改和组合 |

#### 2. 具体设计方案

##### 语法设计

```kaubo
// 定义一个向量类型
struct Vector2D {
    x: float,
    y: float,
}

// 创建类型的元表
let Vector2DMeta = {
    __add: fn(a, b) -> Vector2D {
        return Vector2D {
            x: a.x + b.x,
            y: a.y + b.y,
        }
    },
    
    __sub: fn(a, b) -> Vector2D {
        return Vector2D {
            x: a.x - b.x,
            y: a.y - b.y,
        }
    },
    
    __mul: fn(a, b) -> Vector2D {
        // 支持向量 * 标量
        if type(b) == "number" {
            return Vector2D {
                x: a.x * b,
                y: a.y * b,
            }
        }
        // 向量 * 向量（点乘）
        return a.x * b.x + a.y * b.y
    },
    
    __eq: fn(a, b) -> bool {
        return a.x == b.x && a.y == b.y
    },
    
    __tostring: fn(v) -> string {
        return "Vector2D(${v.x}, ${v.y})"
    },
}

// 关联元表到类型
setmetatable(Vector2D, Vector2DMeta)

// 使用
let v1 = Vector2D { x: 1.0, y: 2.0 }
let v2 = Vector2D { x: 3.0, y: 4.0 }
let v3 = v1 + v2  // 调用 __add
```

##### 核心实现思路

1. **扩展 ObjShape 支持元表指针**

```rust
// kaubo-core/src/runtime/object.rs
pub struct ObjShape {
    pub shape_id: u16,
    pub name: String,
    pub field_names: Vec<String>,
    pub methods: Vec<*mut ObjFunction>,
    pub method_names: HashMap<String, u8>,
    // 新增：元表指针
    pub metatable: Option<Value>,  // 指向包含元方法的表
}
```

2. **VM 中运算符指令处理**

```rust
// kaubo-core/src/runtime/vm.rs
fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
    // 1. 快速路径：基础类型
    if let (Some(ai), Some(bi)) = (a.as_int(), b.as_int()) {
        return Ok(Value::int(ai + bi));
    }
    
    // 2. 检查左操作数的元表
    if let Some(meta) = self.get_metatable(a) {
        if let Some(add_fn) = self.get_metamethod(meta, "__add") {
            return self.call_metamethod(add_fn, &[a, b]);
        }
    }
    
    // 3. 检查右操作数的元表（反向运算）
    if let Some(meta) = self.get_metatable(b) {
        if let Some(add_fn) = self.get_metamethod(meta, "__add") {
            return self.call_metamethod(add_fn, &[a, b]);
        }
    }
    
    // 4. 错误：不支持的运算
    Err(format!("Cannot add {} and {}", a.type_name(), b.type_name()))
}
```

3. **内置类型的默认元表**

```rust
// 为内置类型（如 List、String）预定义元表
// 在 VM 初始化时设置
impl VM {
    fn init_builtin_metatables(&mut self) {
        // List 的元表
        let list_meta = self.create_table();
        list_meta.set("__len", self.get_native_fn(list_length));
        list_meta.set("__index", self.get_native_fn(list_index));
        list_meta.set("__tostring", self.get_native_fn(list_to_string));
        self.set_builtin_metatable(ValueType::List, list_meta);
        
        // String 的元表
        let string_meta = self.create_table();
        string_meta.set("__add", self.get_native_fn(string_concat));
        string_meta.set("__len", self.get_native_fn(string_length));
        self.set_builtin_metatable(ValueType::String, string_meta);
    }
}
```

##### 支持的元方法

| 元方法 | 运算符/操作 | 优先级说明 |
|--------|-------------|------------|
| `__add` | `+` | 高（基础运算）|
| `__sub` | `-` | 高 |
| `__mul` | `*` | 高 |
| `__div` | `/` | 高 |
| `__mod` | `%` | 高 |
| `__pow` | `^` | 高 |
| `__unm` | `-`（一元） | 高 |
| `__eq` | `==` | 高 |
| `__lt` | `<` | 中 |
| `__le` | `<=` | 中 |
| `__gt` | `>`（通过 `__lt`） | 中 |
| `__ge` | `>=`（通过 `__le`） | 中 |
| `__index` | `obj[key]` | 高（基础操作）|
| `__newindex` | `obj[key] = value` | 高 |
| `__len` | `#obj` 或 `len()` | 中 |
| `__tostring` | 字符串转换 | 中 |
| `__call` | `obj(args)` | 中 |
| `__iter` | `for ... in` | 低 |

#### 3. 性能优化策略

1. **元表缓存**
   - 在 Value 或 ObjShape 中缓存元表指针
   - 避免每次运算符调用都进行哈希查找

2. **快速路径优化**
   ```rust
   // 为常见类型（int, float, string, list）保留内联快速路径
   fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
       // 快速路径：两个都是整数
       if a.is_int() && b.is_int() {
           // 直接整数加法，不走元表
           return Ok(Value::int(a.as_int().unwrap() + b.as_int().unwrap()));
       }
       // ... 元表查找
   }
   ```

3. **内联缓存（Inline Cache）**
   - 在字节码指令中缓存上次使用的元方法
   - 如果类型匹配，直接调用缓存的方法

4. **Shape 级元表**
   - 所有相同 Shape 的 struct 实例共享同一个元表
   - 元表存储在 Shape 中，而非每个实例

#### 4. 与现有系统的集成

1. **Struct 类型**：通过 Shape 的 metatable 字段支持
2. **List 类型**：内置支持，使用预定义元表
3. **String 类型**：内置支持，使用预定义元表
4. **原生类型**：不允许修改元表（安全性考虑）

#### 5. 示例代码

```kaubo
// 完整的向量库示例
module vector {
    pub struct Vec2 {
        x: float,
        y: float,
    }
    
    // 创建元表
    let META = {
        __add: fn(a, b) -> Vec2 {
            return Vec2 { x: a.x + b.x, y: a.y + b.y }
        },
        
        __sub: fn(a, b) -> Vec2 {
            return Vec2 { x: a.x - b.x, y: a.y - b.y }
        },
        
        __mul: fn(a, b) {
            // 标量乘法
            if type(b) == "number" {
                return Vec2 { x: a.x * b, y: a.y * b }
            }
            // 向量乘法（点乘）
            return a.x * b.x + a.y * b.y
        },
        
        __div: fn(a, b) -> Vec2 {
            if type(b) == "number" {
                return Vec2 { x: a.x / b, y: a.y / b }
            }
            panic("Cannot divide Vec2 by non-number")
        },
        
        __unm: fn(a) -> Vec2 {
            return Vec2 { x: -a.x, y: -a.y }
        },
        
        __eq: fn(a, b) -> bool {
            return a.x == b.x && a.y == b.y
        },
        
        __tostring: fn(v) -> string {
            return "Vec2(${v.x}, ${v.y})"
        },
        
        __len: fn(v) -> float {
            return sqrt(v.x * v.x + v.y * v.y)
        },
    }
    
    // 工厂函数
    pub fn new(x: float, y: float) -> Vec2 {
        let v = Vec2 { x: x, y: y }
        setmetatable(v, META)
        return v
    }
    
    // 静态方法
    pub fn dot(a: Vec2, b: Vec2) -> float {
        return a.x * b.x + a.y * b.y
    }
    
    pub fn normalize(v: Vec2) -> Vec2 {
        let len = #v  // 调用 __len
        return v / len  // 调用 __div
    }
}

// 使用
use vector

let a = vector.new(3.0, 4.0)
let b = vector.new(1.0, 2.0)
let c = a + b * 2.0

println(c)  // 输出: Vec2(5, 8)
println(#c) // 输出: 9.43398...
```

### 替代方案考虑

| 方案 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **元表（推荐）** | 灵活、与 Lua 兼容、易扩展 | 运行时开销 | 动态脚本语言 |
| **Trait（Rust 风格）** | 类型安全、零开销 | 需要静态类型系统 | 静态类型语言 |
| **特殊方法（Python 风格）** | 直观、广泛认知 | 命名冗长 | 面向对象语言 |
| **不支持运算符重载** | 简单、安全 | 数学代码冗长 | 极简语言 |

### 结论

对于 Kaubo 这样的动态类型脚本语言，**元表方案**是最佳选择，因为它：

1. 与 Kaubo 现有的 Shape-based 对象系统自然融合
2. 提供了 Lua 用户熟悉的 API 设计
3. 可以通过快速路径优化实现可接受的性能
4. 支持运行时的动态修改，符合脚本语言特性
5. 实现复杂度适中，不会大幅增加代码复杂性

---

## 参考资源

- [Python Data Model](https://docs.python.org/3/reference/datamodel.html)
- [Rust std::ops](https://doc.rust-lang.org/std/ops/index.html)
- [Lua 5.4 Reference Manual - Metatables and Metamethods](https://www.lua.org/manual/5.4/manual.html#2.4)
- [ECMAScript 6 - New OOP Features](https://exploringjs.com/es6/ch_oop-besides-classes.html)
- [C++ Operator Overloading](https://en.cppreference.com/w/cpp/language/operators)
- [Ruby Operator Overloading](https://www.geeksforgeeks.org/ruby/operator-overloading-in-ruby/)
