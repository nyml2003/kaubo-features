# Kaubo 开发指南

## 项目结构

```
kaubo-workspace/
├── kaubo-core/          # 核心实现（VM、编译器、类型系统）
│   ├── src/core/        # 核心类型：Value、Chunk、OpCode、内置方法
│   ├── src/compiler/    # 词法分析、语法分析、类型检查
│   ├── src/runtime/     # VM 执行、内存管理、标准库
│   └── tests/           # 集成测试
├── kaubo-api/           # API 层（配置、错误处理）
├── kaubo-cli/           # 命令行工具
├── kaubo-config/        # 配置管理
├── kaubo-log/           # 日志系统
└── examples/            # 示例程序
```

## 内置类型方法 (Builtin Type Methods)

### 已实现方法

**List (12 个方法)**
- 基础: `push`, `len`, `remove`, `clear`, `is_empty`
- 函数式: `foreach`, `map`, `filter`, `reduce`, `find`, `any`, `all`

**String (2 个方法)**
- `len`, `is_empty`

**Json (2 个方法)**
- `len`, `is_empty`

### 架构要点

1. **文件位置**: `kaubo-core/src/core/builtin_methods.rs`
2. **方法类型**: `BuiltinMethodFn = fn(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String>`
3. **指令**: `OpCode::CallBuiltin (0xDC)`
4. **编译支持**: `kaubo-core/src/runtime/compiler/expr.rs` 中 `compile_function_call`

### 添加新方法步骤

1. 在 `list_methods`/`string_methods`/`json_methods` 模块中添加常量
2. 更新 `COUNT` 常量
3. 实现方法函数
4. 在 `XXX_METHOD_TABLE` 静态数组中添加方法
5. 在 `resolve_XXX_method` 函数中添加方法名映射
6. 添加测试

## 测试

```bash
# 运行所有测试
cargo make test

# 运行特定包测试
cargo test --package kaubo-core

# 运行特定测试
cargo test --package kaubo-core test_list_method_chain
```

## 示例运行

```bash
cargo run --package kaubo-cli -- examples/hello/package.json
```
