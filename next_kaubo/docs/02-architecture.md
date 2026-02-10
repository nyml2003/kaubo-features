# Kaubo 项目架构

> 整体架构、核心组件、日志系统与配置管理

## 目录

1. [整体架构](#1-整体架构)
2. [分层设计](#2-分层设计)
3. [日志系统](#3-日志系统)
4. [配置管理](#4-配置管理)
5. [核心组件](#5-核心组件)
6. [内存布局](#6-内存布局)
7. [文件结构](#7-文件结构)

---

## 1. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                        应用层 (Application)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │    CLI       │  │   静态库API   │  │   测试框架    │      │
│  │  (main.rs)   │  │  (lib.rs)    │  │  (tests/)    │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
└─────────┼─────────────────┼─────────────────┼───────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                      编排层 (Orchestration)                   │
│                        src/api.rs                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  pub fn compile_and_run(source: &str) -> Result<...>  │  │
│  │  pub fn compile(source: &str) -> Result<...>          │  │
│  │  pub fn execute(chunk: &Chunk) -> Result<...>         │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│   编译阶段     │  │   执行阶段     │  │   基础设施     │
├───────────────┤  ├───────────────┤  ├───────────────┤
│ • Lexer       │  │ • VM          │  │ • Config      │
│ • Parser      │  │ • Memory      │  │ • Logger      │
│ • Compiler    │  │ • Stdlib      │  │ • Error       │
└───────────────┘  └───────────────┘  └───────────────┘
```

---

## 2. 分层设计

### 2.1 API 分层

| 层级 | 模块 | 职责 | 使用场景 |
|------|------|------|---------|
| **应用层** | `main.rs` | CLI 参数解析、流程调度 | 命令行工具 |
| **编排层** | `api.rs` | 阶段组合、错误处理 | 库用户、测试 |
| **阶段层** | `lexer/parser/compiler/vm` | 独立功能实现 | 单独调用、调试 |
| **基础设施** | `config/logger` | 全局配置、日志 | 贯穿所有层 |

### 2.2 调用示例

```rust
// 应用层 (CLI)
fn main() {
    let cli = Cli::parse();
    kaubo::config::init(build_config(&cli));
    kaubo::logger::init();
    
    let result = kaubo::compile_and_run(&source)?;
    println!("{}", result.value);
}

// 编排层 (API)
pub fn compile_and_run(source: &str) -> Result<ExecuteOutput> {
    let tokens = lex(source)?;      // 阶段1
    let ast = parse(&tokens)?;       // 阶段2
    let compiled = compile(&ast)?;   // 阶段3
    execute(&compiled)               // 阶段4
}

// 阶段层 (独立使用)
let tokens = kaubo::lex(source)?;
```

---

## 3. 日志系统

### 3.1 设计目标

- **分阶段控制**: 可独立控制 lexer/parser/compiler/vm 的日志级别
- **结构化输出**: 支持 JSON 格式供工具解析
- **低开销**: 未启用的日志在编译期优化掉
- **跨度追踪**: 支持嵌套调用链追踪

### 3.2 日志分级

| 级别 | 用途 | 示例 |
|------|------|------|
| `TRACE` | 最详细，逐行调试 | 每个字符处理、每条指令执行 |
| `DEBUG` | 开发调试 | Token生成、AST节点、函数调用 |
| `INFO` | 正常流程信息 | 阶段开始/完成、编译成功 |
| `WARN` | 警告 | 不建议的用法、性能问题 |
| `ERROR` | 错误 | 编译错误、运行时错误 |

### 3.3 阶段过滤

```rust
// 只查看 Parser 的 DEBUG 以上日志
$ kaubo script.kaubo --log-parser=debug

// 查看全部 TRACE，但 VM 只显示 INFO
$ kaubo script.kaubo -vvv --log-vm=info
```

### 3.4 日志格式

**Pretty 格式**（开发使用）:
```
2024-02-10T10:23:45.123Z DEBUG kaubo::parser: parse variable_declaration
2024-02-10T10:23:45.124Z DEBUG kaubo::compiler: compile binary_op Add
2024-02-10T10:23:45.125Z TRACE kaubo::vm: op=LoadConst0 stack=[]
2024-02-10T10:23:45.126Z TRACE kaubo::vm: op=LoadConst1 stack=[SMI(1)]
```

**JSON 格式**（工具集成）:
```json
{
  "timestamp": "2024-02-10T10:23:45.123Z",
  "level": "DEBUG",
  "target": "kaubo::parser",
  "fields": {
    "message": "parse variable_declaration",
    "line": 10
  }
}
```

### 3.5 代码中使用

```rust
use tracing::{debug, error, info, instrument, trace, warn};

// 简单日志
debug!("Processing {} tokens", count);

// 结构化日志
debug!(token.kind = ?kind, token.value = %value, "Produced token");

// 函数级跨度（自动记录进入/退出）
#[instrument(target = "kaubo::lexer", skip(source))]
pub fn lex(source: &str) -> Result<Vec<Token>> {
    info!("Starting lexer");
    // ...
}

// 条件日志
if tracing::enabled!(tracing::Level::TRACE) {
    trace!("Expensive debug: {:?}", expensive_operation());
}
```

---

## 4. 配置管理

### 4.1 全局配置单例

```rust
// src/config.rs
use once_cell::sync::OnceCell;

static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

pub fn init(config: Config) {
    GLOBAL_CONFIG.set(config).ok();
}

pub fn config() -> &'static Config {
    GLOBAL_CONFIG.get().expect("Config not initialized")
}
```

### 4.2 配置结构

```rust
pub struct Config {
    /// 日志配置
    pub log: LogConfig,
    /// 执行限制
    pub limits: LimitConfig,
    /// 编译选项
    pub compiler: CompilerConfig,
}

pub struct LogConfig {
    pub global: Level,           // 默认级别
    pub lexer: Option<Level>,    // None = 使用 global
    pub parser: Option<Level>,
    pub compiler: Option<Level>,
    pub vm: Option<Level>,
}

pub struct LimitConfig {
    pub max_stack_size: usize,      // 默认 1024
    pub max_recursion_depth: usize, // 默认 256
}

pub struct CompilerConfig {
    pub emit_debug_info: bool,  // 是否生成调试信息
}
```

### 4.3 CLI 配置映射

```bash
# 基础级别
kaubo script.kaubo              # WARN (默认)
kaubo script.kaubo -v           # INFO
kaubo script.kaubo -vv          # DEBUG
kaubo script.kaubo -vvv         # TRACE

# 阶段级控制
kaubo script.kaubo --log-lexer=trace
kaubo script.kaubo --log-parser=off --log-vm=debug
```

---

## 5. 核心组件

### 5.1 词法分析器 (Lexer)

**位置**: `src/compiler/lexer/`

**日志目标**: `kaubo::lexer`

**关键日志点**:
- 字符输入 (`TRACE`)
- Token 生成 (`DEBUG`)
- 状态机匹配 (`TRACE`)
- 未匹配警告 (`WARN`)

### 5.2 语法分析器 (Parser)

**位置**: `src/compiler/parser/`

**日志目标**: `kaubo::parser`

**关键日志点**:
- 规则进入/退出 (`DEBUG` span)
- Token 消费 (`TRACE`)
- AST 节点生成 (`DEBUG`)

### 5.3 字节码编译器 (Compiler)

**位置**: `src/runtime/compiler.rs`

**日志目标**: `kaubo::compiler`

**关键日志点**:
- 变量解析 (`DEBUG`)
- 指令生成 (`TRACE`)
- ShapeID 分配 (`DEBUG`)

### 5.4 虚拟机 (VM)

**位置**: `src/runtime/vm.rs`

**日志目标**: `kaubo::vm`

**关键日志点**:
- 指令执行 (`TRACE`)
- 函数调用 (`DEBUG`)
- 栈操作 (`TRACE`)

---

## 6. 内存布局

### 6.1 Value 类型 (NaN Boxing)

```
[63] Sign [62-52] Exponent(0x7FF) [51] QNAN [50-44] Tag [43-0] Payload

标签分配:
  0-7   : 特殊值 (null, true, false, SMI)
  8-23  : 内联整数 (-8 ~ +7)
  32    : 堆对象 (Boxed)
  33    : 字符串
  34    : 函数
  35    : 列表
  37    : 闭包
  38    : 协程
  42    : 模块
  43    : 原生函数
```

### 6.2 模块 ShapeID

```rust
// 编译期确定
"std" 模块:
  "print"     -> ShapeID 0
  "assert"    -> ShapeID 1
  "type"      -> ShapeID 2
  "sqrt"      -> ShapeID 4
  "PI"        -> ShapeID 9

// 运行时访问
ModuleGet 0   // O(1) 直接索引
```

---

## 7. 文件结构

```
src/
├── lib.rs              # 库入口：导出公共 API
├── main.rs             # CLI 入口：参数解析、流程调度
├── api.rs              # 【新增】高层 API (compile, run 等)
├── config.rs           # 【新增】全局配置
├── logger.rs           # 【新增】日志初始化
│
├── compiler/           # 编译器前端
│   ├── lexer/
│   │   ├── builder.rs
│   │   ├── token_kind.rs
│   │   └── logging.rs  # 【新增】Lexer 日志工具
│   └── parser/
│       ├── parser.rs
│       ├── expr.rs
│       ├── stmt.rs
│       └── logging.rs  # 【新增】Parser 日志工具
│
├── runtime/            # 运行时
│   ├── bytecode/
│   ├── stdlib/
│   ├── vm.rs
│   ├── vm/
│   │   └── logging.rs  # 【新增】VM 日志工具
│   ├── compiler.rs
│   ├── object.rs
│   └── value.rs
│
└── kit/                # 工具库

tests/
├── api_tests.rs        # 【新增】API 层测试
├── lexer_tests.rs      # 【新增】Lexer 独立测试
├── parser_tests.rs     # 【新增】Parser 独立测试
├── compiler_tests.rs   # 【新增】Compiler 独立测试
├── vm_tests.rs         # VM 执行测试
├── stdlib_tests.rs     # 标准库测试
└── common/
    └── mod.rs          # 测试共享工具
```

---

## 8. 关键设计决策

| 决策 | 方案 | 原因 |
|------|------|------|
| 日志框架 | `tracing` | 结构化、异步、支持 span |
| CLI 框架 | `clap` | 声明式、功能全、文档好 |
| 全局配置 | `once_cell` | 线程安全、延迟初始化 |
| 错误处理 | `thiserror` | 类型安全、自动实现 Error |
| 配置传递 | 单例模式 | 避免层层传递参数 |
| 日志级别 | 分阶段独立控制 | 便于调试特定阶段问题 |

---

*最后更新: 2026-02-10*  
*版本: 3.0 (架构重构)*
