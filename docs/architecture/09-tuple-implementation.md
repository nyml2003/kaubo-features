# Phase 1 实施记录：元组 + TypedArray ✅ 已完成

> 2026-06-28 交付。全量测试通过，benchmark 无退化。

## 改动总览

| 层 | 文件 | 行数 | 性质 |
|----|------|------|------|
| AST | `kaubo-ast/src/lib.rs` | ~30 | 新增 Tuple/TupleType；Call.args → Call.arg |
| 全局迁移 | 全仓库 | (穿插) | ~150 处 Call 构造点批量迁 |
| Parser | `kaubo-syntax/src/parser.rs` | ~80 | 括号逗号判定；parse_call 改单 arg |
| Infer | `kaubo-infer/src/infer.rs` + `types.rs` | ~80 | Tuple type；边界条件 1+3 |
| CPS IR | `kaubo-cps/src/lib.rs` | ~20 | NewTuple/TupleIndex/NewInt64Array/NewFloat64Array |
| CPS Build | `kaubo-ir/src/cps_build.rs` | ~150 | Call 单 arg；Tuple 构建；TupleIndex 拆解；TypedArray emit；边界条件 2 |
| CPS Emit | `kaubo-ir/src/cps_emit.rs` | ~30 | emit_tuple/emit_tuple_index/emit_new_*_array |
| VM | `kaubo-vm/src/execute.rs` + `gc_heap.rs` | ~120 | TupleObj/Int64Array/Float64Array；TupleIndex handler；IndexGet/Set tag 分派 |
| Tests | 各层 | ~150 | 迁移 + 新功能测试 |
| Docs | `language/` | ~40 | 元组/类型/函数/列表文档更新 |

**合计 ~700 行**。一次性硬切，不留旧 `Call` 节点。

---

## 步骤 1：AST（~30 行）— 先改数据结构

**文件**：`kaubo-ast/src/lib.rs`

### 变更

```rust
// 新增
Expr::Tuple(Vec<Expr>),                    // (1, "a")  /  (1,)  /  ()
TypeExpr::Tuple(Vec<TypeExpr>),            // (Int64, String)

// 修改
Call { func: Box<Expr>, args: Vec<Expr> }  // 旧
  → Call { func: Box<Expr>, arg: Box<Expr> }  // 新
```

### 验收

- `cargo check -p kaubo-ast` 通过（本 crate 内构造点修复）
- 新建 `Expr::Tuple` 和 `TypeExpr::Tuple` 的 Display 输出正确

### 风险

- `Call` 改签名后所有下游 crate 编译报错——这是预期的，步骤 2-5 逐个修

---

## 步骤 2：Parser（~80 行）— 元组 vs 分组判定

**文件**：`kaubo-syntax/src/parser.rs`

### 变更

**2a. 括号表达式改造**（`parse_atom` 中 `TokenKind::LParen` 分支）

```
当前：LParen → parse_expr → expect(RParen) → 直接返回 expr（无分组/元组区分）

改为：
LParen →
  1. 空括号 () → Expr::Tuple(vec![])
  2. parse_expr
     - 看下一个 token：
       Comma → 进入元组模式，继续收集元素直到 RParen → Expr::Tuple(elements)
       RParen → 分组，折叠为 expr（不产生 Tuple 节点）
```

LL(1) 判定：逗号是唯一决策 token。

**2b. `parse_call` 改造**

```
当前：func(args...) → Call { func, args: Vec<Expr> }
改为：func(tuple_or_group) → Call { func, arg: Box<Expr> }
  - func()    → arg = Expr::Tuple([])
  - func(1, 2) → arg = Expr::Tuple([LitInt(1), LitInt(2)])
  - func(x)   → arg = VarRef("x")（分组折叠）
```

**2c. 修复 parser 内部测试**

`parser.rs` 中 ~100 个 inline test 直接构造 `Expr::Call { args: ... }`，需要批量更新为 `Expr::Call { arg: ... }`。

### 验收

- `cargo test -p kaubo-syntax` 全绿
- 新测试：`()` → `Expr::Tuple([])`、`(1,)` → `Expr::Tuple([1])`、`(1,2)` → `Expr::Tuple([1,2])`、`(1)` → `Expr::LitInt(1)`
- 新测试：`f(1,2)` → `Call { f, Tuple([1,2]) }`、`f()` → `Call { f, Tuple([]) }`

### 风险

- parser 内部 ~100 个测试构造点需要批量改——机械操作但量大。建议用 regex 批量替换 `args: vec!` → `arg: Box::new(Expr::Tuple(vec!`，再手动修非元组场景

---

## 步骤 3：Type Inference（~80 行）— 元组类型 + 形状统一

**文件**：`kaubo-infer/src/types.rs`、`kaubo-infer/src/infer.rs`

### 关键设计决策（必须在实现前锁定）

**边界条件 1：单参数不自动降级为元组**

函数签名中参数列表的类型收集规则：

```
有逗号（≥2 参数）  → Type::Tuple([Int64, Int64])  → 类型为 (Int64, Int64) -> Ret
无逗号（1 参数）   → 直接使用参数类型             → 类型为 Int64 -> Ret
无参数             → Type::Tuple([])              → 类型为 () -> Ret
```

```
| x: Int64, y: Int64 | -> Int64   // Arrow(Tuple([Int64, Int64]), Int64)
| x: Int64 | -> Int64             // Arrow(Int64, Int64)  ← 不是 Arrow(Tuple([Int64]), Int64)
|| -> Int64                       // Arrow(Tuple([]), Int64)
```

调用匹配：

```
inc(x) 定义为 |x: Int64| -> Int64（类型 Int64 -> Int64）
  inc(1)       → ArgType=Int64, ParamType=Int64 → 匹配 ✅
  inc((1,))    → ArgType=(Int64,), ParamType=Int64 → 不匹配 ❌
```

这确保用户不需要为普通单参数函数写 `inc((1,))`。

**边界条件 3：Call 的类型匹配是「形状统一」，不是递归解构**

Infer 层不展开元组。CPS 层负责将元组寄存器拆成局部变量，Infer 只负责类型形状的整体匹配：

```
Call { func, arg }:
  1. infer(func) → Arrow(ParamType, RetType)
  2. infer(arg)  → ArgType
  3. unify(ArgType, ParamType)  // 整体统一，不递归拆解
```

| func 类型 | arg | 匹配方式 |
|-----------|-----|---------|
| `(Int64, Int64) -> Int64` | `(1, 2): (Int64, Int64)` | unify((Int64,Int64), (Int64,Int64)) ✅ |
| `Int64 -> Int64` | `1: Int64` | unify(Int64, Int64) ✅ |
| `(Int64, String) -> Bool` | `(1, "a"): (Int64, String)` | unify shape ✅ |

### 变更

**3a. 类型枚举**（`types.rs`）

```rust
Type::Tuple(Vec<Type>),  // 新增：元组类型
```

**3b. Infer 逻辑**（`infer.rs`）

- `TypeExpr::Tuple(elements)` → 推断每个元素类型 → `Type::Tuple(element_types)`
- `Expr::Tuple(elements)` → 推断每个元素 → `Type::Tuple(element_types)`
- 函数参数列表 → 按「边界条件 1」规则：逗号判定决定是否包 Tuple
- `Expr::Call { func, arg }` → 按「边界条件 3」规则：整体 unify，不解构

### 验收

- `cargo test -p kaubo-infer` 全绿
- `f(1, 2)` 中 f 为 `|x: Int64, y: Int64| -> Int64` 时，推断 arg `(1,2)` 为 `(Int64, Int64)`，与 ParamType unify 成功
- `inc(1)` 中 inc 为 `|x: Int64| -> Int64` 时，推断 arg `1` 为 `Int64`，直接与 ParamType=Int64 unify
- `inc((1,))` → 类型不匹配，编译报错

### 风险

- `Type` 枚举新增变体后，所有 match 需要覆盖 `Tuple`——编译器会报 exhaustive pattern 错误，逐个补即可

---

## 步骤 4：CPS（~200 行）— Lowering + Emit + TypedArray

**文件**：`kaubo-cps/src/lib.rs`、`kaubo-ir/src/cps_build.rs`、`kaubo-ir/src/cps_emit.rs`

### 关键设计决策

**边界条件 2：新增专用 `CpsInstr::TupleIndex`，不复用 `GetField`**

```
CpsInstr::TupleIndex(dst, tuple_reg, index)
  — dst: 目标寄存器
  — tuple_reg: 元组所在寄存器
  — index: 字段索引（0-based）

CpsInstr::GetField(dst, obj_reg, field_idx)
  — 保留给 struct，按字段名查找
```

**不可复用理由**：struct 的 `GetField` 按字段名查找（field_idx 映射到 struct 声明的字段名），Tuple 按位置索引查找。数据结构不同，复用会导致 VM handler 内部需要 match obj 类型分派，逻辑混乱。直接分成两条指令，VM 层各自处理。

### 变更

**4a. CPS 指令**（`kaubo-cps/src/lib.rs`）

```rust
// 元组
CpsInstr::NewTuple(usize, Vec<usize>),        // dst, element_regs
CpsInstr::TupleIndex(usize, usize, usize),     // dst, tuple_reg, index — 不复用 GetField

// TypedArray
CpsInstr::NewInt64Array(usize, Vec<usize>),    // dst, element_regs
CpsInstr::NewFloat64Array(usize, Vec<usize>),  // dst, element_regs
```

**4b. CPS Build**（`kaubo-ir/src/cps_build.rs`）

- `Expr::Call { func, arg }` → 构建 `Call` terminator，单 arg
- `Expr::Tuple(elements)` → `NewTuple` + 逐元素构建
- 函数入口拆解（按「边界条件 1」）：
  - 多参数函数 → 插入 `TupleIndex(tuple_reg, 0)`, `TupleIndex(tuple_reg, 1)`, ... 提取各参数到局部寄存器
  - 单参数函数 → **不插入 TupleIndex**，直接使用 arg 寄存器作为参数
- `Expr::ListLit(...)` + 类型标注 → 根据 `TypeExpr::List(inner)` 选择：
  - `List<Int64>` → `NewInt64Array`
  - `List<Float64>` → `NewFloat64Array`
  - 无标注 / 混合 → `NewList`（现状）

**4c. CPS Emit**（`kaubo-ir/src/cps_emit.rs`）

- `emit_new_tuple(dst, elements)` → 编码 `NewTuple`
- `emit_tuple_index(dst, tuple_reg, index)` → 编码 `TupleIndex`
- `emit_new_int64_array(dst, elements)` → 编码 `NewInt64Array`
- `emit_new_float64_array(dst, elements)` → 编码 `NewFloat64Array`

### 验收

- `cargo test -p kaubo-ir` 全绿
- `cargo test -p kaubo-cps` 全绿
- 多参数 `f(1, 2)` → CPS：`NewTuple` → `Call`(tuple_reg) → 函数入口 `TupleIndex` 拆解
- 单参数 `inc(5)` → CPS：`Call`(int_reg) → 函数入口直接使用，无 TupleIndex
- `[1, 2] : List<Int64>` 生成 `NewInt64Array`

### 风险

- `cps_build.rs` 是 ~3400 行的大文件，`Expr::Call` 的匹配分支遍及全局。需要在步骤 1 完成后立刻全局 grep 迁移所有构造点
- `CpsTerminator::Call` 可能也需要改为单 arg（目前 args: Vec<usize>）。如果是，同步改 VM 的 Call handler

---

## 步骤 5：VM（~120 行）— 运行时表示 + TypedArray 分派

**文件**：`kaubo-vm/src/execute.rs`、`kaubo-vm/src/gc_heap.rs`

### 变更

**5a. 堆对象**（`execute.rs` + `gc_heap.rs`）

```rust
HeapObj::TupleObj(Vec<usize>),        // 元素为 reg 值（或堆 handle）
HeapObj::Int64Array(Vec<i64>),        // 密集整数，连续内存
HeapObj::Float64Array(Vec<f64>),      // 密集浮点，连续内存
```

Int64Array/Float64Array 在 GC 堆中以 `Vec<i64>` / `Vec<f64>` 存储为连续内存。

**5b. Opcode handler**

- `NewTuple` — 构建 `HeapObj::TupleObj(elements)`
- `TupleIndex` — 从 `TupleObj` 中按索引读取。**不复用 GetField 的 handler 逻辑**：直接 `match obj { TupleObj(v) => v[index] }`，与 struct 的字段名查找走不同路径
- `NewInt64Array` / `NewFloat64Array` — 构建 `Int64Array(Vec<i64>)` / `Float64Array(Vec<f64>)`
- `IndexGet` / `IndexSet` — **按 heap obj tag 分派**，必须检查 `HeapObj` 的变体 tag 确保动态类型安全：

```
IndexGet 分派：
  List(v)           → v[idx] as i64（现状）
  Int64Array(v)     → v[idx] as i64
  Float64Array(v)   → f64::to_bits(v[idx])（f64 → u64 写入 reg）
  _                 → runtime error: "IndexGet on non-indexable type"

IndexSet 分派：
  List(v)           → v[idx] = val（现状）
  Int64Array(v)     → v[idx] = val as i64
  Float64Array(v)   → v[idx] = f64::from_bits(reg_val)（u64 → f64）
  _                 → runtime error
```

### 验收

- `cargo test -p kaubo-vm` 全绿
- `TupleIndex` 从 `TupleObj` 中按索引取字段
- TypedArray `[1, 2, 3]: List<Int64>` → `Int64Array`
- 运行时对 `Float64Array` 做 `IndexGet` 得正确的 f64 bit pattern
- 运行时对 `Int64Array` 做 `IndexGet` 返回裸 i64
- 试图对 `String("hi")` 做 `IndexGet` → runtime error

### 风险

- `IndexGet`/`IndexSet` 改为按类型 tag 分派后，增加一层 match，需确认不引入性能退化
- Float64Array 的 `f64::from_bits` ↔ `f64::to_bits` 转换确保无损

---

## 步骤 6：全量测试 + 文档（~200 行）

### 测试修复

- **全仓库 grep** `Expr::Call` 构造点，`args:` → `arg:`
- parser 测试中 `args: vec![` → 改为新的 tuple 包装
- driver 集成测试：分组 `(1+2)*3` 不受影响；元组 `(1, 2)` + 函数调用 `f(1, 2)` 端到端通过

### 文档更新

- `language/03-types.md` — 新增元组类型 `(Int64, String)` 语法
- `language/04-expressions.md` — 元组字面量 + 分组规则
- `language/05-functions.md` — 函数调用 = 标识符 + 元组
- `language/10-partial-features.md` — 列表改为 TypedArray 说明

### 验收

- `python kaubo-ops ci` 全绿
- benchmark 输出无退化（TypedArray 不应比 List 差）

---

## 步骤 1.5：全局 Call 构造点批量迁移（穿插在步骤 1 之后）

步骤 1（AST 改 `Call` 签名）之后，全仓库会出现大量编译错误。为避免在步骤 4（`cps_build.rs`，~3400 行）中大海捞针，**在 Parser 步骤完成之前**先批量迁移所有 `Expr::Call` 构造点。

### 迁移规则

```
1. 全局 grep: Expr::Call { func, args:
   → 改为:    Expr::Call { func, arg:

2. 多参数 args: vec![a, b, c]
   → arg: Box::new(Expr::Tuple(vec![a, b, c]))

3. 单参数 args: vec![x]
   → arg: Box::new(x)（不套 Tuple，保持直接传递）

4. 零参数 args: vec![]
   → arg: Box::new(Expr::Tuple(vec![]))
```

### 操作方式

```bash
# 找出所有引用点
rg "Expr::Call" --type rust -l
rg "Call\s*{" --type rust -l

# 对每个文件逐个迁移，不要用 sed 批量替换——单/多/零参数三种情况需要人眼判断
```

### 预计范围

- `parser.rs` 测试区：~80 处
- `cps_build.rs`：~30 处
- `infer.rs` 测试：~10 处
- `driver` 测试：~10 处
- 其他 crate 测试：~20 处

---

## 执行顺序

```
步骤 1: AST   ──→ 步骤 1.5: 全局Call迁移 ──→ 步骤 2: Parser
                                                    │
                                                    ▼
                                              步骤 3: Infer
                                                    │
                                              步骤 4: CPS
                                                    │
步骤 6: Tests+Doc ←─────────────────────────── 步骤 5: VM
```

每一步验收标准独立。步骤 1 后全仓库编译报错是预期行为——步骤 1.5 批量消掉机械错误，步骤 2-5 各层修到编译通过 + 测试绿。
