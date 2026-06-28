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

### 1. 元组与函数调用语义（L3 · 设计阶段）

**核心模型**：函数调用 = 标识符 + 元组。所有函数本质上是单参数函数，参数是一个元组，`|params|` 是对元组的模式解构。

```kaubo
// 值语法
()                // 空元组 / unit
(1,)              // 单元素元组
(1, "a")          // 二元组
(x + 1, y * 2)    // 元素可以是任意表达式

// 类型语法
()                 // 空元组类型 / unit
(Int64,)           // 单元素元组类型
(Int64, String)    // 二元组类型
((Int64,), Bool)   // 嵌套元组类型

// 函数调用 = 标识符 + 元组
add(1, 2)          // add 接受二元组 (Int64, Int64)
f()                // f 接受空元组 ()
g((1,),)           // g 接受单元素元组 ((Int64,),)

// 函数定义：参数是元组解构模式
| x: Int64, y: Int64 | -> Int64 { x + y }    // 解构二元组 → 类型 (Int64, Int64) -> Int64
| x: Int64, | -> Int64 { x * 2 }             // 解构单元素元组 → 类型 (Int64,) -> Int64
|| -> Int64 { 42 }                           // 解构空元组 → 类型 () -> Int64
```

### 语法与类型的对称映射

| 概念 | 值语法 | 类型语法 |
|------|--------|---------|
| 空元组 | `()` | `()` |
| 单元素元组 | `(1,)` | `(Int64,)` |
| 二元组 | `(1, "a")` | `(Int64, String)` |
| 函数类型 | — | `(Int64, Int64) -> Int64` |

### LL(1) 解析规则

```
( expr ,     → 元组模式（至少一个逗号）
( expr )     → 分组，AST 折叠为 expr
( )          → 空元组 / unit
```

符号表不参与解析决策。逗号的存在/缺失是唯一判定依据。

### 各层改动

| 层 | 改动 |
|----|------|
| AST | `Expr::Tuple(Vec<Expr>)`、`TypeExpr::Tuple(Vec<TypeExpr>)` |
| Parser | 括号内逗号判定 → 元组 vs 分组；`parse_call` 改为单 arg 元组 |
| Infer | 元组类型推断；函数参数 → 元组解构模式匹配 |
| CPS | `Call` 指令改为单 arg；新增 `GetField(tuple_reg, index)` 解构元组 |
| VM | `HeapObj::TupleObj(Vec<usize>)`；`GetField` 复用 struct 逻辑 |

代价：~500 行。**元组是泛型的前置**——泛型 `struct Container<T>` 和 `Result<T, E>` 直接受益于元组类型系统。

---

### 2. 显式泛型（L3 · 设计阶段）

```kaubo
struct Container<T> { value: T };
const id = |x: T| -> T { x };
```

| 层 | 改动 |
|----|------|
| AST | `StructDef`/`Param` 加泛型参数（`<>` 定界，无歧义——类型标注上下文 `:` 后 `<` 不可能是小于号） |
| Type | `Type::Record` 加类型参数位 |
| Infer | 泛型参数绑定、实例化；HM 推断消除显式类型参数需求 |
| CPS | Monomorphization——函数体复制+类型替换 |
| VM | 无（单态化后全是具体类型） |

代价：~1200 行。

### 3. 内置模块化 / prelude.kb（L3 · 部分完成）

编译器只给 ~25 个 `@builtins` 原子操作，其余全走 interface。

当前状态：9 个虚拟 interface + 40+ 内置方法已通过 `inject_builtin_interfaces`/`inject_builtin_impls` 硬编码注入。**待做**：真实 `prelude.kb` 文件 + 编译器去硬编码（移除 CPS 层 `to_string`/`IToS` 等重写）。

### 4. 效应系统（L4 · 设计阶段）

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
| 3 | 元组 + 函数调用语义 | L3 | ~500 | 设计阶段 |
| 4 | 显式泛型 | L3 | ~1200 | 设计阶段 |
| 5 | 内置模块化 (prelude.kb) | L3 | ~600 | 🔶 部分完成 |
| 6 | 效应系统 | L4 | ~2000 | 设计阶段 |

## 推荐路线

```
已完成 ── 语法糖 + enum/ADT + match + interface/operator + 模块系统
  ▼
下一步 ── Phase 1 LSP（LspCoordinator 基于 SemanticArtifact）
  ▼
之后 ── 元组（函数调用语义基础，泛型前置）
  ▼
之后 ── 泛型 + 效应系统（按需推进）
```
