# Kaubo 文档中心

> **TL;DR** - 第一次看？先读 [核心原则](00-principles/README.md)，再看 [语法规范](20-language/spec/syntax.md)。

---

## 按角色阅读

### 我是语言用户（用 Kaubo 写代码）

阅读顺序：
1. [核心原则](00-principles/README.md) - 了解设计哲学
2. [语法规范](20-language/spec/syntax.md) - 语言语法参考
3. [类型系统](20-language/spec/types.md) - 类型规则
4. [标准库](20-language/std/README.md) - 内置函数
5. [内置方法指南](20-language/guide/builtin-methods.md) - List/String/Json 方法

### 我是核心开发者（改 Kaubo 本身）

阅读顺序：
1. [核心原则](00-principles/README.md) - 不可妥协的原则
2. [约束条件](10-constraints/) - 复杂度/性能红线
3. [架构全景图](architecture.md) - 当前架构总览
4. [现状总结](current-state.md) - 各模块实现完成度
5. [路线图](roadmap.md) - v0.2.0 路线图
6. [类型系统审计](type-system.md) - 已知 Gap 与修复方案

---

## 文档结构

```
docs/
├── README.md                    ← 你在这里（本文档）
│
├── architecture.md              # 架构全景图 (水平×竖直, 数据流, 扩展点)
├── current-state.md             # 实现现状总结 (各模块完成度矩阵)
├── roadmap.md                   # v0.2.0 路线图
├── type-system.md               # 类型系统审计与修复方案
├── artifacts.md                 # 交付产物清单 (库/二进制/wasm/FFI)
├── package-json.md              # 项目配置
│
├── 00-principles/               # 设计哲学（稳定）
│   └── README.md                # 10条核心原则
│
├── 10-constraints/              # 硬性约束（不可违反）
│   ├── compatibility.md         # 兼容性契约
│   ├── complexity-budget.md     # 复杂度上限
│   └── performance-limits.md    # 性能红线
│
├── 20-language/                 # 语言文档
│   ├── guide/                   # 用户指南
│   │   ├── builtin-methods.md   # 内置类型方法
│   │   └── getting-started.md   # 快速开始（待写）
│   │
│   ├── spec/                    # 语言规范
│   │   ├── README.md
│   │   ├── syntax.md            # 语法参考
│   │   └── types.md             # 类型系统
│   │
│   └── std/                     # 标准库
│       └── README.md
│
├── 30-implementation/           # 实现细节
│   ├── architecture/            # 架构文档
│   │   └── config.md            # 配置系统
│   │
│   ├── completed/               # 已实现功能归档
│   │   ├── operators.md         # 运算符重载
│   │   ├── builtin-methods.md   # 内置方法设计
│   │   └── kaubo-log.md         # 日志系统
│   │
│   └── design/                  # 设计文档
│       ├── README.md            # 设计文档索引
│       ├── module-system.md          # 模块系统
│       ├── multi-module-system-final.md  # 多模块编译
│       ├── method-type-inference.md  # 类型推断增强
│       ├── binary-module-system.md    # 二进制模块系统
│       ├── vfs-mapped-layer-v4.md     # VFS 映射层
│       ├── vfs-middleware-system.md   # VFS 中间件系统
│       └── vfs-middleware-roadmap.md  # VFS 中间件路线图
│
├── 50-lessons/                  # 经验总结
│   ├── README.md                # 经验总结索引
│   ├── discovered-constraints.md    # 发现的约束
│   ├── rejected-approaches.md       # 否决的方案
│   └── tracing-migration.md         # 日志迁移记录
│
└── 90-archive/                  # 归档
    ├── README.md                # 归档说明
    ├── major-pivots/            # 重大决策快照
    └── operator-research/       # 运算符重载调研
```

---

## 快速参考

| 我想了解... | 去哪找 |
|------------|--------|
| 目标架构长什么样 | [架构全景图](architecture.md) |
| 各模块实现完成度 | [现状总结](current-state.md) |
| v0.2 路线图 | [roadmap.md](roadmap.md) |
| 交付产物有哪些 | [artifacts.md](artifacts.md) |
| 类型系统有什么问题 | [type-system.md](type-system.md) |
| 配置文件格式 | [package-json.md](package-json.md) |

---

## 状态说明

- ✅ **稳定**：原则、约束（改变意味着项目本质改变）
- 📝 **活跃**：spec/ 规范（随语法演进更新）
- ⚠️ **实验**：design/ 设计（待实现）
- 📦 **归档**：completed/ 已实现（历史记录）、90-archive/ 废弃文档

---

*最后更新：2026-06-14*
