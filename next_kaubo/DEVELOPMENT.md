# Kaubo 开发指南

## 快速开始

```bash
# 检查环境
cargo make check-env

# 构建项目
cargo make build

# 运行测试
cargo make test

# 运行示例项目
cd examples/hello
kaubo

# 或指定配置文件路径
kaubo examples/hello/package.json
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
| `cargo make test-cli` | 运行 kaubo-cli 测试 |
| `cargo make test-watch` | 持续测试 (需 cargo-watch) |

### 运行示例

| 命令 | 说明 |
|------|------|
| `cargo make run` | 运行默认项目 (examples/hello) |
| `cargo make run PROJECT=examples/fib` | 运行指定项目 |
| `cargo make run PROJECT=examples/calc` | 运行计算器示例 |
| `cargo make run-release` | Release 模式运行 |

每个项目的行为（日志级别、显示源码等）通过 `package.json` 中的 `compiler` 字段配置。

### 代码质量

| 命令 | 说明 |
|------|------|
| `cargo make check` | 检查代码 |
| `cargo make clippy` | 运行 clippy (允许警告) |
| `cargo make lint` | 运行 clippy (严格模式) |
| `cargo make fmt` | 格式化代码 |
| `cargo make fmt-check` | 检查代码格式 |
| `cargo make quality` | 全套代码质量检查 |

### 覆盖率

| 命令 | 说明 |
|------|------|
| `cargo make cov` | 终端覆盖率报告 |
| `cargo make cov-html` | 生成 HTML 报告 |
| `cargo make cov-open` | 生成并打开报告 |
| `cargo make cov-log` | kaubo-log 模块覆盖率 |
| `cargo make cov-py` | 使用 Python 脚本 |

**注意**: 覆盖率需要 nightly 工具链:
```bash
rustup install nightly
cargo install cargo-llvm-cov
```

### 文档

| 命令 | 说明 |
|------|------|
| `cargo make doc` | 生成文档 |
| `cargo make doc-open` | 生成并打开文档 |

### 清理

| 命令 | 说明 |
|------|------|
| `cargo make clean` | 清理构建文件 |
| `cargo make clean-all` | 深度清理 |

## CLI 使用

Kaubo 采用**项目制**管理，所有配置通过 `package.json` 指定。

### 项目结构

```
my_project/
├── package.json      # 项目配置（必须）
└── src/
    └── main.kaubo    # 入口文件
```

### package.json

```json
{
  "name": "my-app",
  "version": "0.1.0",
  "entry": "src/main.kaubo",
  "compiler": {
    "compile_only": false,
    "dump_bytecode": false,
    "show_steps": false,
    "show_source": false,
    "log_level": "warn"
  }
}
```

### 命令行用法

```bash
# 在项目目录下执行（自动读取 package.json）
cd my_project
kaubo

# 指定配置文件路径
kaubo path/to/package.json

# 运行示例项目
kaubo examples/hello/package.json
kaubo examples/fib/package.json
kaubo examples/calc/package.json
```

### 项目配置示例

每个项目通过 `package.json` 独立配置：

```json
{
  "name": "hello",
  "version": "0.1.0",
  "entry": "main.kaubo",
  "compiler": {
    "show_source": true,
    "show_steps": false,
    "log_level": "info"
  }
}
```

### 配置选项

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | string | 项目名称 |
| `version` | string | 版本号 |
| `entry` | string | 入口文件路径（相对 package.json） |
| `compiler.compile_only` | bool | 仅编译，不执行 |
| `compiler.dump_bytecode` | bool | 显示字节码 |
| `compiler.show_steps` | bool | 显示执行步骤 |
| `compiler.show_source` | bool | 显示源码 |
| `compiler.log_level` | string | 日志级别: silent/error/warn/info/debug/trace |

## 项目结构

### 源码结构

```
kaubo/
├── kaubo-cli/       # CLI 入口
├── kaubo-api/       # API 层 (执行编排)
├── kaubo-core/      # 核心 (编译器 + VM)
├── kaubo-log/       # 日志系统
├── kaubo-config/    # 配置数据
├── examples/        # 示例程序
│   ├── hello.kaubo
│   ├── fib.kaubo
│   └── calc.kaubo
├── package.json     # 项目配置（运行必需）
├── scripts/         # 辅助脚本
└── docs/            # 文档
```

### Kaubo 项目结构

```
my_project/
├── package.json          # 项目配置（必须）
├── main.kaubo            # 入口文件（或其他名字）
└── lib/
    └── utils.kaubo       # 模块文件
```

### 示例项目结构

```
examples/
├── hello/
│   ├── package.json
│   └── main.kaubo
├── fib/
│   ├── package.json
│   └── main.kaubo
└── calc/
    ├── package.json
    └── main.kaubo
```

## Kaubo 语言示例

### Hello World
```kaubo
from std import print;
print("Hello, Kaubo!");
```

### 斐波那契
```kaubo
from std import print, to_string;

var n = 10;
var a = 0;
var b = 1;
var i = 0;

while i < n {
    var temp = a + b;
    a = b;
    b = temp;
    i = i + 1;
}

print("Fib(" + to_string(n) + ") = " + to_string(a));
```

### Lambda
```kaubo
var add = |a, b| { return a + b; };
return add(3, 4);
```

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

## 代码质量标准

### 警告零容忍

项目采用**零容忍警告**策略：

```bash
# 检查是否有警告
cargo check --workspace

# 应该显示：Finished dev profile [unoptimized + debuginfo] target(s)
# 如果有 warning，必须处理
```

**处理方式**:

| 情况 | 处理方式 | 示例 |
|------|----------|------|
| 真正的清理遗漏 | 直接删除/修复 | 未使用的 import |
| 未完成的功能 | `#[allow(...)]` + TODO + 文档 | 内联缓存、一元运算符 |
| 开发中代码 | `#[allow(...)]` + TODO + 文档 | 类型检查器变量 |

**文档要求**:
- 代码中的每个 TODO 必须对应文档中的条目
- 类型检查器 TODO → `docs/20-current/type-checker-tech-debt.md`
- VM 基础设施 TODO → `docs/20-current/vm-tech-debt.md`

## 技术债

- `docs/20-current/type-checker-tech-debt.md` - 类型系统未完成项
- `docs/20-current/vm-tech-debt.md` - VM 基础设施未完成项

## 最近完成

### Phase 3: 优化完善 (进行中)

#### ✅ 已完成功能

**1. 移除 struct 字符串/整数键访问**
- struct 字段访问只能通过 `.field_name` 语法
- `struct["field"]` 和 `struct[0]` 被编译期禁止（类型检查错误）或运行时拒绝
- 文件变更:
  - `kaubo-core/src/runtime/vm/index.rs` - 移除 struct 键访问逻辑
  - `kaubo-core/src/runtime/vm/operators.rs` - 更新运算符调用
  - `kaubo-core/src/compiler/parser/type_checker.rs` - 添加编译期检查

**2. 类型推断增强**
- 扩展 `VarType` 枚举支持基础类型: `Int`, `Float`, `String`, `Bool`
- 字面量类型推断: 编译期识别 int/float/string/bool 字面量类型
- 运算符返回类型推断:
  - Unary (`-`): 支持 `neg` 运算符返回类型推断
  - Binary (`+`, `-`, `*`, `/`, `%`): 支持 `add`/`sub`/`mul`/`div`/`mod` 及反向运算符 (`radd`, `rmul` 等)
- 变量类型跟踪: 变量声明时记录类型，支持跨表达式类型传播

**3. MemberAccess 编译期优化**
- 识别 struct 类型的 MemberAccess 表达式
- 编译期生成 `GetField` 指令（直接字段索引访问）
- 非 struct 类型回退到 `IndexGet`（JSON 动态访问）

#### 📊 测试状态

```bash
# 全部测试通过 (449 个)
cargo test -p kaubo-core
```

关键测试:
- `test_operator_neg` - 一元运算符重载
- `test_operator_overloading_add` - 二元运算符重载
- `test_operator_add_struct_field_order` - 字段访问顺序
- `test_operator_rmul` - 反向运算符
- `test_inline_cache_integration` - 内联缓存集成
- `test_inline_cache_multiple_calls` - 多调用缓存

---

## 性能测试方案

### 1. 基准测试目标

| 指标 | 说明 | 优先级 |
|------|------|--------|
| 编译速度 | 源码 → Chunk 的时间 | P1 |
| 执行速度 | VM 指令执行效率 | P1 |
| 内存占用 | 运行时内存使用 | P2 |
| 内联缓存命中率 | shape-based 缓存效果 | P1 |
| 字段访问延迟 | `.field` vs `["field"]` 对比 | P2 |

### 2. 测试方案设计

#### 2.1 编译性能测试

```rust
// benches/compile_benchmark.rs
// 测试场景:
// 1. 大型 struct 定义 (100+ 字段)
// 2. 嵌套表达式 (深度 20+)
// 3. 复杂运算符重载
// 4. 大量 lambda 定义
```

#### 2.2 执行性能测试

```rust
// benches/execution_benchmark.rs
// 测试场景:
// 1. 斐波那契递归 (n=30)
// 2. 向量运算 (10000 次加法)
// 3. Struct 字段访问 (1000000 次循环)
// 4. 内联缓存对比 (冷启动 vs 热路径)
```

#### 2.3 微基准测试

| 测试项 | 代码示例 | 预期指标 |
|--------|----------|----------|
| GetField | `p.x` (100万次) | < 10ms |
| IndexGet | `obj["key"]` (100万次) | < 50ms |
| 运算符调用 | `v1 + v2` (100万次) | < 20ms |
| 内联缓存命中 | 重复调用 shape 相同的方法 | 接近原生调用 |
| 内联缓存未命中 | 频繁改变 shape 的调用 | 比普通调用慢 2-3x |

### 3. 性能对比基准

#### 与 Python 对比
```bash
# 斐波那契测试
python3 -m timeit -n 5 -r 2 "exec(open('fib.py').read())"
cargo run --release -- examples/fib.kaubo
```

#### 与 Lua 对比
```bash
# 向量运算测试
lua vec_test.lua
kaubo vec_test.kaubo
```

### 4. 实现计划

```bash
# Phase 1: 基础设施
cargo add --dev criterion
mkdir -p kaubo-core/benches

# Phase 2: 编写基准测试
# - compile_benchmark.rs
# - execution_benchmark.rs  
# - cache_hit_benchmark.rs

# Phase 3: 性能分析
cargo bench -- --profile-time 10
```

### 5. 预期优化方向

| 优化项 | 当前状态 | 目标 |
|--------|----------|------|
| GetField | 已实现直接索引 | 基准测试验证 |
| 内联缓存 | Level 2 集成 | 95%+ 命中率 |
| 类型推断 | 基础支持 | 减少 IndexGet 回退 |
| 常量折叠 | 未实现 | 编译期计算常量表达式 |
| 死代码消除 | 未实现 | 移除未使用变量/函数 |

---

## 性能测试计划 (端到端)

### 目标
与 Python 3 进行端到端对比，识别 Kaubo 的整体性能瓶颈，**不关注具体函数实现细节**。

### 测试场景 (5个)

| 场景 | 描述 | 预期瓶颈 |
|------|------|----------|
| **A. 计算密集型** | 斐波那契递归 (n=35) | 函数调用开销、递归深度 |
| **B. 内存访问型** | 大列表求和 (100万元素) | List 索引访问、边界检查 |
| **C. 对象操作型** | Struct 创建 + 字段访问 (100万次) | 内存分配、shape 系统 |
| **D. 字符串型** | 字符串拼接 (生成 10MB 文本) | 字符串拷贝、GC 压力 |
| **E. 混合型** | 简单表达式求值器 | 综合：函数调用+内存+分支 |

### 测试方法

```bash
benchmarks/
├── cases/              # 测试用例源码
│   ├── fibonacci.py
│   ├── fibonacci.kaubo
│   └── ...
├── run_benchmark.py    # 测试驱动脚本
└── report.md           # 结果报告
```

**计时方式**:
- Python: `time.perf_counter()` （含解释器启动）
- Kaubo: 同一起点测量（含编译+执行）
- 每场景 **10 轮**，取**中位数**

**收集指标**:
| 指标 | Python | Kaubo |
|------|--------|-------|
| 总耗时 | ✓ | ✓ |
| 编译耗时 | N/A | ✓ |
| 峰值内存 | ✓ | ✓ |

### 瓶颈分析框架

#### Slowdown 分级
| slowdown | 评估 | 行动 |
|----------|------|------|
| 1-3x | 良好 | 进入功能完善 |
| 3-10x | 可接受 | 针对性优化 |
| 10-50x | 需关注 | 优先修复对应场景 |
| 50x+ | 严重 | 架构级重构 |

#### 瓶颈定位
```
编译耗时 > 20%  →  编译器优化
计算型 slowdown 大  →  VM 指令/函数调用
内存型 slowdown 大  →  内存布局/GC
字符串型 slowdown 大 → 拷贝/不可变设计
```

### 实施步骤
1. **准备测试用例**: 5个场景的 Python + Kaubo 版本
2. **搭建框架**: run_benchmark.py 脚本
3. **执行测试**: 10轮运行，收集数据
4. **生成报告**: Markdown 表格 + 瓶颈分析
5. **优化验证**: 针对性优化后重新测试

### 预期产出
```markdown
## 性能基准报告 (vs Python 3.11)

| 场景 | Python | Kaubo | Slowdown | 瓶颈 |
|------|--------|-------|----------|------|
| 斐波那契 | 120ms | 850ms | 7.1x | 函数调用 |
| 列表求和 | 45ms | 420ms | 9.3x | 索引访问 |
| ... | ... | ... | ... | ... |

**结论**: 平均 slowdown 8x，建议优先优化函数调用...
```

---

## 所有可用任务

查看所有可用任务：
```bash
cargo make --list-all-steps
```

## 所有可用任务

查看所有可用任务：
```bash
cargo make --list-all-steps
```
