# 技术债务

> 已知未完成的功能和待办事项。按优先级和组件组织。

---

## 标准库 (std) 模块

### 🚧 当前问题

std 模块的类型定义在类型检查器中**硬编码且分散**，存在多处技术债务。

#### 1. 重复的类型定义 🔴

**问题**：相同函数的类型在多处重复定义

```rust
// init_stdlib_types() - 函数声明类型
self.env.define(
    "len".to_string(),
    TypeExpr::function(vec![TypeExpr::named("any")], Some(TypeExpr::named("int"))),
);

// get_stdlib_function_type() - 重复定义
"len" => Some(TypeExpr::function(
    vec![TypeExpr::named("any")],
    Some(TypeExpr::named("int")),
)),
```

**影响**：修改一个函数类型需要改两处，容易遗漏导致不一致。

**建议方案**：
- 创建 `StdFunctionRegistry` 统一注册
- 使用宏或代码生成避免重复

#### 2. 方法名映射混乱 🔴

**问题**：存在多层映射关系，难以追踪

```rust
// 映射 1：std.length -> string_len
"length" => "string_len",

// 映射 2：string_len -> 实际函数
"string_len" => Some(TypeExpr::function(...)),

// 映射 3：内置方法解析
BuiltinMethodTable::resolve_string_method("len") -> Some(0)
```

**影响**：
- 用户困惑：`std.len` 和 `std.length` 和 `.len()` 有什么区别？
- 维护困难：添加新方法需要改 3+ 处

**建议方案**：
- 统一命名规范：内置方法使用简短名（`len`），std 函数保持一致
- 废弃冗余别名（`length` → `len`）

#### 3. 类型精度不足 🟡

**问题**：大量使用 `any` 类型，失去类型检查价值

```rust
// 当前：push 接受 any, any
"push" => TypeExpr::function_void(vec![
    TypeExpr::named("any"),
    TypeExpr::named("any"),
]),

// 理想：泛型函数 |List<T>, T| -> void
"push" => TypeExpr::function_void(vec![
    TypeExpr::list(TypeExpr::type_param("T")),
    TypeExpr::type_param("T"),
]),
```

**影响**：
- `list.push("string")` 在 `List<int>` 上不会报错
- 链式调用类型丢失：`list.filter(...)` 返回 `List<any>`

**建议方案**：
- Phase 1：增加具体类型重载（`list_push_int`, `list_push_string`）
- Phase 2：实现泛型类型参数推导

#### 4. 缺少子模块组织 🟡

**问题**：所有函数扁平化暴露在 std 命名空间

```rust
// 当前
std.len(list)
std.sin(x)
std.read_file(path)

// 理想
std.list.len(list)
std.math.sin(x)
std.file.read(path)
```

**影响**：
- 命名冲突风险（如 `len`  vs `length`）
- 难以扩展（100+ 函数后难以管理）
- 不利于 IDE 补全

**建议方案**：
- 引入子模块：`std.list`, `std.math`, `std.file`, `std.string`
- 向后兼容：保留顶层别名一段时间

#### 5. 内置方法与 std 函数重复 🟡

**问题**：相同功能有两种调用方式，但实现分离

| 功能 | 内置方法 | std 函数 | 类型定义位置 |
|------|---------|---------|-------------|
| 列表长度 | `list.len()` | `std.len(list)` | 两个地方 |
| 列表添加 | `list.push(x)` | `std.push(list, x)` | 两个地方 |
| 字符串长度 | `str.len()` | `std.len(str)` | 两个地方 |

**风险**：
- 内置方法类型在 `BuiltinMethodTypeTable`
- std 函数类型在 `get_stdlib_function_type`
- 两者可能不一致（如返回类型不同）

**建议方案**：
- 统一底层实现：内置方法调用转换为 std 函数调用
- 或统一类型定义：从同一数据源生成两种类型

#### 6. 无动态注册机制 🟢

**问题**：无法在不修改源码的情况下扩展 std

```rust
// 当前：必须修改 type_checker.rs
fn init_stdlib_types(&mut self) {
    // 硬编码所有函数
}

// 理想：支持插件式注册
registry.register("my_module", "my_func", my_func_type);
```

**影响**：
- 用户无法添加自定义 std 函数
- 内置函数与第三方函数体验不一致

**优先级**：P3（Phase 4+ 考虑）

### 修复计划

| 序号 | 任务 | 优先级 | 依赖 |
|------|------|--------|------|
| 1 | 统一类型定义（消除重复） | P1 | - |
| 2 | 清理方法名映射 | P1 | 任务 1 |
| 3 | 提升列表/字符串操作类型精度 | P2 | 泛型支持 |
| 4 | 引入子模块命名空间 | P2 | 语法支持 |
| 5 | 统一内置方法与 std 函数 | P2 | 架构决策 |
| 6 | 动态注册机制 | P3 | 模块系统完善 |

### 相关文档

- [方法调用类型推断](../impl/method-call-type-inference.md) - 内置方法返回类型推导
- [内置方法设计](../impl/builtin-methods-design.md) - 内置方法架构

---

## 类型检查器

### ✅ 已完成（2026-02-14）

- if/while 条件 bool 检查
- 函数调用参数类型检查（数量、类型、any支持）
- Struct 字段类型检查
- 成员访问类型推导
- `as` 类型转换（int↔float, int/float/bool→string）
- `any` 顶层类型

### 🚧 未完成

| 功能 | 现状 | 优先级 |
|------|------|--------|
| 列表元素类型检查 | 推导为 `List<any>` 而非验证一致性 | P2 |
| string→int/float解析 | 仅支持基础类型转换 | P2 |
| 类型别名 `type Point = ...` | 未实现 | P3 |
| 联合类型 `int \| string` | 未实现，需大量设计 | P3 |

---

## VM 虚拟机

### ✅ 已完成（2026-02-14）

- 字节码解释器完整实现
- 一元运算符重载（neg）
- 比较运算符（lt/le）
- 索引运算符（get/set）
- 反向运算符（radd/rmul）
- operator call（可调用对象）
- operator str/mod

### 🚧 未完成（当前阶段）

#### 1. Level 2 内联缓存 ✅ 已完成

**状态**：✅ 已集成并测试通过

**代码位置**：

- `kaubo-orchestrator/src/core/operators.rs` - `InlineCacheEntry`
- `kaubo-orchestrator/src/runtime/vm/mod.rs` - `interpret_with_locals` (加载 Chunk 缓存到 VM)
- `kaubo-orchestrator/src/runtime/vm/execution.rs` - Add/Sub/Mul/Div 指令缓存检查逻辑
- `kaubo-orchestrator/src/runtime/vm/operators.rs` - 缓存操作函数

**实现概要**：

1. **编译阶段**：`kaubo-orchestrator/src/runtime/compiler/expr.rs` 为二元运算指令分配内联缓存槽位
2. **加载阶段**：`interpret_with_locals` 将 Chunk 的 `inline_caches` 加载到 VM
3. **执行阶段**：算术指令先检查缓存命中，未命中则查找并更新缓存

**关键修改**：

```rust
// VM::interpret_with_locals - 加载内联缓存
self.inline_caches.clear();
self.inline_caches.extend(chunk.inline_caches.clone());
```

**测试**：
- `test_inline_cache_integration` - 验证缓存加载和基本功能
- `test_inline_cache_multiple_calls` - 验证多次调用缓存命中

**预期性能**：Level 3 (~30-100ns) → Level 2 (~15ns)，提升 2-6 倍

### 📋 未来阶段（未开始）

| 功能 | 阶段 | 状态 | 说明 |
|------|------|------|------|
| JIT编译器 | Phase 4 | 📋 规划中 | 基于Cranelift，解释器兜底 |
| 热重载系统 | Phase 5 | 📋 规划中 | 依赖JIT完成 |
| 增量编译 | Phase 3+ | 📋 规划中 | 函数级增量解析 |

#### 2. Struct 字符串/整数键字段访问（待移除）

**状态**：过渡阶段，将在 release 版移除

**背景**：

- 当前 IndexGet 支持 `struct["field"]` 和 `struct[0]` 访问字段
- 但这与 operator get 语义冲突，且性能较差

**计划**：

1. 当前：保留字符串/整数键字段访问（兼容）
2. 过渡：添加编译器警告，建议使用 `.field`
3. Release：完全移除，只保留 `.field` 方式

---

## 实现路线图

### Phase 3（当前）- 优化与完善

| 组件 | 功能 | 优先级 | 状态 |
|------|------|--------|------|
| VM | Level 2 内联缓存 | P1 | 🚧 基础设施就绪 |
| VM | 移除 struct 字符串键访问 | P2 | 📋 计划中 |
| Typer | 列表元素类型检查 | P2 | 📋 待办 |
| Typer | string→int/float 解析 | P2 | 📋 待办 |
| Typer | 类型别名 | P3 | 📋 待办 |
| Typer | 联合类型 | P3 | 📋 待办 |

### Phase 4（规划）- JIT编译器

| 组件 | 功能 | 优先级 | 状态 |
|------|------|--------|------|
| JIT | Cranelift集成 | P0 | 📋 规划中 |
| JIT | 热点检测 | P1 | 📋 规划中 |
| JIT | 解释器→JIT切换 | P1 | 📋 规划中 |

### Phase 5（规划）- 热重载

| 组件 | 功能 | 优先级 | 状态 |
|------|------|--------|------|
| HotReload | 状态序列化 | P0 | 📋 规划中 |
| HotReload | 代码替换 | P0 | 📋 规划中 |
| HotReload | @hot注解 | P1 | 📋 规划中 |

---

## 已修复债务

### Shape ID 冲突（2026-02-14）

**问题**：基础类型 shape_id（0-99）与自定义 struct shape_id 冲突

- float = 1，但第一个 struct 也被分配了 shape_id = 1
- 导致 `3.0 * v` 时，float 查找到了 Vector 的 operator Mul

**修复**：struct shape_id 起始值从 1 改为 100

```rust
// 基础类型使用 0-99，struct 从 100 开始避免冲突
let mut next_shape_id: u16 = 100;
```

### 配置未落实问题（2026-02-16）

**问题**：多个组件硬编码配置值，未使用配置系统

#### 1. Lexer builder

**修复前**：`build_lexer()` 硬编码 `102400` 缓冲区大小

**修复后**：

- 收敛为单一入口 `build_lexer_with_config(&LexerConfig, logger)`
- 保留 `build_lexer()` 仅用于测试（向后兼容）

#### 2. VM 初始化

**修复前**：`VM::with_logger()` 硬编码：

- `stack: Vec::with_capacity(256)`
- `frames: Vec::with_capacity(64)`
- `inline_caches: Vec::with_capacity(64)`

**修复后**：

- 新增 `VMConfig` 结构体
- 使用 `VM::with_config(VMConfig, logger)`
- `kaubo-orchestrator` 传入 `config.vm.*` 值

**相关文件**：

- `kaubo-orchestrator/src/compiler/lexer/builder.rs`
- `kaubo-orchestrator/src/runtime/vm.rs`
- `kaubo-orchestrator/src/lib.rs`

### 包导出优化（2026-02-16）

**问题**：各 crate 导出过于宽泛，增加了 API 维护负担

**优化内容**：

#### kaubo-orchestrator

| 优化前 | 优化后 |
|--------|--------|
| `pub use kaubo_config::{...}` | 移除（由调用方直接使用 kaubo-config） |
| `pub mod compiler/kit/runtime` | 精简的重新导出 |
| 无顶层快捷导出 | 新增 `Value`, `VM`, `Chunk`, `InterpretResult`, `VMConfig`, `ObjShape` |

#### kaubo-orchestrator

| 优化前 | 优化后 |
|--------|--------|
| 导出 12 个 `kaubo_config` 单个类型 | 统一 `pub use kaubo_config;` |
| 导出 `LexerError`, `ParserError`, `TypeError` | 封装在 `KauboError` 中，不暴露底层 |
| `pub use kaubo_core::Value/Phase` | 仅保留 `pub use kaubo_core::Value;` |

**设计原则**：

- 顶层 crate（kaubo-orchestrator）提供统一入口
- 底层 crate（kaubo-orchestrator）只导出核心类型
- 配置 crate（kaubo-config）完整导出供上层使用

**相关文件**：

- `kaubo-orchestrator/src/lib.rs`
- `kaubo-orchestrator/src/lib.rs`
- `kaubo-orchestrator/src/error.rs`

---

## Clippy 警告（有意忽略）

以下 clippy 警告经过评估，决定**暂时保留**（非阻塞）：

| 警告 | 位置 | 保留原因 | 决策时间 |
|------|------|---------|---------|
| `should_implement_trait` | `object.rs:201` | `ObjIterator::next()` 命名与 `Iterator::next` 冲突，但实现 `Iterator` trait 需要返回值是引用，与当前设计不符。需要 API 设计决策。 | 2026-02-16 |
| `module_inception` | `parser/mod.rs`<br>`lexer/mod.rs`<br>`ring_buffer/mod.rs` | 模块与父模块同名是故意设计的（`parser` 模块包含 `parser` 子模块）。重构需要大量文件移动，收益有限。 | 2026-02-16 |
| `not_unsafe_ptr_arg_deref` | `vm.rs:1553` | ✅ **已修复** - `register_shape` 已标记为 `unsafe` | 2026-02-16 |
| `implicit_autoref` | `stdlib/mod.rs:461,514` | 原始指针解引用时的隐式自动引用是安全的，但显式处理会使代码更冗长。属于风格问题。 | 2026-02-16 |

### 已修复的 Clippy 警告（2026-02-16）

通过 `cargo clippy --fix` 和手动修复解决了 60+ 个警告：

- ✅ `uninlined_format_args` - 内联 format 参数
- ✅ `redundant_field_names` - 移除冗余字段名
- ✅ `derivable_impls` - 使用 derive 宏实现 Default
- ✅ `unnecessary_cast` - 移除不必要的类型转换
- ✅ `mixed_attributes_style` - 合并内部/外部文档属性
- ✅ `len_without_is_empty` - 为 ObjList/ObjJson 添加 is_empty 方法
- ✅ `missing_safety_doc` - 为 unsafe 函数添加 Safety 文档
- ✅ `needless_range_loop` - 使用迭代器替代索引循环
- ✅ `collapsible_match` - 折叠嵌套的 if let
- ✅ `len_zero` - 使用 is_empty() 替代 len() == 0

**修复命令**：

```bash
cargo clippy --workspace --fix --allow-dirty --allow-staged
```

---

## 模块拆分记录（2026-02-16）

### Compiler 拆分

将 `kaubo-orchestrator/src/runtime/compiler.rs` (2258行) 拆分为模块：

| 文件 | 内容 | 行数 |
|------|------|------|
| `compiler/mod.rs` | 主模块：Compiler 结构体、构造函数、测试 | ~580 |
| `compiler/error.rs` | CompileError 枚举 | ~40 |
| `compiler/context.rs` | Export, ModuleInfo, StructInfo, VarType | ~35 |
| `compiler/var.rs` | Local, Upvalue, Variable, 作用域管理 | ~230 |
| `compiler/expr.rs` | 表达式编译方法 | ~620 |
| `compiler/stmt.rs` | 语句编译方法 | ~520 |

### VM 拆分

将 `kaubo-orchestrator/src/runtime/vm.rs` (3530行) 拆分为模块：

| 文件 | 内容 | 行数 |
|------|------|------|
| `vm/mod.rs` | 主模块：公共 API、测试 | ~480 |
| `vm/execution.rs` | run() 主循环、指令执行 | ~1650 |
| `vm/stack.rs` | 栈操作：push, pop, peek | ~80 |
| `vm/operators.rs` | 运算符实现、内联缓存 | ~1050 |
| `vm/call.rs` | upvalue 捕获和关闭 | ~110 |
| `vm/shape.rs` | Shape 注册和查找 | ~100 |
| `vm/index.rs` | 索引操作 | ~180 |

### 拆分后的变化

- ✅ 文件大小更合理，便于维护
- ✅ 模块职责更清晰
- ⚠️ 新增一些 clippy 警告（见下表）

---

## Clippy 警告（有意忽略）

以下 clippy 警告经过评估，决定**暂时保留**（非阻塞）：

| 警告 | 位置 | 保留原因 | 决策时间 |
|------|------|---------|---------|
| `should_implement_trait` | `object.rs:201` | `ObjIterator::next()` 命名与 `Iterator::next` 冲突，但实现 `Iterator` trait 需要返回值是引用，与当前设计不符。需要 API 设计决策。 | 2026-02-16 |
| `module_inception` | `parser/mod.rs`<br>`lexer/mod.rs`<br>`ring_buffer/mod.rs` | 模块与父模块同名是故意设计的（`parser` 模块包含 `parser` 子模块）。重构需要大量文件移动，收益有限。 | 2026-02-16 |
| `module_inception` | `compiler/mod.rs`<br>`vm/mod.rs` | 新增的子模块与父模块同名，遵循原有设计模式。 | 2026-02-16 |
| `implicit_autoref` | `stdlib/mod.rs:461,514` | 原始指针解引用时的隐式自动引用是安全的，但显式处理会使代码更冗长。属于风格问题。 | 2026-02-16 |
| `dead_code` | `compiler/mod.rs`<br>`vm/mod.rs` | 子模块中的方法（如 `compile_expr`, `add_local`）通过 `impl Compiler`/`impl VM` 的包装方法调用，clippy 跨文件检测不到。实际已使用。 | 2026-02-16 |
| `dead_code` | `vm/shape.rs:42` | `register_operators_from_chunk` 是公共 API，等待外部调用者使用。 | 2026-02-16 |
| `dead_code` | `vm/stack.rs:7` | `push` 函数是公共 API，等待外部调用者使用。 | 2026-02-16 |

**当前状态（2026-02-16）**：

```bash
$ cargo clippy -p kaubo-orchestrator --lib
warning: `kaubo-orchestrator` (lib) generated 10 warnings

$ cargo test -p kaubo-orchestrator --lib
test result: ok. 265 passed; 0 failed; 0 ignored
```

### 已修复的 Clippy 警告

**2026-02-16 第一轮修复**：

- ✅ `uninlined_format_args` - 内联 format 参数
- ✅ `redundant_field_names` - 移除冗余字段名
- ✅ `derivable_impls` - 使用 derive 宏实现 Default
- ✅ `unnecessary_cast` - 移除不必要的类型转换
- ✅ `mixed_attributes_style` - 合并内部/外部文档属性
- ✅ `len_without_is_empty` - 为 ObjList/ObjJson 添加 is_empty 方法
- ✅ `missing_safety_doc` - 为 unsafe 函数添加 Safety 文档
- ✅ `needless_range_loop` - 使用迭代器替代索引循环
- ✅ `collapsible_match` - 折叠嵌套的 if let
- ✅ `len_zero` - 使用 is_empty() 替代 len() == 0

**2026-02-16 第二轮修复（模块拆分后）**：

- ✅ `unused_imports` - 清理未使用的导入（`ObjClosure`, `ObjFunction`, `ObjShape` 等）
- ✅ `approximate_constant` - 为测试中的 3.14 浮点数字面量添加 `#[allow]`
- ✅ `unused_variables` - 使用 `drop(vm)` 显式标记未使用的参数

**修复命令**：

```bash
# 自动修复
cargo clippy --workspace --fix --allow-dirty --allow-staged

# 检查剩余警告
cargo clippy --workspace --all-targets
```

---

## 相关文档

- [运算符重载](../impl/operators/README.md) - 四级分发策略
- [架构设计](../impl/README.md) - JIT 与优化方向
- [模块架构设计](../../20-current/impl/module-refactor.md) - 类型定义与实现分离
