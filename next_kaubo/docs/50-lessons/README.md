# 经验总结

> 项目演进过程中的经验教训和约束发现。

---

## 文档列表

| 文档 | 内容 | 类型 |
|------|------|------|
| [discovered-constraints.md](discovered-constraints.md) | 发现的隐藏约束 | 约束记录 |
| [operator-research.md](operator-research.md) | 运算符重载调研 | 调研报告 |
| [rejected-approaches.md](rejected-approaches.md) | 否决的方案 | 决策记录 |
| [tracing-migration.md](tracing-migration.md) | 日志系统迁移记录 | 迁移记录 |

---

## 快速参考

### 已发现的隐藏约束

1. **热重载不能破坏调用栈** - 只能替换非活跃函数
2. **Shape ID 必须编译期稳定** - 用于状态序列化
3. **JIT 与解释器栈布局兼容** - 需要统一 ABI
4. **文档命令必须用 PowerShell 语法** - 开发环境约束

### 已否决的方案

| 方案 | 状态 | 原因 |
|------|------|------|
| Rc<RefCell> 管理 AST | ❌ 永久否决 | 运行时开销不可接受 |
| 完整 GC | ❌ 当前否决 | 复杂度过高，Arena 足够 |
| LLVM 作为 JIT 后端 | ⏸️ 搁置 | 编译速度不满足 <100ms |

---

## 如何贡献经验

发现新的约束或教训时，在此目录下添加文档，并在本 README 中更新索引。

---

*最后更新：2026-02-17*
