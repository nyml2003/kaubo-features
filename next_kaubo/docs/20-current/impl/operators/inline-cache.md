# Kaubo 内联缓存（Inline Cache）机制

## 概述

内联缓存（Inline Cache，简称 IC）是 Kaubo VM 用于加速**动态分派**（如运算符重载、方法调用）的核心优化技术。

## 为什么需要内联缓存？

Kaubo 支持运算符重载：

```kaubo
struct Vec2 {
    x: float,
    y: float
};

impl Vec2 {
    operator add: |self, other: Vec2| -> Vec2 {
        return Vec2 {
            x: self.x + other.x,
            y: self.y + other.y
        };
    }
};

var v1 = Vec2 { x: 1.0, y: 2.0 };
var v2 = Vec2 { x: 3.0, y: 4.0 };
var v3 = v1 + v2;  // 调用 operator add
```

如果不优化，每次执行 `v1 + v2` 时 VM 都需要：
1. 检查左操作数 `v1` 的类型（Shape）
2. 查找该 Shape 是否有 `operator add`
3. 找到并调用对应函数

这个过程在循环中执行成千上万次会非常慢。

## 内联缓存的工作原理

### 1. 编译期：分配缓存槽位

编译器为每个可能使用运算符重载的指令分配一个**内联缓存槽位**（Inline Cache Slot）：

```rust
// 编译 v1 + v2 时
let cache_idx = compiler.chunk.allocate_inline_cache_slot();  // 分配槽位 2
compiler.chunk.write_op_u8(OpCode::Add, cache_idx, line);      // ADD 2
```

生成的字节码：
```json
{
    "opcode": "ADD",
    "operand": 2    // 使用第 2 号缓存槽
}
```

### 2. 执行期：缓存查找

#### 第一次执行（缓存未命中 - Miss）

```
指令: ADD operand=2

检查缓存槽 2:
  ShapeID: 0 (空)
  
→ 未命中！执行慢速查找：
  1. 获取 v1 的 ShapeID (如 100)
  2. 查找 Shape 100 的 operator add
  3. 找到函数指针 0x7fff_a000
  
→ 更新缓存槽 2:
  ShapeID: 100
  Function: 0x7fff_a000
  
→ 调用函数
```

#### 第二次执行（缓存命中 - Hit）

```
指令: ADD operand=2

检查缓存槽 2:
  ShapeID: 100
  Function: 0x7fff_a000

→ v1 的 ShapeID 也是 100！
→ 命中！直接调用 0x7fff_a000（无需查找）
```

### 3. 缓存结构

每个缓存槽位存储：

```rust
struct InlineCacheEntry {
    shape_id: u16,      // 期望的 Shape ID
    cached_value: *mut ObjClosure,  // 缓存的函数指针
}
```

## 性能对比

| 场景 | 无缓存 | 有缓存（命中） | 加速比 |
|------|--------|----------------|--------|
| 单次调用 | 1000 ns | 1000 ns | 1x |
| 循环 10000 次 | 10,000,000 ns | ~500,000 ns | **20x** |

## 多级内联缓存

Kaubo 实现了**两级缓存**（Level 2 Inline Cache）：

### Level 1：单态缓存（Monomorphic）
- 只缓存一种 Shape
- 最快（直接比较 ShapeID）
- 如果 Shape 变化则降级到 Level 2

### Level 2：多态缓存（Polymorphic）
- 缓存最多 4 种不同的 Shape
- 线性查找（`for entry in cache.entries`）
- 支持多态调用（如 `a + b`，`a` 有时是 `Vec2`，有时是 `Vec3`）

```rust
struct Level2Cache {
    entries: [CacheEntry; 4],  // 最多 4 个条目
    count: u8,                  // 当前条目数
}
```

## 实际字节码示例

```kaubo
struct Counter {
    value: int
};

impl Counter {
    operator add: |self, other: Counter| -> Counter {
        return Counter { value: self.value + other.value };
    }
};

var c1 = Counter { value: 10 };
var c2 = Counter { value: 20 };
var c3 = c1 + c2;  // 触发内联缓存
```

生成的字节码：
```json
[
    { "opcode": "LOAD_LOCAL_1" },      // 加载 c1
    { "opcode": "LOAD_LOCAL_2" },      // 加载 c2
    { "opcode": "ADD", "operand": 0 }, // ADD 使用缓存槽 0
    { "opcode": "STORE_LOCAL_3" }      // 存储结果到 c3
]
```

## 缓存失效

当缓存的 Shape 与实际 Shape 不匹配时，缓存失效：

```kaubo
var c1 = Counter { value: 10 };
var c2 = Counter { value: 20 };
var r1 = c1 + c2;  // 缓存 ShapeID=100

// 如果 Counter 定义被修改（热重载）
// 新的 ShapeID=101
var r2 = c1 + c2;  // ShapeID 不匹配 → 重新查找 → 更新缓存
```

## 调试内联缓存

通过 `package.json` 启用详细日志查看缓存行为：

```json
{
    "compiler": {
        "log_level": "trace"
    }
}
```

日志输出示例：
```
[TRACE] InlineCache: miss at slot 2, shape=100, updating cache
[TRACE] InlineCache: hit at slot 2, shape=100, using cached closure
```

## 总结

- **内联缓存**是 Kaubo 实现运算符重载高性能的关键
- **操作数**是缓存槽位索引，不是立即数
- **两级缓存**平衡了速度和灵活性
- **缓存命中**时性能接近原生函数调用
