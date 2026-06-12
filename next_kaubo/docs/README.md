# Kaubo 文档中心

> **TL;DR** - 第一次看？先读 [核心原则](00-principles/README.md)，再看 [语法规范](20-language/spec/syntax.md)。

---

## 按角色阅读

### 👤 我是语言用户（用 Kaubo 写代码）

阅读顺序：
1. [核心原则](00-principles/README.md) - 了解设计哲学
2. [语法规范](20-language/spec/syntax.md) - 语言语法参考
3. [类型系统](20-language/spec/types.md) - 类型规则
4. [标准库](20-language/std/README.md) - 内置函数
5. [内置方法指南](20-language/guide/builtin-methods.md) - List/String/Json 方法

### 🔧 我是核心开发者（改 Kaubo 本身）

阅读顺序：
1. [核心原则](00-principles/README.md) - 不可妥协的原则
2. [约束条件](10-constraints/) - 复杂度/性能红线
3. [实现总览](30-implementation/architecture/README.md) - 架构方向
4. [迭代路线图](30-implementation/architecture/roadmap.md) - 当前阶段
5. [演进计划](40-evolution/README.md) - MVP 收敛与后续方向
6. [技术债务](30-implementation/tech-debt.md) - 待办事项

---

## 文档结构

```
docs/
├── README.md                    ← 你在这里（本文档）
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
│   │   ├── README.md            # 架构总览
│   │   ├── roadmap.md           # 迭代路线图
│   │   └── config.md            # 配置系统
│   │
│   ├── completed/               # 已实现功能归档
│   │   ├── operators.md         # 运算符重载
│   │   ├── builtin-methods.md   # 内置方法设计
│   │   └── kaubo-log.md         # 日志系统
│   │
│   ├── design/                  # 设计文档
│   │   ├── README.md            # 设计文档索引
│   │   ├── module-system.md          # 模块系统改造
│   │   └── method-type-inference.md  # 类型推断增强
│   │
│   └── tech-debt.md             # 技术债务清单
│
├── 40-evolution/                # 演进计划
│   └── README.md                # MVP 收敛与后续方向
│
├── 50-lessons/                  # 经验总结
│   ├── README.md                # 经验总结索引
│   ├── discovered-constraints.md    # 发现的约束
│   ├── operator-research.md         # 运算符重载调研
│   ├── rejected-approaches.md       # 否决的方案
│   └── tracing-migration.md         # 日志迁移记录
│
└── 90-archive/                  # 归档
    └── README.md                # 归档说明
```

---

## 快速参考

| 我想了解... | 去哪找 |
|------------|--------|
| 当前实现架构 | [架构总览](30-implementation/architecture/README.md) |
| 运算符重载怎么实现 | [运算符重载](30-implementation/completed/operators.md) |
| `as` 支持哪些类型转换 | [类型系统](20-language/spec/types.md) → as 类型转换 |
| 配置文件格式 | [配置系统](package-json.md) |
| 性能目标 | [性能约束](10-constraints/performance-limits.md) |
| 为什么不能重载 `and`/`or` | [运算符重载](30-implementation/completed/operators.md) → 不支持重载 |
| MVP 路线图 | [DEVELOPMENT.md](../DEVELOPMENT.md) |

---

## 状态说明

- ✅ **稳定**：原则、约束（改变意味着项目本质改变）
- 📝 **活跃**：spec/ 规范（随语法演进更新）
- ⚠️ **实验**：design/ 设计（待实现）
- 📦 **归档**：completed/ 已实现（历史记录）
- 🚧 **待办**：tech-debt.md（已知未完成）

---

*最后更新：2026-06-11*
