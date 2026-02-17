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
| `cargo make test` | 运行所有测试 (486 个) |
| `cargo make test-core` | 运行 kaubo-core 测试 |
| `cargo make test-api` | 运行 kaubo-api 测试 |
| `cargo make test-log` | 运行 kaubo-log 测试 |
| `cargo make test-cli` | 运行 kaubo-cli 测试 |
| `cargo make test-watch` | 持续测试 (需 cargo-watch) |

### 运行示例

| 命令 | 说明 |
|------|------|
| `cargo make run` | 运行默认项目 (examples/hello) |
| `cargo make run-multi` | 运行多模块示例 |
| `cargo make run-diamond` | 运行菱形依赖示例 |
| `cargo make run-chain` | 运行导入链示例 |
| `cargo make run-nested` | 运行嵌套导入示例 |
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
├── kaubo-vfs/       # 虚拟文件系统
├── examples/        # 示例程序
│   ├── hello/
│   ├── fib/
│   ├── calc/
│   ├── multi_module/      # 多模块示例
│   ├── import_chain/      # 传递依赖示例
│   ├── diamond_deps/      # 菱形依赖示例
│   └── nested_import/     # 嵌套导入示例
├── package.json     # 项目配置
├── scripts/         # 辅助脚本
└── docs/            # 文档
```

### 多模块项目结构

```
my_project/
├── package.json          # 项目配置
├── main.kaubo            # 入口模块
├── math.kaubo            # 数学模块
├── utils/
│   ├── string.kaubo      # 字符串工具
│   └── io.kaubo          # I/O 工具
└── std/
    ├── list.kaubo        # 列表操作
    └── json.kaubo        # JSON 处理
```

### 模块导入规则

| 导入语句 | 解析路径 |
|----------|----------|
| `import math;` | `math.kaubo` |
| `import std.list;` | `std/list.kaubo` |
| `import app.utils;` | `app/utils.kaubo` |

## Kaubo 语言示例

### Hello World
```kaubo
print("Hello, Kaubo!");
```

### 多模块项目
```kaubo
// math.kaubo
pub var PI = 3.14159;
pub var add = |a, b| { return a + b; };

// main.kaubo
import math;
print("PI = " + math.PI);
print("2 + 3 = " + math.add(2, 3));
```

### Lambda
```kaubo
var add = |a, b| { return a + b; };
return add(3, 4);
```

### JSON 对象
```kaubo
var person = json {
    name: "Alice",
    age: 30,
    skills: ["Rust", "Kaubo"]
};
print(person.name);
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
4. 全部测试 (486 个)
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

## 技术债

- `docs/30-implementation/tech-debt.md` - 技术债务记录

---

## 开发进度

### 当前阶段：Phase 1 - 二进制模块系统 (进行中)

**已完成阶段：**

#### ✅ Phase 0: 基础设施 (2025-02 至 2025-Q2)
- Lexer、Parser、AST、字节码 VM、类型检查、运算符重载等

#### 🚧 Phase 1: 模块系统与二进制格式 (进行中)

**Phase 1.1: 源文件模块系统 ✅**

| 功能 | 状态 | 说明 |
|------|------|------|
| 虚拟文件系统 (VFS) | ✅ | `kaubo-vfs` crate，Memory/Native FS |
| 模块解析器 | ✅ | 路径解析、缓存、循环检测 |
| 多文件编译器 | ✅ | 拓扑排序、传递依赖、菱形依赖 |
| CLI 集成 | ✅ | 自动检测 `import`，4 个示例 |

**Phase 1.2: 二进制格式 (进行中)**

| 功能 | 状态 | 说明 |
|------|------|------|
| Debug 模式 (`.kaubod`) | 📋 | 完整调试信息、内嵌 Source Map |
| Release 模式 (`.kaubor`) | 📋 | zstd 压缩、可选剥离调试信息 |
| Source Map (`.kmap`) | 📋 | VLQ 编码、支持分离 |
| Chunk 序列化 | 📋 | Encoder/Decoder |

**Phase 1.3: 链接器 (待开始)**

| 功能 | 状态 | 说明 |
|------|------|------|
| 符号表 | 📋 | 跨模块符号解析 |
| KPK 格式 (`.kpk`) | 📋 | 可执行包格式 |
| 静态链接 | 📋 | 多模块合并 |

**Phase 1.4: 运行时加载器 (待开始)**

| 功能 | 状态 | 说明 |
|------|------|------|
| 格式检测 | 📋 | .kaubo/.kaubod/.kaubor/.kpk |
| 版本检查 | 📋 | ABI 兼容性验证 |
| 缓存管理 | 📋 | 编译产物缓存 |

**Phase 1.5: 动态链接预留 (待开始)**

| 功能 | 状态 | 说明 |
|------|------|------|
| ABI 稳定 | 📋 | 32 位版本字段 |
| 重定位表 | 📋 | 相对偏移设计 |
| 动态加载器接口 | 📋 | `DynamicModule` trait |

**测试统计：**
```
kaubo-vfs:     24 tests
kaubo-core:   462 tests (288 单元 + 13 多文件 + 63 集成 + 4 示例 + 90 VM + 4 其他)
总计:         486 tests ✅
```

**设计文档：**
- `docs/30-implementation/design/module-system.md` - 源文件模块系统
- `docs/30-implementation/design/binary-module-system.md` - 二进制格式

---

#### ✅ Phase 0: 基础设施与核心功能 (2025-02 至 2026-02)

**已完成：**
- Lexer、Parser、AST
- 字节码 VM（栈机 + 局部变量）
- 类型检查器（基础）
- 运算符重载（Level 3 元表查找）
- Struct 和 Impl
- 协程（yield）
- 标准库（math、list、string）
- 日志系统（kaubo-log）
- 内联缓存（Level 2）

---

### 下一阶段：Phase 2 - 泛型类型系统

**目标：** 实现完整的编译时泛型系统

**核心功能：**

| 功能 | 示例 | 状态 |
|------|------|------|
| 泛型匿名函数 | `\|[T] x: T\| -> T { return x; }` | 📋 待实现 |
| 泛型 struct | `struct Box[T] { value: T }` | 📋 待实现 |
| 泛型 impl | `impl[T] Box[T] { ... }` | 📋 待实现 |
| 类型推导 | `identity(42)` → `\|int\| -> int` | 📋 待实现 |
| 多类型参数 | `\|[T, U] x: T, y: U\|` | 📋 待实现 |
| 嵌套泛型 | `Box[List[T]]` | 📋 待实现 |

**设计文档：**
- `docs/30-implementation/design/generic-type-system.md`

**语法规范：**
统一使用 `[]` 表示泛型参数：
```kaubo
// 类型定义
struct Box[T] { value: T }
impl[T] Box[T] { ... }

// 表达式
|[T] x: T| -> T { return x; }

// 类型标注
var b: Box[int] = Box[int] { value: 42 };
var list: List[List[string]] = [];
```

---

### 未来阶段

#### Phase 3: JIT 编译器 (规划中)
- Cranelift 集成
- 热点检测与编译
- 解释器 ↔ JIT 切换

#### Phase 4: 热重载 (规划中)
- 代码热更新
- 状态保持
- `@hot` 注解

---

## 所有可用任务

查看所有可用任务：
```bash
cargo make --list-all-steps
```

---

*最后更新：2026-02-17*
