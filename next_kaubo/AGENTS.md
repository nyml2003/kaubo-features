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

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Source    │ -> │    Lexer    │ -> │   Parser    │ -> │  Compiler   │
│   (.kaubo)  │    │ (kaubo::lexer)│   │(kaubo::parser)│  │(kaubo::compiler)│
│             │    │  src/kit/lexer/│   │              │  │              │
└─────────────┘    └─────────────┘    └─────────────┘    └──────┬──────┘
                                                                  │
┌─────────────┐    ┌─────────────┐    ┌─────────────┐            │
│   Output    │ <- │     VM      │ <- │    Chunk    │ <──────────┘
│  (stdout)   │    │ (kaubo::vm) │    │  (bytecode) │
└─────────────┘    └─────────────┘    └─────────────┘
```

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
```

## 已知限制

1. **无浮点数字面量** - 当前 `3.14` 无法直接解析，需用 `std.sqrt` 等间接获得
2. **逻辑与/或** - 需要短路求值实现（当前测试已跳过）
3. **GC 缺失** - 只分配不回收
4. **文档测试** - 6 个 doc test 失败（示例需要 config 初始化）

## 最近改进

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

*最后更新: 2026-02-10*
