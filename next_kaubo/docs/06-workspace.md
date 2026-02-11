# Workspace 架构指南

> **状态**: ✅ 已实施（2026-02-12）
>
> 本文档说明 Kaubo 的 Workspace 架构设计和最佳实践。
>
> 实际代码结构请参考 `AGENTS.md`。

## 背景

原始的单 crate 架构存在以下问题：
- CLI 特有的逻辑（参数解析、文件读取）混杂在核心代码中
- 平台代码边界不够清晰
- 无法方便地支持多平台（Web/WASM）

Workspace 架构明确分离：
- **核心逻辑**（`kaubo-core`）- 纯内存操作，无全局状态
- **执行编排**（`kaubo-api`）- 跨平台通用接口，含全局单例
- **平台实现**（`kaubo-cli`/`kaubo-web`）- 平台特定的入口和 IO
- **配置数据**（`kaubo-config`）- 纯数据结构，无依赖

## Crate 详细设计

### kaubo-core

**定位**：核心编译器，纯逻辑，无 IO

**公开 API**：
```rust
// 编译
pub fn compile(source: &str) -> Result<Chunk, CompileError>;

// Token/Lexer（供高级用途）
pub use kit::lexer::{Lexer, Token, TokenKind};

// AST（供高级用途）
pub use compiler::ast::{Expr, Stmt, Program};

// 运行时类型
pub use runtime::value::Value;
pub use runtime::bytecode::Chunk;
pub use runtime::vm::VM;
```

**私有实现**：
- Lexer V2（手写 Scanner）
- Parser（递归下降）
- Compiler（AST → Bytecode）
- VM（字节码执行）
- 标准库函数

### kaubo-api

**定位**：执行流程编排，提供跨平台一致的高层接口

**核心类型**：
```rust
/// 执行配置
pub struct RunConfig {
    pub show_steps: bool,
    pub dump_bytecode: bool,
    pub log_config: LogConfig,
}

/// 执行输出
pub struct ExecuteOutput {
    pub value: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
}

/// 统一错误
#[derive(Error, Debug, Clone)]
pub enum KauboError {
    #[error("{0}")]
    Lexer(#[from] kaubo_core::LexerError),
    #[error("{0}")]
    Parser(#[from] kaubo_core::ParserError),
    #[error("{0}")]
    Runtime(#[from] kaubo_core::RuntimeError),
}

impl KauboError {
    pub fn to_report(&self) -> ErrorReport;
    pub fn line(&self) -> Option<usize>;
    pub fn column(&self) -> Option<usize>;
}
```

**核心函数**：
```rust
/// 执行 Kaubo 脚本
/// 
/// 这是最常用的入口，CLI 和 Web 都调用这个函数。
pub fn run(source: &str, config: &RunConfig) -> Result<ExecuteOutput, KauboError>;

/// 仅编译（用于调试）
pub fn compile_only(source: &str) -> Result<Chunk, KauboError>;
```

**设计原则**：
- `kaubo-api` 决定执行流程（编译 → 可选dump → 执行）
- `kaubo-api` 统一错误类型，方便上层处理
- `kaubo-api` 不处理任何平台特定的 IO

### kaubo-cli

**定位**：CLI 平台实现，包含所有终端特有的代码

**结构**：
```
kaubo-cli/
├── Cargo.toml           # 依赖 clap, tracing-subscriber
└── src/
    ├── main.rs          # 入口：clap 解析 → 读取文件 → 调用 runner
    ├── config.rs        # Args → RunConfig 转换
    ├── output.rs        # 终端输出封装
    └── logging.rs       # tracing-subscriber 初始化
```

**main.rs 示意**：
```rust
use clap::Parser;
use kaubo_api::{run, RunConfig};
use std::process;

#[derive(Parser)]
struct Args {
    file: String,
    #[arg(short, long)]
    verbose: bool,
    // ...
}

fn main() {
    let args = Args::parse();
    
    // 1. 读取源文件（CLI 特有）
    let source = std::fs::read_to_string(&args.file)
        .expect("Failed to read file");
    
    // 2. 初始化日志（CLI 特有）
    init_logging(&args);
    
    // 3. 转换配置
    let config = RunConfig::from(&args);
    
    // 4. 执行（调用 kaubo-api）
    match run(&source, &config) {
        Ok(output) => {
            if let Some(value) = output.value {
                println!("{}", value);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            print_source_context(&source, e.line(), e.column());
            process::exit(1);
        }
    }
}
```

### kaubo-web（将来）

**定位**：WASM 平台实现，用于 Web Playground

**结构**：
```
kaubo-web/
├── Cargo.toml           # crate-type = ["cdylib", "rlib"]
└── src/
    ├── lib.rs           # WASM 入口
    ├── config.rs        # WebConfig
    └── output.rs        # 浏览器输出
```

**使用方式**：
```javascript
// JavaScript 调用
import init, { run_kaubo } from './kaubo_web.js';

await init();
const result = run_kaubo("print('Hello')", { show_steps: true });
console.log(result.value);
```

## Workspace 配置

### 根 Cargo.toml

```toml
[workspace]
members = ["kaubo-core", "kaubo-api", "kaubo-cli"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["..."]
repository = "https://github.com/.../kaubo"

[workspace.dependencies]
# 共享依赖
thiserror = "1.0"
tracing = "0.1"

# core 依赖
# (无特殊依赖)

# api 依赖
kaubo-core = { path = "kaubo-core" }

# cli 依赖
kaubo-api = { path = "kaubo-api" }
clap = { version = "4", features = ["derive"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### kaubo-core/Cargo.toml

```toml
[package]
name = "kaubo-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
thiserror = { workspace = true }
tracing = { workspace = true }
```

### kaubo-api/Cargo.toml

```toml
[package]
name = "kaubo-api"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
kaubo-core = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

### kaubo-cli/Cargo.toml

```toml
[package]
name = "kaubo-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "kaubo"
path = "src/main.rs"

[dependencies]
kaubo-api = { workspace = true }
clap = { workspace = true }
tracing-subscriber = { workspace = true }
```

## 迁移步骤

### Phase 1: 创建 Workspace 结构

1. **备份现有代码**
   ```bash
   git checkout -b workspace-refactor
   ```

2. **创建目录结构**
   ```bash
   mkdir -p kaubo-core/src kaubo-api/src kaubo-cli/src
   ```

3. **创建根 Cargo.toml**
   - 定义 workspace members
   - 定义 workspace.dependencies

### Phase 2: 迁移 kaubo-core

1. 移动 `src/core/` → `kaubo-core/src/`
2. 移动 `src/kit/` → `kaubo-core/src/kit/`
3. 创建 `kaubo-core/src/lib.rs`，公开必要类型
4. 更新内部导入路径
5. 确保 `kaubo-core` 能独立编译通过

### Phase 3: 迁移 kaubo-api

1. 移动 `src/api/` → `kaubo-api/src/`
2. 从 `src/platform/cli.rs` 提取平台无关的错误处理到 `kaubo-api/src/error.rs`
3. 创建 `kaubo-api/src/runner.rs`，实现 `run()` 函数
4. 创建 `kaubo-api/src/config.rs`，定义 `RunConfig`
5. `kaubo-api` 依赖 `kaubo-core`，确保编译通过

### Phase 4: 迁移 kaubo-cli

1. 移动 `src/bin/kaubo.rs` → `kaubo-cli/src/main.rs`
2. 移动 `src/platform/cli.rs` → `kaubo-cli/src/output.rs`
3. 创建 `kaubo-cli/src/config.rs`，实现 `Args → RunConfig`
4. 创建 `kaubo-cli/src/logging.rs`，初始化 tracing-subscriber
5. 更新 `main.rs`，使用 `kaubo_api::run()`
6. 确保 CLI 能正常编译运行

### Phase 5: 清理旧代码

1. 删除旧 `src/` 目录
2. 更新根目录下的文档
3. 更新 CI/CD 配置
4. 运行完整测试

### Phase 6: 发布准备（可选）

1. 为 `kaubo-core` 和 `kaubo-api` 添加 crate 元数据
2. 编写 README 和文档
3. 发布到 crates.io

## 最佳实践

### 1. Crate 边界维护

- **禁止循环依赖**：如果 A 依赖 B，B 不能依赖 A
- **最小公开接口**：每个 crate 只公开必要的类型和函数
- **语义版本**：修改公共 API 时考虑版本兼容性

### 2. 测试策略

```bash
# 单元测试（在每个 crate 内）
cargo test -p kaubo-core
cargo test -p kaubo-api

# 集成测试（在 kaubo-cli）
cargo test -p kaubo-cli --test integration

# 所有测试
cargo test --workspace
```

### 3. 文档维护

- 每个 crate 的 `lib.rs` 顶部添加 crate-level 文档
- 公开 API 必须有文档注释
- 复杂逻辑添加示例代码

### 4. 依赖管理

```toml
# 优先使用 workspace 依赖
[dependencies]
thiserror = { workspace = true }  # ✅

# 只在必要时使用具体版本
some-crate = "1.0"  # 只在单个 crate 使用的话
```

## 常见问题

### Q: kaubo-core 和 kaubo-api 的边界在哪里？

A: 
- `kaubo-core`：只做"一件事"（编译/执行），不关心流程
- `kaubo-api`：编排流程，处理配置，统一错误

例如：
- `compile()` 在 `kaubo-core`（只做编译）
- `run()` 在 `kaubo-api`（根据配置决定 compile → dump → execute）

### Q: 为什么要单独一个 kaubo-api，而不是 CLI 直接依赖 core？

A:
1. **代码复用**：Web 和 CLI 共享同样的执行逻辑
2. **一致性**：不同平台的行为一致
3. **可测试性**：API 层可以独立测试
4. **演进灵活性**：以后可以替换 core 的实现，API 保持稳定

### Q: 第三方想嵌入 Kaubo，应该依赖哪个 crate？

A:
- **只需要编译**：依赖 `kaubo-core`
- **完整的执行流程**：依赖 `kaubo-api`
- **不需要自定义平台逻辑**：不需要直接依赖 `kaubo-cli`

## 参考项目

| 项目 | 结构 | 说明 |
|------|------|------|
| [swc](https://github.com/swc-project/swc) | `swc_core` + `swc_cli` + `binding_*` | 类似的 core/cli/绑定分离 |
| [biome](https://github.com/biomejs/biome) | `biome_*` crates | 细粒度的 crate 拆分 |
| [ruff](https://github.com/astral-sh/ruff) | `ruff` + `ruff_cli` | core + cli 两层结构 |

---

*创建时间: 2026-02-11*
