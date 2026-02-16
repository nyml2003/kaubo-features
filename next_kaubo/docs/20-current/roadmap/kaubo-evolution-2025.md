# Kaubo 演进路线图 2025

> 状态：规划文档 | 目标：泛型系统 + 模块系统改造 + 完整类型推断
> 
> **重要**：Kaubo 不支持 `fn` 函数声明，只支持匿名函数（lambda）赋值给变量

---

## 1. 演进概览

### 1.1 当前状态

| 组件 | 状态 | 主要限制 |
|------|------|----------|
| **类型系统** | 基础 | 无泛型，`List[any]` 丢失类型信息 |
| **模块系统** | 单文件多模块 | `module` 关键字语义混乱，无文件系统抽象 |
| **类型推断** | 部分 | 链式调用 `.filter().map()` 类型丢失 |
| **Std** | 硬编码 | `runtime/stdlib/mod.rs` 722行，无法独立更新 |

### 1.2 目标状态

```kaubo
// 目标：完整泛型 + 单文件模块 + 链式类型推断

// math.kaubo - 单文件即模块
var add = |a: int, b: int| -> int {
    return a + b;
};

// main.kaubo
import math;
import std.list;

struct Point[T] {
    x: T,
    y: T,
}

impl[T] Point[T] {
    map: |[U] self, f: |T| -> U| -> Point[U] {
        return Point[U] { 
            x: f(self.x), 
            y: f(self.y) 
        };
    },
}

// 完整类型推导
var nums: List[int] = [1, 2, 3, 4, 5];
var result = nums
    .filter(|x| -> bool { return x > 2; })     // List[int]
    .map(|[U] x| -> float { return x as float; })  // List[float]
    .reduce(|[U] acc, x| -> float { return acc + x; }, 0.0);  // float
```

---

## 2. Phase 1：基础设施（4周）

### 2.1 虚拟文件系统（VFS）

**目的**：统一文件操作，支持多平台

```rust
pub trait VirtualFileSystem: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, VfsError>;
    fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), VfsError>;
    fn exists(&self, path: &Path) -> bool;
}

// 实现：NativeFileSystem, MemoryFileSystem, LayeredFileSystem
```

**实施任务**：

| 周次 | 任务 | 文件 | 验收标准 |
|------|------|------|----------|
| 1 | VFS trait + MemoryFS | `vfs.rs` | 内存读写测试通过 |
| 1 | NativeFileSystem | `vfs.rs` | 封装 std::fs |
| 2 | LayeredFileSystem | `vfs.rs` | 多层叠加 |
| 2 | VFS 集成 | `module_resolver.rs` | 模块加载走 VFS |

### 2.2 模块系统改造（单文件单模块）

**语法变更**：

```kaubo
// ❌ 删除：module 关键字
module math {
    var add = |a: int, b: int| -> int { return a + b; };
}

// ✅ 新语法：文件即模块
// math.kaubo
var add = |a: int, b: int| -> int {
    return a + b;
};
```

**实施任务**：

| 周次 | 任务 | 文件 | 详细 |
|------|------|------|------|
| 3 | 删除 module 语句 | `parser.rs` | 移除 `StmtKind::Module` |
| 3 | 更新 import 解析 | `parser.rs` | 基于文件路径 |
| 4 | ModuleResolver | `module_resolver.rs` | VFS + 缓存 |
| 4 | 多文件编译 | `compiler/mod.rs` | 递归编译依赖 |

### 2.3 类型系统基础扩展

**核心变更**：

```rust
pub enum TypeExpr {
    Named(NamedType),
    TypeParam(TypeParam),           // NEW
    List(Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),
    Function(FunctionType),
    GenericInstance(GenericInstance), // NEW
}
```

---

## 3. Phase 2：泛型系统（6周）

### 3.1 泛型语法

**统一使用 `[]`**：

```kaubo
// 类型定义
struct Box[T] { value: T }
impl[T] Box[T] { ... }

// 表达式
|[T] x: T| -> T { return x; }

// 类型标注
var b: Box[int] = Box[int] { value: 42 };
```

### 3.2 实施任务

| 周次 | 任务 | 文件 | 详细 |
|------|------|------|------|
| 5 | Parser 扩展 | `parser.rs` | `[T]` 类型参数 |
| 5 | 泛型 struct | `parser.rs` | `struct Box[T]` |
| 5 | 泛型 impl | `parser.rs` | `impl[T] Box[T]` |
| 5 | 泛型 lambda | `parser.rs` | `|[T] x: T|` |
| 6 | 类型检查扩展 | `type_checker.rs` | 泛型定义检查 |
| 7 | 泛型实例化 | `type_checker.rs` | 替换 + shape |
| 8 | 约束求解器 | `constraint_solver.rs` | unify 算法 |
| 9 | 单态化 | `monomorphizer.rs` | 生成具体函数 |
| 10 | 编译器集成 | `compiler.rs` | 延迟实例化 |

### 3.3 单态化策略

| 类型 | 策略 | 理由 |
|------|------|------|
| **基本类型** (int/float/bool) | 单态化 | 零开销 |
| **堆类型** (struct/List) | 类型擦除 | 避免代码膨胀 |

---

## 4. Phase 3：类型推断增强（4周）

### 4.1 目标

```kaubo
// 当前：类型丢失
var result = [1, 2, 3]
    .filter(|x| { return x > 2; })   // ❌ 无法推断
    .map(|x| { return x * 10; });    // ❌ 在 None 上调用

// 目标：完整推导
var result = [1, 2, 3]
    .filter(|x| -> bool { return x > 2; })  // ✅ List[int]
    .map(|[U] x| -> int { return x * 10; }); // ✅ List[int]
```

### 4.2 实施任务

| 周次 | 任务 | 文件 | 详细 |
|------|------|------|------|
| 11 | 成员访问推导 | `type_checker.rs` | 内置方法返回类型 |
| 11 | 泛型方法类型 | `builtin_generics.rs` | map/filter/reduce 类型 |
| 12 | 链式调用 | `type_checker.rs` | 类型传递 |
| 13 | List 泛型方法 | `builtin_generics.rs` | List[T] 方法 |
| 14 | 集成测试 | `tests/` | 复杂链式调用 |

---

## 5. Phase 4：插件化 Std（4周）

### 5.1 设计原则

- **独立 crate**：`kaubo-std` 不依赖 `kaubo-core`
- **自动包装**：宏自动生成 Value ↔ Rust 转换
- **插件化**：实现 `StdModule` trait 即可注册

**重要**：Std 提供的是 Rust 原生函数，通过 `#[export]` 宏包装，不是 Kaubo 函数。

```rust
// kaubo-std/src/math.rs
use kaubo_std_api::{std_module, export, Result};

#[std_module(name = "math", version = "1.0.0")]
pub struct MathModule;

impl MathModule {
    // ✅ Rust 函数，#[export] 自动包装
    #[export(name = "sqrt", arity = 1)]
    fn sqrt(x: f64) -> Result<f64> {
        if x < 0.0 { Err("domain error".to_string()) }
        else { Ok(x.sqrt()) }
    }
    
    // ❌ 不是 Kaubo 函数
    // fn sqrt(x: float) -> float { return x.sqrt(); }
}
```

### 5.2 架构

```
kaubo-core (编译器)
    ↓ 加载
kaubo-std-api (最小接口)
    ↑ 实现
kaubo-std (独立包)
    ├── core: print, assert, type
    ├── math: sqrt, sin, cos
    └── list: 列表方法
```

### 5.3 实施任务

| 周次 | 任务 | 详细 |
|------|------|------|
| 15 | kaubo-std-api | Value, StdModule trait |
| 15 | 宏设计 | #[std_module], #[export] |
| 16 | kaubo-std 框架 | 项目结构 |
| 16 | core 模块 | print, assert, type |
| 17 | math 模块 | sqrt, sin, cos, PI, E |
| 17 | list 模块 | 列表方法 |
| 18 | NativeRegistry | 编译器侧加载 |
| 18 | 集成测试 | 端到端 |

---

## 6. Phase 5：整合与优化（2周）

### 6.1 集成验证

| 任务 | 验证内容 |
|------|----------|
| 泛型 + 模块 | 泛型 struct 跨文件导入 |
| 泛型 + 类型推断 | 链式调用完整推导 |
| 模块 + Std | 独立 std 可加载 |
| 端到端 | 复杂项目可编译运行 |

### 6.2 性能优化

| 优化点 | 策略 | 预期收益 |
|--------|------|----------|
| 实例化缓存 | 基本类型组合缓存 | 编译时间 -30% |
| 惰性实例化 | 按需编译泛型函数 | 减少代码体积 |
| VFS 缓存 | 文件内容缓存 | IO 减少 |
| 模块缓存 | 已编译模块复用 | 增量编译 |

---

## 7. 依赖关系

```
Phase 1 (基础设施)
├── VFS ──────────────────────────────────┐
├── 模块系统改造 ──────────────────────────┤
└── 类型系统扩展 ──────────────────────────┤
                                           │
Phase 2 (泛型系统)                         │
├── 语法支持 ──────────────────────────────┤
├── 类型检查 ──────────────────────────────┤
├── 约束求解 ──────────────────────────────┤
└── 单态化 ◄───────────────────────────────┘
                                           │
Phase 3 (类型推断)                         │
├── 方法调用推断 ──────────────────────────┤
├── 链式调用 ◄─────────────────────────────┘
└── 内置泛型方法 ◄─────────────────────────┘
                                           │
Phase 4 (插件化 Std)                       │
├── kaubo-std-api ─────────────────────────┤
├── 宏系统 ────────────────────────────────┤
├── std 迁移 ◄─────────────────────────────┘
└── NativeRegistry ◄───────────────────────┘
                                           │
Phase 5 (整合)                             │
└── 集成测试 ◄─────────────────────────────┘
```

---

## 8. 文件改动汇总

### 新增文件

| 文件 | 大小 | 用途 |
|------|------|------|
| `vfs.rs` | ~400行 | 虚拟文件系统 |
| `module_resolver.rs` | ~300行 | 模块解析 |
| `constraint_solver.rs` | ~300行 | 约束求解 |
| `monomorphizer.rs` | ~400行 | 单态化 |
| `builtin_generics.rs` | ~200行 | 内置泛型方法 |
| `native_registry.rs` | ~150行 | Std 加载 |
| `kaubo-std/` | ~1000行 | 独立标准库 |

### 修改文件

| 文件 | 改动 | 说明 |
|------|------|------|
| `parser.rs` | ~+200行 | 泛型语法，删除 module |
| `type_checker.rs` | ~+600行 | 泛型检查，类型推导 |
| `compiler.rs` | ~+100行 | 集成单态化 |

**总计：约 3600 行新增**

---

## 9. MVP 方案（12周）

| Phase | 范围 | 产出 |
|-------|------|------|
| MVP 1 | Phase 1 (简化) | VFS + 单文件模块 |
| MVP 2 | Phase 2 (简化) | 显式泛型（无自动推导） |
| MVP 3 | Phase 4 (核心) | std 核心函数迁移 |

**MVP 语法**：
```kaubo
// 支持
struct Box[T] { value: T; }
var b = Box[int] { value: 42; }
|[T] x: T| -> T { return x; }

// 不支持（后续迭代）
identity(42)              // 自动推导
list.filter().map()       // 链式推导
```

---

## 10. 注意事项

### 10.1 无 `fn` 声明

```kaubo
// ✅ 正确：匿名函数赋值
var add = |a: int, b: int| -> int { return a + b; };

// ❌ 错误：不支持 fn 声明
fn add(a: int, b: int) -> int { return a + b; }
```

### 10.2 Std 是 Rust 函数

Std 提供的是 **Rust 原生函数**，不是 Kaubo 函数：

```rust
// kaubo-std/src/math.rs
#[export(name = "sqrt")]
fn sqrt(x: f64) -> f64 { x.sqrt() }  // ✅ Rust 函数

// ❌ 不是 Kaubo 函数
// fn sqrt(x: float) -> float { return x.sqrt(); }
```

---

*文档版本：1.1 | 创建日期：2026-02-16*
