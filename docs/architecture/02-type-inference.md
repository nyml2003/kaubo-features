# 02 — 类型推断

**管线位置**：AST → 类型检查 + 符号收集 → SemanticArtifact

## 输入 / 输出

```
Module → infer_module() → (TypeEnv, struct_fields, interface_registry)
       → + symbol/reference 收集 → SemanticArtifact

多文件：Module + ImportTable → infer_module_with_imports() → 同上
```

## 核心类型

| 类型 | 所在 | 说明 |
|------|------|------|
| `Type` | `kaubo-infer/src/types.rs:15` | `Int64` / `Float64` / `String` / `Bool` / `Null` / `Record(id, fields)` / `Arrow(params, ret)` / `Enum(id)` / `Unbound(tvar)` |
| `TypeVar` | `kaubo-infer/src/types.rs:11` | 类型变量，unify 时被替换 |
| `Scheme` | `kaubo-infer/src/types.rs:31` | 多态类型（`forall a. a → a`） |
| `Subst` | `kaubo-infer/src/types.rs:39` | 替换表 `HashMap<TypeVar, Type>` |
| `TypeEnv` | `kaubo-infer/src/infer.rs` | `HashMap<String, Scheme>` — 名称到类型方案 |
| `TypeError` | `kaubo-infer/src/infer.rs:30` | 类型不匹配、未定义变量、循环类型等 |
| `ImportSpec` | `kaubo-infer/src/types.rs:132` | 跨模块导入的类型信息 |
| `SemanticArtifact` | `kaubo-driver/src/stages.rs` | symbols + type_env + references + symbol_names |

## 关键 API

```rust
pub fn infer_module(module: &Module)
    -> Result<(TypeEnv, HashMap<usize, Vec<(String, Type)>>), TypeError>;

pub fn infer_module_with_imports(module: &Module, imports: &ImportTable)
    -> Result<(TypeEnv, ..., HashSet<SymbolId>), TypeError>;

// Algorithm W 核心
pub fn infer(expr: &Expr, env: &mut TypeEnv, structs: &...) -> Result<Type, TypeError>;
pub fn unify(t1: &Type, t2: &Type) -> Result<Subst, String>;
pub fn generalize(env: &TypeEnv, ty: &Type) -> Scheme;
pub fn instantiate(scheme: &Scheme) -> Type;
```

## 推断流程

```
Pass 1  →  收集 struct/enum/interface 声明
Pass 2  →  注入内置接口（9 个 interface + 40+ 方法 impl）
Pass 3  →  逐语句推断：Algorithm W（infer + unify + generalize + instantiate）
Pass 4  →  接口匹配检查 + vtable 生成
```

### Interface / Vtable

Phase 4a 新增的接口系统在推断阶段完成：

- `interface Add { operator add: ... }` → 注册到 `interface_registry`
- `impl Add for Vec2 { ... }` → 检查方法签名匹配 → 生成 vtable
- `a + b` → infer 识别为 `Add.add(a, b)` → CPS 层生成 `LoadVtable` + `CallIndirect`
- 9 个内置接口（Add/Subtract/…/IntoInt）在每轮推断开始时自动注入

### 模块感知推断

`infer_module_with_imports` 额外处理：
- 注入导入符号的类型到 env（从 `ImportTable`）
- 标记 `export` 符号到 `exports: HashSet<SymbolId>`
- struct 字段信息跨模块透传

## 当前限制

- 不支持泛型/类型参数（Phase 5a 规划）
- interface 类型变量未实现（`const x: Display = ...`）
- `type_of` 的签名在推断中存在但 lowering/VM 未实现

## 代码位置

```
kaubo-infer/src/
├── lib.rs          # re-export
├── infer.rs        ~2500 行（主逻辑 + inject_builtin_interfaces/impls）
└── types.rs        ~150 行（Type / TypeVar / Scheme / Subst / ImportSpec）

kaubo-driver/src/
└── stages.rs       SemanticStage 包装了 infer_module + 符号收集
```
