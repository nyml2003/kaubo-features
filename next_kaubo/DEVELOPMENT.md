# Kaubo 开发指南

## 快速开始

```bash
# 安装必要工具
cargo make install-tools

# 检查环境
cargo make check-env

# 构建项目
cargo make build

# 运行测试
cargo make test

# 运行示例
cargo make run
```

## 常用命令

### 构建

| 命令 | 说明 |
|------|------|
| `cargo make build` | 构建 CLI release 版本 |
| `cargo make build-dev` | 构建 CLI debug 版本 |
| `cargo make build-all` | 构建所有 workspace 成员 |

### 测试

| 命令 | 说明 |
|------|------|
| `cargo make test` | 运行所有测试 (491 个) |
| `cargo make test-core` | 运行 kaubo-core 测试 |
| `cargo make test-api` | 运行 kaubo-api 测试 |
| `cargo make test-log` | 运行 kaubo-log 测试 |
| `cargo make test-watch` | 持续测试 (需 cargo-watch) |

### 运行示例

| 命令 | 说明 |
|------|------|
| `cargo make run` | 运行 Hello World 示例 |
| `cargo make run-fib` | 运行斐波那契示例 |
| `cargo make run-calc` | 运行计算器示例 |
| `cargo make run-verbose` | 运行 (详细输出) |
| `cargo make run-file FILE=examples/test.kaubo` | 运行指定文件 |
| `cargo make compile` | 编译并显示字节码 |

### 代码质量

| 命令 | 说明 |
|------|------|
| `cargo make check` | 检查代码 |
| `cargo make lint` | 运行 clippy |
| `cargo make fmt` | 格式化代码 |
| `cargo make fmt-check` | 检查代码格式 |
| `cargo make quality` | 全套代码质量检查 |

### 覆盖率

| 命令 | 说明 |
|------|------|
| `cargo make cov` | 终端覆盖率报告 |
| `cargo make cov-html` | 生成 HTML 报告 |
| `cargo make cov-open` | 生成并打开报告 |
| `cargo make cov-log` | kaubo-log 覆盖率 |
| `python scripts/coverage.py` | 使用 Python 脚本 |

**注意**: 覆盖率需要 nightly 工具链和 cargo-llvm-cov

### 文档

| 命令 | 说明 |
|------|------|
| `cargo make doc` | 生成文档 |
| `cargo make doc-open` | 生成并打开文档 |

## CLI 使用

### 基础用法

```bash
# 直接运行文件
cargo run -p kaubo-cli -- examples/hello.kaubo

# 或使用已构建的二进制
./target/release/kaubo examples/hello.kaubo
```

### 命令行选项

```bash
kaubo [OPTIONS] <FILE>

Options:
  -v, --verbose      日志级别 (-v=info, -vv=debug, -vvv=trace)
      --compile-only 仅编译，不执行
      --dump-bytecode 显示字节码
      --show-steps    显示执行步骤
      --show-source   显示源码
  -h, --help         显示帮助
  -V, --version      显示版本
```

### 示例

```bash
# 基本运行
cargo run -p kaubo-cli -- examples/hello.kaubo

# 带日志
cargo run -p kaubo-cli -- -v examples/hello.kaubo

# 显示字节码
cargo run -p kaubo-cli -- --dump-bytecode examples/fib.kaubo

# 详细模式
cargo run -p kaubo-cli -- -v --show-steps examples/calc.kaubo
```

## 项目结构

```
kaubo/
├── kaubo-cli/       # CLI 入口
├── kaubo-api/       # API 层 (执行编排)
├── kaubo-core/      # 核心 (编译器 + VM)
├── kaubo-log/       # 日志系统
├── kaubo-config/    # 配置数据
├── examples/        # 示例程序
├── scripts/         # 辅助脚本
└── docs/            # 文档
```

## 技术债

见 `docs/20-current/type-checker-tech-debt.md`

## CI 检查

提交前请运行：

```bash
cargo make ci
```

这会运行：
1. 格式检查
2. 代码检查
3. clippy
4. 全部测试
5. release 构建
