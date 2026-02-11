# Kaubo 开发指南 (For AI Agents)

> 本指南反映项目**当前实际状态**，帮助 AI 助手快速理解代码结构和开发规范。

## 项目概览

**Kaubo** 是一门现代脚本语言，当前采用**Workspace 架构**。

```
测试状态: 187 passed, 0 failed
架构状态: Workspace（kaubo-config, kaubo-core, kaubo-api, kaubo-cli）
```

## 当前 Workspace 目录结构

```
kaubo/
├── Cargo.toml              # Workspace 定义
├── kaubo-config/           # 配置数据结构（纯数据，无逻辑）
│   └── src/
│       └── lib.rs          # CompilerConfig, LimitConfig, Phase
├── kaubo-core/             # 核心编译器（纯逻辑，无全局状态）
│   ├── src/
│   │   ├── compiler/       # Parser、AST
│   │   │   ├── lexer/
│   │   │   └── parser/
│   │   ├── kit/            # Lexer V2（手写 Scanner）
│   │   │   └── lexer/
│   │   └── runtime/        # VM、Bytecode、Stdlib
│   │       ├── bytecode/
│   │       └── stdlib/
│   └── tests/              # 集成测试
├── kaubo-api/              # 编排层/API（含全局单例）
│   └── src/
│       ├── config.rs       # RunConfig + GLOBAL_CONFIG
│       ├── error.rs        # KauboError
│       ├── lib.rs          # run(), compile(), execute()
│       └── types.rs        # CompileOutput
└── kaubo-cli/              # CLI 平台（二进制）
    └── src/
        ├── main.rs         # CLI 入口
        ├── config.rs       # LogConfig (CLI特有)
        ├── logging.rs      # tracing-subscriber 初始化
        └── platform/       # CLI 格式化输出
```

## Crate 职责

| Crate | 职责 | 依赖 |
|-------|------|------|
| `kaubo-config` | 纯配置数据结构 | 无 |
| `kaubo-core` | Lexer, Parser, Compiler, VM | `kaubo-config` |
| `kaubo-api` | 执行编排、全局单例、错误统一 | `kaubo-config`, `kaubo-core` |
| `kaubo-cli` | 参数解析、日志初始化、文件 IO | `kaubo-config`, `kaubo-core`, `kaubo-api` |

## 配置分层架构

```
┌─────────────────────────────────────────────────────────────┐
│  kaubo-cli                                                   │
│  ├── LogConfig (CLI特有：日志级别、格式)                      │
│  └── 初始化：tracing-subscriber                               │
├─────────────────────────────────────────────────────────────┤
│  kaubo-api                                                   │
│  ├── RunConfig (执行配置：show_steps, dump_bytecode)          │
│  │   ├── compiler: CompilerConfig                            │
│  │   └── limits: LimitConfig                                 │
│  └── GLOBAL_CONFIG: OnceCell<RunConfig> (全局单例)           │
├─────────────────────────────────────────────────────────────┤
│  kaubo-core                                                  │
│  └── 通过参数接收配置，无全局状态                             │
├─────────────────────────────────────────────────────────────┤
│  kaubo-config                                                │
│  ├── CompilerConfig { emit_debug_info }                      │
│  ├── LimitConfig { max_stack_size, max_recursion_depth }     │
│  └── Phase { Lexer, Parser, Compiler, Vm }                   │
└─────────────────────────────────────────────────────────────┘
```

## 功能清单（当前实际实现）

### 1. 词法分析 (Lexer V2)

| 功能 | 状态 | 说明 |
|------|------|------|
| 关键字识别 | ✅ | var, if, else, elif, while, for, return, in, true, false, null, break, continue, import, from, as, and, or, not, async, await, module, pub, json |
| 整数字面量 | ✅ | 十进制整数（i64 范围） |
| 字符串字面量 | ✅ | 双引号字符串，支持多行 |
| 标识符 | ✅ | 字母/下划线开头，含数字 |
| 运算符 | ✅ | +, -, *, /, ==, !=, <, >, <=, >=, =, !, \|\| |
| 分隔符 | ✅ | (), {}, [], :, ;, ,, . |
| 行注释 | ✅ | // 到行尾 |
| 块注释 | ✅ | /* */ 多行注释 |
| 源位置追踪 | ✅ | line, column, byte_offset, utf16_column |
| **浮点数字面量** | ❌ | `3.14` 无法直接解析，需用 `std.sqrt` 等间接获得 |
| **转义序列** | ❌ | `"\n"` 等转义不支持 |

### 2. 语法分析 (Parser)

| 功能 | 状态 | 说明 |
|------|------|------|
| 表达式 | ✅ | 二元运算、一元运算、括号、字面量、变量引用、函数调用、成员访问、索引访问 |
| 变量声明 | ✅ | `var x = expr;` |
| 赋值 | ✅ | `x = expr;`, `x.y = expr;`, `x[i] = expr;` |
| 条件语句 | ✅ | `if/elif/else` |
| 循环 | ✅ | `while`, `for...in` |
| 函数定义 | ✅ | `fn name() {}` |
| 匿名函数 | ✅ | `\|x\| { ... }` |
| 返回 | ✅ | `return;`, `return expr;` |
| 列表字面量 | ✅ | `[1, 2, 3]` |
| JSON 字面量 | ✅ | `json { "key": value }` |
| 模块定义 | ✅ | `module name { ... }` |
| 导入 | ✅ | `import module;`, `from module import item;` |
| 装饰器 | ✅ | 语法解析 `@Decorator`，**运行时未实现** |
| Yield | ✅ | 协程 yield 表达式 |
| **逻辑与/或** | ⚠️ | Parser 支持 `and`/`or`，**VM 未实现短路求值** |

### 3. 字节码编译器

| 功能 | 状态 | 说明 |
|------|------|------|
| 表达式编译 | ✅ | 所有表达式类型 |
| 语句编译 | ✅ | 所有语句类型 |
| 局部变量 | ✅ | 栈槽分配、作用域管理 |
| 全局变量 | ✅ | 全局表访问 |
| 函数编译 | ✅ | 函数对象、参数处理 |
| 闭包编译 | ✅ | Upvalue 捕获 |
| 跳转指令 | ✅ | if/while/for 控制流 |
| 模块编译 | ✅ | 导出表、ShapeID 分配 |
| **装饰器** | ❌ | 仅语法支持，无运行时语义 |

### 4. 虚拟机 (VM)

| 功能 | 状态 | 说明 |
|------|------|------|
| 字节码执行 | ✅ | 主解释循环 |
| 栈操作 | ✅ | push, pop, dup, swap |
| 常量加载 | ✅ | 16 个内联常量 + 常量池 |
| 局部变量 | ✅ | 0-7 优化槽位 + 一般槽位 |
| 全局变量 | ✅ | HashMap 存储 |
| 算术运算 | ✅ | +, -, *, /, neg |
| 比较运算 | ✅ | ==, !=, <, >, <=, >= |
| 逻辑非 | ✅ | `not` 操作符 |
| 函数调用 | ✅ | 参数传递、返回值 |
| 闭包 | ✅ | Upvalue 捕获与访问 |
| 协程 | ✅ | create, resume, yield, status |
| 列表 | ✅ | BuildList, IndexGet, IndexSet, GetIter, IterNext |
| JSON | ✅ | BuildJson, JsonGet, JsonSet |
| 模块 | ✅ | BuildModule, ModuleGet(ShapeID) |
| **逻辑与/或短路** | ❌ | `true \|\| foo()` 仍会执行 `foo()` |
| **垃圾回收** | ❌ | 只分配不回收 |
| **方法调用** | ❌ | `obj.method()` 语法不支持 |

### 5. 标准库 (std)

| 函数/常量 | 状态 | 说明 |
|-----------|------|------|
| `std.print(x)` | ✅ | 打印值 |
| `std.assert(cond, msg?)` | ✅ | 断言（变参 1-2） |
| `std.type(x)` | ✅ | 返回类型名字符串 |
| `std.to_string(x)` | ✅ | 值转字符串 |
| `std.sqrt(x)` | ✅ | 平方根 |
| `std.sin(x)` | ✅ | 正弦 |
| `std.cos(x)` | ✅ | 余弦 |
| `std.floor(x)` | ✅ | 向下取整 |
| `std.ceil(x)` | ✅ | 向上取整 |
| `std.PI` | ✅ | 圆周率常量 |
| `std.E` | ✅ | 自然对数常量 |
| **std.len(x)** | ❌ | 未实现 |
| **std.push(list, x)** | ❌ | 未实现 |
| **std.pop(list)** | ❌ | 未实现 |
| **字符串方法** | ❌ | `.length`, `.substring` 等 |
| **列表方法** | ❌ | `.map`, `.filter`, `.append` 等 |

### 6. Value 类型系统

| 类型 | 状态 | 存储方式 |
|------|------|----------|
| Null | ✅ | NaN Boxing Tag |
| Bool | ✅ | NaN Boxing Tag (true/false) |
| Integer | ✅ | 内联整数(-8~7) + SMI(-2^30~2^30-1) |
| Float | ✅ | 原生 f64（非 NaN） |
| String | ✅ | 堆对象 |
| List | ✅ | 堆对象 |
| Function | ✅ | 堆对象 |
| Closure | ✅ | 堆对象 |
| Coroutine | ✅ | 堆对象 |
| Module | ✅ | 堆对象 |
| JSON | ✅ | 堆对象 |
| NativeFn | ✅ | 堆对象 |
| Result | ✅ | 堆对象（类型系统预留） |
| Option | ✅ | 堆对象（类型系统预留） |

### 7. API 层

| 函数 | 状态 | 说明 |
|------|------|------|
| `run(source, &RunConfig)` | ✅ | 执行（推荐 API） |
| `compile(source)` | ✅ | 编译（使用全局配置） |
| `compile_and_run(source)` | ✅ | 编译+执行（使用全局配置） |
| `quick_run(source)` | ✅ | 快速运行（自动初始化默认配置） |
| `init_config(RunConfig)` | ✅ | 初始化全局配置 |

### 8. 错误处理

| 功能 | 状态 | 说明 |
|------|------|------|
| 词法错误 | ✅ | 非法字符、未终止字符串 |
| 语法错误 | ✅ | 缺少括号、意外 token |
| 编译错误 | ✅ | 变量未定义等 |
| 运行时错误 | ✅ | 类型错误、除零等 |
| 错误位置 | ✅ | 行号、列号 |
| 源码上下文 | ✅ | CLI 显示错误行前后上下文 |
| 错误报告 JSON | ✅ | ErrorReport.to_json() |

## 已知限制与缺陷（透明清单）

### 高优先级（影响使用）

| 限制 | 影响 | 详情 |
|------|------|------|
| **无浮点数字面量** | 无法写 `3.14` | 需修改 Lexer 识别小数点，或用 `std.sqrt` 间接获得 |
| **无短路求值** | 逻辑运算效率低 | `true \|\| func()` 仍会执行 `func()`，VM 未实现跳转优化 |
| **标准库不完整** | 缺少常用功能 | `len`, `push`, `pop` 未实现 |

### 中优先级（已知问题）

| 限制 | 影响 | 详情 |
|------|------|------|
| **无垃圾回收** | 内存泄漏 | 只分配不回收，长时间运行内存持续增长 |
| **转义序列** | 字符串处理受限 | `"\n"` 等转义序列无法使用 |
| **装饰器未实现** | 语法糖无效 | `@Decorator` 仅语法解析，无实际运行时语义 |
| **执行限制未生效** | 配置无效 | max_stack_size, max_recursion_depth 配置存在但 VM 未检查 |

### 低优先级（改进空间）

| 限制 | 影响 | 详情 |
|------|------|------|
| **无方法调用语法** | 需用函数式风格 | `obj.method()` 不支持，需用 `method(obj)` |
| **无字符串/列表方法** | 需用标准库函数 | `"str".length` 不支持 |
| **文档测试失败** | 2 个 doc test | 示例需要 config 初始化 |

## 关键文件职责

| 文件 | 职责 | 修改频率 |
|------|------|----------|
| `kaubo-config/src/lib.rs` | 纯配置数据结构 | 低 |
| `kaubo-core/src/kit/lexer/scanner.rs` | 手写 Scanner 核心 | 中 |
| `kaubo-core/src/compiler/parser/parser.rs` | 递归下降 Parser | 中 |
| `kaubo-core/src/runtime/compiler.rs` | AST → Bytecode | 高 |
| `kaubo-core/src/runtime/vm.rs` | 虚拟机执行循环 | 高 |
| `kaubo-core/src/runtime/stdlib/mod.rs` | 标准库函数 | 高 |
| `kaubo-core/src/runtime/value.rs` | NaN Boxing Value | 低 |
| `kaubo-api/src/config.rs` | RunConfig + 全局单例 | 低 |
| `kaubo-api/src/lib.rs` | API 接口 | 中 |
| `kaubo-cli/src/main.rs` | CLI 入口 | 低 |
| `kaubo-cli/src/config.rs` | LogConfig | 低 |
| `kaubo-cli/src/logging.rs` | 日志初始化 | 低 |

## 开发规范

### 1. 日志使用

```rust
use tracing::{debug, trace};

// ✅ 正确：使用 tracing
trace!(target: "kaubo::lexer", "Processing char: {}", ch);
debug!(op = ?op, "Compiling");

// ❌ 错误：直接使用 println
println!("Debug: {:?}", value);
```

**例外**：`kaubo-cli/src/main.rs` 可输出程序结果和用户错误信息。

### 2. 配置使用

**Core 层（纯逻辑）**：
```rust
// 通过参数接收配置
pub fn compile(source: &str, config: &CompilerConfig) -> Result<...>
```

**API 层（全局单例）**：
```rust
// 使用全局配置
use kaubo_api::{init_config, run, RunConfig};

init_config(RunConfig::default());
let result = run(source, get_config())?;
```

**CLI 层（初始化）**：
```rust
// 解析参数 -> 构建配置 -> 初始化
let run_config = build_run_config(&cli_args);
init_config(run_config);
init_logger(&log_config, LogFormat::Pretty, None);
```

### 3. 添加标准库函数

在 `kaubo-core/src/runtime/stdlib/mod.rs`：

```rust
fn my_function(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("my_function() takes 1 argument ({} given)", args.len()));
    }
    // 实现...
    Ok(Value::NULL)
}

// 在 create_stdlib_modules() 中注册
exports.push(create_native_value(my_function, "my_function", 1));
name_to_shape.insert("my_function".to_string(), shape_id);
```

## 常用命令

```bash
# 运行测试
cargo test --workspace

# 运行特定 crate 的测试
cargo test -p kaubo-core
cargo test -p kaubo-api

# 构建 CLI
cargo build -p kaubo-cli --release

# 运行示例
cargo run -p kaubo-cli --release -- assets/test_simple.kaubo

# 检查整个 workspace
cargo check --workspace

# 格式化代码
cargo fmt --all

# 检查警告
cargo clippy --workspace --all-targets
```

## 技术债务（当前遗留）

- [ ] 浮点数字面量支持（Lexer）
- [ ] 逻辑与/或短路求值（VM）
- [ ] 垃圾回收实现
- [ ] 标准库完整导出（len, push, pop）
- [ ] 执行限制生效（VM 检查 stack/depth）
- [ ] 文档测试修复

## 参考文档

| 文档 | 状态 | 说明 |
|------|------|------|
| `AGENTS.md` | ✅ 当前 | 本文档，反映实际代码状态 |
| `docs/01-syntax.md` | ✅ 当前 | 语法参考 |
| `docs/02-architecture.md` | ⚠️ 部分过时 | 包含一些已完成计划的描述 |
| `docs/03-stdlib.md` | ⚠️ 部分过时 | 描述了未完全实现的功能 |
| `docs/04-testing.md` | ✅ 当前 | 测试指南 |
| `docs/05-development.md` | ✅ 当前 | 开发手册 |
| `docs/06-workspace.md` | ✅ 当前 | Workspace 拆分指南 |
| `docs/07-config-refactor.md` | ✅ 当前 | 配置系统重构方案 |
| `docs/CHANGELOG.md` | ✅ 当前 | 变更历史 |

---

## 相关文档

| 文档 | 说明 |
|------|------|
| `docs/01-syntax.md` | 语法参考 |
| `docs/02-architecture.md` | 项目架构 |
| `docs/03-stdlib.md` | 标准库 API |
| `docs/04-testing.md` | 测试指南 |
| `docs/05-development.md` | 开发手册 |
| `docs/06-workspace.md` | Workspace 架构 |
| `docs/07-config-refactor.md` | 配置系统重构 |
| `docs/08-coverage-plan.md` | 测试与开发计划 |
| `docs/CHANGELOG.md` | 变更日志 |

---

*最后更新: 2026-02-12*  
*版本: 4.0（Workspace + 配置分层架构已实施）*
