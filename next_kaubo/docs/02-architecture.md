# Kaubo 项目架构

> 整体架构、核心组件、日志系统与配置管理
>
> **当前状态**: Workspace 架构已实施 (4.0)

## 目录

1. [整体架构](#1-整体架构)
2. [Workspace 分层](#2-workspace-分层)
3. [日志系统](#3-日志系统)
4. [配置管理](#4-配置管理)
5. [核心组件](#5-核心组件)
6. [内存布局](#6-内存布局)
7. [已知限制与未来方向](#7-已知限制与未来方向)

---

## 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                        应用层 (Application)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    CLI       │  │   静态库API   │  │   测试框架    │      │
│  │  (kaubo-cli) │  │  (kaubo-api) │  │  (tests/)    │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
└─────────┼─────────────────┼─────────────────┼───────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                      编排层 (Orchestration)                   │
│                        kaubo-api                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  pub fn run(source, &RunConfig) -> Result<...>        │  │
│  │  pub fn compile(source) -> Result<...>                │  │
│  │  pub fn quick_run(source) -> Result<...>              │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│   编译阶段     │  │   执行阶段     │  │   基础设施     │
├───────────────┤  ├───────────────┤  ├───────────────┤
│ • Lexer       │  │ • VM          │  │ • Config      │
│ • Parser      │  │ • Memory      │  │ • Error       │
│ • Compiler    │  │ • Stdlib      │  │               │
└───────────────┘  └───────────────┘  └───────────────┘
        │
        ▼
  kaubo-core (纯逻辑，无全局状态)
```

---

## 2. Workspace 分层

### 2.1 Crate 职责

| Crate | 职责 | 依赖 |
|-------|------|------|
| `kaubo-config` | 纯配置数据结构 | 无 |
| `kaubo-core` | 核心编译器（纯逻辑，无 IO） | kaubo-config |
| `kaubo-api` | 执行编排、全局单例、错误统一 | kaubo-config, kaubo-core |
| `kaubo-cli` | 参数解析、日志初始化、文件 IO | kaubo-config, kaubo-core, kaubo-api |

### 2.2 配置分层

```
┌─────────────────────────────────────────────────────────────┐
│  kaubo-cli                                                   │
│  ├── LogConfig (CLI特有：日志级别、格式)                      │
│  └── 初始化：tracing-subscriber                               │
├─────────────────────────────────────────────────────────────┤
│  kaubo-api                                                   │
│  ├── RunConfig (执行配置：show_steps, dump_bytecode)          │
│  │   ├── compiler: CompilerConfig                             │
│  │   └── limits: LimitConfig                                  │
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

### 2.3 调用示例

```rust
// 应用层 (CLI)
fn main() {
    let cli = Cli::parse();
    let run_config = build_run_config(&cli);
    init_config(run_config.clone());
    init_logger(&log_config, LogFormat::Pretty, None);
    
    match run(&source, &run_config) {
        Ok(output) => println!("{}", output.value.unwrap_or_default()),
        Err(e) => print_error_with_source(&e, &source),
    }
}

// 编排层 (API) - 推荐方式
pub fn run(source: &str, config: &RunConfig) -> Result<ExecuteOutput, KauboError> {
    let compiled = compile_with_config(source, &config.compiler)?;
    if config.dump_bytecode {
        compiled.chunk.disassemble("main");
    }
    execute_with_config(&compiled.chunk, compiled.local_count, &config.limits)
}

// 快速运行（使用默认配置）
let result = quick_run("return 42;")?;
```

---

## 3. 日志系统

### 3.1 设计目标

- **分阶段控制**: 可独立控制 lexer/parser/compiler/vm 的日志级别
- **结构化输出**: 支持 JSON 格式供工具解析
- **低开销**: 未启用的日志在编译期优化掉

### 3.2 日志分级

| 级别 | 用途 | 示例 |
|------|------|------|
| `TRACE` | 最详细，逐行调试 | 每个字符处理、每条指令执行 |
| `DEBUG` | 开发调试 | Token生成、AST节点、函数调用 |
| `INFO` | 正常流程信息 | 阶段开始/完成、编译成功 |
| `WARN` | 警告 | 不建议的用法、性能问题 |
| `ERROR` | 错误 | 编译错误、运行时错误 |

### 3.3 CLI 日志控制

```bash
# 基础级别
kaubo script.kaubo              # WARN (默认)
kaubo script.kaubo -v           # INFO
kaubo script.kaubo -vv          # DEBUG
kaubo script.kaubo -vvv         # TRACE

# 阶段级控制
kaubo script.kaubo --log-lexer=trace --log-vm=debug

# 输出格式
kaubo script.kaubo --format=json  # JSON 格式
kaubo script.kaubo --format=compact
```

### 3.4 代码中使用

```rust
use tracing::{debug, error, info, instrument, trace, warn};

// 简单日志
debug!("Processing {} tokens", count);

// 结构化日志
debug!(token.kind = ?kind, token.value = %value, "Produced token");

// 函数级跨度（自动记录进入/退出）
#[instrument(target = "kaubo::compiler", skip(ast))]
pub fn compile_ast(ast: &Module) -> Result<CompileOutput, KauboError> {
    info!("Starting compiler");
    // ...
}
```

---

## 4. 配置管理

### 4.1 配置结构

```rust
// kaubo-config (纯数据)
pub struct CompilerConfig {
    pub emit_debug_info: bool,
}

pub struct LimitConfig {
    pub max_stack_size: usize,      // 默认 10240
    pub max_recursion_depth: usize, // 默认 256
}

// kaubo-api
pub struct RunConfig {
    pub show_steps: bool,
    pub dump_bytecode: bool,
    pub compiler: CompilerConfig,
    pub limits: LimitConfig,
}

// kaubo-cli
pub struct LogConfig {
    pub global: Level,
    pub lexer: Option<Level>,
    pub parser: Option<Level>,
    pub compiler: Option<Level>,
    pub vm: Option<Level>,
}
```

### 4.2 全局单例（API 层）

```rust
// kaubo-api/src/config.rs
static GLOBAL_CONFIG: OnceCell<RunConfig> = OnceCell::new();

pub fn init(config: RunConfig) {
    GLOBAL_CONFIG.set(config).expect("Config already initialized");
}

pub fn config() -> &'static RunConfig {
    GLOBAL_CONFIG.get().expect("Config not initialized")
}
```

---

## 5. 核心组件

### 5.1 词法分析器 (Lexer V2)

**位置**: `kaubo-core/src/kit/lexer/`  
**日志目标**: `kaubo::lexer`

三层架构设计：

```
┌─────────────────────────────────────────────────────────────────┐
│  LAYER 3: Language Frontends                                     │
│  ┌─────────┐                                                     │
│  │ Kaubo   │   声明式语法定义                                    │
│  └────┬────┘                                                     │
├───────┼─────────────────────────────────────────────────────────┤
│       │        LAYER 2: Core                                     │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Universal Scanner Engine                    │    │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐    │    │
│  │  │ Scanner  │ │ Token    │ │ Error    │ │ Mode     │    │    │
│  │  │ Trait    │ │ Builder  │ │ Recovery │ │ System   │    │    │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘    │    │
│  └─────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  LAYER 1: Infrastructure                                         │
│  ┌──────────┐ ┌──────────┐                                       │
│  │CharStream│ │ Position │                                       │
│  │(UTF-8)   │ │Tracker   │                                       │
│  └──────────┘ └──────────┘                                       │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 语法分析器 (Parser)

**位置**: `kaubo-core/src/compiler/parser/`  
**日志目标**: `kaubo::parser`

递归下降 + Pratt 表达式解析：

```
┌─────────────────────────────────────────────────────────────────┐
│                         Parser                                   │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Pratt Parser (表达式)                    │   │
│  │   parse_expression(min_precedence)                       │   │
│  │     ├── parse_unary()                                    │   │
│  │     │     └── parse_primary()                            │   │
│  │     └── while (current.precedence > min)                 │   │
│  │           └── parse_expression(current.precedence)       │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │               Recursive Descent (语句)                    │   │
│  │   parse_statement()                                      │   │
│  │     ├── parse_var_declaration()                          │   │
│  │     ├── parse_if_statement()                             │   │
│  │     ├── parse_while_loop()                               │   │
│  │     └── parse_for_loop()                                 │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 字节码编译器

**位置**: `kaubo-core/src/runtime/compiler.rs`  
**日志目标**: `kaubo::compiler`

AST → Bytecode，包含：
- 局部变量管理（栈槽分配、作用域）
- Upvalue 捕获
- 跳转指令生成
- ShapeID 分配

### 5.4 虚拟机 (VM)

**位置**: `kaubo-core/src/runtime/vm.rs`  
**日志目标**: `kaubo::vm`

栈式虚拟机，129 个操作码，支持：
- 函数调用和闭包
- 协程（create, resume, yield, status）
- 列表、JSON、模块操作

---

## 6. 内存布局

### 6.1 Value 类型 (NaN Boxing)

```
[63] Sign [62-52] Exponent(0x7FF) [51] QNAN [50-44] Tag [43-0] Payload

标签分配:
  0-7   : 特殊值 (null, true, false, SMI)
  8-23  : 内联整数 (-8 ~ +7)
  32    : 堆对象
  33    : 字符串
  34    : 函数
  35    : 列表
  37    : 闭包
  38    : 协程
  41    : JSON
  42    : 模块
  43    : 原生函数
  44    : VM-aware 原生函数
```

### 6.2 模块 ShapeID

```rust
// 编译期确定
"std" 模块:
  "print"     -> ShapeID 0
  "assert"    -> ShapeID 1
  "sqrt"      -> ShapeID 4
  "PI"        -> ShapeID 9
  "len"       -> ShapeID 14
  "range"     -> ShapeID 17

// 运行时访问
ModuleGet 0   // O(1) 直接索引
```

---

## 7. 已知限制与未来方向

### 7.1 当前限制

| 限制 | 影响 | 详情 |
|------|------|------|
| **无浮点数字面量** | 无法写 `3.14` | 需用 `std.sqrt` 等间接获得 |
| **逻辑与/或短路** | 效率低 | `true \|\| func()` 仍会执行 `func()` |
| **无垃圾回收** | 内存泄漏 | 只分配不回收 |
| **转义序列** | 字符串受限 | `"\n"` 等转义不支持 |
| **装饰器** | 语法糖无效 | `@Decorator` 仅语法解析 |
| **执行限制** | 配置无效 | max_stack_size 未检查 |

### 7.2 未来方向

1. **字节码版本号** - 魔数 + 版本头，支持兼容性检查
2. **错误处理完善** - 调用堆栈追踪、源码位置保留
3. **浮点数字面量** - Lexer 支持小数点
4. **垃圾回收** - 标记-清除或增量回收
5. **REPL 增强** - 基于增量解析的实时反馈
6. **LSP 支持** - Language Server Protocol

---

*最后更新: 2026-02-12*  
*版本: 4.0 (Workspace + 配置分层架构)*
