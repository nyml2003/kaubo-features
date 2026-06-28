# Phase 1 实施计划：元组 + TypedArray

## 改动总览

| 层 | 文件 | 行数 | 性质 |
|----|------|------|------|
| AST | `kaubo-ast/src/lib.rs` | ~30 | 新增 Tuple/TupleType 节点；Call.args → Call.arg |
| Parser | `kaubo-syntax/src/parser.rs` | ~80 | 括号内逗号判定；parse_call 改单 arg |
| Infer | `kaubo-infer/src/infer.rs` + `types.rs` | ~80 | Tuple type；函数参数解构元组 |
| CPS IR | `kaubo-cps/src/lib.rs` | ~20 | NewTuple/NewInt64Array/NewFloat64Array 指令 |
| CPS Build | `kaubo-ir/src/cps_build.rs` | ~150 | Call 单 arg；Tuple 构建/解构；TypedArray emit |
| CPS Emit | `kaubo-ir/src/cps_emit.rs` | ~30 | emit_tuple / emit_new_int64_array / emit_new_float64_array |
| VM | `kaubo-vm/src/execute.rs` + `gc_heap.rs` | ~120 | TupleObj/Int64Array/Float64Array；IndexGet/Set 按类型分派 |
| Tests | 各层 | ~150 | 修复所有 Call 构造点 + 新功能测试 |
| Docs | `language/` | ~40 | 更新字面量/类型/模块文档 |

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

## 步骤 3：Type Inference（~80 行）— 元组类型 + 函数参数解构

**文件**：`kaubo-infer/src/types.rs`、`kaubo-infer/src/infer.rs`

### 变更

**3a. 类型枚举**（`types.rs`）

```rust
Type::Tuple(Vec<Type>),  // 新增：元组类型
```

**3b. Infer 逻辑**（`infer.rs`）

- `TypeExpr::Tuple(elements)` → `Type::Tuple(elements.map(infer))`
- 函数参数列表 → 收集为 `Type::Tuple(param_types)`，函数类型保持 `Arrow(Tuple(params), ret)`
- `Expr::Tuple(elements)` → 推断每个元素类型 → `Type::Tuple(element_types)`
- `Expr::Call { func, arg }` → 推断 arg → 若是 Tuple 则解构匹配函数参数

### 验收

- `cargo test -p kaubo-infer` 全绿
- `f(1, 2)` 中 f 为 `|x: Int64, y: Int64| -> Int64` 时，`(1, 2)` 正确推断为 `(Int64, Int64)`

### 风险

- `Type` 枚举新增变体后，所有 match 需要覆盖 `Tuple`——编译器会报 exhaustive pattern 错误，逐个补即可

---

## 步骤 4：CPS（~200 行）— Lowering + Emit + TypedArray

**文件**：`kaubo-cps/src/lib.rs`、`kaubo-ir/src/cps_build.rs`、`kaubo-ir/src/cps_emit.rs`

### 变更

**4a. CPS 指令**（`kaubo-cps/src/lib.rs`）

```rust
// 元组
CpsInstr::NewTuple(usize, Vec<usize>),        // dst, element_regs
CpsInstr::GetField(usize, usize, usize),       // dst, tuple_reg, field_idx (复用 struct 的)

// TypedArray
CpsInstr::NewInt64Array(usize, Vec<usize>),    // dst, element_regs
CpsInstr::NewFloat64Array(usize, Vec<usize>),  // dst, element_regs
```

**4b. CPS Build**（`kaubo-ir/src/cps_build.rs`）

这是改动最大的文件。核心变更：

- `Expr::Call { func, arg }` → 构建 `Call` terminator，单 arg
- `Expr::Tuple(elements)` → `NewTuple` + 逐元素构建
- 函数参数 → 从元组解构：`GetField(tuple_reg, 0)`, `GetField(tuple_reg, 1)`, ...
- `Expr::ListLit(...)` + 类型标注 → 根据 `TypeExpr::List(inner)` 选择：
  - `List<Int64>` → `NewInt64Array`
  - `List<Float64>` → `NewFloat64Array`
  - 无标注 / 混合 → `NewList`（现状）

**4c. CPS Emit**（`kaubo-ir/src/cps_emit.rs`）

- `emit_new_tuple(dst, elements)` → 编码 `NewTuple`
- `emit_new_int64_array(dst, elements)` → 编码 `NewInt64Array`
- `emit_new_float64_array(dst, elements)` → 编码 `NewFloat64Array`

### 验收

- `cargo test -p kaubo-ir` 全绿
- `cargo test -p kaubo-cps` 全绿
- `f(1, 2)` 的 CPS 输出：`NewTuple` + `Call` 用 tuple reg 做 arg
- `[1, 2] : List<Int64>` 生成 `NewInt64Array`

### 风险

- `cps_build.rs` 是 ~3400 行的大文件，`Expr::Call` 的匹配分支遍及全局。需要系统性地 grep 所有 `Call { args` 引用，逐个修改
- `CpsTerminator::Call` 可能也需要改为单 arg（目前 args: Vec<usize>）。如果是，同步改 VM 的 `Call` handler

---

## 步骤 5：VM（~120 行）— 运行时表示 + TypedArray 分派

**文件**：`kaubo-vm/src/execute.rs`、`kaubo-vm/src/gc_heap.rs`

### 变更

**5a. 堆对象**（`execute.rs` + `gc_heap.rs`）

```rust
HeapObj::TupleObj(Vec<usize>),        // 元素为 reg 值（或堆 handle）
HeapObj::Int64Array(Vec<i64>),        // 密集整数
HeapObj::Float64Array(Vec<f64>),      // 密集浮点（不再走 bitcast）
```

**5b. Opcode handler**

- `NewTuple` — 同 `NewStruct` 逻辑，构建 `HeapObj::TupleObj`
- `NewInt64Array` / `NewFloat64Array` — 类似 `NewList`，直接存对应 Vec
- `GetField(tuple_reg, idx)` — 复用 struct 的 `GetField`，对 `TupleObj` 按索引取字段
- `IndexGet` / `IndexSet` 改为按 heap obj 类型分派：
  ```
  List(v)           → 现状
  Int64Array(v)     → 直接 i64 索引
  Float64Array(v)   → 直接 f64 索引（不再 f64::from_bits 往返）
  ```

### 验收

- `cargo test -p kaubo-vm` 全绿
- TypedArray `[1, 2, 3]: List<Int64>` 在 VM 中创建 `Int64Array`
- Tuple `(1, "a")` 在 VM 中创建 `TupleObj`

### 风险

- `IndexGet`/`IndexSet` 对 `Float64Array` 读写时注意 f64 ↔ reg（u64）转换
- `GetField` 目前按 field name 查找（struct），对 Tuple 需要改为按索引，可能需要新增一个 `GetFieldByIndex` variant 或者在 handler 里 match obj 类型

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

## 执行顺序

```
步骤 1: AST       ──→ 步骤 2: Parser   ──→ 步骤 3: Infer
                                              │
步骤 6: Tests+Doc ←── 步骤 5: VM       ←── 步骤 4: CPS
```

每一步的验收标准独立，不依赖后续步骤。步骤 1 改完后全仓库编译报错是预期行为——每个步骤都将自己那层修到编译通过 + 测试绿。
