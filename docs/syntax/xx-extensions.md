# Kaubo 扩展语法与架构改造设计

代价层级：L0（~50行纯parser）L1（~200行含脱糖）L2（~500行涉类型/CPS）L3（~1000行涉VM）L4（结构性改动）

---

# 第一部分：低成本语法糖（第一阶段可实施）

只改 lexer/parser/AST，脱糖到已有能力。不改类型系统/CPS/VM（SAdd 除外——是填坑）。

## 1. 简写属性（L0 · ~20行）

```kaubo
// 入
const p = Point { x, y };
// 出
const p = Point { x: x, y: y };
```

改：parser 中 struct literal 字段 ident 后无 `:` 则自动生成 VarRef。

## 2. Lambda 单表达式体（L0 · ~20行）

```kaubo
// 入
const add = |a, b| a + b;
// 出
const add = |a, b| { a + b };
```

改：`|params|` 后非 `{` 则单表达式包 Block。遇 `;` `,` `)` `}` `]` 结束。

## 3. 尾逗号容错（L0 · ~10行）

```kaubo
const p = Point { x: 1, y: 2, };
const xs = [1, 2, 3,];
```

改：`}` `]` 前接受可选逗号。

## 4. 模板字符串（L1 · ~140行）

```kaubo
// 入
const msg = `hello {name}, age {age}`;
// 出
const msg = "hello " + name.to_string() + ", age " + age.to_string();
```

改：lexer 加 `` ` `` 模板模式（`{` 切表达式 `}` 回字面量），parser 每段 `{expr}` 生成 `.to_string()`，段间 `+` 拼接。依赖 SAdd lowering（#8）。

## 5. Null 合并 `??`（L1 · ~85行）

```kaubo
// 入
const name = input ?? "default";
// 出
const name = { var t = input; if t != null { t } else { "default" } };
```

改：lexer `??` token（无独立 `?` 无歧义），脱糖 if/else。

## 6. 可选链 `?.`（L1 · ~110行）

```kaubo
// 入
const name = user?.profile?.name;
// 出
const name = {
    var t = user;
    if t != null { t = t.profile; } else { t = null; };
    if t != null { t = t.name; } else { t = null; };
    t
};
```

改：lexer `?.` `?[` token，AST `OptionalMember`/`OptionalIndex`，脱糖展开。

## 7. 结构体 spread（L1 · ~110行）

```kaubo
// 入
const p2 = Point { ...p1, y: 3 };
// 出（按 struct 声明字段序展开，后覆盖前）
const p2 = Point { x: p1.x, y: 3 };
```

改：lexer `...` token，AST `StructSpread`，lowering 按 struct 字段列表展开。

## 8. 字符串拼接 lowering 补全（L1 · ~60行）

AST 已有 `BinOp::SAdd`，infer 已有推断，CPS 已有 `CpsBinOp::SAdd`。仅 lowering 返回未实现。

改：cps_emit 补 emit_sadd，VM 补 SAdd opcode（读两 heap handle→拼接→写回堆）。解除 #4 的依赖。

## 9. Match 表达式（常量匹配）（L1 · ~250行）

```kaubo
// 入
const desc = match x { 0 => "zero"; 1 => "one"; _ => "many"; };
// 出
const desc = {
    var t = x;
    if t == 0 { "zero" } else if t == 1 { "one" } else { "many" }
};
```

改：`match` 关键字，`=>` token，AST `Match`。要求最后 arm 为 `_`（穷尽性检查等 enum 后补）。脱糖生成 if/else 链。

---

## 第一阶段汇总（9 项，~800 行）

```
A（半天）: #1 简写属性 + #2 lambda 单表达式 + #3 尾逗号      ~50行
B（半天）: #4 模板字符串 + #5 ?? + #6 ?.                     ~335行
C（一天）: #7 struct spread + #8 SAdd 补全 + #9 match        ~420行
```

全在 lexer/parser/AST/cps_emit/VM 五层。仅 #4 依赖 #8。

---

## 暂缓的语法糖

| 特性 | 暂缓原因 |
|------|---------|
| 区间字面量 `0..10` | 需 Range struct 配套 + for 循环 |
| 参数默认值 | 调用方填充逻辑需更多测试覆盖 |
| 列表推导式 | 依赖 for 循环 + list push lowering |
| match 解构 | 依赖 enum |

---

# 第二部分：核心能力

## 10. Enum/ADT（L3 · 设计阶段）

```kaubo
enum Option<T> { Some(T), None };
enum Result<T, E> { Ok(T), Err(E) };
```

90% 复用现有 Record/NewStruct 模式：

| 层 | 改动 |
|----|------|
| AST | `Stmt::EnumDef { name, params, variants }` |
| Type | `Type::Variant(id, type_args, variants)` |
| Infer | variant 构造/解构推断，穷尽性（可选） |
| CPS | `NewVariant`、`MatchVariant` 指令 |
| VM | `HeapObj::Variant(tag, payload)`，2 个新 opcode |

## 11. 显式泛型（L3 · 依赖 enum · 设计阶段）

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

## 12. Interface（L4 · 依赖 enum · 设计阶段）

动态分派 + 显式声明（非 Go 隐式结构化）。接口值 = 胖指针 `(vtable, data)`。

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
| VM | `CallIndirect` opcode（或复用 CallNative） |

最小可行：1 新指令 + 1 opcode + vtable 表，~300 行核心改动。

## 13. 错误处理（L2 · 依赖 enum）

```kaubo
// Result —— enum 落地后标准库实现，零编译器改动
const divide = |a, b| -> Result<Float64, String> { ... };

// panic —— VM 栈展开
panic("empty list");
```

- `Result<T,E>`：标准库级别，enum 后自动解锁
- `panic`：CallFrame 链天然支持栈展开，~200 行
- `?` 传播：match 糖上叠加，~100 行

## 14. 模块系统（L3 · 设计阶段）

```kaubo
import { fetch } from "./http/client";
import * as http from "./http/client";
export const answer = 42;
export effect io;
```

| 层 | 改动 |
|----|------|
| AST | import/export 已 parse-only，补语义 |
| Driver | 模块图构建、路径解析、拓扑排序 |
| Infer | 跨模块名称解析 |
| CPS | 多模块链接 |

---

# 第三部分：架构改造

## 15. 内置类型/函数模块化（L3 · 依赖 interface · 设计阶段）

问题：当前 `to_string()` 硬编码在 infer.rs、`print`/`sqrt` 硬编码在 stdlib.rs。
加一个新内置类型要碰 infer + lowering + VM 三层。

方案：编译器只给 ~25 个 `@builtins` 原子操作，其余全走接口。

```
编译器魔法:
  @addInt @subInt @mulInt ...   整数原语
  @fadd @fsub @fmul ...         浮点原语
  @eqInt @ltInt ...             比较
  @intToString @floatToString   格式化
  @sqrt @sin @cos               原生数学

接口层（prelude.kb）:
  interface Add { add() }
  interface Eq { eq() }
  interface Display { to_string() }

原语实现（primitive.kb）:
  impl Add for Int64 { add: |a,b| @addInt(a,b) }
  impl Display for Int64 { to_string: |x| @intToString(x) }

标准库（stdlib/*.kb）:
  const print = |x: Display| { @print(x.to_string()) }
```

收益：此后加新内置类型**一行编译器代码都不用改**，写 `impl Display for NewType` 即可。

业界参考：

| 语言 | 做法 | Kaubo 借鉴 |
|------|------|-----------|
| Rust | lang item 标记 ~40 个，其余是库代码 | 最小化编译器魔法 |
| Swift | stdlib 即普通 Swift 模块，print 是库函数 | print 不是内置 |
| Zig | `@` 前缀 intrinsics | 编译器 vs 标准库边界清晰 |
| Haskell | prelude 隐式导入 | prelude.kb 自动注入 |

## 16. 效应系统（L4 · 设计阶段）

效应 = 行多态，非泛型。CPS 的 `Suspend` 是效应触发点。

```kaubo
effect io;
effect fork<T>;

const fetch = |url: String|: io -> Response { ... };
const crawl = |urls: List<String>|: io + fork -> List<Page> { ... };

handle crawl(urls) with {
    io   => http_handler(),
    fork => thread_pool(8),
};
```

| 层 | 改动 |
|----|------|
| AST | `EffectDecl`、`Do`、`Handle` |
| Type | `Type::Arrow` 加 `EffectRow`，行合并推断 |
| Infer | 效应传播（不参与 HM unify） + 完备性检查 |
| CPS | Suspend 语义化 + handler 注册表 |
| VM | Suspend→查 handler→调度 continuation |

架构评估：CPS 已有 `Suspend`，VM 已有 `AsyncScheduler`，主要是连接工作。

---

# 成本总表

| # | 特性 | 层级 | 行数 | 状态 |
|----|------|------|------|------|
| 1 | 简写属性 | L0 | ~20 | 可实施 |
| 2 | lambda 单表达式 | L0 | ~20 | 可实施 |
| 3 | 尾逗号容错 | L0 | ~10 | 可实施 |
| 4 | 模板字符串 | L1 | ~140 | 可实施（依赖 #8） |
| 5 | `??` | L1 | ~85 | 可实施 |
| 6 | `?.` | L1 | ~110 | 可实施 |
| 7 | struct spread | L1 | ~110 | 可实施 |
| 8 | SAdd lowering | L1 | ~60 | 可实施 |
| 9 | match 常量匹配 | L1 | ~250 | 可实施 |
| 10 | enum/ADT | L3 | ~800 | 设计阶段 |
| 11 | 显式泛型 | L3 | ~1200 | 依赖 enum |
| 12 | Interface | L4 | ~500 | 依赖 enum |
| 13 | 错误处理 | L2 | ~300 | 依赖 enum |
| 14 | 模块系统 | L3 | ~1500 | 设计阶段 |
| 15 | 内置模块化 | L3 | ~600 | 依赖 interface |
| 16 | 效应系统 | L4 | ~2000 | 设计阶段 |

# 推荐路线

```
现在 ── 第一阶段  语法糖（9项，~800行）
  │   简写属性、lambda单表达式、尾逗号、
  │   模板字符串、??、?.、struct spread、SAdd、match
  ▼
之后 ── enum      （~800行，开启 Result/Option/解构 match）
  ▼
之后 ── interface  （~500行，开启 Display/Eq/Add/运算符重载）
  ▼
之后 ── 内置模块化 （~600行，prelude.kb，编译器去硬编码）
  ▼
之后 ── 模块系统 + 泛型 + 效应系统（按需推进）
```
