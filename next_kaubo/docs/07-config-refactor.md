# 配置系统重构方案

## 问题分析

当前配置系统存在以下问题：

1. **全局单例在 core 层** - `kaubo-core/src/config.rs` 包含 `OnceCell<Config>`，但 core 应该是纯逻辑层
2. **隐式依赖** - core 代码通过 `config::config()` 全局函数读取配置，增加了隐式状态依赖
3. **平台耦合** - `LogConfig` 包含日志级别，但日志初始化在 CLI 层，配置却在 core 层
4. **Web 不友好** - 全局单例模式不适合 Web/WASM 环境

## 目标架构

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

## 实施步骤

### Phase 1: 创建 kaubo-config crate
1. 创建 `kaubo-config/` 目录
2. 移动 `Config`, `CompilerConfig`, `LimitConfig`, `Phase` 数据结构
3. 移除 `LogConfig`（移到 kaubo-cli）
4. 移除全局单例逻辑（`init()`, `config()`, `is_initialized()`）

### Phase 2: 重构 kaubo-core
1. 移除 `config.rs` 和 `logger.rs`
2. 所有函数改为接受配置参数：
   - `compile(source, &CompilerConfig)`
   - `VM::new(&LimitConfig)`
3. 更新所有内部调用

### Phase 3: 重构 kaubo-api
1. 创建新的 `RunConfig` 结构体
2. 提供全局单例（供 CLI 使用）：`static GLOBAL_CONFIG: OnceCell<RunConfig>`
3. 新的 API 函数接受 `&RunConfig` 参数
4. 保留向后兼容的 API（使用默认配置）

### Phase 4: 重构 kaubo-cli
1. 创建 `CliConfig` 结构体，包含 `LogConfig` 和 `RunConfig`
2. 日志初始化移到 `logging.rs`
3. 参数解析映射到 `CliConfig`
4. 更新 main.rs 调用流程

### Phase 5: 更新测试
1. 测试不再依赖全局配置初始化
2. 每个测试独立构造配置

## 兼容性考虑

- **向后兼容**：kaubo-api 提供默认配置 API，旧代码仍能编译
- **Web 友好**：Web 版本可以实现自己的配置源
- **测试友好**：测试可以传入特定配置，无需修改全局状态

## 时间表

预计工作量：2-3 小时
影响范围：4 个 crate 的配置相关代码
测试验证：确保所有 187 个测试仍通过
