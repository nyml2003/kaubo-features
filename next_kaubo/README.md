# Kaubo 编程语言

> 🚧 活跃开发中 - 当前阶段: 2.6/3.0 (模块系统)

Kaubo 是一门现代、简洁的脚本语言，专为嵌入式场景和快速原型设计。

[![Tests](https://img.shields.io/badge/tests-227%20passing-green)](docs/PROJECT_STATUS.md)
[![Phase](https://img.shields.io/badge/phase-2.6%20Modules-yellow)](docs/PROJECT_STATUS.md)
[![License](https://img.shields.io/badge/license-MIT-blue)]()

---

## 快速开始

```bash
# 构建项目
cargo build --release

# 运行示例
./target/release/next_kaubo assets/test_fibonacci.txt

# 运行测试
cargo test
```

---

## 语言特性

### 1. 基础语法

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

### 2. 函数与 Lambda

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

### 3. 协程与迭代器

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

### 4. JSON 支持

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

### 5. 模块系统 (🚧 开发中)

```kaubo
// 定义模块
module math {
    pub var PI = 3.14159;
    
    pub var add = |a, b| {
        return a + b;
    };
    
    pub var square = |x| {
        return x * x;
    };
}

// 导入（即将支持）
import math;
print math.PI;
print math.add(1, 2);
```

---

## 技术架构

```
源代码 → 词法分析器 → 语法分析器 → 字节码编译器 → 虚拟机执行
         (Lexer)      (Parser)       (Compiler)       (VM)
```

### 核心组件

| 组件 | 描述 | 文件 |
|------|------|------|
| **Lexer** | 词法分析，Token 生成 | `src/compiler/lexer.rs` |
| **Parser** | 语法分析，AST 生成 | `src/compiler/parser/` |
| **Compiler** | 字节码编译 | `src/runtime/compiler.rs` |
| **VM** | 栈式虚拟机 | `src/runtime/vm.rs` |
| **Object** | 运行时对象系统 | `src/runtime/object.rs` |

### NaN Boxing

Value 类型使用 64-bit NaN Boxing 技术：

```
[63] Sign [62-52] Exponent(0x7FF) [51] QNAN [50-44] Tag(7-bit) [43-0] Payload

标签类型:
  1-10 : 堆对象 (String, List, Function, Closure, etc.)
  37   : Closure
  38   : Coroutine
  39   : Result
  40   : Option
  41   : JSON
```

---

## 开发状态

详见 [PROJECT_STATUS.md](docs/PROJECT_STATUS.md)

### 已实现 ✅

- 完整词法/语法分析
- 字节码编译与执行
- 函数与闭包
- 协程与迭代器
- JSON 字面量与访问
- 模块定义与导出（单文件）

### 进行中 🚧

- 模块访问指令（`math.PI`）
- 标准库（`std.core`）
- 多文件模块系统
- `break` / `continue`

### 规划中 ⏸️

- Result/Option 完整支持
- 模式匹配 `match`
- 错误传播 `?`
- 类型系统（可选）

---

## 测试

```bash
# 运行所有测试
cargo test

# 仅运行单元测试
cargo test --lib

# 运行特定测试
cargo test fibonacci

# 查看输出
cargo test -- --nocapture
```

### 测试文件

所有示例在 `assets/` 目录：

```
assets/
├── test_hello.txt          # 基础语法
├── test_lambda.txt         # Lambda 函数
├── test_y.txt              # Y 组合子（递归）
├── test_phase_2_4.txt      # 协程测试
├── test_fibonacci.txt      # Fibonacci 生成器
├── test_json.txt           # JSON 功能
├── test_module2.txt        # 模块系统
└── ...
```

---

## 性能

### 基准测试（待完善）

| 测试 | Kaubo | Python 3 | Node.js |
|------|-------|----------|---------|
| Fibonacci(35) | ~ | ~ | ~ |
| 列表操作 1M | ~ | ~ | ~ |
| JSON 解析 | ~ | ~ | ~ |

*基准测试仍在开发中*

---

## 贡献

### 项目结构

```
next_kaubo/
├── src/
│   ├── compiler/       # 编译器前端
│   │   ├── lexer.rs
│   │   ├── parser/
│   │   └── token.rs
│   ├── runtime/        # 运行时与 VM
│   │   ├── vm.rs
│   │   ├── compiler.rs
│   │   ├── object.rs
│   │   └── value.rs
│   ├── debug.rs        # 调试工具
│   ├── error.rs        # 错误处理
│   └── lib.rs          # 库入口
├── tests/              # 集成测试
├── assets/             # 测试文件
├── docs/               # 文档
│   ├── PROJECT_STATUS.md
│   └── TEST_PLAN.md
└── Cargo.toml
```

### 开发指南

1. **代码风格**: 遵循现有代码风格
2. **测试要求**: 新功能必须附带测试
3. **文档**: 更新相关文档

---

## 路线图

### Phase 2.x (当前)
- [x] 闭包支持
- [x] 协程与迭代器
- [x] JSON 支持
- [x] 模块系统基础

### Phase 3.0 (即将到来)
- [ ] 模块访问
- [ ] 标准库
- [ ] 错误处理完善
- [ ] 控制流完善（break/continue）

### Phase 4.0 (未来)
- [ ] 模式匹配
- [ ] 类型系统（可选）
- [ ] LSP 支持
- [ ] 包管理器

---

## 许可证

MIT License

---

*Kaubo - 简单、优雅、强大*
