# Kaubo 编译器

> **详细设计文档**: [KAUBO_DESIGN.md](./KAUBO_DESIGN.md)

## 项目简介

Kaubo 是一个现代编程语言的完整编译器实现，包含前端（词法/语法/语义分析）和后端（字节码编译/虚拟机执行）。

| 属性 | 值 |
|------|-----|
| 版本 | 0.1.0 |
| 语言 | Rust (Edition 2024) |
| 架构 | 状态机 Lexer + Pratt Parser + 字节码 VM |

## 快速开始

```bash
# 构建
cargo build --release

# 运行测试（211 个测试）
cargo test

# 运行示例
cargo run -- assets/a.txt
```

## 当前状态

**Phase 2.2: 变量系统与控制流** 🚧

- ✅ W1-W2: 局部变量、变量读写赋值
- 🚧 W3: 控制流（if/while/for）

## 项目结构

```
src/
├── compiler/    # 前端：词法/语法分析器
├── kit/         # 通用工具
└── runtime/     # 后端：字节码 VM
```

## 测试

```bash
cargo test          # 运行 211 个测试
cargo make cov      # 查看覆盖率（82.11%）
```

---

*最后更新: 2026-02-08*
