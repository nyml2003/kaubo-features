# 测试覆盖率报告

## 当前覆盖率

**行覆盖率: 85.43%** (1354/1585 行)

> ⚠️ **注意**: cargo-tarpaulin 目前**只支持行覆盖率**，分支覆盖率功能尚未实现 (`--branch` 参数显示 `NOT IMPLEMENTED`)。
> 如需分支覆盖率，可考虑使用 `cargo-llvm-cov` 替代。

## 各文件覆盖率

| 文件 | 覆盖率 | 已覆盖/总行数 | 状态 |
|------|--------|---------------|------|
| `tests/integration_tests.rs` | 100% | 223/223 | ✅ |
| `src/compiler/parser/parser.rs` | 98% | 400/408 | ✅ |
| `src/compiler/parser/utils.rs` | 87.5% | 14/16 | ✅ |
| `src/kit/lexer/state_machine/builder.rs` | 94.3% | 148/157 | ✅ |
| `src/kit/lexer/state_machine/manager.rs` | 94.8% | 109/115 | ✅ |
| `src/kit/ring_buffer/ring_buffer.rs` | 95.6% | 195/204 | ✅ |
| `src/kit/lexer/c_lexer.rs` | 75.5% | 145/192 | ⚠️ |
| `src/kit/lexer/state_machine/machine.rs` | 70.6% | 36/51 | ⚠️ |
| `src/compiler/lexer/builder.rs` | 0% | 0/64 | ❌ |
| `src/compiler/lexer/token_kind.rs` | 15.5% | 11/71 | ❌ |
| `src/compiler/parser/error.rs` | 0% | 0/8 | ❌ |
| `src/compiler/parser/expr.rs` | 0% | 0/26 | ❌ |
| `src/compiler/parser/stmt.rs` | 0% | 0/30 | ❌ |
| `src/kit/lexer/types.rs` | 0% | 0/9 | ❌ |
| `src/main.rs` | 0% | 0/11 | ❌ |

## 使用方式

### 使用 cargo-make (推荐)

```bash
# 生成覆盖率报告 (终端输出)
cargo make cov

# 生成 HTML 报告
cargo make cov-html

# 生成并打开 HTML 报告
cargo make cov-open

# 运行测试
cargo make test

# 运行程序
cargo make run
```

### 使用 Python 脚本

```bash
# 终端输出
python scripts/coverage.py

# 生成 HTML
python scripts/coverage.py --html

# 生成并打开
python scripts/coverage.py --open
```

### 直接使用 cargo-tarpaulin

```bash
# 终端查看
cargo tarpaulin --include-tests --all-targets

# 生成 HTML
cargo tarpaulin --out Html --output-dir target/tarpaulin --include-tests --all-targets
```

## 为什么只有行覆盖率？

当前使用的是 **cargo-tarpaulin**，它基于 ptrace/seccomp 在 Linux 上或使用 LLVM profiling 在 Windows 上工作。截至 0.35.x 版本：

- ✅ **行覆盖率** - 完全支持
- ❌ **分支覆盖率** - 尚未实现 (`--branch` 参数显示 `NOT IMPLEMENTED`)

### 替代方案

如需分支覆盖率，可使用 **cargo-llvm-cov**：

```bash
# 安装
cargo install cargo-llvm-cov

# 运行 (支持行覆盖率和分支覆盖率)
cargo llvm-cov --branch
```

## 未覆盖代码分析

### 低覆盖率文件说明

1. **`src/main.rs` (0%)** - 程序入口，当前只打印 token，未在测试中覆盖
2. **`src/compiler/lexer/token_kind.rs` (15.5%)** - Token 类型定义，大量变体未在测试中使用
3. **`src/compiler/lexer/builder.rs` (0%)** - Lexer 构建器只在测试辅助函数中使用，未被直接测试
4. **`src/compiler/parser/error.rs` (0%)** - 错误类型定义，部分错误变体未被触发
5. **`src/compiler/parser/expr.rs` (0%)** - 表达式 AST 结构定义，只在类型检查中使用
6. **`src/compiler/parser/stmt.rs` (0%)** - 语句 AST 结构定义，只在类型检查中使用

### 建议

这些 0% 覆盖率的文件主要是：
- **结构定义文件** (`expr.rs`, `stmt.rs`, `error.rs`, `token_kind.rs`) - 包含大量 AST 类型定义，被代码使用但未被直接测试
- **builder.rs** - 被测试代码使用但测试的是其输出，而非 builder 本身

要提高覆盖率，可以：
1. 添加针对 `builder.rs` 的单元测试
2. 在测试中触发更多错误路径
3. 测试 main.rs 的功能（可以提取逻辑到 lib.rs）

## 配置说明

- **cargo-make 配置**: `Makefile.toml`
- **Python 脚本**: `scripts/coverage.py`
- **tarpaulin 配置**: `Cargo.toml` 中的 `[package.metadata.tarpaulin]`
