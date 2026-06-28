# 路线图

目标读者：规划架构和功能工作的维护者。

## 当前进度

| 状态 | Phase | 关键交付 |
|------|-------|---------|
| ✅ 完成 | Phase 1 | 日志 + 死循环防护 |
| ✅ 完成 | Phase 4a | Interface + operator 重载 + dyn Trait |
| 🔶 部分 | Phase 4b | 虚拟 prelude 注入完成，真实 prelude.kb 待做 |
| ▶ 下一步 | **Phase 2b** | DAG 调度器 + Semantic 语义事实层 |
| ⏸ 推迟 | Phase 2a | VM 性能（当前 ~1.5x CPython，可接受） |
| 🔲 规划 | 3a/3b/5a/5b | LSP 完善 / 模块系统 / 泛型 / 效应 |

## 当前痛点

| # | 痛点 | 现象 | 状态 |
|---|------|------|------|
| 1 | VM 运行时性能差 | 比 CPython 慢 ~1.5x，未做优化 | ⏸ 推迟（可接受范围） |
| 2 | 特性支持不够 | 已实现 interface/operator，缺模块系统、泛型、效应 | 🔶 部分解决 |
| 3 | 架构死板 | 编排层硬编码线性路径，无法注入缓存、无法并行 | ▶ 下一步 |
| 4 | 语言服务器勉强能跑 | 每次都要跑完整编译，无语义事实层可停下 | ▶ 依赖 Phase 2b |
| 5 | 没有日志 | 排查问题靠 `eprintln!` 改代码 | ✅ 已解决 |

## 依赖关系总览

```
Phase 1 ─────── ✅ 可观测性 + 死循环防护
  │
  ├── Phase 4a ─ ✅ Interface + operator + dyn Trait（跳过 3b，限定单文件）
  │     └── Phase 4b ─ 🔶 虚拟 prelude 完成，真实 prelude.kb 待做
  │
  ├── Phase 2a ─ ⏸ VM 性能（推迟，当前 ~1.5x CPython 可接受）
  │
  └── Phase 2b ─ ▶ 下一步：DAG 调度器 + Semantic artifact
        │         解锁 LSP 基础 + 模块系统前提
        │
        ├── Phase 3a ─ 🔲 LSP 完善（go-to-def, hover, find-refs）
        │
        └── Phase 3b ─ 🔲 模块系统（import/export, 传递哈希）
              │
              ├── Phase 5a ─ 🔲 泛型（可与 3b 并行）
              │
              └── Phase 5b ─ 🔲 效应系统（可与 5a 并行）
```

## Phase 1：可观测性 + 死循环防护 ✅ 已完成

解决痛点 #5 + while 死循环。

| 交付 | 说明 | 状态 |
|------|------|------|
| `kaubo-log` | `EventHandler` trait + 事件类型 + `emit!` 宏。纯抽象，零平台代码 | ✅ |
| `kaubo-log-handlers` | `ConsoleHandler` + `CompositeHandler` + `KAUBO_LOG` 解析 | ✅ |
| VM 死循环检测 | `LoopExceeded` 错误，backward jump IP 比较，`(func_idx, block_id)` 独立计数 | ✅ |
| Driver 透传 | `RunConfig` 携带 `EventHandler`，沿调用链透传到各 Stage | ✅ |
| CLI 接入 | `--log-level` / `--max-loop-iterations` 参数 | ✅ |
| WASM 接入 | `set_log_level()` 暴露给 JS | ✅ |

**不改**：Driver 架构、VM 执行核心、parser、type inference。

**对终态的兼容**：`RunConfig` → 未来 `BuildContext` 的构造参数。`&dyn EventHandler` 透传模式在 DAG 下不变。详见 [架构](architecture.md)。

### Phase 1 附带修复

| 修复 | 说明 |
|------|------|
| flatten 幽灵前驱 | 已内联 block 残留 terminator 计入 predecessor count，导致后续 block 无法内联 → 物理 IP 乱序 → forward jump 误判为 backward |
| 默认循环上限 | CLI 默认 `u64::MAX`（不限制），Web playground 可设较低值。`* 8 / 10` 改用 `saturating_mul` 防溢出 |
| CLI 输出 | `render_run` 去掉冗余的 `= <result>` 尾行 |
| Benchmark 校验 | 每个 suite 新增 `expected.txt`，runner 在 warmup 前校验输出一致性 |
| Node.js benchmark | 修复 `_fn()` 漏传参 + V8 常量折叠导致 benchmark 数字虚低

## Phase 2a：VM 运行时性能 ⏸ 推迟

解决痛点 #1。前置：Phase 1。

当前 Kaubo 比 CPython 慢 ~1.5x，在没有优化的情况下可接受。DAG 做完后可用日志系统精准 profiling，优化更有针对性。

| 方向 | 说明 |
|------|------|
| Profile 热点 | 用日志系统 benchmark，定位 VM 执行循环中的瓶颈 |
| 指令分派优化 | `match opcode` 的分支预测优化 |
| 寄存器文件 | 访问模式优化 |
| GC heap | RC 操作热点优化 |
| Native call | 调用约定优化 |

**不改**：Driver、parser、type inference、CPS lowering。仅 VM 内部。

## Phase 2b：编排解耦 + 语义事实 ▶ 下一步

解决痛点 #3 + #4 基础。前置：Phase 1。

| 交付 | 说明 |
|------|------|
| DAG 调度器 | `Rule` trait + `ArtifactKey` + `BuildContext`。惰性求值 + 拓扑排序 + 缓存 |
| 语义事实层 | `InferRule` → `ArtifactKey::Semantic`。symbols、scopes、types、member resolution |
| LSP 基础 | `kaubo-language-service` 只求值到 `Semantic`，不触发 Lowering/VM |
| 向后兼容 | `compile_and_run(source)` 保留为 `driver.build(RunResult::key(source))` 的别名 |

**不改**：VM、parser、token/ast/syntax。Driver 从线性函数重构为规则图。

## Phase 3a：LSP 完善

解决痛点 #4。前置：Phase 2b。

| 交付 | 说明 |
|------|------|
| Go-to-definition | 基于 Semantic 符号表 |
| Hover type info | 基于 Semantic 类型事实 |
| Find references | 基于 Semantic 引用图 |
| Completion 增强 | 作用域感知补全 |
| Diagnostics 增强 | 类型错误精确定位 |

**模式**：只消费 `Semantic` artifact，不改编译器核心。

## Phase 3b：模块系统

解决痛点 #2——多文件。前置：Phase 2b。

| 交付 | 说明 |
|------|------|
| `import`/`export` 语义 | 当前 parse-only → 完整语义 |
| 模块图构建 | Driver 解析模块依赖图、路径解析、拓扑排序 |
| 传递闭包哈希 | 模块 Key = 源码哈希 + 所有传递依赖 Key 哈希。自动缓存失效 |
| 跨模块名称解析 | Infer 通过 `driver.build(ImportedSemantic(key))` 获取依赖类型 |
| 循环依赖检测 | 模块图构建或 `build` 时检测并报错 |

**改动层**：AST import/export 语义化、Driver 模块图、Infer 跨模块查询、CPS 多模块链接。~1500 行。

## Phase 4a：Interface ✅ 已完成

解决痛点 #2——动态分派。前置：Phase 1（实际实现跳过了 Phase 3b 的模块系统依赖，限定在单文件范围内）。

| 交付 | 说明 | 状态 |
|------|------|------|
| 接口定义 | `interface Eq { eq: ... }` | ✅ |
| 实现块 | `impl Eq for Point { ... }` | ✅ |
| 胖指针 | `(vtable, data)` → `HeapObj::InterfaceObj` | ✅ |
| CPS `LoadVtable` | 新指令 | ✅ |
| CPS `NewInterfaceObj` | 构造胖指针 | ✅ |
| VM `CallIndirect` | 新 opcode (0x53) | ✅ |
| Operator 关键字 | `operator add:` 方法声明 | ✅ |
| 用户 struct 运算符重载 | `a + b` 走 vtable 分派 | ✅ |
| Interface 类型标注 | `\|x: Speakable\|` dyn Trait 传参 | ✅ |
| 模板字符串 | `` `hello {name}` `` + `{{` `}}` 转义 | ✅ |

**改动层**：AST `InterfaceDef/ImplBlock/MethodSig.operator`、Parser 解析 `interface/impl/operator`、Infer 接口匹配+ vtable 生成、CPS `LoadVtable/NewInterfaceObj/CallIndirect`、VM 3 个新 opcode。实际 ~290 行核心。

**已跳过**：模块系统（跨文件 interface 可见性）、interface 类型变量（`const x: Display = ...`）。

## Phase 4b：内置模块化（部分完成）

解决痛点 #2——编译器去硬编码。

| 交付 | 说明 | 状态 |
|------|------|------|
| 接口层 | `interface Add`、`interface Display`、`interface Eq` ... | ✅ 9 个虚拟接口 |
| 内置类型 impl | `impl Add for Int64` 等 40+ 方法 | ✅ 虚拟注入 |
| `prelude.kb` | 标准库，每个 Kaubo 程序自动导入 | 🔲 仍是硬编码注入 |
| 编译器去硬编码 | 加新类型不再需要改编译器代码 | 🔲 |
| 真实 `.kb` 文件 | prelude 从文件加载 | 🔲 |

**已完成**：Add/Subtract/Multiply/Divide/Modulo/Compare/Display/IntoFloat/IntoInt 共 9 个接口 + 40+ 内置方法已通过 `inject_builtin_interfaces`/`inject_builtin_impls` 虚拟注入。用户的 `impl Add for Vec2` 可直接使用，无需显式声明 `interface Add { ... }`。

**待完成**：真实的 prelude.kb 文件、编译器去硬编码（移除 CPS 层的 `to_string`/`IToS` 等重写）。

## Phase 5a：显式泛型

解决痛点 #2——类型参数。前置：无硬依赖（可与 Phase 4 并行）。

| 交付 | 说明 |
|------|------|
| 泛型 struct | `struct Container<T> { value: T }` |
| 泛型函数 | `const id = \|x: T\| -> T { x }` |
| Monomorphization | CPS 层函数体复制 + 类型替换。VM 无改动 |

**改动层**：AST 泛型参数、Type 参数化、Infer 绑定+实例化、CPS 函数体复制。~1200 行。

## Phase 5b：效应系统

解决痛点 #2——副作用追踪。前置：无硬依赖（可与 Phase 5a 并行）。

| 交付 | 说明 |
|------|------|
| 效应声明 | `effect io` |
| 效应触发 | `do io` |
| 效应处理 | `handle expr with { io => handler }` |
| 行多态类型 | `Type::Arrow` 加 `EffectRow` |
| Suspend 语义化 | CPS Suspend + handler 注册表 + VM 调度 continuation |

**改动层**：AST 新节点、Type 扩展、Infer 效应传播+完备性检查、CPS Suspend 语义化、VM handler dispatch。~2000 行。

## 并行度

```
Phase 1 ────────────────── ✅ 完成（日志 + 死循环）
  │
  ├── Phase 2a ────────── 🔲（VM 性能，可与 2b 并行）
  │
  ├── Phase 4a ────────── ✅ 完成（Interface + operator + dyn Trait）
  │     │                   跳过了 3b 模块系统依赖，限定单文件
  │     └── Phase 4b ──── 🔲 部分完成（虚拟 prelude 注入已完成）
  │                        真实 prelude.kb + 编译器去硬编码待做
  ├── Phase 2b ────────── 🔲（DAG + Semantic）
  │     │
  │     ├── Phase 3a ──── 🔲（LSP，可与 3b 并行）
  │     │
  │     └── Phase 3b ──── 🔲（模块系统）
  │
  ├── Phase 5a ────────── 🔲（泛型，可与 2b/3b 并行）
  │
  └── Phase 5b ────────── 🔲（效应系统，可与 5a 并行）
```

## 各 Phase 成本估算

| Phase | 改动规模 | 风险 | 对用户可见 |
|-------|---------|------|-----------|
| **1** 可观测性 + 死循环 | ✅ 已完成：新建 2 crate，修改 5 crate | 低（不改核心逻辑） | CLI flags, WASM API |
| **2a** VM 性能 | 仅 VM 内部 | 低（不改语义） | 程序跑得更快 |
| **2b** DAG + Semantic | Driver 重构 + Infer 扩展 | 中（编排层改架构） | LSP 变快 |
| **3a** LSP 完善 | 仅 language-service | 低（不改编译器） | IDE 体验提升 |
| **3b** 模块系统 | AST + Driver + Infer + CPS | 中（跨多层） | import/export |
| **4a** Interface | ✅ 已完成：AST + Parser + Infer + CPS + VM + 虚拟 prelude | 中（新 opcode） | interface + operator + dyn Trait |
| **4b** 内置模块化 | 编译器 + 标准库 | 低（删代码为主） | 标准库 |
| **5a** 泛型 | AST + Type + Infer + CPS | 中（Monomorphization） | 泛型语法 |
| **5b** 效应系统 | 全层 | 高（结构性改动） | 效应语法 |

## 优先级原则

1. **先基础设施，后语言特性**：DAG 调度器是 LSP + 模块系统的地基，必须在泛型/效应之前
2. **先架构解耦，后性能优化**：Phase 2a（VM 性能）推迟——当前 ~1.5x CPython 可接受，等 DAG 做完后 profiling 更有针对性
3. **先解耦，后加功能**：Phase 4a 已验证——跳过模块系统直接做 interface，架构灵活后成本低
4. **向后兼容**：每个 Phase 不应破坏已有测试和 CLI/WASM 行为
