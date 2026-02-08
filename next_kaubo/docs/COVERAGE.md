# 测试覆盖率报告

## 当前覆盖率

**行覆盖率: 82.11%** (2185/2661 行)

**测试数量: 204 个** (全部通过)

> ⚠️ **注意**: cargo-tarpaulin 目前**只支持行覆盖率**，分支覆盖率功能尚未实现 (`--branch` 参数显示 `NOT IMPLEMENTED`)。
> 如需分支覆盖率，可考虑使用 `cargo-llvm-cov` 替代。

## 各文件覆盖率

| 文件 | 覆盖率 | 已覆盖/总行数 | 状态 |
|------|--------|---------------|------|
| `tests/integration_tests.rs` | 99.6% | 238/239 | ✅ |
| `src/compiler/lexer/builder.rs` | 100% | 203/203 | ✅ |
| `src/compiler/parser/parser.rs` | 98.3% | 408/415 | ✅ |
| `src/kit/ring_buffer/ring_buffer.rs` | 95.6% | 195/204 | ✅ |
| `src/kit/lexer/state_machine/manager.rs` | 94.8% | 109/115 | ✅ |
| `src/kit/lexer/state_machine/builder.rs` | 94.9% | 149/157 | ✅ |
| `src/kit/lexer/c_lexer.rs` | 73.9% | 181/245 | ⚠️ |
| `src/runtime/vm.rs` | 65.5% | 184/281 | ⚠️ |
| `src/runtime/compiler.rs` | 64.8% | 105/162 | ⚠️ |
| `src/runtime/bytecode/chunk.rs` | 80.7% | 88/109 | ⚠️ |
| `src/runtime/value.rs` | 74.1% | 86/116 | ⚠️ |
| `src/runtime/bytecode/mod.rs` | 28.9% | 26/90 | ❌ |
| `src/main.rs` | 0% | 0/64 | ❌ |
| `src/compiler/parser/stmt.rs` | 81.4% | 57/70 | ⚠️ |
| `src/compiler/parser/expr.rs` | 75.9% | 41/54 | ⚠️ |
| `src/compiler/lexer/token_kind.rs` | 83.3% | 10/12 | ✅ |
| `src/compiler/parser/error.rs` | 100% | 25/25 | ✅ |
| `src/compiler/parser/utils.rs` | 87.5% | 14/16 | ✅ |
| `src/kit/lexer/types.rs` | 100% | 11/11 | ✅ |
| `src/kit/lexer/state_machine/machine.rs` | 70.6% | 36/51 | ⚠️ |
| `src/lib.rs` | 86.4% | 19/22 | ✅ |

## 新增运行时模块覆盖率

Phase 2 新增的运行时模块:

| 模块 | 覆盖率 | 说明 |
|------|--------|------|
| `runtime/value.rs` | 74.1% | NaN Boxing 值表示，基础功能已覆盖 |
| `runtime/vm.rs` | 65.5% | 虚拟机执行引擎，核心指令已覆盖 |
| `runtime/compiler.rs` | 64.8% | AST → Bytecode 编译器，基础表达式已覆盖 |
| `runtime/bytecode/chunk.rs` | 80.7% | 字节码块，基础操作已覆盖 |
| `runtime/bytecode/mod.rs` | 28.9% | OpCode 定义，大量变体待使用 |

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
2. **`src/runtime/bytecode/mod.rs` (28.9%)** - OpCode 定义，大量变体待实现使用
3. **`src/kit/lexer/c_lexer.rs` (73.9%)** - 包含调试输出和错误处理路径
4. **`src/runtime/vm.rs` (65.5%)** - 未实现指令和错误处理路径
5. **`src/runtime/compiler.rs` (64.8%)** - 未实现语法结构的编译路径

### Phase 2 进展

已覆盖:
- Value 的 SMI、Float、Special 值创建和判断
- VM 的基础算术、比较、跳转指令
- Compiler 的字面量、二元运算、return 语句

待覆盖:
- 变量读写 (局部/全局)
- 控制流 (if/while/for)
- 函数调用
- 列表操作
- 字符串操作

## 配置说明

- **cargo-make 配置**: `Makefile.toml`
- **Python 脚本**: `scripts/coverage.py`
- **tarpaulin 配置**: `Cargo.toml` 中的 `[package.metadata.tarpaulin]`

---

*最后更新: 2026-02-08*
