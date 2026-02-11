# 配置系统重构方案

> **状态**: ✅ 已实施（2026-02-12）

本文档记录配置系统重构的方案和结果。

## 重构前的问题

1. **全局单例在 core 层** - `kaubo-core/src/config.rs` 包含 `OnceCell<Config>`，但 core 应该是纯逻辑层
2. **隐式依赖** - core 代码通过 `config::config()` 全局函数读取配置，增加了隐式状态依赖
3. **平台耦合** - `LogConfig` 包含日志级别，但日志初始化在 CLI 层，配置却在 core 层
4. **Web 不友好** - 全局单例模式不适合 Web/WASM 环境

## 最终架构 (已实施)

```
kaubo-config (新 crate)
├── src/lib.rs
│   ├── Config { compiler, limits }     # 纯数据结构
│   ├── CompilerConfig                  # 编译选项
│   ├── LimitConfig                     # 执行限制
│   └── Phase                           # 编译阶段枚举
└── Cargo.toml                          # 无依赖，纯数据

kaubo-core
├── 所有函数接受 &CompilerConfig 参数   # 不再读取全局配置
└── 移除 config.rs 和 logger.rs          # 移到对应层

kaubo-api
├── RunConfig {                         # 执行配置
│   pub show_steps: bool,
│   pub dump_bytecode: bool,
│   pub compiler: CompilerConfig,       # 从 kaubo-config
│   pub limits: LimitConfig,            # 从 kaubo-config
│ }
├── static GLOBAL_CONFIG: OnceCell<RunConfig>  # CLI 用的全局单例
└── pub fn run(source: &str, config: &RunConfig) -> Result<...>  # 新 API

kaubo-cli
├── LogConfig { global, lexer, parser, ... }   # CLI 特有的日志配置
├── logging.rs: init_logger(&LogConfig)        # 日志初始化
├── config.rs: CliConfig { log: LogConfig, run: RunConfig }  # CLI 完整配置
└── main.rs: 解析 CLI args -> CliConfig -> 调用 init_logger + run()

kaubo-web (将来)
├── WebConfig { ... }                    # Web 特有的配置
└── 使用 LocalStorage 存储配置，无需 OnceCell
```

## 依赖关系

```
kaubo-config (无依赖)
    ↑
kaubo-core (依赖 kaubo-config)
    ↑
kaubo-api (依赖 kaubo-core, kaubo-config)
    ↑
kaubo-cli (依赖 kaubo-api, kaubo-core, kaubo-config)
```

## API 变更

### 当前用法（将被移除）
```rust
// 旧代码 - 隐式依赖全局配置
use kaubo::config::{init, Config};
init(Config::default());
let result = kaubo::compile_and_run(source)?;
```

### 新用法
```rust
// 显式传递配置
use kaubo_api::{run, RunConfig};
use kaubo_config::{CompilerConfig, LimitConfig};

let config = RunConfig {
    show_steps: true,
    dump_bytecode: false,
    compiler: CompilerConfig::default(),
    limits: LimitConfig::default(),
};
let result = run(source, &config)?;
```

### CLI 层用法
```rust
use kaubo_cli::{CliConfig, LogConfig, init_logger};

let cli_config = CliConfig::from_args();  // 从 clap 解析
init_logger(&cli_config.log);             // 初始化日志
let result = run(source, &cli_config.run)?;  // 执行
```

## 实施结果

### 已完成的变更

| Phase | 状态 | 说明 |
|-------|------|------|
| Phase 1 | ✅ | `kaubo-config` crate 已创建，包含纯数据结构 |
| Phase 2 | ✅ | `kaubo-core` 已移除全局状态，通过参数接收配置 |
| Phase 3 | ✅ | `kaubo-api` 已实现 `RunConfig` 和全局单例 |
| Phase 4 | ✅ | `kaubo-cli` 已实现 `LogConfig` 和日志初始化 |
| Phase 5 | ✅ | 所有测试已更新，187+ 测试通过 |

### 代码示例

**重构前（单 crate）**:
```rust
// 隐式依赖全局配置
use kaubo::config::{init, Config};
init(Config::default());
let result = kaubo::compile_and_run(source)?;
```

**重构后（Workspace）**:
```rust
// 显式传递配置
use kaubo_api::{run, RunConfig};
let config = RunConfig::default();
let result = run(source, &config)?;

// 或使用全局单例（CLI 便利）
use kaubo_api::{init_config, quick_run};
init_config(RunConfig::default());
let result = quick_run(source)?;
```

## 架构验证

### 依赖关系

```
kaubo-config (无依赖)
    ↑
kaubo-core (依赖 kaubo-config)
    ↑
kaubo-api (依赖 kaubo-core, kaubo-config)
    ↑
kaubo-cli (依赖 kaubo-api, kaubo-core, kaubo-config)
```

### 配置分层

```
┌─────────────────────────────────────┐
│  kaubo-cli - LogConfig              │
├─────────────────────────────────────┤
│  kaubo-api - RunConfig + 全局单例   │
├─────────────────────────────────────┤
│  kaubo-core - 纯逻辑，参数接收配置  │
├─────────────────────────────────────┤
│  kaubo-config - 纯数据结构          │
└─────────────────────────────────────┘
```

### 测试验证

```bash
$ cargo test --workspace
测试状态: 187 passed, 0 failed
```
