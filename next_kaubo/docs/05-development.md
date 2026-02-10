# Kaubo 开发手册

> 面向开发者的技术文档、日志使用指南和贡献说明

## 目录

1. [开发环境](#1-开发环境)
2. [项目结构](#2-项目结构)
3. [日志使用指南](#3-日志使用指南)
4. [CLI 参数](#4-cli-参数)
5. [添加新特性](#5-添加新特性)
6. [调试技巧](#6-调试技巧)

---

## 1. 开发环境

### 1.1 依赖

```bash
# Rust 1.70+ (推荐 latest stable)
rustup update

# 克隆项目
git clone <repo-url>
cd next_kaubo
```

### 1.2 构建与运行

```bash
# 调试构建
cargo build

# 发布构建
cargo build --release

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 代码检查
cargo clippy
```

---

## 2. 项目结构

### 2.1 新增文件说明

| 文件 | 用途 |
|------|------|
| `src/api.rs` | 高层 API：compile, run, compile_and_run |
| `src/config.rs` | 全局配置：LogConfig, LimitConfig 等 |
| `src/logger.rs` | 日志初始化：tracing subscriber 配置 |
| `src/*/logging.rs` | 各阶段日志工具函数 |

### 2.2 关键路径

```
src/
├── main.rs          # CLI 入口（只负责参数解析和调度）
├── lib.rs           # 库入口（导出 api, config, logger）
├── api.rs           # 编排层（组合各阶段）
├── config.rs        # 全局配置单例
├── logger.rs        # 日志系统初始化
├── compiler/        # 编译器前端
└── runtime/         # 运行时与 VM
```

---

## 3. 日志使用指南

### 3.1 日志级别选择

| 场景 | 级别 | 示例 |
|------|------|------|
| 逐字符/逐指令处理 | `trace!` | 消费字符、执行指令 |
| 重要节点信息 | `debug!` | Token生成、AST节点 |
| 阶段开始/完成 | `info!` | Lexer完成、编译成功 |
| 不推荐用法 | `warn!` | 过时语法、性能问题 |
| 错误 | `error!` | 未定义变量、运行时错误 |

### 3.2 在代码中添加日志

```rust
use tracing::{debug, error, info, instrument, trace, warn};

// 1. 简单日志
debug!("Processing {} tokens", count);

// 2. 结构化日志（推荐）
debug!(
    token.kind = ?kind,      // Debug 格式化
    token.value = %value,     // Display 格式化
    "Produced token"
);

// 3. 函数级跨度（自动记录调用链）
#[instrument(target = "kaubo::lexer", skip(source))]
pub fn lex(source: &str) -> Result<Vec<Token>> {
    info!("Starting lexer");
    // 函数内所有日志自动包含 span 信息
    debug!("Read {} chars", source.len());
}

// 4. 条件日志（避免昂贵计算）
if tracing::enabled!(tracing::Level::TRACE) {
    trace!("Expensive: {:?}", expensive_debug_info());
}
```

### 3.3 各阶段日志目标

```rust
// Lexer
trace!(target: "kaubo::lexer", "feed: '{}'", ch);
debug!(target: "kaubo::lexer", "token: {:?}", token);

// Parser  
debug!(target: "kaubo::parser", "parse variable_declaration");

// Compiler
debug!(target: "kaubo::compiler", "compile binary_op Add");

// VM
trace!(target: "kaubo::vm", "op={:?} stack={:?}", op, stack);
```

### 3.4 使用日志工具模块

```rust
// src/compiler/lexer/logging.rs
pub struct LexerLogger;

impl LexerLogger {
    #[inline]
    pub fn token_produced(kind: &str, value: &str) {
        debug!(target: "kaubo::lexer", kind, value, "token produced");
    }
}

// 使用
LexerLogger::token_produced("Identifier", "x");
```

---

## 4. CLI 参数

### 4.1 基本用法

```bash
# 默认运行（仅警告）
kaubo script.kaubo

# 显示编译和执行过程
kaubo script.kaubo -v

# 详细调试信息
kaubo script.kaubo -vv

# 最详细（逐指令追踪）
kaubo script.kaubo -vvv
```

### 4.2 分阶段日志控制

```bash
# 只查看 Lexer 的详细日志
kaubo script.kaubo --log-lexer=trace

# Parser 关闭，其他 DEBUG
kaubo script.kaubo --log-parser=off -vv

# 组合使用
kaubo script.kaubo \
    --log-lexer=warn \
    --log-parser=debug \
    --log-compiler=info \
    --log-vm=trace
```

### 4.3 输出格式

```bash
# 默认：彩色格式化（开发）
kaubo script.kaubo

# JSON 格式（工具集成）
kaubo script.kaubo --format=json -vvv 2> trace.json

# 紧凑格式
kaubo script.kaubo --format=compact
```

### 4.4 开发调试

```bash
# 仅编译，查看字节码
kaubo script.kaubo --compile-only --dump-bytecode

# 完整追踪重定向到文件
kaubo script.kaubo -vvv 2> debug.log

# 使用环境变量（临时）
RUST_LOG=kaubo::vm=trace,kaubo::lexer=off kaubo script.kaubo
```

### 4.5 参数参考

| 参数 | 说明 | 示例 |
|------|------|------|
| `-v`, `--verbose` | 增加日志级别（可叠加） | `-v`, `-vv`, `-vvv` |
| `--log-lexer` | Lexer 日志级别 | `--log-lexer=debug` |
| `--log-parser` | Parser 日志级别 | `--log-parser=trace` |
| `--log-compiler` | Compiler 日志级别 | `--log-compiler=warn` |
| `--log-vm` | VM 日志级别 | `--log-vm=info` |
| `--format` | 日志格式 | `--format=json` |
| `--compile-only` | 仅编译不执行 | `--compile-only` |
| `--dump-bytecode` | 打印字节码 | `--dump-bytecode` |

---

## 5. 添加新特性

### 5.1 添加新的日志点

```rust
// 1. 在对应模块的 logging.rs 中添加工具函数
// src/runtime/vm/logging.rs

impl VmLogger {
    pub fn new_instruction(op: OpCode) {
        trace!(target: "kaubo::vm", "new instruction");
    }
}

// 2. 在代码中使用
VmLogger::new_instruction(op);
```

### 5.2 添加新的 CLI 参数

```rust
// src/main.rs

#[derive(Parser)]
struct Cli {
    // 现有参数...
    
    /// 新的参数
    #[arg(long)]
    new_option: Option<String>,
}

fn build_config(cli: &Cli) -> Config {
    Config {
        // 使用新参数
        new_field: cli.new_option.clone(),
        ..Default::default()
    }
}
```

### 5.3 添加 API 函数

```rust
// src/api.rs

/// 新的公共 API
pub fn compile_only(source: &str) -> Result<Chunk> {
    let _span = span!(Level::INFO, "compile_only").entered();
    
    let tokens = lex(source)?;
    let ast = parse(&tokens)?;
    let output = compile_ast(&ast)?;
    
    Ok(output.chunk)
}
```

---

## 6. 调试技巧

### 6.1 定位问题阶段

```bash
# 1. 确认 Lexer 输出是否正确
kaubo script.kaubo --log-lexer=debug --compile-only

# 2. 确认 Parser AST 是否正确
kaubo script.kaubo --log-parser=debug --compile-only

# 3. 确认字节码生成
kaubo script.kaubo --log-compiler=debug --dump-bytecode

# 4. 追踪 VM 执行
kaubo script.kaubo --log-vm=trace 2> vm.log
```

### 6.2 使用 span 追踪调用链

```rust
#[instrument(target = "kaubo::compiler", skip_all)]
pub fn compile_function(func: &Function) -> Result<Chunk> {
    // 此函数内的所有日志自动包含函数名和参数
    debug!("Compiling function");  // 会自动带上 function=xxx
}
```

### 6.3 测试中的日志

```rust
// tests/common/mod.rs

pub fn run_code_debug(source: &str) -> Result<ExecResult> {
    // 测试时使用详细日志
    let config = Config {
        log: LogConfig {
            global: Level::DEBUG,
            lexer: Some(Level::TRACE),
            vm: Some(Level::DEBUG),
            ..Default::default()
        },
        ..Default::default()
    };
    
    init(config);
    compile_and_run(source)
}

// 测试中使用
#[test]
fn test_feature() {
    let result = run_code_debug("...");  // 详细日志
}
```

### 6.4 性能分析

```bash
# 启用时间戳
kaubo script.kaubo --format=compact -vvv

# 过滤特定阶段计时
cargo run --release -- script.kaubo -vvv 2>&1 | grep "duration"
```

---

## 附录：常用命令速查

```bash
# 开发调试
cargo test -- --nocapture
cargo test test_name -- --nocapture
RUST_LOG=kaubo::vm=trace cargo test

# 性能测试
cargo build --release
time ./target/release/kaubo script.kaubo

# 日志分析
kaubo script.kaubo -vvv --format=json 2> trace.json
cat trace.json | jq 'select(.target == "kaubo::vm")'
```

---

*最后更新: 2026-02-10*  
*版本: 3.0 (架构重构)*
