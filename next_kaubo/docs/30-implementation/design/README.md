# 设计文档

> 已确认或待实现的功能设计文档。

---

## 设计文档列表

| 文档 | 内容 | 优先级 |
|------|------|--------|
| [module-system.md](module-system.md) | 单文件单模块 + 插件化 Std | P1 |
| [multi-module-system-final.md](multi-module-system-final.md) | 多模块编译最终设计 | P1 |
| [method-type-inference.md](method-type-inference.md) | 链式调用类型推断 | P2 |
| [binary-module-system.md](binary-module-system.md) | 二进制模块系统 | P2 |
| [vfs-mapped-layer-v4.md](vfs-mapped-layer-v4.md) | VFS 映射层 (逻辑→物理路径) | P2 |
| [vfs-middleware-system.md](vfs-middleware-system.md) | VFS 中间件系统设计 | P2 |
| [vfs-middleware-roadmap.md](vfs-middleware-roadmap.md) | VFS 中间件路线图 | P3 |

> 泛型系统设计已归档至 [90-archive/generic-type-system.md](../../90-archive/generic-type-system.md)

---

## 设计原则

1. **文档先行**：设计必须在代码之前成型
2. **架构预留**：当前不做的功能不能堵死未来
3. **渐进实现**：复杂功能分阶段实现，先MVP后完善

---

*最后更新：2026-06-14*
