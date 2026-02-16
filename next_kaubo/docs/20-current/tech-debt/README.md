# 技术债务

> 已知未完成的功能和待办事项。按优先级和组件组织。

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

- `kaubo-core/src/core/operators.rs` - `InlineCacheEntry`
- `kaubo-core/src/runtime/vm/mod.rs` - `interpret_with_locals` (加载 Chunk 缓存到 VM)
- `kaubo-core/src/runtime/vm/execution.rs` - Add/Sub/Mul/Div 指令缓存检查逻辑
- `kaubo-core/src/runtime/vm/operators.rs` - 缓存操作函数

**实现概要**：

1. **编译阶段**：`kaubo-core/src/runtime/compiler/expr.rs` 为二元运算指令分配内联缓存槽位
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
- `kaubo-api` 传入 `config.vm.*` 值

**相关文件**：

- `kaubo-core/src/compiler/lexer/builder.rs`
- `kaubo-core/src/runtime/vm.rs`
- `kaubo-api/src/lib.rs`

### 包导出优化（2026-02-16）

**问题**：各 crate 导出过于宽泛，增加了 API 维护负担

**优化内容**：

#### kaubo-core

| 优化前 | 优化后 |
|--------|--------|
| `pub use kaubo_config::{...}` | 移除（由调用方直接使用 kaubo-config） |
| `pub mod compiler/kit/runtime` | 精简的重新导出 |
| 无顶层快捷导出 | 新增 `Value`, `VM`, `Chunk`, `InterpretResult`, `VMConfig`, `ObjShape` |

#### kaubo-api

| 优化前 | 优化后 |
|--------|--------|
| 导出 12 个 `kaubo_config` 单个类型 | 统一 `pub use kaubo_config;` |
| 导出 `LexerError`, `ParserError`, `TypeError` | 封装在 `KauboError` 中，不暴露底层 |
| `pub use kaubo_core::Value/Phase` | 仅保留 `pub use kaubo_core::Value;` |

**设计原则**：

- 顶层 crate（kaubo-api）提供统一入口
- 底层 crate（kaubo-core）只导出核心类型
- 配置 crate（kaubo-config）完整导出供上层使用

**相关文件**：

- `kaubo-core/src/lib.rs`
- `kaubo-api/src/lib.rs`
- `kaubo-api/src/error.rs`

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

将 `kaubo-core/src/runtime/compiler.rs` (2258行) 拆分为模块：

| 文件 | 内容 | 行数 |
|------|------|------|
| `compiler/mod.rs` | 主模块：Compiler 结构体、构造函数、测试 | ~580 |
| `compiler/error.rs` | CompileError 枚举 | ~40 |
| `compiler/context.rs` | Export, ModuleInfo, StructInfo, VarType | ~35 |
| `compiler/var.rs` | Local, Upvalue, Variable, 作用域管理 | ~230 |
| `compiler/expr.rs` | 表达式编译方法 | ~620 |
| `compiler/stmt.rs` | 语句编译方法 | ~520 |

### VM 拆分

将 `kaubo-core/src/runtime/vm.rs` (3530行) 拆分为模块：

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
$ cargo clippy -p kaubo-core --lib
warning: `kaubo-core` (lib) generated 10 warnings

$ cargo test -p kaubo-core --lib
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
