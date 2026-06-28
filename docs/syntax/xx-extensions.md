# Kaubo 扩展特性状态

记录已完成的语法扩展和待实现特性的最新状态。

---

## 已完成的特性

以下特性已实现并写入正式文档：

| 特性 | 正式文档 | 实现 Phase |
|------|---------|-----------|
| 简写属性 `Point { x, y }` | [07-structs-and-impls](07-structs-and-impls.md) | — |
| Lambda 单表达式体 `\|x\| x+1` | [05-functions](05-functions.md) | — |
| 尾逗号容错 | [04-expressions](04-expressions.md) | — |
| 模板字符串 `` `hello {name}` `` | [01-lexical](01-lexical.md) [04-expressions](04-expressions.md) | Phase 4a |
| Null 合并 `??` | [04-expressions](04-expressions.md) | — |
| 可选链 `?.` `?[` | [04-expressions](04-expressions.md) | — |
| Struct spread `{ ...p, y: 3 }` | [07-structs-and-impls](07-structs-and-impls.md) | — |
| 字符串拼接 `SAdd` | [04-expressions](04-expressions.md) | — |
| Match 表达式 | [06-control-flow](06-control-flow.md) | — |
| Enum/ADT | [03-types](03-types.md) [06-control-flow](06-control-flow.md) | — |
| Interface + operator 重载 | [03-types](03-types.md) [07-structs-and-impls](07-structs-and-impls.md) | Phase 4a ✅ |
| dyn Trait (`\|x: Speakable\|`) | [05-functions](05-functions.md) | Phase 4a ✅ |
| 模块系统 (import/export) | [08-modules](08-modules.md) | Phase 3b ✅ |

### v2.x 已修问题

- 比较运算类型检查：`1 == 2.0` 现在报错
- VM 统一寄存器组：`ints[]/floats[]` → `regs: Vec<u64>`
- CPS Build 类型导向分派：`(op, lhs_type, rhs_type)` 精确匹配指令
- 0x3A opcode 冲突修复

---

## 待实现：核心能力

### 1. 显式泛型（L3 · 设计阶段）

```kaubo
struct Container<T> { value: T };
const id = |x: T| -> T { x };
```

| 层 | 改动 |
|----|------|
| AST | `StructDef`/`Param` 加泛型参数 |
| Type | `Type::Record` 加类型参数位 |
| Infer | 泛型参数绑定、实例化 |
| CPS | Monomorphization——函数体复制+类型替换 |
| VM | 无（单态化后全是具体类型） |

代价：~1200 行。

### 2. 内置模块化 / prelude.kb（L3 · 部分完成）

编译器只给 ~25 个 `@builtins` 原子操作，其余全走 interface。

当前状态：9 个虚拟 interface + 40+ 内置方法已通过 `inject_builtin_interfaces`/`inject_builtin_impls` 硬编码注入。**待做**：真实 `prelude.kb` 文件 + 编译器去硬编码（移除 CPS 层 `to_string`/`IToS` 等重写）。

### 3. 效应系统（L4 · 设计阶段）

效应 = 行多态。CPS 的 `Suspend` 是效应触发点。

```kaubo
effect io;
const fetch = |url: String|: io -> Response { ... };
handle fetch(url) with { io => http_handler() };
```

| 层 | 改动 |
|----|------|
| AST | `EffectDecl`、`Do`、`Handle` |
| Type | `Type::Arrow` 加 `EffectRow` |
| Infer | 效应传播 + 完备性检查 |
| CPS | Suspend 语义化 + handler 注册表 |
| VM | Suspend→查 handler→调度 continuation |

代价：~2000 行。

---

## 暂缓的语法糖

| 特性 | 暂缓原因 |
|------|---------|
| 区间字面量 `0..10` | 需 Range struct 配套 + for 循环 |
| 参数默认值 | 调用方填充逻辑需更多测试覆盖 |
| 列表推导式 | 依赖 for 循环 + list push lowering |
| match 解构 | 枚举模式绑定待实现 |

---

## 成本总表

| # | 特性 | 层级 | 行数 | 状态 |
|----|------|------|------|------|
| 1 | Interface + operator | L4 | ~500 | ✅ 已完成 |
| 2 | 模块系统 | L3 | ~720 | ✅ 已完成 |
| 3 | 显式泛型 | L3 | ~1200 | 设计阶段 |
| 4 | 内置模块化 (prelude.kb) | L3 | ~600 | 🔶 部分完成 |
| 5 | 效应系统 | L4 | ~2000 | 设计阶段 |

## 推荐路线

```
已完成 ── 语法糖 + enum/ADT + match + interface/operator + 模块系统
  ▼
下一步 ── Phase 3a LSP（LspCoordinator 基于 SemanticArtifact）
  ▼
之后 ── Phase 4b 内置模块化收尾（prelude.kb + 去硬编码）
  ▼
之后 ── 泛型 + 效应系统（按需推进）
```
