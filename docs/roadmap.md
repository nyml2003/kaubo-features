# 路线图

| 状态 | Phase | 关键交付 |
|------|-------|---------|
| ▶ 下一步 | **Phase 1** | 元组 + TypedArray（函数调用语义基础，同构数组运行时优化） |
| 🔶 部分 | Phase 2 | 虚拟 prelude 注入完成，prelude.kb + 编译器去硬编码待做 |
| 🔲 规划 | Phase 3 | LSP 编排层独立化（LspCoordinator + go-to-def/hover） |
| 🔲 规划 | Phase 4 | 显式泛型 |
| 🔲 规划 | Phase 5 | 效应系统 |
| ⏸ 推迟 | Phase 6 | VM 性能（当前 ~1.5x CPython，可接受） |

---

## Phase 1：元组 + TypedArray ▶ 下一步

**核心模型**：函数调用 = 标识符 + 元组。所有函数是单参数函数，`|params|` 对传入元组做模式解构。TypedArray 利用类型标注在运行时选择密集同构表示。

### 元组：值语法与类型语法对称

| 概念 | 值语法 | 类型语法 |
|------|--------|---------|
| 空元组 / unit | `()` | `()` |
| 单元素元组 | `(1,)` | `(Int64,)` |
| 二元组 | `(1, "a")` | `(Int64, String)` |
| 函数类型 | — | `(Int64, Int64) -> Int64` |

### LL(1) 解析规则

```
( expr ,     → 元组（有逗号）
( expr )     → 分组（无逗号），AST 折叠为 expr
( )          → 空元组 / unit
```

### TypedArray

根据类型标注在 CPS 层选择运行时表示：

```
List<Int64>  [1, 2, 3]  →  NewInt64Array   →  HeapObj::Int64Array(Vec<i64>)
List<Float64> [1.0, 2.0] →  NewFloat64Array →  HeapObj::Float64Array(Vec<f64>)
无标注/混合  [1, "a"]    →  NewList         →  HeapObj::List(Vec<i64>)  (现状)
```

列表语法 `[1, 2, 3]` 不变，parser 不改。

### 各层改动

| 层 | 元组 | TypedArray |
|----|------|------------|
| AST | `Expr::Tuple`、`TypeExpr::Tuple` | — |
| Parser | 括号内逗号判定；`Call` 改为单 arg | — |
| Infer | 元组类型推断；函数参数解构元组 | 列表类型标注 → CPS hint |
| CPS | `Call` 单 arg；`GetField(tuple, idx)` | `NewInt64Array` / `NewFloat64Array` |
| VM | `HeapObj::TupleObj`；`GetField` 复用 struct | `Int64Array` / `Float64Array` + `IndexGet`/`IndexSet` 按类型分派 |

**合计**：~700 行（元组 ~500 + TypedArray ~200）。元组是泛型前置——做完后 `Result<T, E>` 的类型系统基础就位。

---

## Phase 2：内置模块化（部分完成）

| 已完成 | 待做 |
|--------|------|
| 9 个虚拟接口注入（Add/Subtract/…/IntoInt） | 真实 `prelude.kb` 文件 |
| 40+ 内置方法 impl（`impl Add for Int64` 等） | 编译器去硬编码（移除 CPS 层 `to_string`/`IToS` 重写） |
| 用户可直接 `impl Add for Vec2` | 加新类型不再需要改编译器代码 |

---

## Phase 3：LSP 编排层独立化 🔲

前置：Phase 1（元组改变函数调用 AST，LSP 应基于新语义）。

| 交付 | 说明 |
|------|------|
| LspCoordinator | 独立编排层：Frontend→Semantic，不到 CPS/VM |
| Go-to-definition | 基于 `SemanticArtifact.references` |
| Hover type info | 基于 `SemanticArtifact.symbols` |
| Completion 增强 | `SemanticArtifact` + 原有 token 补全 |
| Semantic tokens | AST 节点类型 + 原有 token fallback |
| WASM 适配 | hover/semantic_tokens/complete 改用 LspCoordinator |

**改动层**：仅 `kaubo-language-service`，~260 行。不改编译器核心。

---

## Phase 4：显式泛型 🔲

| 交付 | 说明 |
|------|------|
| 泛型 struct | `struct Container<T> { value: T }` |
| 泛型函数 | `const id = \|x: T\| -> T { x }` |
| Monomorphization | CPS 层函数体复制 + 类型替换，VM 无改动 |

`<>` 定界（类型标注上下文 `:` 后无歧义）。HM 推断消除显式类型参数需求。

**改动层**：AST 泛型参数、Type 参数化、Infer 绑定+实例化、CPS 函数体复制。~1200 行。

---

## Phase 5：效应系统 🔲

| 交付 | 说明 |
|------|------|
| 效应声明 | `effect io` |
| 效应触发 | `do io` |
| 效应处理 | `handle expr with { io => handler }` |
| 行多态类型 | `Type::Arrow` 加 `EffectRow` |
| Suspend 语义化 | CPS Suspend + handler 注册表 + VM 调度 continuation |

**改动层**：全层。~2000 行。

---

## Phase 6：VM 运行时性能 ⏸ 推迟

当前 Kaubo 比 CPython 慢 ~1.5x，可接受。

| 方向 | 说明 |
|------|------|
| Profile 热点 | 日志系统 benchmark，定位 VM 执行循环瓶颈 |
| 指令分派 | `match opcode` 分支预测优化 |
| 寄存器文件 | 访问模式优化 |
| GC heap | RC 操作热点 |
| Native call | 调用约定优化 |

仅改 VM 内部。

---

## 依赖关系

```
元组 + TypedArray (Phase 1) ─── 泛型 (Phase 4) ─── 效应 (Phase 5)
        │
        └── LSP (Phase 3)

Interface ─── prelude.kb (Phase 2)

独立: VM 性能 (Phase 6)
```

## 成本

| Phase | 改动范围 | 风险 |
|-------|---------|------|
| 1 元组 + TypedArray | 全管线（AST→VM） | 中（破坏性变更：函数调用 AST） |
| 2 prelude.kb | 编译器 + 标准库 | 低（删代码为主） |
| 3 LSP | 仅 language-service | 低 |
| 4 泛型 | AST + Type + Infer + CPS | 中（Monomorphization） |
| 5 效应 | 全层 | 高（结构性改动） |
| 6 VM 性能 | 仅 VM 内部 | 低 |
