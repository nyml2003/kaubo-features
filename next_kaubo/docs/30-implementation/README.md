# Kaubo 实现文档

> 面向核心开发者的实现细节文档。

---

## 文档结构

| 目录 | 内容 | 状态 |
|------|------|------|
| [architecture/](architecture/) | 架构文档 | ✅ 稳定 |
| [completed/](completed/) | 已实现功能归档 | 📦 归档 |
| [design/](design/) | 待实现设计 | ⚠️ 实验 |

---

## 架构文档

- [README.md](architecture/README.md) - 实现架构总览
- [roadmap.md](architecture/roadmap.md) - 迭代路线图（MVP v0.1）
- [config.md](architecture/config.md) - 配置系统设计

---

## 已实现功能

- [operators.md](completed/operators.md) - 运算符重载（四级分发 + 内联缓存）
- [builtin-methods.md](completed/builtin-methods.md) - 内置类型方法设计
- [kaubo-log.md](completed/kaubo-log.md) - 日志系统设计

---

## 待实现设计

- [module-system.md](design/module-system.md) - 模块系统改造
- [method-type-inference.md](design/method-type-inference.md) - 方法调用类型推断

> 泛型系统设计已归档至 [90-archive/generic-type-system.md](../90-archive/generic-type-system.md)

---

## 技术债务

参见 [tech-debt.md](tech-debt.md) 了解已知问题和待办事项。

---

*最后更新：2026-02-17*
