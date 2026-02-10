# Kaubo 编程语言

> 🚧 活跃开发中 - 当前阶段: 2.7 模块静态化

Kaubo 是一门现代、简洁的脚本语言，专为嵌入式场景和快速原型设计。

**核心特性**：
- 静态内存布局（ShapeID 系统）
- 扁平化模块设计
- 显式导入，无隐式作用域
- 原生函数支持（Rust 实现）

**详细文档**: [docs/KAUBO.md](docs/KAUBO.md)

---

## 快速开始

```bash
# 构建项目
cargo build --release

# 运行示例
./target/release/next_kaubo assets/test_fibonacci.txt

# 运行测试
cargo test
```

## Hello World

```kaubo
@ProgramStart
module main {
    import std.core;
    
    pub var run = || {
        std.core.print("Hello, World!");
        return 0;
    };
}
```

---

*详细设计、开发计划和路线图请查看 [docs/KAUBO.md](docs/KAUBO.md)*
