# Kaubo 开发指南 (For AI Agents)

> 本指南帮助 AI 助手快速理解项目结构和开发规范

## 项目概览

**Kaubo** 是一门现代脚本语言，当前处于 **架构 3.0 阶段**（已稳定）。

```
阶段: 3.0 (架构重构完成)
测试: 187 passed, 0 failed
警告: 0
```

## 核心架构

### 目录结构（目标）

```
src/
├── bin/                 # CLI 二进制入口（纯前端，无业务逻辑）
│   └── kaubo.rs         # 参数解析 + 调用 lib
├── lib.rs               # 库入口
├── api/                 # API 层（对外接口）
│   ├── mod.rs           # compile, compile_and_run
│   ├── error.rs         # KauboError, ErrorReport（统一错误）
│   └── types.rs         # CompileOutput, ExecuteOutput
├── core/                # 核心编译器（纯逻辑，无 IO）
│   ├── config.rs        # 配置定义（纯数据结构）
│   ├── logger.rs        # 日志接口定义
│   ├── kit/             # 通用工具（Lexer V2 等）
│   ├── compiler/        # 编译器前端（Parser、AST）
│   └── runtime/         # 运行时（VM、Bytecode）
└── platform/            # 平台适配层（所有 IO 副作用）
    ├── cli.rs           # CLI 格式化、错误打印、终端输出
    ├── fs.rs            # 文件操作
    └── log.rs           # tracing 初始化实现
```

### 分层边界

| 层级 | 职责 | 依赖规则 |
|------|------|----------|
| **bin** | 参数解析，调用 `kaubo::run_cli()` | 只依赖 lib |
| **api** | 对外承诺的接口，输入 → 输出 | 依赖 core，不依赖 platform |
| **core** | 纯编译逻辑，只操作内存数据结构 | 无 IO，不依赖 platform |
| **platform** | 所有 IO 副作用（文件、日志、终端） | 可被 bin 使用，core 不能依赖 |

### 数据流向

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────────────────┐
│   CLI 参数   │ --> │  bin/kaubo  │ --> │        platform/            │
│  (clap 解析) │     │  (入口转发) │     │  - cli.rs: 格式化输出       │
└─────────────┘     └─────────────┘     │  - fs.rs: 文件读写          │
                                        │  - log.rs: 日志初始化       │
                                        └─────────────┬───────────────┘
                                                      │
                              ┌───────────────────────┘
                              v
┌─────────────┐     ┌─────────────────┐     ┌─────────────────────────┐
│   返回结果   │ <-- │    api/         │ <-- │        core/            │
│  (格式化后)  │     │  - compile()    │     │  - kit/: Lexer          │
└─────────────┘     │  - run_cli()    │     │  - compiler/: Parser    │
                    │  - KauboError   │     │  - runtime/: VM         │
                    └─────────────────┘     └─────────────────────────┘
                            │
                            └--> 纯内存操作，无 IO
```

### 设计原则

1. **CLI 与编译器隔离**：`main.rs` 只解析参数，业务逻辑在 `api/` 或 `core/`
2. **纯逻辑无 IO**：`core/` 只操作内存，所有副作用在 `platform/`
3. **API 层呈现中立**：`api/` 返回结构化数据，不决定如何打印
4. **平台适配可替换**：`platform/` 的实现可被替换（如 Web 版不需要文件 IO）

## 关键文件

| 文件 | 职责 | 修改频率 |
|------|------|----------|
| `src/api.rs` | 公共 API (`compile`, `run`) | 低 |
| `src/config.rs` | 全局配置 | 低 |
| `src/logger.rs` | 日志初始化 | 低 |
| `src/main.rs` | CLI 入口 | 中 |
| `src/kit/lexer/` | 词法分析器 (手写 Scanner) | 中 |
| `src/compiler/parser/` | 语法分析 | 中 |
| `src/compiler/lexer/token_kind.rs` | Token 类型定义 | 低 |
| `src/runtime/compiler.rs` | AST → Bytecode | 高 |
| `src/runtime/vm.rs` | 虚拟机执行 | 高 |
| `src/runtime/stdlib/` | 标准库 | 高 |

## 开发规范

### 1. 日志使用

**必须使用 `tracing`，禁止直接使用 `println!`/`eprintln!`**

```rust
use tracing::{debug, error, info, trace, warn};

// ✅ 正确：使用 tracing
trace!(target: "kaubo::lexer", "Processing char: {}", ch);
debug!(target: "kaubo::compiler", op = ?op, "Compiling");

// ❌ 错误：直接使用 println
println!("Debug: {:?}", value);
```

**例外情况**（仅允许在 `main.rs`）：
- 程序的实际输出（脚本执行结果）
- 用户可见的错误信息

### 2. 错误处理

使用 `thiserror` 定义错误类型：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Unexpected EOF")]
    UnexpectedEof,
}
```

### 3. 配置访问

```rust
use crate::config;

// 读取配置
let cfg = config::config();
let level = cfg.log.level_for(Phase::Lexer);
```

### 4. 添加标准库函数

参考 `src/runtime/stdlib/mod.rs`：

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

### 5. 先对齐再执行

**任何非 trivial 的改动，必须先讨论方案，达成共识后再动手。**

- ✅ 正确：先讨论设计，确认后再实现
- ❌ 错误：直接开始编码，可能导致返工

**什么情况下需要先对齐：**
- 架构调整或重构
- 新增公共 API
- 修改核心数据结构
- 破坏性变更（breaking changes）

### 6. 长期优先，拒绝临时方案

**以长期合理架构为优先，不为兼容老 API 而妥协设计。**

- ✅ 正确：重构 API 使其更 Rust 化，即使要修改调用处
- ❌ 错误：为兼容老代码而保留奇怪的设计

**原则：**
- 接受必要的破坏性变更
- 不积累技术债务
- 当下就做好，不指望"以后再重构"

## 常用命令

```bash
# 运行测试
cargo test

# 运行特定测试
cargo test --test integration_test

# 检查警告
cargo check

# 运行示例
cargo run --release -- assets/hello.kaubo

# 带日志运行
cargo run --release -- assets/hello.kaubo -vv

# 仅查看词法分析阶段日志
cargo run --release -- assets/hello.kaubo --log-lexer=trace

# 查看产生的 Token 列表
cargo run --release -- assets/hello.kaubo --log-lexer=debug 2>&1 | grep "produced token"
```

## 已知限制

1. **无浮点数字面量** - 当前 `3.14` 无法直接解析，需用 `std.sqrt` 等间接获得
2. **逻辑与/或** - 需要短路求值实现（当前测试已跳过）
3. **GC 缺失** - 只分配不回收
4. **文档测试** - 6 个 doc test 失败（示例需要 config 初始化）

## 最近改进

### 结构化错误处理 (2026-02-11)

**分层错误设计** - 底层提供结构化数据，上层负责格式化呈现：

```rust
// 底层结构化错误
pub struct LexerError {
    pub kind: ErrorKind,           // InvalidChar, UnterminatedString, ...
    pub position: SourcePosition,  // 精确位置
    pub message: String,
}

// 统一错误枚举
#[derive(Error, Debug, Clone)]
pub enum KauboError {
    #[error("{0}")]
    Lexer(#[from] LexerError),
    #[error("{0}")]
    Parser(#[from] ParserError),
    ...
}

// 错误报告（用于跨层传递）
pub struct ErrorReport {
    pub phase: &'static str,       // lexer, parser, compiler, runtime
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub error_kind: String,        // 机器可读的错误类型
    pub message: String,           // 人类可读的消息
    pub details: Option<ErrorDetails>,
}
```

**分层使用**:

```rust
// 底层 API 返回结构化错误
let result = compile_and_run(source);

match result {
    Err(e) => {
        // 1. 获取结构化报告
        let report = e.to_report();
        
        // 2. CLI 场景 - 直接打印
        println!("{}", report);  // [10:5] parser error: ...
        
        // 3. Web API 场景 - 输出 JSON
        let json = report.to_json();
        // {"phase":"parser","line":10,"column":5,...}
        
        // 4. 自定义格式
        println!("{}: {}", report.phase, report.message);
    }
    Ok(_) => {}
}
```

**设计原则**:
- 底层（API）只提供数据，不做格式化
- 上层（CLI/Web/LSP）决定如何呈现
- 支持多种输出格式：`Display`（CLI）、`to_json()`（Web）、字段访问（自定义）

### 错误定位 (2026-02-10)

语法分析错误现在包含精确的行号和列号，并显示多行源代码上下文：

```
❌ Parser error: [14:15] Missing right parenthesis ')'
----|--
 11 | // 第11行
 12 | // 第12行
 13 | // 第13行 - 错误在这里
 14 | var y = (1 + 2;
    |               ^
 15 | // 第15行
 16 | // 第16行
----|--
```

**实现概要**:
- `ParserError` 包含 `kind` 和 `location` 字段
- `ErrorLocation` 支持 `At(Coordinate)`、`After(Coordinate)`、`Eof`、`Unknown`
- Parser 在产生错误时自动捕获当前 token 的位置
- API 错误类型 `KauboError::Parser` 保留 `line` 和 `column` 字段
- CLI 显示错误行前后各2行上下文，用 `^` 标记错误位置
- 行号自动对齐，分隔线自适应宽度

## Token 结构变更

Lexer V2 改造后 Token 结构发生变化：

| 旧字段 | 新字段 | 说明 |
|--------|--------|------|
| `token.coordinate` | `token.span.start` | Position 在 span 内 |
| `token.value` | `token.text` | 类型改为 `Option<String>` |

**迁移示例**:
```rust
// 访问行号
// 旧: token.coordinate.line
// 新: token.span.start.line

// 访问文本
// 旧: token.value.clone()
// 新: token.text.clone().unwrap_or_default()
```

## 扩展方向

| 优先级 | 任务 | 复杂度 |
|--------|------|--------|
| 高 | 浮点数字面量支持 | 中 |
| 中 | Phase 1: SourceSpan 集成到错误系统 | 中 |
| 中 | 字符串/列表标准库方法 | 低 |
| 中 | `@ProgramStart` 装饰器 | 中 |
| 低 | 垃圾回收 | 高 |
| 低 | 调试器支持 | 高 |

## 技术债务

- [x] ~~`parse()` API 需要重构以支持直接传入 tokens~~ ✅ 已完成（Lexer V2）
- [x] ~~Lexer 状态机需要简化~~ ✅ 已完成（替换为手写 Scanner）
- [ ] 部分 `#[allow(dead_code)]` 需要清理或实现
- [ ] 文档测试需要配置初始化支持
- [ ] Parser 错误系统需要迁移到 SourceSpan

## 参考文档

- `docs/01-syntax.md` - 语法参考
- `docs/02-architecture.md` - 架构设计
- `docs/03-stdlib.md` - 标准库 API
- `docs/04-testing.md` - 测试指南
- `docs/05-development.md` - 开发手册

---

*最后更新: 2026-02-11*
