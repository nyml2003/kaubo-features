# Kaubo 架构全景图

> v0.1.0 现状 → v0.2.0 目标 · 2026-06-14

---

## 一、Crate 结构 — 4 层

```
                         kaubo-ir (零依赖类型层)
                     ┌─────────────────────────────┐
                     │ Value, Chunk, OpCode, Obj*   │
                     │ VM struct, HIR types         │
                     │ 零外部依赖, no_std compatible  │
                     └──────────┬──────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
  ┌───────▼──────────┐  ┌───────▼──────────┐  ┌───────▼──────────┐
  │ kaubo-compiler   │  │ kaubo-runtime    │  │ kaubo-pipeline   │
  │ ──────────────── │  │ ──────────────── │  │ ──────────────── │
  │ Lexer            │  │ VM 执行循环      │  │ Stage trait      │
  │ Parser           │  │ Stdlib (30 fn)   │  │ Pipeline 组合器  │
  │ TypeChecker      │  │ 二进制读/写      │  │ Observer 观测    │
  │ Codegen          │  │ Platform trait   │  │ Adapter 适配     │
  │ HIR (实验性)     │  │ 运算符/缓存       │  │                  │
  └────────┬─────────┘  └────────┬─────────┘  └──────────────────┘
           │                     │
           └──────────┬──────────┘
                      │
              ┌───────▼──────────┐
              │    kaubo-cli     │  ← 胶水层
              │  编译 + 运行入口  │
              └──────────────────┘

支撑层:
  kaubo-log    ← 日志系统 (支持 no_std / WASM)
  kaubo-config ← 配置数据 (纯数据结构)
  kaubo-vfs    ← 虚拟文件系统 (Memory/Native + 中间件)
```

---

## 二、依赖方向 —— 严格单向

```
kaubo-ir  ← 零依赖
    ↑
    ├── kaubo-compiler  ← 依赖 kaubo-ir + kaubo-pipeline + kaubo-log + kaubo-vfs
    ├── kaubo-runtime   ← 依赖 kaubo-ir + kaubo-config
    └── kaubo-pipeline  ← 零外部依赖

kaubo-cli  ← 依赖 compiler + runtime + ir + log
```

---

## 三、Compiler Line —— 编译管线 (当前实际)

```
┌──────────────────────────────────────────────────────────────────────┐
│                     COMPILER PIPELINE (主路径)                       │
│                                                                      │
│  Step 1  Parser ────────────────────────────────────────────────────│
│  │ IN:  &str              "var x = 1 + 2;"                           │
│  │ OUT: Module (AST)      Module { statements: [VarDecl("x", ...)] }│
│  │ 状态: ✅ 完整, 支持所有语法特性                                    │
│  ▼                                                                   │
│  Step 2  TypeChecker ───────────────────────────────────────────────│
│  │ IN:  &Module (AST)                                                │
│  │ OUT: Module           type-checked, 标记 type_info                │
│  │ 状态: ⚠️ 已实现 (968 行), 但 CLI 默认不启用                        │
│  ▼                                                                   │
│  Step 3  Codegen ───────────────────────────────────────────────────│
│  │ IN:  &Module (AST)                                                │
│  │ OUT: Chunk           { code, constants, method_table, ... }       │
│  │ 状态: ✅ AST→Chunk 完整, 支持所有语言特性                         │
│  ▼                                                                   │
│  Step 4  Binary Writer ─────────────────────────────────────────────│
│  │ IN:  &Chunk                                                       │
│  │ OUT: Vec<u8>          .kaubod (debug) / .kaubor (release)        │
│  │ 状态: ✅ 完整序列化, Section-based binary format                  │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│                     HIR PATH (实验性, 未接入主线)                     │
│                                                                      │
│  AST → HIR Lowering → Optimizer → HIR→Chunk Codegen                 │
│                                                                      │
│  HIR Lowering:  ⚠️ 部分实现 (Binary/VarDecl/While/Return/Print)     │
│  Optimizer:     ⚠️ ConstantFolding + Peephole 已有, 未启用           │
│  HIR→Chunk:     ⚠️ 部分实现                                          │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 四、Runtime Line —— 运行时管线

```
┌──────────────────────────────────────────────────────────────────────┐
│                     RUNTIME PIPELINE                                 │
│                                                                      │
│  Step 1  Binary Reader ──────────────────────────────────────────── │
│  │ IN:  Vec<u8>           .kaubor 文件内容                           │
│  │ OUT: Chunk             反序列化 bytecode                          │
│  │ 状态: ✅ 完整, 支持 Header/Section/StringPool 等全部段           │
│  ▼                                                                   │
│  Step 2  VM Init ────────────────────────────────────────────────── │
│  │ IN:  &Chunk                                                       │
│  │ OUT: VM              shapes, method_table, operator_table         │
│  │ 状态: ✅ 完整                                                     │
│  ▼                                                                   │
│  Step 3  Interpret ──────────────────────────────────────────────── │
│  │ IN:  &mut VM, &Chunk                                              │
│  │ OUT: InterpretResult  Ok / CompileError / RuntimeError            │
│  │ 内部: loop { read_byte → match OpCode → exec → push/pop }        │
│  │ 状态: ✅ 完整, 146 个 OpCode, 内联缓存, 协程支持                 │
│  ▼                                                                   │
│  Step 4  Stdlib ─────────────────────────────────────────────────── │
│  │ 30 个原生函数: math, string, file, coroutine, crypto, time       │
│  │ I/O 通过函数指针 (print_callback) 实现输出注入                    │
│  │ Platform trait 定义在 kaubo-ir::interfaces (已有, 待接入)         │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 五、结构化中间产物

```
编译管线中流转的具体类型：

  Module (AST)
  │ Stmt = VarDecl | If | While | For | Return | Break | Continue
  │        | Pass | Print | Import | Struct | Impl | Block | Expr
  │ Expr = LiteralInt | LiteralFloat | LiteralBool | LiteralString
  │        | LiteralNull | Binary | Unary | VarRef | Lambda | Call
  │        | MemberAccess | IndexAccess | As | Yield | Grouping
  │        | JsonLiteral | StructLiteral | LiteralList

  Chunk
  │ code: Vec<u8>           ← 扁平字节码 (146 OpCode 变体)
  │ constants: Vec<Value>   ← 常量池
  │ lines: Vec<usize>       ← PC→源码行映射
  │ method_table, operator_table, inline_caches

  .kaubod / .kaubor
  │ Section-based binary format
  │ Header { magic: "KAUB", version: 0.1.0, build_mode }
  │ Sections: StringPool | ChunkData | ModuleTable | ShapeTable
  │           | FunctionPool | ExportTable | DebugInfo | SourceMap

  VmState (运行时)
  │ stack: Vec<Value>             ← 操作数栈
  │ frames: Vec<CallFrame>        ← 调用栈
  │ globals: HashMap<String, Value>
  │ open_upvalues: Vec<*mut ObjUpvalue>
  │ shapes: HashMap<u16, *const ObjShape>
  │ inline_caches: Vec<InlineCacheEntry>
```

---

## 六、可观测扩展点

```
  全部 7 个扩展点：

  ┌────────────────────┬──────────────────┬───────────────────────┐
  │ 扩展点              │ trait             │ 位置                   │
  ├────────────────────┼──────────────────┼───────────────────────┤
  │ 新优化 pass         │ OptimizationPass  │ kaubo-compiler::hir   │
  │ 新平台              │ Platform          │ kaubo-ir::interfaces  │
  │ 新编译阶段          │ Stage<In, Out>    │ kaubo-pipeline        │
  │ 新部署目标          │ 无 (独立 binary)  │ CLI / WASM / embed    │
  │ 新内存策略          │ Allocator         │ kaubo-ir::interfaces  │
  │ 新源码来源          │ SourceRepo        │ kaubo-ir::interfaces  │
  │ 新日志后端          │ LogSink           │ kaubo-log             │
  └────────────────────┴──────────────────┴───────────────────────┘
```

---

## 七、当前 vs 目标差距

```
当前 (v0.1.0):
  源码 ──[Parser + Codegen]──→ Chunk ──[VM]──→ Result
                │
          [Binary Writer]
                ↓
          .kaubod/.kaubor

  ✅ Crate 已拆分: ir / compiler / runtime / pipeline
  ✅ 二进制格式完整
  ✅ Stdlib 30 个函数
  ⚠️ TypeChecker 已实现但未强制
  ⚠️ HIR/Optimizer 框架已有但未接主线
  ⚠️ 53 处 panic (expect/unwrap) 待消除

目标 (v0.2.0):
  Phase 1 (🔴): 0 panic → 结构化 RuntimeError
  Phase 2:      Reference Counting → 0 Box::into_raw
  Phase 3:      TypeChecker 接线 (激活已有 968 行)
  Phase 4:      HIR lowering + HIR→Chunk codegen (贯通)
  Phase 5:      Platform trait 注入 + WASM Playground

  保留框架但 v0.2 不实现:
    - 优化 pass (OptimizationPass trait + 2 demo 已有)
    - C ABI / FFI
    - 泛型
```

---

## 八、端 × 模块矩阵 (现状)

```
│  Target        │ ir │compiler│runtime│ vfs │kaubo-log│
│────────────────│────│────────│───────│─────│─────────│
│ kaubo-cli      │ ✅ │  ✅    │  ✅   │ ✅  │  ✅     │
│ (wasm 端待实现) │ ✅ │   -    │   -   │  -  │  ✅     │
```

---

*v0.1.0 现状 → v0.2.0 目标 · 2026-06-14*
