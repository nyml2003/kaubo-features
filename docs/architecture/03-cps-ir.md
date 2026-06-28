# 03 — CPS IR：Lowering + Flatten + Passes

**管线位置**：AST + 类型信息 → CPS IR → 优化后的 CpsModule

## 输入 / 输出

```
(Module, SemanticArtifact) → build_module() → CpsModule（分层嵌套）
                           → flatten_module() → CpsModule（扁平 blocks）
                           → PassPipeline.run() → CpsModule（优化后）
```

## CPS IR 核心类型

### 模块结构

```rust
// kaubo-cps/src/lib.rs
CpsModule {
    functions: Vec<CpsFunction>,
    structs: Vec<StructDef>,
    enums: Vec<EnumDef>,
    vtables: Vec<VtableDef>,
    constants: Vec<Constant>,
    symbol_map: HashMap<(String, String), usize>,  // (module, name) → global_func_idx
    func_owners: Vec<String>,
}
```

### 函数→块→指令→终止器

```rust
CpsFunction { name, blocks: Vec<CpsBlock> }
CpsBlock    { id, params: Vec<usize>, instrs: Vec<CpsInstr>, term: CpsTerminator }

CpsInstr       ← 各种指令（算术/加载/存储/构造/调用）
CpsTerminator  ← Jump / Branch / Call / CallExternal / CallIndirect / Return / Suspend
```

### 指令分类

| 类别 | 指令 | 说明 |
|------|------|------|
| 字面量 | `LoadConst`, `LoadFloat`, `LoadString` | 常量加载 |
| 二元运算 | `AddInt`, `SubInt`, `MulInt`, `DivInt`, `FAdd`, …, `SAdd` | 按类型+运算符精确匹配（不再靠 `ValueHint` 猜） |
| 一元运算 | `Eqz`, `FToI`, `IToF` | |
| 构造 | `NewStruct`, `NewVariant`, `NewList`, `NewClosure` | |
| 字段 | `GetField`, `SetField`, `GetVariantTag`, `GetVariantField`, `SetVariantField` | |
| 比较 | `CmpEq`, `CmpLt`, … | |
| 动态分派 | `LoadVtable`, `NewInterfaceObj`, `CallIndirect` | Phase 4a 新增 |
| 终止器 | `Jump`, `Branch`, `Call`, `CallExternal`, `Return`, `Suspend` | |

## Lowering（CpsBuildStage）

```
build_module(module, semantic) → CpsModule（分层嵌套 block）
```

入口 `kaubo-ir/src/cps_build.rs::build_module()`（~3400 行）。核心是 `CpsBuilder` 结构体：

- **类型驱动分派**：`build_binary(op, lhs_type, rhs_type)` 精确匹配 `(Add, Int64, Int64) → AddInt`
- **Interface 分派**：`a + b`（struct 有 `impl Add`）→ 查 interface_registry → `LoadVtable` + `CallIndirect`
- **while/for 循环** → 生成 header/body/exit block 三元组 + `Branch` 回边
- **Lambda** → 独立 CpsFunction + `NewClosure` 捕获 upvalue

## Flatten

```
flatten_module(&mut CpsModule)  // kaubo-ir/src/flatten.rs:9
```

将嵌套 block 结构展开为物理 IP 线性排列的 blocks。所有跳转目标重映射为物理 IP。flatten **必须在所有 Pass 之前**执行。

## Pass Pipeline

```rust
// kaubo-driver 中的默认管线
Pipeline::new()
    .add(EmptyBlockElim)   // 消除无指令空 block
    .add(MoveFold)         // 折叠冗余 move 指令
    .add(ConstantFold)     // 常量折叠
```

Pass trait：`fn run(&self, module: &mut CpsModule, events: Option<&dyn EventHandler>)`。Pass 之间按编排层指定的顺序串行执行。

## 编码

```rust
encode_module(&CpsModule) -> Vec<u8>   // 32-bit 定长编码
decode_module(&[u8]) -> Result<CpsModule, String>
```

用于持久化和跨进程传输。存在已知约束（`NewVariant tag` 只 8bit、`Branch fb` 只 8bit），变长编码在待做列表中。

## 代码位置

```
kaubo-cps/src/
└── lib.rs           CpsModule / CpsFunction / CpsBlock / CpsInstr / CpsTerminator (~160 行定义)

kaubo-ir/src/
├── lib.rs           re-export
├── cps_build.rs     ★ lowering 核心 ~3400 行
├── flatten.rs       block 扁平化
├── cps_emit.rs      指令构造辅助（emit_binary / emit_call / ...）
└── pass/
    ├── binary.rs        编码/解码
    ├── empty_block.rs   EmptyBlockElim pass
    ├── fold.rs          ConstantFold pass
    ├── move_fold.rs     MoveFold pass
    └── loop_inline.rs   LoopInline（实验中，有 TODO）
```
