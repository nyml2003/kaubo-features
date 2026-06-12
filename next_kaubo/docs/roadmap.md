# Kaubo Roadmap

> v0.1.0 → v0.2.0 · 2026-06-12

---

## v0.1.0 — MVP ✅

```
428 tests, 0 failures
git tag: v0.1.0

  Lexer → Parser → Codegen → Chunk → VM
  break/continue/pass
  struct/impl/operator overloading
  closures + upvalue capture
  coroutines (yield/resume)
  29 stdlib functions
  binary output (.kaubod/.kaubor)
  --emit-binary, --production
```

---

## v0.2.0 — 跨端安全 + 架构拆分

### Phase 1: 消除 panic → 结构化错误

**风险: 🔴 HIGH — 53 处 in hot path**

| 任务 | 量 |
|------|-----|
| `execution.rs`: 53 处 `.expect("Stack underflow")` → `?`传播 Result | ~80 行 |
| `chunk.rs`: 3 处 panic → `Result<u8, ChunkError>` | ~20 行 |
| `stack.rs`: `peek()` → `Option<Value>` | ~15 行 |
| `RuntimeError` 结构化: 激活已有 `RuntimeError` enum 替代 `String` | ~30 行 |
| 全量回归测试 (428 tests) | — |

**产出:** 0 panic, 0 unwrap。所有错误可定位到 IP。跨端安全。

---

### Phase 2: 轻量 GC (Reference Counting)

**风险: 🟡 MEDIUM — 147 push/pop + 24 alloc 点, 但每处只改 1 行**

| 任务 | 量 |
|------|-----|
| `object.rs`: 12 个 Obj\* 各加 `ref_count: Cell<usize>` | ~20 行 |
| `value.rs`: `Value::retain()` / `Value::release()` | ~30 行 |
| `execution.rs`: 147 处 push/pop → retain/release | ~30 行 |
| 帧销毁: locals 逐一 release | ~10 行 |
| `stdlib` + `builtin_methods`: 24 处 `Box::into_raw` → RC | ~24 处 |
| `Allocator` trait 预留 (当前 NativeAllocator) | 已有 |

**产出:** 无 manual 内存管理。C ABI / WASM 场景内存安全。

---

### Phase 3: Crate Split

**风险: 🟡 MEDIUM — 1 个跨层 import 要重构 + 7 个测试搬家**

```
kaubo-ir ──────────── 新建 (vm/core/ → crates/kaubo-ir)
kaubo-compiler ────── 新建 (pipeline/ + domain/ + stages/)
kaubo-runtime ─────── 新建 (vm/runtime/ + binary reader)
kaubo-orchestrator ── 删除
kaubo-cli ─────────── 重写为胶水
```

| 任务 | 量 |
|------|-----|
| `kaubo-ir`: 移动 Value/Chunk/OpCode/Object (15 文件) | ~15 files |
| `kaubo-compiler`: 移动 pipeline + domain + stages (25 文件) | ~25 files |
| `kaubo-runtime`: 移动 vm/runtime + binary reader (15 文件) | ~15 files |
| 修复 `builtin_methods.rs:16` 跨层 import (core→runtime) | 1 行重构 |
| 7 个集成测试搬家 (同时引用 compiler + runtime) | 7 tests |
| 更新 kaubo-cli 依赖 | ~50 行 |

**产出:** `libkaubo_ir.a`, `libkaubo_compiler.a`, `libkaubo_runtime.a`

---

### Phase 4: HIR Lowering + Codegen

**风险: 🟡 MEDIUM — 基本块构建正确性 + HIR→Chunk 等价性**

**目标:** AST → HIR → Chunk 全线贯通 + desugar break/continue

| 任务 | 量 |
|------|-----|
| HIR Lowering: AST → HirBlock + Jump/Branch | ~150 行 |
| break/continue desugar: continuation = BlockId | ~30 行 |
| 加 `Loop` terminator to HirTerminator | ~5 行 |
| HIR → Chunk codegen (替代直接 AST → Chunk) | ~300 行 |
| 等价性验证: 428 tests 新旧路径 Chunk bytes 比较 | — |
| 保留旧 AST→Chunk 作为 fallback (feature flag) | 1 flag |

**产出:** 跳转目标编译时已知 (不是运行时 patch)。去掉 `break_jumps: Vec<usize>`。

**不做 (v0.2 内):**
- SSA 构建 + Phi 节点
- 优化 pass (留 `OptimizationPass` trait + `ConstantFolding`/`Peephole` 框架)
- 数据流分析
- 循环不变量外提

---

### Phase 5: TypeChecker 接线

**风险: 🟡 MEDIUM — 接线后 428 tests 可能报新类型错误**

**目标:** 激活现有 TypeChecker (968 行已实现)，让它进入编译管线

| 任务 | 量 |
|------|-----|
| `check_module()` 方法 | ~15 行 |
| SemanticPass: 接为 Pass (Parser → Checker → Codegen) | ~50 行 |
| 修复: check_binary Slash → 永远 float (和 VM 一致) | ~5 行 |
| 修复: as 类型检查分支 | ~10 行 |
| 修复: VarDecl codegen 读 `type_annotation` 字段 | ~5 行 |
| 修复: is_compatible null → any | ~3 行 |
| TypeError → CompileError 映射 | ~15 行 |

**产出:** `var x: int = "hello"` → 编译期 TypeError

**不做 (v0.2 内):**
- 泛型 `struct Box[T]`
- 泛型 lambda
- 子类型/协变/逆变
- List 运行时类型擦除修复

---

### Phase 6: Platform 注入 + WASM

**风险: 🟢 LOW — 7 个函数注入, Platform trait 已有**

| 任务 | 量 |
|------|-----|
| kaubo-platform-native crate | ~50 行 |
| Stdlib 重构: 通过 `Arc<dyn Platform>` 做 I/O | ~100 行 |
| NativeFn 签名: `fn(&[Value])` → `fn(&[Value], &dyn Platform)` | 30+ 注册处 |
| kaubo-platform-wasm crate | ~80 行 |
| kaubo-wasm target: compiler.wasm + runtime.wasm | ~100 行 |
| Web Playground HTML/JS | ~100 行 |

**产出:** Web Playground。`kaubo_compiler.wasm` + `kaubo_runtime.wasm`

---

## 不做 (留框架, 暂不实现)

| 项 | 原因 | 保留什么 |
|----|------|---------|
| **CPS 变换** | 性能下降 10x+, 用 HIR desugar 替代 | — |
| **优化 pass** | 先保证正确性, 再谈优化 | `OptimizationPass` trait + `ConstantFolding` + `Peephole` |
| **覆盖率** | 先做跨端, 再做观测 | `PipelineObserver` trait |
| **C ABI / FFI** | 先做 WASM 端验证架构 | — |
| **泛型** | 复杂度高 | — |

---

## 风险矩阵

```
Phase 1: Panic ❌  🔴  53 expect in hot loop → 最危险
Phase 2: RC          🟡  147 insert points but 1-line each
Phase 3: Split       🟡  1 cross-layer dep to break
Phase 4: HIR         🟡  correctness of block building
Phase 5: TypeChecker 🟡  may surface latent type errors
Phase 6: Platform    🟢  nearly mechanical
```

---

## 每个 Phase 结束的可验证物

| Phase | 验证方式 |
|-------|---------|
| 1 | `RUST_BACKTRACE=1 kaubo bad.kaubo` → 不 panic, 返回 RuntimeError(ip=X) |
| 2 | valgrind / miri 验证 0 use-after-free |
| 3 | `cargo test -p kaubo-ir && cargo test -p kaubo-compiler && cargo test -p kaubo-runtime` |
| 4 | `diff <(kaubo --emit-bytecode-old src.kaubo) <(kaubo --emit-bytecode-new src.kaubo)` → 等价 |
| 5 | `var x: int = "hello";` → 编译期 TypeError |
| 6 | `curl https://kaubo.dev/playground` → 浏览器可运行 |

---

*2026-06-12*
