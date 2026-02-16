# 模块架构重构设计

## 问题分析

当前循环依赖：
```
Value → ObjXxx (通过 as_function 等方法)
ObjUpvalue → Value (closed: Option<Value>)
ObjList → Value (elements: Vec<Value>)
Chunk → Value (constants: Vec<Value>)
ObjFunction → Chunk
```

## 解耦策略：类型定义与实现分离

### Core 层（纯类型定义）
```
core/
  mod.rs       # 统一导出
  value.rs     # Value 结构体 + Tag 常量
  object.rs    # ObjXxx 类型定义（字段只有原始类型/Box）
  bytecode.rs  # Chunk, OpCode
  vm.rs        # VM, InterpretResult, VMConfig
  error.rs     # 错误类型
```

### Runtime 层（实现）
```
runtime/
  mod.rs           # 重新导出 core 类型
  value_impl.rs    # Value 方法实现（as_xxx, is_xxx, 运算）
  object_impl.rs   # ObjXxx 方法实现
  vm_impl.rs       # VM 执行逻辑
  operators.rs     # 运算符实现
  gc.rs            # 垃圾回收
```

## 关键设计决策

### 1. Value 与 ObjXxx 解耦
- Core 层：Value 只存储 `u64`，ObjXxx 使用 `*mut T` 或 `Box<T>`
- Runtime 层：通过 `impl Value { pub fn as_string(&self) -> Option<*mut ObjString> {...} }` 添加方法

### 2. Chunk 依赖 Value
- Chunk.constants 使用 `Vec<Value>` 是可以的，因为 Value 在 Core 层是完整的类型

### 3. ObjUpvalue 处理
- `closed: Option<Value>` 保留，因为 Value 是完整的类型
- 指针字段 `location: *mut Value` 也保留

## 迁移步骤

1. **创建 core/object.rs** - 移动所有 ObjXxx 定义
2. **创建 core/value.rs** - 移动 Value 定义和基础构造
3. **创建 core/bytecode.rs** - 移动 Chunk 和 OpCode
4. **创建 core/vm.rs** - 移动 VM 定义
5. **创建 core/error.rs** - 移动错误类型
6. **更新 runtime/** - 保留实现，改为 `use crate::core::*`
7. **更新 lib.rs** - 重新组织导出

## 兼容性

- 外部使用 `kaubo_core::Value` 不变
- 内部使用 `crate::core::Value` 或 `crate::Value`（通过 runtime/mod.rs 重新导出）
