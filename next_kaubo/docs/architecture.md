# Kaubo 架构全景图

> v0.1.0 → v0.2.0 目标架构 · 2026-06-12

---

## 一、水平 × 竖直 —— 2D 网格总览

```
                          ← 水平: crate 边界 →

          kaubo-ir    kaubo-compiler   kaubo-runtime   kaubo-platform
        ┌───────────┬────────────────┬───────────────┬────────────────┐
 模型    │           │                │               │                │
(models) │ Value     │ Token, AST     │ (复用 ir)     │ (无)           │
        │ Chunk     │ HirModule      │               │                │
        │ OpCode    │ TypedModule    │               │                │
        │ Obj*      │ TypeExpr       │               │                │
        │           │ SourceMap      │               │                │
        ├───────────┼────────────────┼───────────────┼────────────────┤
 服务    │           │                │               │                │
(services)│ (无)     │ lex()          │ interpret()   │ (无)           │
        │           │ parse()        │ run()         │                │
        │           │ lower()        │ operators     │                │
        │           │ optimize()     │ stdlib        │                │
        │           │ codegen()      │ coroutine     │                │
        │           │ type_check()   │               │                │
        │           │ HIR→Chunk      │               │                │
        ├───────────┼────────────────┼───────────────┼────────────────┤
 平台适配│           │                │               │                │
(infra)  │ (无)     │ binary_writer  │ binary_reader │ Platform trait │
        │           │                │ Allocator    │ (impl per OS)  │
        │           │                │               │                │
        └───────────┴────────────────┴───────────────┴────────────────┘
                                │
                    ┌───────────┴──────────┐
                    │    可观测切面          │ ← 所有 cell 都可挂载
                    │  PipelineObserver    │
                    │  StageSnapshot<T>    │
                    │  CoverageCollector   │
                    └──────────────────────┘
```

### 每格的三坐标定位

```
方块位置: crate.crate 内部层级.功能名
                                      ↓
  例: kaubo-compiler / services / codegen()
      ─────────────────────────────────────

  源码位置:
    crates/kaubo-compiler/src/services/codegen.rs
```

### 各层职责

```
✅ models/     可以: struct Foo { x: i32 }       纯数据，零行为，零依赖
               不能: fn do_something()

✅ services/   可以: fn compile(ast) -> Chunk     纯函数，操作 models
               可以: fn run(vm) -> Result
               可以: 持有 Arc<dyn Platform>
               不能: println!(), std::fs::read()
               不能: Box::into_raw()

✅ infra/      可以: 实现 Platform trait
               可以: 实现 Allocator trait
               可以: read / write / env / now
               可以: Box::into_raw / Box::from_raw
```

---

## 二、依赖方向 —— 严格单向

```
【platform 层】 trait 定义在 crates 内部，impl 在独立 crate
                      ↑ trait object 注入

  kaubo-ir ────────────────────────────────────────────
  │ Value, Chunk, OpCode, Obj*, InterpretResult         │
  │ 零外部依赖，no_std compatible                        │
  └──────────────┬──────────────────┬───────────────────┘
                 │                  │
  ┌──────────────▼────────┐  ┌──────▼──────────┐  ┌──────────────────┐
  │ kaubo-compiler        │  │ kaubo-runtime   │  │ kaubo-platform-* │
  │ ───────────────────── │  │ ─────────────── │  │ ──────────────── │
  │ models: AST, HIR      │  │ (复用 ir)       │  │ native / wasm    │
  │ services:             │  │ services:       │  │ 不依赖 kaubo     │
  │  compile(lower→opt→   │  │  interpret()    │  └──────────────────┘
  │   codegen→write)      │  │  stdlib         │
  │ 纯函数，不需要 Platform │  │ 需要 Platform   │
  └───────┬───────────────┘  └──────┬──────────┘
          │                         │
          └─────────┬───────────────┘
                    ▼
  ┌─────────────────────────────────────────────────────┐
  │  Targets / 部署目标                                  │
  │                                                     │
  │  kaubo-cli    ← compiler + runtime + native platform │
  │  kaubo-wasm   ← compiler + runtime + wasm platform   │
  │  kaubo-bevy   ← runtime only + embed .kaubor         │
  │  kaubo-ffi    ← runtime only + C ABI                 │
  └─────────────────────────────────────────────────────┘
```

---

## 三、Compiler Line —— 每一步 IN / OUT

```
┌──────────────────────────────────────────────────────────────────────┐
│                     COMPILER PIPELINE                                │
│                                                                      │
│  Step 1  Lexer ───────────────────────────────────────────────────  │
│  │ IN:  &str           "var x = 1 + 2;"                              │
│  │ OUT: TokenStream    { tokens, spans }                             │
│  │ 状态: ✅                                                          │
│  ▼                                                                   │
│  Step 2  Parser ──────────────────────────────────────────────────  │
│  │ IN:  TokenStream                                                  │
│  │ OUT: Module (AST)   Module { statements: [VarDecl("x", ...)] }    │
│  │ 状态: ✅                                                          │
│  ▼                                                                   │
│  Step 3  Type Checker ────────────────────────────────────────────  │
│  │ IN:  &Module (AST)                                                │
│  │ OUT: TypedModule    每个节点带类型标注 | 或 TypeError              │
│  │ 状态: ⚠️ 已实现，待接入主线                                        │
│  ▼                                                                   │
│  Step 4  HIR Lowering ────────────────────────────────────────────  │
│  │ IN:  TypedModule                                                  │
│  │ OUT: HirModule      { functions, blocks, instrs, constants }      │
│  │ 状态: 🆕 框架已有，lowering pass 待实现                            │
│  │ 值:   基本块 + 三地址码 (add %0, %1, %2)，Phi 节点                │
│  ▼                                                                   │
│  Step 5  Optimizer ───────────────────────────────────────────────  │
│  │ IN:  HirModule                                                    │
│  │ OUT: HirModule      常量折叠 / dead code / peephole / 内联        │
│  │ 状态: 🆕 OptimizationPass trait + ConstantFold + Peephole 已有   │
│  │ 执行: pipeline.run(hir, [ConstantFolding, Peephole, ...])         │
│  ▼                                                                   │
│  Step 6  Codegen ─────────────────────────────────────────────────  │
│  │ IN:  HirModule (当前直接 AST → Chunk)                              │
│  │ OUT: Chunk          { code: Vec<u8>, constants, source_map }      │
│  │ 状态: ✅ AST→Chunk 完整，HIR→Chunk 待实现                         │
│  ▼                                                                   │
│  Step 7  Binary Writer ─────────────────────────────────────────── │
│  │ IN:  &Chunk                                                       │
│  │ OUT: Vec<u8>        .kaubod (debug) / .kaubor (release)          │
│  │ 状态: ✅, CLI: --emit-binary / --production                       │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 四、Runtime Line —— 每一步 IN / OUT

```
┌──────────────────────────────────────────────────────────────────────┐
│                     RUNTIME PIPELINE                                 │
│                                                                      │
│  Step 1  Binary Reader ──────────────────────────────────────────── │
│  │ IN:  Vec<u8>          .kaubor (文件 / embed_bytes! / wasm)       │
│  │ OUT: Chunk + SourceMap 反序列化                                   │
│  │ 状态: ✅                                                          │
│  ▼                                                                   │
│  Step 2  VM Init ────────────────────────────────────────────────── │
│  │ IN:  &Chunk                                                       │
│  │ OUT: VM              method_table, operator_table, shapes        │
│  │ 状态: ✅                                                          │
│  ▼                                                                   │
│  Step 3  Interpret ──────────────────────────────────────────────── │
│  │ IN:  &mut VM, &Chunk                                              │
│  │ OUT: InterpretResult Ok / CompileError / RuntimeError            │
│  │ 内部: looper { read_byte → match OpCode → exec → push/pop }      │
│  │ 状态: ✅, 内联缓存 Level 2, fast path 算术                       │
│  ▼                                                                   │
│  Step 4  Stdlib ─────────────────────────────────────────────────── │
│  │ IN:  函数签名 + args (贯穿 Step 3)                                │
│  │ OUT: Value                                                        │
│  │ 通过 Platform trait:                                              │
│  │   print(msg)     ──→ Platform::stdout_write ──→ console/callback  │
│  │   read_file(p)   ──→ Platform::read_file ──→ fs / wasm / memory  │
│  │   now()          ──→ Platform::now_secs ──→ SystemTime / Date    │
│  │   env(name)      ──→ Platform::env_var ──→ std::env / embed map  │
│  │ 状态: ⚠️ Platform trait 已定义，待注入到 stdlib                   │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 五、结构化中间产物

```
编译管线中流转的具体类型（非 enum，每步各有独立 struct 定义）：

  TokenStream ─────────────────────────────────────────────────────
  │ tokens: Vec<Token>
  │ Token { kind: TokenKind, span: Span { start, end }, text }
  │ 标注: 每个 token 带精确的 Position(line, col, offset)

  Module (AST) ────────────────────────────────────────────────────
  │ Stmt = VarDecl | If | While | For | Return | Break | Continue
  │        | Pass | Print | Import | Struct | Impl | Block | Expr
  │ Expr = LiteralInt | LiteralFloat | LiteralBool | LiteralString
  │        | LiteralNull | Binary | Unary | VarRef | Lambda | Call
  │        | MemberAccess | IndexAccess | As | Yield | Grouping
  │ 标注: 每个节点带 Span

  TypedModule ─────────────────────────────────────────────────────
  │ = Module + 类型标注 (TypedStmt, TypedExpr)
  │ 每个 Expr 节点带 type_info: Option<TypeExpr>
  │ struct_types: HashMap<name → Vec<(field, type)>>

  HirModule ───────────────────────────────────────────────────────
  │ functions: Vec<HirFunction> { name, blocks, entry, locals }
  │ HirBlock { id, instrs: Vec<HirInstr>, term: HirTerminator }
  │ HirInstr = Binary | Unary | LoadConst | Move | Call | Ret
  │ HirTerminator = Jump | Branch | Return | End
  │ Operand = Temp(usize) | Local(String) | Const(usize)
  │ Phi 节点: 支持 SSA 未来扩展

  Chunk ───────────────────────────────────────────────────────────
  │ code: Vec<u8>           ← 扁平字节码
  │ constants: Vec<Value>   ← 常量池
  │ source_map: SourceMap   ← IP→Span 映射
  │ method_table, operator_table, inline_caches

  .kaubor ─────────────────────────────────────────────────────────
  │ Section-based binary format
  │ Header { magic, version, build_mode }
  │ Sections: StringPool | ChunkData | ModuleTable | DebugInfo
  │ DebugInfo { SourceMap, LineTable }

  VmState (运行时) ────────────────────────────────────────────────
  │ stack: Vec<Value>
  │ frames: Vec<CallFrame> { closure, ip, locals }
  │ globals: HashMap<String, Value>
  │ ip: usize
```

---

## 六、可观测扩展点

```
  全部 7 个扩展点，遵循「加文件不改老代码」:

  ┌────────────────────┬──────────────────┬───────────────────────┐
  │ 扩展点              │ trait             │ 新实现只需写          │
  ├────────────────────┼──────────────────┼───────────────────────┤
  │ 新优化 pass         │ OptimizationPass  │ 1 文件 + 注册 1 行    │
  │ 新观测工具          │ PipelineObserver  │ 1 文件               │
  │ 新平台              │ Platform          │ 1 文件               │
  │ 新编译阶段          │ Pass (已有)       │ 1 文件 + 注册 1 行    │
  │ 新部署目标          │ 无 (独立 binary)  │ 1 文件 (main.rs)     │
  │ 新内存策略          │ Allocator         │ 1 文件               │
  │ 新源码来源          │ SourceRepo        │ 1 文件               │
  └────────────────────┴──────────────────┴───────────────────────┘

  新实现和旧实现零耦合。Pipeline 通过 Option<Arc<dyn Trait>> 注入。
```

### PipelineObserver —— 贯穿编译/运行时的观测能力

```rust
trait PipelineObserver {
    fn on_tokens(&self, snapshot: &StageSnapshot<TokenStream>) {}    // Lexer 完成
    fn on_ast(&self, snapshot: &StageSnapshot<Module>) {}             // AST 就绪
    fn on_typed_ast(&self, snapshot: &StageSnapshot<TypedModule>) {}  // 类型检查完成
    fn on_hir(&self, snapshot: &StageSnapshot<HirModule>) {}          // HIR 就绪
    fn on_hir_optimized(&self, snapshot: &StageSnapshot<HirModule>, pass: &str) {} // 优化步骤
    fn on_chunk(&self, snapshot: &StageSnapshot<Chunk>) {}            // 字节码就绪
    fn on_vm_step(&self, snapshot: &VmStepSnapshot) {}               // VM 每条指令
    fn on_coverage_hit(&self, ip: usize, loc: &SourceLocation) {}    // 覆盖率
}
// 所有回调有 default {}，观测工具只覆写关心的阶段
```

---

## 七、端 × 模块矩阵

```
│  Target        │ ir │compiler│runtime│ vfs │kaubo-log│Platform impl│
│────────────────│────│────────│───────│─────│─────────│─────────────│
│ kaubo-cli      │ ✅ │  ✅    │  ✅   │ ✅  │  ✅     │ native      │
│ kaubo-wasm     │ ✅ │  ✅    │  ✅   │ ✅  │  ✅     │ wasm        │
│ kaubo-bevy     │ ✅ │  ❌    │  ✅   │ ❌  │  ✅     │ bevy VFS    │
│ kaubo-godot    │ ✅ │  ❌    │  ✅   │ ❌  │  ✅     │ godot FS    │
│ kaubo-mobile   │ ✅ │  ❌    │  ✅   │ ❌  │  ✅     │ mobile FS   │
│ kaubo-edge     │ ✅ │  ❌    │  ✅   │ ❌  │  ✅     │ wasm+fetch  │
│ kaubo-srv      │ ✅ │  ❌    │  ✅   │ ✅  │  ✅     │ native+axum │
│ kaubo-ffi      │ ✅ │  ❌    │  ✅   │ ❌  │  ❌     │ caller 提供 │
│────────────────│────│────────│───────│─────│─────────│─────────────│
│ compile size   │~80K│ ~200K   │ ~120K │ ~30K│ ~20K   │ 每端不同    │
│ wasm (最小)    │~40K│ ❌      │ ~60K  │ ❌  │ ❌      │ wasm        │
```

---

## 八、编译产物流动

```
    开发时                                     部署时
    ──────                                     ──────

    src.kaubo                                   ┌──────────────┐
        │                                       │  游戏引擎     │
        ▼                                       │              │
   [kaubo-compiler]       embed 场景            │ include_bytes!│
        │                 只需一次              │ (".kaubor")   │
        ▼                       │               │              │
    out.kaubor ◄────────────────┘               └──────┬───────┘
        │                                             │
        │                                    ┌────────▼───────┐
        │                                    │  kaubo-runtime  │
        │  runtime 场景                       │  load & execute │
        │  直接加载                           └──────┬─────────┘
        │                       │                     │
        ▼                       │                     ▼
   ┌──────────────┐             │              ┌──────────────┐
   │ kaubo-runtime│             │              │ 游戏内效果    │
   │ load(.kaubor)│             │              │ 脚本逻辑运行   │
   │ execute()    │             │              └──────────────┘
   └──────┬───────┘             │
          │                     │
          ▼                     ▼
     ┌─────────┐          ┌──────────┐
     │ 终端输出 │          │ 文件大小  │
     │ CLI 结果 │          │ 极小 (~K) │
     └─────────┘          └──────────┘
```

---

## 九、当前 vs 目标差距

```
当前 (v0.1.0):
  src.kaubo ──[Lexer+Parser]──→ AST ──[Codegen]──→ Chunk ──[VM]──→ Result
                                                      │
                                                [Binary Writer]
                                                      ↓
                                                  .kaubod/.kaubor

  monolithic kaubo-orchestrator crate
  TypeChecker 已实现但未接主线
  HIR/Optimizer 框架已有但未接主线

目标 (v0.2.0):
  Phase 1 (🔴): 0 panic → 结构化 RuntimeError
  Phase 2:       Reference Counting → 0 Box::into_raw
  Phase 3:       crate split → ir / compiler / runtime
  Phase 4:       HIR lowering + HIR → Chunk codegen
  Phase 5:       TypeChecker 接线
  Phase 6:       Platform trait 注入 + WASM Playground

  保留框架但 v0.2 不实现:
    - 优化 pass (OptimizationPass trait + 2 demo 已有)
    - Coverage (PipelineObserver trait 设计)
    - C ABI / FFI
```
  crates/kaubo-compiler    ← 独立编译产物
  crates/kaubo-runtime     ← 独立编译产物
```

---

*v0.2.0 目标架构 · 2026-06-12*
