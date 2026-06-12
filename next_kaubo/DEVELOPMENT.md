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
| `cargo make test-orchestrator` | 运行 kaubo-orchestrator 测试 |
| `cargo make test-orchestrator` | 运行 kaubo-orchestrator 测试 |
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

### CLI 命令

| 命令 | 说明 |
|------|------|
| `cargo run -p kaubo-cli -- <package.json>` | 编译并执行 |
| `cargo run -p kaubo-cli -- <package.json> --verbose` | 显示详细步骤 |
| `cargo run -p kaubo-cli -- <package.json> --emit-binary` | 生成 .kaubod |
| `cargo run -p kaubo-cli -- <package.json> --mode binary` | 执行二进制 |
| `cargo run -p kaubo-cli -- <package.json> --dump-bytecode` | 转储字节码 |
| `cargo run -p kaubo-cli -- <package.json> --compile-only` | 仅编译 |

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

项目配置示例：

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
    "log_level": "warn",
    "mode": "auto",
    "emit_binary": false
  }
}
```

📖 **完整配置文档**: [docs/package-json.md](docs/package-json.md)

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
| `compiler.mode` | string | 执行模式: auto/source/binary (见下方说明) |
| `compiler.emit_binary` | bool | 编译时生成 .kaubod 二进制文件 |

#### 执行模式 (`mode`)

- **`auto`** (默认): 自动选择执行方式
  - 如果存在 `.kaubod` 二进制文件且比源码新，则直接执行二进制
  - 否则编译并解释执行源码
  
- **`source`**: 总是解释执行源码（忽略二进制缓存）

- **`binary`**: 执行二进制文件
  - 如果 `.kaubod` 文件不存在，报错并退出
  - 适用于生产环境部署

#### 二进制文件生成 (`emit_binary`)

当设置为 `true` 时，编译成功后会生成 `.kaubod` 文件（与源码同名，扩展名改为 `.kaubod`）。

**支持的特性**: 当前二进制格式支持整数、浮点数、字符串、列表、结构体、函数和闭包。大部分程序都可以生成二进制文件。

```bash
# 示例: 启用二进制缓存
{
  "compiler": {
    "mode": "auto",
    "emit_binary": true
  }
}
# 第一次: 编译源码 → 生成 main.kaubod → 执行
# 第二次: 检测到 main.kaubod 存在且最新 → 直接执行二进制（更快）
```

## 项目结构

### 源码结构

```
kaubo/
├── kaubo-cli/           # CLI 入口
├── kaubo-orchestrator/  # 编排引擎 (组件管理 + 流水线执行)
│   ├── component/       # 组件 trait 定义
│   ├── loader/          # 加载器组件
│   ├── converter/       # 转换器组件
│   ├── pass/            # 处理阶段组件
│   ├── emitter/         # 输出器组件
│   ├── registry/        # 组件注册表
│   ├── pipeline/        # 流水线引擎
│   └── context/         # 执行上下文
├── kaubo-core/          # 核心编译器 (将被拆分为独立 pass crates)
├── kaubo-log/           # 日志系统
├── kaubo-config/        # 配置数据
├── kaubo-vfs/           # 虚拟文件系统
├── kaubo-api/           # 旧 API 层 (将被 orchestrator 取代)
├── examples/            # 示例程序
│   ├── hello/
│   ├── fib/
│   ├── calc/
│   ├── multi_module/      # 多模块示例
│   ├── import_chain/      # 传递依赖示例
│   ├── diamond_deps/      # 菱形依赖示例
│   └── nested_import/     # 嵌套导入示例
├── package.json         # 项目配置
├── scripts/             # 辅助脚本
└── docs/                # 文档
```

#### 组件架构 (New)

Kaubo 正在迁移到组件化架构，通过 `kaubo-orchestrator` 管理：

| 组件类型 | 职责 | 示例 |
|----------|------|------|
| **Loader** | 从各种来源加载源代码 | FileLoader, HttpLoader |
| **Converter** | 在不同 IR 格式间转换 | Source→Tokens, AST→Bytecode |
| **Pass** | 编译处理阶段 | Lexer, Parser, TypeChecker, CodeGen |
| **Emitter** | 输出结果到目标 | FileEmitter, StdoutEmitter |

流水线通过 `package.json` 中的 `pipeline` 字段配置：

```json
{
  "pipeline": {
    "stages": [
      { "name": "lex", "pass": "lexer" },
      { "name": "parse", "pass": "parser" },
      { "name": "typecheck", "pass": "type_checker" },
      { "name": "codegen", "pass": "codegen" }
    ]
  }
}
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

### MVP v0.1 (进行中)

**目标：** 交付可用的 Kaubo v0.1 — 稳定、可靠、完整的脚本语言运行时。

**当前状态：425 tests，0 failures**

**MVP 交付清单：**

| 任务 | 状态 |
|------|------|
| 修复所有 `panic!()` / `unimplemented!()` | ✅ |
| 删除废弃 token (`interface`/`async`/`await`) | ✅ |
| 标准库测试覆盖 (49 tests) | ✅ |
| 类型检查器 strict 模式 | ✅ |
| Spec 文档收敛 | ✅ |
| 实现 `break` / `continue` / `pass` | ✅ |
| 修复闭包捕获 bug (`enclosing` 指针) | ✅ |
| 修复 unsafe raw pointer autoref (4 处 UB) | ✅ |
| 新增 std: `substring`/`contains`/`starts_with`/`ends_with`/`env`/`now` | ✅ |
| 修复 config 访问 stub | ✅ |
| `kaubo build` 命令 + Release 模式 | ✅ |
| 打 tag v0.1.0 | ✅ |

**已删除（非 MVP）：**
- `interface` / `async` / `await` token
- `val` / `runtime` / `cfg` 编译期元编程
- 泛型 / JIT / 热重载
- 链接器 / 包管理器

---

## 所有可用任务

查看所有可用任务：
```bash
cargo make --list-all-steps
```

---

*最后更新：2026-06-11*
