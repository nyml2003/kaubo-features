# Kaubo Programming Language

Kaubo 是一门静态类型的脚本语言，专注于提供清晰的语法和可控的性能。

```kaubo
// Hello World
print("Hello, Kaubo!");

// Lambda 函数
var add = |a, b| { return a + b; };
print(add(2, 3));

// 多模块导入
import math;
print("PI = " + math.PI);
```

## 特性

- **单文件即模块**：每个 `.kaubo` 文件是一个独立模块，使用 `pub var` 导出，`import` 导入
- **静态类型**：编译期类型检查，可选类型标注
- **运算符重载**：通过 `operator add` 等语法实现自定义类型的运算符
- **Lambda 函数**：匿名函数支持闭包
- **Shape-based 对象系统**：高效的字段访问和内联缓存
- **虚拟文件系统**：支持多平台（Windows/Mac/Linux/Web）

## 快速开始

```bash
# 克隆仓库
git clone <repo-url>
cd kaubo

# 运行测试
cargo test --workspace

# 运行示例（源码模式）
cargo run -p kaubo-cli -- examples/hello/package.json

# 编译为二进制（开发模式，待实现）
# kaubo build main.kaubo --debug -o main.kaubod

# 编译为二进制（生产模式，待实现）
# kaubo build main.kaubo --release -o main.kaubor

# 链接为可执行包（待实现）
# kaubo link *.kaubor -o app.kpk
```

## 项目结构

```
kaubo/
├── kaubo-cli/           # CLI 入口 (基于 Orchestrator)
├── kaubo-orchestrator/  # 编排引擎 (组件管理 + 流水线执行)
├── kaubo-core/          # 核心 (编译器 + VM)
├── kaubo-log/           # 日志系统
├── kaubo-config/        # 配置数据
├── kaubo-vfs/           # 虚拟文件系统
└── examples/            # 示例程序
```

### 新架构：组件化编排器

Kaubo 正在迁移到组件化架构 (`kaubo-orchestrator`)：

| 组件类型 | 职责 | 示例 |
|----------|------|------|
| **Loader** | 加载源代码 | `FileLoader` |
| **Converter** | IR 格式转换 | `Source→Tokens` |
| **Pass** | 编译阶段 | `Lexer`, `Parser`, `CodeGen` |
| **Emitter** | 输出结果 | `FileEmitter`, `StdoutEmitter` |

流水线通过 `package.json` 中的 `pipeline` 字段配置。

```rust
// 使用示例
use kaubo_orchestrator::{Orchestrator, FileLoader, VmConfig};

let mut orch = Orchestrator::new(VmConfig::default());
orch.register_loader(Box::new(FileLoader::new()));
```

## 开发状态

| 阶段 | 名称 | 状态 |
|------|------|------|
| Phase 0 | 基础设施 | ✅ 完成 |
| Phase 1 | 模块系统与二进制格式 | 🚧 进行中 |
| Phase 2 | 组件化架构迁移 | 🚧 进行中 |
| Phase 3 | 泛型类型系统 | 📋 规划中 |
| Phase 4 | JIT 编译器 | 📋 规划中 |
| Phase 5 | 热重载 | 📋 规划中 |

### Phase 1 详情

| 子阶段 | 内容 | 状态 |
|--------|------|------|
| 1.1 | 源文件模块系统 (VFS + 多文件编译) | ✅ 完成 |
| 1.2 | 二进制格式 (.kaubod/.kaubor + Source Map) | 🚧 进行中 |
| 1.3 | 链接器 (KPK 打包) | 📋 待开始 |
| 1.4 | 运行时加载器 | 📋 待开始 |
| 1.5 | 动态链接预留 | 📋 待开始 |

### Phase 2 详情 (组件化架构) ✅ 完成

| 子阶段 | 内容 | 状态 |
|--------|------|------|
| 2.1 | Orchestrator 基础架构 | ✅ 完成 |
| 2.2 | 组件 trait 系统 | ✅ 完成 |
| 2.3 | Loader/Emitter 实现 | ✅ 完成 |
| 2.4 | Core→Passes 迁移 | ✅ 完成 |
| 2.5 | CLI 迁移 | ✅ 完成 |
| 2.6 | 删除旧 API | ✅ 完成 |

**架构特点：**
- 组件化：Loader、Converter、Pass、Emitter 四大组件类型
- 流水线：通过 JSON 配置定义编译流程
- 可扩展：动态注册组件，支持插件

## 文档

- [package.json 配置](docs/package-json.md) - 项目配置完整指南
- [开发指南](DEVELOPMENT.md) - 构建、测试、命令参考
- [模块系统设计](docs/30-implementation/design/module-system.md)
- [泛型类型系统设计](docs/30-implementation/design/generic-type-system.md)
- [迭代路线图](docs/30-implementation/architecture/roadmap.md)

## 示例

### 基础示例
```kaubo
// 变量与运算
var x = 10;
var y = 20;
return x + y;
```

### 多模块项目
```kaubo
// math.kaubo
pub var PI = 3.14159;
pub var add = |a, b| { return a + b; };

// main.kaubo
import math;
print(math.add(2, 3));
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

## License

MIT License
