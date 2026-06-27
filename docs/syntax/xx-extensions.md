# Kaubo 未实现特性设计（设计阶段）

代价层级：L2（~500行涉类型/CPS）L3（~1000行涉VM）L4（结构性改动）

## v2.4 已修问题

- 比较运算类型检查：`1 == 2.0` 现在报错
- VM 统一寄存器组：`ints[]/floats[]` → `regs: Vec<u64>`
- CPS Build 类型导向分派：`(op, lhs_type, rhs_type)` 精确匹配指令
- 0x3A opcode 冲突修复

---

# 已完成的语法

以下特性已实现并写入正式文档：

| 特性 | 正式文档 |
|------|---------|
| 简写属性 `Point { x, y }` | [07-structs-and-impls](07-structs-and-impls.md) |
| Lambda 单表达式体 `\|x\| x+1` | [05-functions](05-functions.md) |
| 尾逗号容错 | [04-expressions](04-expressions.md) |
| 模板字符串 `` `hello {name}` `` | [01-lexical](01-lexical.md) [04-expressions](04-expressions.md) |
| Null 合并 `??` | [04-expressions](04-expressions.md) |
| 可选链 `?.` `?[` | [04-expressions](04-expressions.md) |
| Struct spread `{ ...p, y: 3 }` | [07-structs-and-impls](07-structs-and-impls.md) |
| 字符串拼接 `SAdd` | [04-expressions](04-expressions.md) |
| Match 表达式 | [06-control-flow](06-control-flow.md) |

---

# 待实现：核心能力

## 0. Enum/ADT（L3 · 设计阶段）

`enum` 声明代数数据类型，变体可以是单元变体或带字段变体。当前 parser/infer/CPS/VM 均未实现，
`03-types.md` 中的语法为设计目标。

```kaubo
enum Color { Red, Green };
enum Option { Some(value: Int64), None };
```

| 层 | 改动 |
|----|------|
| Token | 新增 `Enum` 关键字 |
| AST | 新增 `Stmt::EnumDef`、`Expr::VariantLit` |
| Type | 新增 `Type::Variant` sum type |
| Parser | 解析 enum 声明和变体构造子 |
| Infer | Unification 处理 sum types，match 臂类型检查 |
| CPS | 新增 `GetVariantTag`、`NewVariant` 指令 |
| VM | 对应 opcode 实现 |

依赖此特性：`Option<T>`、`Result<T, E>` 标准库类型。

---

## 1. 显式泛型（L3 · 依赖 enum · 设计阶段）

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

## 2. Interface（L4 · 依赖 enum · 设计阶段）

动态分派 + 显式声明。接口值 = 胖指针 `(vtable, data)`。

```kaubo
interface Eq { eq: |self: Self, other: Self| -> Bool; };

impl Eq for Point {
    eq: |self: Point, other: Point| -> Bool {
        return self.x == other.x and self.y == other.y;
    };
};

const contains = |xs: List<Eq>, target: Eq| -> Bool { ... };
```

| 层 | 改动 |
|----|------|
| AST | `Stmt::InterfaceDef`、`ImplBlock` 扩展 |
| Infer | 接口匹配检查，vtable 生成 |
| CPS | `LoadVtable` 指令 |
| VM | `CallIndirect` opcode |

最小可行：1 新指令 + 1 opcode + vtable 表，~300 行核心改动。

## 3. Option / Result（依赖 Enum/ADT · 设计阶段）

`Option` 和 `Result` 设计为普通 enum，依赖 Enum/ADT 实现。错误通过返回值传播，
不需要特殊的错误处理运行时（无 panic/catch/栈展开）。

```kaubo
// 设计语法，等 Enum/ADT 实现后方可运行：
enum Option { Some(value: Int64), None };
enum Result { Ok(value: Int64), Err(msg: String) };

const divide = |a: Float64, b: Float64| -> Result {
    if b == 0.0 { return Err("division by zero"); };
    return Ok(a / b);
};
```

泛型 `Option<T>` / `Result<T, E>` 需等显式泛型落地。

## 4. 模块系统（L3 · 设计阶段）

```kaubo
import { fetch } from "./http/client";
import * as http from "./http/client";
export const answer = 42;
```

| 层 | 改动 |
|----|------|
| AST | import/export 已 parse-only，补语义 |
| Driver | 模块图构建、路径解析、拓扑排序 |
| Infer | 跨模块名称解析 |
| CPS | 多模块链接 |

---

# 待实现：架构改造

## 5. 内置类型/函数模块化（L3 · 依赖 interface · 设计阶段）

编译器只给 ~25 个 `@builtins` 原子操作，其余全走接口。

```
编译器魔法: @addInt @subInt @eqInt @intToString ...
接口层:    interface Add { add() } interface Display { to_string() }
标准库:    const print = |x: Display| { @print(x.to_string()) }
```

收益：加新内置类型一行编译器代码都不用改。

## 6. 效应系统（L4 · 设计阶段）

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

---

## 暂缓的语法糖

| 特性 | 暂缓原因 |
|------|---------|
| 区间字面量 `0..10` | 需 Range struct 配套 + for 循环 |
| 参数默认值 | 调用方填充逻辑需更多测试覆盖 |
| 列表推导式 | 依赖 for 循环 + list push lowering |
| match 解构 | 枚举模式绑定待实现 |

---

# 成本总表

| # | 特性 | 层级 | 行数 | 状态 |
|----|------|------|------|------|
| 1 | 显式泛型 | L3 | ~1200 | 设计阶段 |
| 2 | Interface | L4 | ~500 | 设计阶段 |
| 3 | 模块系统 | L3 | ~1500 | 设计阶段 |
| 4 | 内置模块化 | L3 | ~600 | 依赖 interface |
| 5 | 效应系统 | L4 | ~2000 | 设计阶段 |

# 推荐路线

```
已完成 ── 语法糖（9项） + match
  ▼
下一步 ── Enum/ADT   （代数数据类型，Option/Result 基础）
  ▼
之后 ── interface  （动态分派，开启 Display/Eq/Add）
  ▼
之后 ── 内置模块化 （prelude.kb，编译器去硬编码）
  ▼
之后 ── 模块系统 + 泛型 + 效应系统（按需推进）
```
