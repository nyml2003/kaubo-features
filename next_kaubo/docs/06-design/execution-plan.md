# Kaubo v2 执行计划

> 基于 [`kaubo-v2.md`](./kaubo-v2.md) 设计文档制定
> 状态: 讨论稿

---

## 总览

| 阶段 | 耗时 | 核心产出 | 依赖 |
|------|------|---------|------|
| Phase 0 | 2天 | 死代码清零，目录归一 | — |
| Phase 1 | 1周 | 新 parser + AST | Phase 0 |
| Phase 2 | 1周 | HM 类型推断引擎 | Phase 1 |
| Phase 3 | 1周 | CPS blocks 降级 | Phase 2 |
| Phase 4 | 1周 | 优化 + 32-bit 编码 | Phase 3 |
| Phase 5 | 1.5周 | 寄存器 VM | Phase 4 |
| Phase 6 | 1周 | async/await + .kauboi | Phase 5 |
| Phase 7 | 1周 | .kaubop 模块系统 | Phase 5 |
| Phase 8 | 1周 | WASM + VSCode + 文档 | Phase 5 |
| **总计** | **~8 周** | | |

Phase 0-5 必须串行。Phase 6/7/8 可在 Phase 5 完成后并行。

---

## Phase 0: 清理（2天）

**目标：** 删除所有死代码，统一目录结构，为 v2 腾出干净空间。

### 0.1 删除死 crate

| # | 任务 | 操作 | 验证 |
|---|------|------|------|
| 1 | 删除 `kaubo-config` | `rm -rf kaubo-config/`，从 workspace `Cargo.toml` 移除 member，从 `kaubo-ir/Cargo.toml` 移除依赖 | `cargo check --workspace` 通过 |
| 2 | 删除 `kaubo-pipeline` | `rm -rf crates/kaubo-pipeline/`，从 workspace `Cargo.toml` 移除 member | `cargo check --workspace` 通过 |
| 3 | 清理 stages 中的 pipeline 引用 | 删除 `lex.rs`/`parse.rs`/`check.rs`/`codegen.rs` 四文件中 `impl kaubo_pipeline::Stage` 块和 `use kaubo_pipeline` 导入 | 编译通过 |

### 0.2 删除死 HIR 代码

| # | 任务 | 操作 |
|---|------|------|
| 4 | 删除 `crates/kaubo-compiler/src/hir/` | 整个目录 (4 文件, 807 行) |
| 5 | 删除 `pub mod hir;` | `crates/kaubo-compiler/src/lib.rs` |
| 6 | 删除 `kaubo-ir/src/hir.rs` | HIR 类型定义（仅被死代码使用） |

### 0.3 重命名 lexer

| # | 任务 | 操作 |
|---|------|------|
| 7 | `lexer/v2.rs` → `lexer/re_exports.rs` | 7 行重命名，更新所有 `use ...::v2::*` 引用 |

### 0.4 拆分 execution.rs

| # | 任务 | 操作 |
|---|------|------|
| 8 | `execution.rs` (1780行) → 按 opcode 类别拆 | `ops/arithmetic.rs` (算术+比较), `ops/control.rs` (Jump/JumpIfFalse), `ops/call.rs` (Call/Closure/Return), `ops/coroutine.rs`, `ops/struct.rs`, `ops/list.rs`, `ops/json.rs`, `ops/module.rs`, `ops/cast.rs` |
| 9 | 主文件保留 | `execution.rs` 只留主循环 `run()` + 宏 + 入口逻辑 |

### 0.5 统一编译路径

| # | 任务 | 操作 |
|---|------|------|
| 10 | 切换为 HIR 路径为默认 | 在 `kaubo-cli/src/main.rs` 中改为 `lower_module()` + `compile_hir()` 替代旧 `CodegenStage` 直接调用。**目的：暴露现有 HIR gap，为 Phase 3/4 做准备** |

### 0.6 crate 目录归一

```
当前:  crates/kaubo-ir/     crates/kaubo-compiler/   crates/kaubo-runtime/
       kaubo-cli/           kaubo-log/                kaubo-wasm/

目标:  kaubo-vm/            kaubo-cli/                kaubo-wasm/
       crates/kaubo-syntax/  crates/kaubo-infer/       crates/kaubo-ir/
```

各 crate 改名以匹配 v2 架构。`kaubo-vfs` 合并到 `kaubo-vm`（模块加载）。`kaubo-log` 内联到各 crate。

**交付物：** `cargo check --workspace` 通过，旧 crate 目录已删除，execution.rs 已拆分。

---

## Phase 1: 新语法（1周）

**目标：** 全新的 lexer + parser，表达式导向，23 关键词。

### 1.1 新 Token 定义

**文件：** `crates/kaubo-syntax/src/token.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenKind {
    // 语法 (19)
    Const, Var, If, Else, For, In, While, Break, Continue, Return,
    Struct, Impl, Export, Import, From, As, Async, Await, Self_,
    // 字面量 (5)
    Identifier, IntLiteral, FloatLiteral, StringLiteral,
    True, False, Null,
    // 符号
    Plus, Minus, Asterisk, Slash, Percent,
    Eq, EqEq, NotEq, Lt, Le, Gt, Ge,
    Not, And, Or,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Semicolon, Colon, Dot,
    Pipe, FatArrow, GtGt, // |>  >>  ->
    // 特殊
    Eof, Comment, Whitespace, Error,
}
```

### 1.2 新 Lexer

**文件：** `crates/kaubo-syntax/src/lexer.rs`

- 复用现有 char_stream/Scanner 架构，但 Token 集全部替换
- 数字字面量 (`42` → `IntLiteral`，`3.14` → `FloatLiteral`)
- 字符串转义 `\n \r \t \\ \" \'`
- 块注释 `/* */` + 行注释 `//`

### 1.3 新 Parser

**文件：** `crates/kaubo-syntax/src/parser.rs`

- 表达式导向：无 stmt/expr 分离，所有构造体统一为 `Expr`
- 递归下降 + Pratt 表达式解析（复用现有优先级表但更新运算符集）
- `;` 作为分隔符，无语义

### 1.4 新 AST

**文件：** `crates/kaubo-syntax/src/ast.rs`

```rust
enum Expr {
    LitInt(i64), LitFloat(f64), LitString(String),
    LitTrue, LitFalse, LitNull,
    VarRef(Name), Lambda(Vec<Param>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Block(Vec<Stmt>),               // Block 是 Expr
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    For(Param, Box<Expr>, Box<Expr>),
    Break, Continue,
    Return(Option<Box<Expr>>),
    Member(Box<Expr>, Name),
    Index(Box<Expr>, Box<Expr>),
    Struct(Vec<FieldDef>),
    StructLit(Name, Vec<(Name, Expr)>),
    ListLit(Vec<Expr>),
    ImplBlock(Name, Vec<MethodDef>),
    Assign(Box<Expr>, Box<Expr>),
    Import(Name, Option<Name>, Vec<Name>),
    Export(Box<Stmt>),
    Async(Box<Expr>),
    Await(Box<Expr>),
}

struct MethodDef { name: Name, body: Expr }  // body 是 Lambda
```

### 1.5 语义检查

**文件：** `crates/kaubo-syntax/src/semantic.rs`

- 禁止自递归：`VarRef(name)` 与当前正在定义的名称冲突 → `CompileError::SelfRecursion`
- 禁止前向引用：名称未在当前作用域 → `CompileError::UnboundVariable`

**交付物：** 至少能 parse [设计文档第 2 节](./kaubo-v2.md#二语法) 所有语法示例并输出完整 AST。

---

## Phase 2: HM 类型推断（1周）

**目标：** 实现 Algorithm W，支持 Lambda/const/var/struct/List，产 TypedAST。

### 2.1 数据结构

**文件：** `crates/kaubo-infer/src/types.rs`

```rust
struct TypeVar(usize);           // fresh var counter
enum Type { Var(TypeVar), Con(String), Arrow(Box<Type>, Box<Type>),
            Record(usize), List(Box<Type>) }
struct Scheme { bound: Vec<TypeVar>, body: Box<Type> }
struct Subst(HashMap<TypeVar, Type>);
struct TypeEnv(HashMap<String, Scheme>);
```

### 2.2 Algorithm W 实现

**文件：** `crates/kaubo-infer/src/infer.rs`

```rust
fn infer(env: &TypeEnv, expr: &Expr) -> Result<(Subst, Type), TypeError>;
fn unify(t1: &Type, t2: &Type) -> Result<Subst, TypeError>;
fn generalize(env: &TypeEnv, ty: Type) -> Scheme;
fn instantiate(scheme: &Scheme) -> Type;
fn occurs_check(var: TypeVar, ty: &Type) -> bool;  // 防止无限类型
```

### 2.3 各节点推断规则（按优先级实现）

| 优先级 | 节点 | 工作量 |
|--------|------|--------|
| 1 | LitInt, LitFloat, LitTrue, LitFalse, LitNull | 简单 |
| 2 | VarRef → instantiate | 5 行 |
| 3 | Lambda | 核心逻辑 |
| 4 | Binary → 值类型方法识别 (42.as_float → itof) | HM 特化钩子 |
| 5 | Call | 核心逻辑 |
| 6 | Const/Var → generalize | let-多态入口 |
| 7 | Block | 遍历子语句，返回最后一个表达式类型 |
| 8 | If/While/For | 约束 + 返回 null |
| 9 | Struct/StructLit → Record | struct id 分配 |
| 10 | Member/Index → GetField/IndexGet 类型 | 字段索引推导 |
| 11 | async/await → 特殊标注 | 同步函数含 await → 报错 |

### 2.4 TypedAST

**文件：** `crates/kaubo-infer/src/typed_ast.rs`

每个 AST 节点包装 `{ node: Expr, ty: Type }`，用于 Phase 3/4。

**交付物：** 对每个语法示例手工验证类型推导结果，~20 个单元测试覆盖核心推断规则。

---

## Phase 3: CPS 变换（1周）

**目标：** TypedAST → CPS blocks (HirModule)，消解所有控制流。

### 3.1 Block 结构

**文件：** `crates/kaubo-ir/src/cps.rs`

```rust
struct HirModule { functions: Vec<HirFunction>, constants: Vec<Constant>, structs: Vec<StructDef> }
struct HirFunction { blocks: Vec<Block>, entry: BlockId, reg_count: usize }
struct Block { id: BlockId, params: Vec<Reg>, instrs: Vec<Instr>, term: Terminator }
enum Terminator { Jump(BlockId, Vec<Reg>), Branch(Reg, BlockId, Vec<Reg>, BlockId, Vec<Reg>),
                  Return(Reg), Call(Reg, Vec<Reg>, BlockId), TailCall(Reg, Vec<Reg>), Suspend }
```

### 3.2 变换规则

| 源 | CPS |
|----|-----|
| `if cond { A } else { B }` | `branch(cond, block_A, block_B)` |
| `while cond { body }` | `block_loop → branch(cond, body, exit)` |
| `break` | `jump(block_exit)` |
| `continue` | `jump(block_loop)` |
| `return expr` | `jump(block_exit, val)` |
| `const f = \|x\| { body }` | 新 HirFunction，body 降级为 block_chain |
| `for x in list` | `GetIter → iter_next → IterNext → branch(has, body, exit)` |

### 3.3 块参数与寄存器

每个 block 的 params 列表在 lowering 时确定（来自类型系统推导）。同一 HirFunction 内虚拟寄存器号统一分配。

**交付物：** 对 [设计文档 4.2 节](./kaubo-v2.md#变换示例) 的 while/break/continue 示例输出 CPS blocks 并通过手动验证。

---

## Phase 4: 优化 + 代码生成（1周）

**目标：** CPS blocks → 优化 → 32-bit Chunk。

### 4.1 优化 Passes

| Pass | 描述 |
|------|------|
| 常数折叠 | `BinOp(Imm(1), Add, Imm(2)) → LoadConst(3)` |
| 死块消除 | 无入边的 block 删除 |
| 块合并 | 相邻且仅有一个 jump 的 block 合并 |
| Box elimination | 局部追踪 box/unbox 对，将值保持在寄存器 |

### 4.2 寄存器分配

线性扫描：遍历 block 内指令，每个赋值目标分配下一个可用虚拟寄存器。帧结束时记录 `reg_count`。

### 4.3 32-bit 编码

```rust
fn encode_rrr(op: u8, dst: u16, s1: u16, s2: u16) -> u32 {
    ((op as u32) << 26) | ((dst as u32 & 0xFF) << 18) |
    ((s1 as u32 & 0x1FF) << 9) | (s2 as u32 & 0x1FF)
}
```

### 4.4 类型特化指令选择

HM 已知类型 → 选 opcode：

| 类型 | 指令 |
|------|------|
| Int64+Int64 | `AddInt` |
| Float64+Float64 | `FAdd` |
| String+String | `SAdd` |
| 泛型/BoxedValue | `mov` + boxed 路径 |

**交付物：** 对一个完整的 Kaubo 模块 (`const f = |x|{ x+1 }; f(42)`) 输出 32-bit 字节码 hex dump 并通过模拟验证。

---

## Phase 5: 寄存器 VM（1.5周）

**目标：** 全新 VM，44 条 opcode 的块调度器。

### 5.1 RegFile

**文件：** `kaubo-vm/src/regfile.rs`

```rust
struct RegFile { ints: Vec<i64>, floats: Vec<f64>, ptrs: Vec<GcPtr> }
```

### 5.2 主循环

**文件：** `kaubo-vm/src/execute.rs`

- 44 arm 的 match，零 push/pop
- 每条指令 1-5 行

### 5.3 GC + 内存管理

**文件：** `kaubo-vm/src/gc.rs`

- `Gc<T>`: ptr + rc
- retain/release 自动处理
- 帧退出批量 release

### 5.4 调用栈与闭包

- `CallFrame` — 寄存器基址 + IP，闭包 → Upvalue 捕获

### 5.5 标准库模块

| 模块 | 内容 | 实现 |
|------|------|------|
| `std/prelude` | print, type_of, assert | Rust NativeFn |
| `std/math` | sqrt, sin, cos, floor, ceil, PI, E | Rust NativeFn |
| `std/list` | range, map, filter, fold, find, any, all | Kaubo 源码 |
| `std/string` | trim, to_upper, split, join, replace | Kaubo 源码 |

**交付物：** `kaubo run examples/hello.kaubo` 输出 "Hello, world"。

---

## Phase 6: async/await + C 模块互操作（1周）

### 6.1 AsyncScheduler

**文件：** `kaubo-vm/src/async.rs`

- `Suspend` 指令 → 保存帧 → 返回
- I/O 就绪 → 压回帧 → 从 continuation block 恢复

### 6.2 .kauboi 加载

**文件：** `kaubo-vm/src/module/ffi.rs`

- 解析 `.kauboi` 类型声明
- `dlopen` + `dlsym` 加载 `.so`
- 类型 marshalling（Int64↔int64_t, Float64↔double, String↔const char*）
- 错误映射到 `KbValue{tag=KB_TAG_ERROR}`

### 6.3 kaubo.h

`kaubo.h` 头文件 + 示例 C 嵌入程序。

**交付物：** C 嵌入 Kaubo 并调 Kaubo 函数 + Kaubo 调 C 函数。

---

## Phase 7: 模块系统（1周）

### 7.1 .kaubop 格式

- ZIP 打包/解包
- `package.json` 解析
- `.meta` 导出类型文件读写

### 7.2 Tree-shaking

- DAG 依赖图遍历 (BFS 从入口模块)
- 标记被引用符号
- 只输出标记符号的 chunk

### 7.3 动态 import

`import("path")` → 基于 async/await 的运行时模块加载。

**交付物：** `kaubo build --target kaubop ./src/` 产出 tree-shaken 的 `.kaubop`。

---

## Phase 8: WASM + VSCode + 文档（1周）

### 8.1 kaubo-wasm

- 适配新 crate 名和 API
- 4 个导出函数: `lex`, `diagnose`, `hover`, `compile`+`run`

### 8.2 VSCode

- 更新 TextMate 语法到 v2 关键词/运算符
- 更新 snippets

### 8.3 文档

- 语言教程（基于新语法重写）
- 标准库参考
- C 嵌入指南

**最终交付物：** WASM playground 可运行 v2 代码，VSCode 有语法高亮，语言文档完整。

---

## 依赖图

```
Phase 0 (清理)
    │
Phase 1 (新语法)
    │
Phase 2 (HM 推断)
    │
Phase 3 (CPS)
    │
Phase 4 (优化+编码)
    │
Phase 5 (寄存器 VM)
    │
    ├── Phase 6 (async/await + .kauboi)
    ├── Phase 7 (模块系统)
    └── Phase 8 (WASM + VSCode + 文档)
```

Phase 0-5 串行。Phase 6/7/8 可在 Phase 5 完成后并行。
