# Kaubo 编程语言

> 🚧 活跃开发中 - 当前阶段: 2.9 标准库与显式导入

Kaubo 是一门现代、简洁的脚本语言，专为嵌入式场景和快速原型设计。

**核心特性**:
- ✅ 静态内存布局（ShapeID 系统）
- ✅ 扁平化模块设计
- ✅ 显式导入，无隐式作用域
- ✅ 原生函数支持（Rust 实现）
- ✅ 完善的测试框架

---

## 快速开始

### 安装

```bash
# 克隆仓库
git clone <repo-url>
cd next_kaubo

# 构建
cargo build --release
```

### Hello World

```kaubo
import std;

std.print("Hello, World!");
```

运行:
```bash
./target/release/next_kaubo hello.kaubo
```

### 更多示例

```kaubo
import std;

// 计算圆面积
var circle_area = |r| {
    return std.PI * r * r;
};

std.print(circle_area(5));  // 78.54...

// 使用闭包
var make_counter = || {
    var count = 0;
    return || {
        count = count + 1;
        return count;
    };
};

var counter = make_counter();
std.print(counter());  // 1
std.print(counter());  // 2
```

---

## 文档导航

| 文档 | 内容 |
|------|------|
| [📖 语法参考](docs/01-syntax.md) | 完整语法说明、示例代码 |
| [🏗️ 项目架构](docs/02-architecture.md) | 架构图、日志系统、配置管理 |
| [📦 标准库](docs/03-stdlib.md) | API 参考、使用示例 |
| [🧪 测试文档](docs/04-testing.md) | 分层测试、日志调试 |
| [🔧 开发手册](docs/05-development.md) | CLI 参数、日志使用、调试技巧 |
| [📝 变更日志](docs/CHANGELOG.md) | 版本历史、更新记录 |
| [✅ 任务清单](TODO.md) | 开发计划、已知问题 |

---

## 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --test vm_tests
cargo test --test stdlib_tests

# 显示输出
cargo test -- --nocapture
```

---

## 项目结构

```
next_kaubo/
├── src/
│   ├── compiler/          # 编译器前端 (Lexer/Parser)
│   ├── runtime/           # 运行时 (VM/Compiler/Stdlib)
│   └── kit/               # 通用工具库
├── tests/                 # 集成测试
├── assets/                # 示例代码
├── docs/                  # 文档
│   ├── 01-syntax.md
│   ├── 02-architecture.md
│   ├── 03-stdlib.md
│   ├── 04-testing.md
│   ├── 05-development.md
│   └── CHANGELOG.md
├── README.md
├── TODO.md
└── Cargo.toml
```

---

## 技术亮点

### NaN Boxing

64-bit Value 类型利用浮点数 NaN 空间存储类型标签，实现高效的多态值表示：

```rust
// 无需装箱即可存储
Value::smi(42)      // 小整数
Value::TRUE         // 布尔值
Value::NULL         // 空值
```

### ShapeID 系统

模块字段编译期确定索引，运行时 O(1) 访问：

```kaubo
std.print(123);  // 编译为 LoadGlobal + ModuleGet(0)
```

### 扁平模块设计

```kaubo
// ✅ 支持
import std;
std.print();

// ❌ 不支持（简化设计）
import std.math;
```

---

## 贡献

欢迎贡献！请查看 [开发手册](docs/05-development.md) 了解如何添加新特性。

---

## 许可证

MIT License

---

*最后更新: 2026-02-10*
