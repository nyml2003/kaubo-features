# Kaubo Programming Language

Kaubo 是一门静态类型的编译型脚本语言，专注于清晰的语法和可控的性能。

```kaubo
// Hello World
print("Hello, Kaubo!");

// Lambda 函数
var add = |a, b| { return a + b; };
print(add(2, 3));

// 多模块导入
import math;
print("PI = " + math.PI);
```

## 特性

- **编译至字节码**：源码 → 字节码 → 栈式 VM 执行
- **静态类型**：编译期类型检查（TypeChecker 已实现，暂未强制）
- **Lambda / 闭包**：匿名函数，支持 upvalue 捕获
- **运算符重载**：通过 `operator add` 等语法实现自定义类型运算
- **Shape-based 对象系统**：高效的 struct 字段访问和内联缓存
- **协程**：`yield` / `resume` 支持
- **二进制格式**：`.kaubod` (debug) / `.kaubor` (release)
- **内置方法**：List/JSON 支持 `push` / `map` / `filter` / `reduce` 等函数式方法

## 项目结构

```
kaubo/
├── Cargo.toml            ← workspace 配置
├── crates/
│   ├── kaubo-ir/         ← IR 类型层 (Value, OpCode, Chunk, Object, VM)
│   ├── kaubo-compiler/   ← 编译器 (Lexer, Parser, TypeChecker, Codegen, HIR)
│   ├── kaubo-runtime/    ← 运行时 (VM 执行, Stdlib, 二进制格式, 平台抽象)
│   └── kaubo-pipeline/   ← 流水线框架 (Stage trait + Pipeline 组合器)
├── kaubo-cli/            ← CLI 入口
├── kaubo-log/            ← 日志系统 (支持 no_std / WASM)
├── kaubo-config/         ← 配置数据
├── kaubo-vfs/            ← 虚拟文件系统
├── examples/             ← 示例程序
└── docs/                 ← 文档
```

### 架构概览

```
kaubo-ir (零依赖类型层)
    ├── Value (NaN-boxed), OpCode (146 变体), Chunk
    ├── Obj* (String, List, Function, Closure, Struct, Shape, ...)
    └── HIR (基本块 IR + 优化 pass)

kaubo-compiler
    ├── Lexer    → TokenStream
    ├── Parser   → AST Module
    ├── TypeChecker → typed AST (已实现, 待接入主线)
    ├── Codegen  → Chunk (字节码)
    └── HIR      → basic blocks + optimizations (已有框架, 待接入)

kaubo-runtime
    ├── VM       → 栈式执行器 (1778 行执行循环)
    ├── Stdlib   → 30 个原生函数 (数学、文件、网络、加密、时间)
    ├── Binary   → .kaubod/.kaubor 读/写
    └── Platform → trait-based I/O 抽象
```

## 命令

```bash
# 编译并执行源文件
kaubo <file>                   # 编译 + 运行
kaubo run <file>               # 运行 .kaubod/.kaubor 二进制

# 编译
kaubo compile <file>           # 编译为 .kaubod

# 调试子阶段
kaubo lex <file>               # 仅词法分析
kaubo parse <file>             # 仅语法分析
kaubo check <file>             # 仅类型检查
```

## 快速开始

```bash
git clone <repo-url>
cd kaubo

# 运行测试
cargo test --workspace

# 运行示例
cargo run -p kaubo-cli -- examples/01_hello_world/package.json
```

## 语言语法速览

### 基础
```kaubo
var x = 10;
var y: float = 3.14;
var s = "hello";
var flag = true;
```

### 控制流
```kaubo
if x > 0 {
    print("positive");
} elif x < 0 {
    print("negative");
} else {
    print("zero");
}

while x > 0 {
    x = x - 1;
}

for var item in [1, 2, 3] {
    print(item);
}
```

### 函数与闭包
```kaubo
var add = |a, b| { return a + b; };
var counter = |init| {
    var count = init;
    return || { count = count + 1; return count; };
};
```

### Struct 与运算符重载
```kaubo
struct Point {
    x: float,
    y: float,
}

impl Point {
    operator add = |self, other| {
        return Point { x: self.x + other.x, y: self.y + other.y };
    };
}
```

### 模块系统
```kaubo
// math.kaubo
pub var PI = 3.14159;
pub var add = |a, b| { return a + b; };

// main.kaubo
import math;
print(math.add(2, 3));
```

### 协程
```kaubo
var producer = || {
    yield 1;
    yield 2;
    yield 3;
};
var co = create_coroutine(producer);
print(resume(co));  // 1
print(resume(co));  // 2
```

## 实现状态

| 阶段 | 名称 | 状态 |
|------|------|------|
| Phase 0 | 基础设施 (日志/VFS/配置) | ✅ 完成 |
| Phase 1 | 语言核心 (parser/codegen/VM) | ✅ 完成 |
| Phase 2 | 模块系统 + 二进制格式 | ✅ 基本完成 |
| Phase 3 | 消除 panic → 结构化错误 | 📋 待开始 |
| Phase 4 | 轻量 GC (引用计数) | 📋 待开始 |
| Phase 5 | TypeChecker 接入 + HIR 贯通 | 📋 待开始 |
| Phase 6 | Platform trait 注入 + WASM | 📋 待开始 |

## 文档

- [文档中心](docs/README.md) - 文档导航
- [架构全景图](docs/architecture.md) - 水平×竖直架构总览
- [路线图](docs/roadmap.md) - v0.2.0 路线图
- [类型系统审计](docs/type-system.md) - 已知 Gap 与修复方案
- [交付产物](docs/artifacts.md) - 库/二进制/wasm/FFI

## License

MIT License
