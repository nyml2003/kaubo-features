# Kaubo 编程语言

> 🚧 活跃开发中 - 当前阶段: 2.9 标准库与显式导入

Kaubo 是一门现代、简洁的脚本语言，专为嵌入式场景和快速原型设计。

**核心特性**：
- 静态内存布局（ShapeID 系统）
- 扁平化模块设计
- 显式导入，无隐式作用域
- 原生函数支持（Rust 实现）

---

## 目录

1. [快速开始](#1-快速开始)
2. [语言特性](#2-语言特性)
3. [技术架构](#3-技术架构)
4. [详细设计](#4-详细设计)
5. [开发计划](#5-开发计划)
6. [项目结构](#6-项目结构)

---

## 1. 快速开始

### 1.1 构建与运行

```bash
# 构建项目
cargo build --release

# 运行示例
./target/release/next_kaubo assets/test_fibonacci.txt

# 运行测试
cargo test
```

### 1.2 Hello World

```kaubo
@ProgramStart
module main {
    import std.core;
    
    pub var run = || {
        std.core.print("Hello, World!");
        return 0;
    };
}
```

---

## 2. 语言特性

### 2.1 基础语法

```kaubo
// 变量声明
var x = 5;
var name = "Kaubo";
var pi = 3.14159;

// 列表
var nums = [1, 2, 3, 4, 5];
print nums[0];  // 1

// 条件语句
if (x > 0) {
    print "positive";
} elif (x < 0) {
    print "negative";
} else {
    print "zero";
}

// 循环
var i = 0;
while (i < 10) {
    print i;
    i = i + 1;
}
```

### 2.2 函数与 Lambda

```kaubo
// 函数是一等公民
var add = |a, b| {
    return a + b;
};

// 闭包
var counter = || {
    var count = 0;
    return || {
        count = count + 1;
        return count;
    };
};

var c = counter();
print c();  // 1
print c();  // 2
```

### 2.3 协程与迭代器

```kaubo
// Fibonacci 生成器
var fib_gen = || {
    var a = 0;
    var b = 1;
    while (true) {
        yield a;
        var temp = a;
        a = b;
        b = temp + b;
    }
};

// 使用协程
var fib = create_coroutine(fib_gen);
var i = 0;
while (i < 10) {
    print resume(fib);
    i = i + 1;
}

// for-in 循环
var numbers = [10, 20, 30];
for (n in numbers) {
    print n;
}
```

### 2.4 JSON 支持

```kaubo
// JSON 字面量
var config = json {
    "name": "app",
    "version": "1.0.0",
    "database": json {
        "host": "localhost",
        "port": 5432
    }
};

// 访问方式
print config.name;           // 成员访问
print config["database"];    // 索引访问
print config.database.host;  // 嵌套访问
```

### 2.5 模块系统（扁平化设计）

```kaubo
// file: main.kaubo

// 扁平化模块定义，不嵌套
module math {
    pub var PI = 3.14159;
    pub var add = |a, b| { a + b; };
}

module geometry {
    // 必须显式导入，即使同文件
    import math;
    
    pub var circle_area = |r| {
        math.PI * r * r
    };
}

// 入口模块
@ProgramStart
module main {
    import std.core;
    import geometry;
    
    pub var run = || {
        std.core.print(geometry.circle_area(5));
    };
}
```

**模块设计原则**：
- **扁平化**：模块内不能定义子模块
- **显式导入**：同文件内的模块也需要 `import`
- **静态布局**：模块字段编译期确定 ShapeID，运行时 O(1) 访问

---

## 3. 技术架构

### 3.1 整体流程

```
源代码 → 词法分析器 → 语法分析器 → 字节码编译器 → 虚拟机执行
         (Lexer)      (Parser)       (Compiler)       (VM)
```

### 3.2 核心组件

| 组件 | 描述 | 文件 |
|------|------|------|
| **Lexer** | 词法分析，Token 生成 | `src/compiler/lexer.rs` |
| **Parser** | 语法分析，AST 生成 | `src/compiler/parser/` |
| **Compiler** | 字节码编译 | `src/runtime/compiler.rs` |
| **VM** | 栈式虚拟机 | `src/runtime/vm.rs` |
| **Object** | 运行时对象系统 | `src/runtime/object.rs` |
| **Stdlib** | 标准库（Rust 原生） | `src/runtime/stdlib/` |

### 3.3 NaN Boxing

Value 类型使用 64-bit NaN Boxing 技术：

```
[63] Sign [62-52] Exponent(0x7FF) [51] QNAN [50-44] Tag(7-bit) [43-0] Payload

标签类型:
  0-7  : 特殊值 (null, true, false, SMI)
  8-23 : 内联整数 (-8 ~ +7)
  32   : 通用堆对象
  33   : String
  34   : Function
  35   : List
  37   : Closure
  38   : Coroutine
  39   : Result
  40   : Option
  41   : JSON
  42   : Module
  43   : Native (原生函数)
```

### 3.4 静态内存布局（ShapeID）

```
模块内存布局（编译期确定）：
┌─────────────────────────────────────────┐
│ ObjectHeader │ ShapeID │ Field1 │ Field2 │ ...
└─────────────────────────────────────────┘

访问 math.PI：
  LoadGlobal("math") + ModuleGet(0)  // ShapeID 0，直接偏移访问
```

---

## 4. 详细设计

### 4.1 字节码指令集

| 类别 | 指令 | 说明 |
|------|------|------|
| 常量 | `LoadConst0-15`, `LoadConst` | 加载常量 |
| 变量 | `LoadLocal`, `StoreLocal`, `LoadGlobal`, `StoreGlobal` | 变量访问 |
| 运算 | `Add`, `Sub`, `Mul`, `Div`, `Neg`, `Not` | 算术/逻辑运算 |
| 比较 | `Equal`, `Greater`, `Less`, ... | 比较运算 |
| 控制流 | `Jump`, `JumpIfFalse`, `JumpBack` | 跳转指令 |
| 函数 | `Call`, `Closure`, `Return`, `ReturnValue` | 函数调用 |
| 协程 | `CreateCoroutine`, `Resume`, `Yield` | 协程操作 |
| 列表 | `BuildList`, `IndexGet`, `IndexSet` | 列表操作 |
| 模块 | `BuildModule`, `ModuleGet` | 模块操作 |
| 原生 | `Call` (Native) | 原生函数调用 |

### 4.2 模块系统设计

#### 4.2.1 扁平化原则

```kaubo
// ✅ 支持：多个扁平模块
module math { ... }
module utils { ... }

// ❌ 不支持：模块嵌套
module outer {
    module inner { ... }  // 编译错误
}
```

#### 4.2.2 显式导入规则

```kaubo
// 即使是同文件内的模块，也必须显式 import
module math {
    pub var PI = 3.14;
}

module geometry {
    import math;  // ✅ 必须显式导入
    
    pub var circle_area = |r| {
        math.PI * r * r
    };
}
```

#### 4.2.3 标准库模块

**当前设计**: 扁平化单一 `std` 模块

```kaubo
import std;

std.print("Hello");
var x = std.sqrt(16);
std.print(std.PI);
```

**可用功能**:

| 函数/常量 | ShapeID | 说明 |
|-----------|---------|------|
| `std.print(x)` | 0 | 输出并换行 |
| `std.assert(cond)` | 1 | 断言 |
| `std.type(x)` | 2 | 获取类型名 |
| `std.to_string(x)` | 3 | 转为字符串 |
| `std.sqrt(x)` | 4 | 平方根 |
| `std.sin(x)` | 5 | 正弦 |
| `std.cos(x)` | 6 | 余弦 |
| `std.floor(x)` | 7 | 向下取整 |
| `std.ceil(x)` | 8 | 向上取整 |
| `std.PI` | 9 | 圆周率 |
| `std.E` | 10 | 自然常数 |

### 4.3 装饰器系统

#### 4.3.1 内置装饰器

| 装饰器 | 用途 | 示例 |
|--------|------|------|
| `@ProgramStart` | 标记程序入口模块 | `@ProgramStart module main { ... }` |
| `@EntryPoint` | 标记入口函数 | `@EntryPoint pub var main = ...` |
| `@Test` | 标记测试函数 | `@Test var test_add = ...` |

#### 4.3.2 装饰器语义

```kaubo
// 程序必须有且只有一个 @ProgramStart
@ProgramStart
module main {
    import std.core;
    
    // 启动时自动执行 run 函数
    pub var run = || {
        std.core.print("Hello!");
        return 0;
    };
}
```

### 4.4 原生函数机制

```rust
// Rust 实现的原生函数
pub fn print(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("print() takes 1 argument".to_string());
    }
    println!("{}", args[0]);
    Ok(Value::NULL)
}

// 包装为 ObjNative，存入 std.core 模块
```

---

## 5. 开发计划

### 5.1 当前状态

| 阶段 | 功能 | 状态 |
|------|------|------|
| 2.7 | 模块静态化（ShapeID 系统） | ✅ 已完成 |
| 2.8 | break/continue + 边界测试 | ⏸️ 延后 |
| 2.9 | 标准库完善 + 显式导入 | ✅ 已完成 |
| 3.0 | 测试机制 + 错误处理 | 🚧 进行中 |

### 5.2 优先级矩阵

| 特性 | 阶段 | 难度 | 价值 | 优先级 | 状态 |
|------|------|------|------|--------|------|
| 模块静态化 | 2.7 | ⭐⭐⭐ | ⭐⭐⭐ | - | ✅ 已完成 |
| 显式导入 | 2.9 | ⭐⭐ | ⭐⭐⭐ | 🔥 P0 | ✅ 已完成 |
| 标准库 | 2.9 | ⭐⭐ | ⭐⭐⭐ | 🔥 P0 | ✅ 已完成 |
| 测试机制 | 3.0 | ⭐⭐ | ⭐⭐⭐ | 🔥 P0 | 🚧 进行中 |
| 错误处理 | 3.0 | ⭐⭐ | ⭐⭐⭐ | 🔥 P0 | 🚧 进行中 |
| break/continue | - | ⭐ | ⭐⭐⭐ | ⭐ P1 | ⏸️ 延后 |
| 浮点数修复 | - | ⭐ | ⭐⭐ | ⭐ P1 | ⏸️ 延后 |
| 结构体 struct | 3.0 | ⭐⭐⭐ | ⭐⭐⭐ | ⭐ P1 | ⏸️ 待开始 |
| Interface | 3.1 | ⭐⭐⭐ | ⭐⭐⭐ | ⭐ P1 | ⏸️ 待开始 |
| 装饰器 | 3.x | ⭐⭐ | ⭐⭐ | 🌙 P2 | ⏸️ 待开始 |

### 5.3 近期目标（先跑起来）

1. **完成显式导入**
   - 同文件模块 `import math;`
   - 标准库模块 `import std.core;`
   - 未导入访问编译错误

2. **完善标准库**
   - `std.core`: print, assert, type, to_string
   - `std.math`: sqrt, sin, cos, PI, E

3. **支持 @ProgramStart**
   - 标记入口模块
   - 自动执行 run 函数

---

## 6. 项目结构

```
next_kaubo/
├── src/
│   ├── compiler/          # 编译器前端
│   │   ├── lexer/
│   │   ├── parser/
│   │   └── token.rs
│   ├── runtime/           # 运行时与 VM
│   │   ├── bytecode/      # 字节码定义
│   │   ├── stdlib/        # 标准库（Rust 实现）
│   │   ├── vm.rs
│   │   ├── compiler.rs
│   │   ├── object.rs
│   │   └── value.rs
│   ├── lib.rs
│   └── main.rs
├── tests/                 # 集成测试
├── assets/                # 测试文件
├── docs/
│   └── KAUBO.md          # 本文档
└── Cargo.toml
```

---

*最后更新: 2026-02-10*  
*版本: 2.9*
