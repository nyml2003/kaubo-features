# 模块架构重构设计

> 目标：明确模块边界，消除循环依赖，建立清晰的三层架构

---

## 现状问题

```
当前混乱的依赖关系：
┌─────────────────────────────────────────────────────────┐
│  kaubo-core/src/                                        │
│  ├── compiler/lexer/lexer.rs  ──→ kit/lexer/CharStream  │
│  ├── compiler/parser/parser.rs ──→ kit/lexer/scanner    │
│  ├── runtime/compiler.rs      ──→ runtime::Value        │
│  ├── runtime/object.rs        ──→ crate::runtime::VM    │
│  └── runtime/bytecode/chunk.rs ──→ runtime::Value       │
└─────────────────────────────────────────────────────────┘
```

**问题清单**：
1. 公共类型（Value, Chunk, VM）散落在各子模块
2. 模块间直接交叉依赖，没有统一抽象层
3. `pub use` 重新导出混乱，外部难以知道该用哪个路径
4. 内部实现细节（如 KauboScanner）被不必要地公开

---

## 目标架构

### 三层架构

```
┌─────────────────────────────────────────────────────────┐
│ Layer 3: API 层 (kaubo-api)                              │
│ - 统一入口：run(), compile()                              │
│ - 配置聚合：RunConfig                                     │
│ - 错误封装：KauboError                                    │
└─────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────┐
│ Layer 2: 核心类型层 (kaubo-core::core)                   │
│ 仅包含数据结构和类型定义，无业务逻辑                      │
│ - value.rs: Value, ValueType                             │
│ - bytecode.rs: Chunk, OpCode                             │
│ - vm.rs: InterpretResult, VMConfig                       │
│ - object.rs: ObjShape, ObjXxx                            │
│ - error.rs: LexerError, ParserError, TypeError          │
└─────────────────────────────────────────────────────────┘
                            ↑
┌─────────────────────────────────────────────────────────┐
│ Layer 1: 实现层                                          │
│ 只依赖 core，模块间不直接依赖                            │
│ ├── lexer/     (核心: Lexer, SourcePosition)             │
│ ├── parser/    (核心: Parser, Module)                    │
│ ├── compiler/  (核心: Compiler, CompileError)            │
│ └── runtime/   (核心: VM, Chunk)                         │
└─────────────────────────────────────────────────────────┘
```

### 依赖规则

| 规则 | 说明 |
|------|------|
| **单向依赖** | 实现层 → core → API，禁止反向依赖 |
| **同层隔离** | lexer/parser/compiler/runtime 互不直接依赖 |
| **核心无逻辑** | core 只放数据结构，无方法实现 |
| **内部私有** | 实现细节用 `pub(crate)` 或 `mod` |

---

## 实施步骤

### Phase 1: 创建 core 模块

1. 新建 `kaubo-core/src/core/` 目录
2. 迁移类型：
   - `runtime/value.rs` → `core/value.rs`
   - `runtime/bytecode/` → `core/bytecode.rs`
   - `runtime/vm.rs` (VMConfig, InterpretResult) → `core/vm.rs`
   - `runtime/object.rs` (ObjXxx) → `core/object.rs`
   - `kit/lexer/error.rs` (LexerError) → `core/error.rs`
   - `compiler/parser/error.rs` (ParserError) → `core/error.rs`
   - `compiler/parser/type_checker.rs` (TypeError) → `core/error.rs`

### Phase 2: 更新内部依赖

1. 所有 `use crate::runtime::Value` → `use crate::core::Value`
2. 所有 `use crate::runtime::bytecode::Chunk` → `use crate::core::bytecode::Chunk`
3. 所有 `use crate::kit::lexer::LexerError` → `use crate::core::error::LexerError`

### Phase 3: 收敛导出

**kaubo-core/src/lib.rs**:
```rust
// 核心类型统一导出
pub use core::{
    bytecode::{Chunk, OpCode},
    error::{LexerError, ParserError, TypeError, CompileError},
    object::ObjShape,
    value::Value,
    vm::{InterpretResult, VM, VMConfig},
};

// 实现模块按需导出
pub mod lexer {
    pub use crate::lexer::{Lexer, SourcePosition, SourceSpan};
}
pub mod parser {
    pub use crate::parser::{Parser, Module, TypeChecker};
}
// ... 其他模块
```

### Phase 4: 清理冗余

1. 删除原位置的类型定义
2. 删除不必要的 `pub use` 重新导出
3. 将内部实现标记为 `pub(crate)`

---

## 文件变更清单

| 原位置 | 新位置 | 说明 |
|--------|--------|------|
| `runtime/value.rs` | `core/value.rs` | Value 类型 |
| `runtime/bytecode/mod.rs` | `core/bytecode.rs` | Chunk, OpCode |
| `runtime/bytecode/chunk.rs` | 合并到 `core/bytecode.rs` | 简化 |
| `runtime/object.rs` | `core/object.rs` | ObjXxx 类型 |
| `runtime/vm.rs` (部分) | `core/vm.rs` | VMConfig, InterpretResult |
| `kit/lexer/error.rs` | `core/error.rs` | LexerError |
| `compiler/parser/error.rs` | `core/error.rs` | ParserError |
| `compiler/parser/type_checker.rs` (TypeError) | `core/error.rs` | TypeError |
| `runtime/compiler.rs` (CompileError) | `core/error.rs` | CompileError |

---

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 改动文件过多 | 一次性完成，不拆分提交 |
| 测试失效 | 每步都运行 `cargo test --workspace` |
| API 破坏 | kaubo-api 层保持兼容，只改内部 |
| 性能影响 | 仅移动代码，无逻辑变更 |

---

## 成功标准

- [ ] `cargo build --workspace` 无警告
- [ ] `cargo test --workspace` 全部通过
- [ ] `kaubo-core/src/lib.rs` 导出不超过 20 个类型
- [ ] 各实现模块无直接交叉依赖
- [ ] 文档测试通过

---

## 相关文档

- [技术债务](../tech-debt/README.md) - 记录重构决策
