# Kaubo 开发手册

> 面向开发者的技术文档，记录架构决策、开发规范和注意事项。

## 1. 架构概述

### 1.1 核心流程

```
源代码 → Lexer → Parser → Compiler → VM
          ↓        ↓         ↓        ↓
        Tokens   AST     Bytecode  执行
```

### 1.2 关键设计决策

| 决策 | 说明 | 相关代码 |
|------|------|----------|
| **NaN Boxing** | Value 使用 64-bit NaN Boxing，高效表示多种类型 | `src/runtime/value.rs` |
| **ShapeID** | 模块字段编译期确定索引，运行时 O(1) 访问 | `src/runtime/compiler.rs` |
| **扁平模块** | 模块内不能嵌套子模块 | `src/compiler/parser/parser.rs` |
| **显式导入** | 所有模块依赖必须通过 `import` 声明 | `src/runtime/compiler.rs` |
| **NativeFn** | 标准库用 Rust 实现，通过 `ObjNative` 暴露 | `src/runtime/stdlib/` |

---

## 2. 模块系统

### 2.1 标准库模块（扁平化设计）

**当前设计**：单一扁平 `std` 模块

```rust
// src/runtime/stdlib/mod.rs
// ShapeID 硬编码映射：
// 0: print    1: assert    2: type    3: to_string
// 4: sqrt     5: sin       6: cos     7: floor
// 8: ceil     9: PI       10: E
```

**使用方式**：
```kaubo
import std;

std.print("Hello");
std.print(std.PI);
std.print(std.sqrt(16));
```

### 2.2 模块 ShapeID 查找流程

```rust
// Compiler::find_module_shape_id()
1. 在 self.modules (已编译模块) 中查找
2. 在 self.current_module (当前模块) 中查找  
3. 在标准库 (find_std_module_shape_id) 中查找
```

### 2.3 添加新的标准库函数

1. 在 `src/runtime/stdlib/mod.rs` 中实现函数
2. 在 `create_stdlib_modules()` 中注册 ShapeID
3. 在 `compiler.rs` 的 `find_std_module_shape_id()` 中添加映射

---

## 3. 字节码指令

### 3.1 关键指令

| 指令 | 操作数 | 说明 |
|------|--------|------|
| `LoadGlobal` | `u8` (常量池索引) | 加载全局变量到栈 |
| `ModuleGet` | `u16` (ShapeID) | 获取模块字段，O(1) |
| `Call` | `u8` (参数个数) | 调用函数/原生函数 |
| `Closure` | `u8` (函数常量索引) | 创建闭包 |

### 3.2 模块访问编译示例

```kaubo
std.print(123);
```

编译为：
```
LoadGlobal 0      // 加载 "std" 模块
LoadConst 1       // 加载 123
ModuleGet 0       // 获取 std.print (ShapeID=0)
Call 1            // 调用，1个参数
```

---

## 4. 开发规范

### 4.1 添加新特性 checklist

- [ ] Parser 支持（如需要新语法）
- [ ] AST 节点定义（`src/compiler/parser/expr.rs` / `stmt.rs`）
- [ ] Compiler 实现（`src/runtime/compiler.rs`）
- [ ] VM 指令支持（`src/runtime/vm.rs`）
- [ ] 测试文件（`assets/test_xxx.txt`）
- [ ] 文档更新（`docs/KAUBO.md`）

### 4.2 调试技巧

```rust
// 启用 VM 执行追踪
// 在 src/runtime/vm.rs 中已有详细日志输出
```

### 4.3 常见问题

**Q: `std.xxx` 解析失败？**
- 检查是否是关键字（关键字不能作为成员名）
- 检查 `find_std_module_shape_id()` 是否有该映射

**Q: 模块访问编译错误？**
- 确认模块已通过 `import` 导入
- 检查 `imported_modules` 是否正确跟踪

---

## 5. 已知设计问题（待解决）

### 5.1 高优先级

1. **测试机制缺失**
   - 现状：只有手动测试文件
   - 目标：`cargo test` 运行自动化测试

2. **错误处理不完善**
   - 编译错误缺少行号信息
   - 运行时错误堆栈追踪缺失

3. **语义化版本**
   - 当前字节码无版本号
   - 需要添加魔数和版本校验

### 5.2 中优先级

4. **@ProgramStart 装饰器**
   - 设计已完成但未实现
   - 需要支持标记入口模块

5. **浮点数比较**
   - 当前直接比较，需要 epsilon

6. **GC 缺失**
   - 目前只分配不回收

---

## 6. 文件索引

| 文件 | 用途 |
|------|------|
| `src/runtime/stdlib/mod.rs` | 标准库实现 |
| `src/runtime/compiler.rs` | 字节码编译器 |
| `src/runtime/vm.rs` | 虚拟机 |
| `src/runtime/object.rs` | 运行时对象 |
| `src/runtime/value.rs` | Value 类型（NaN Boxing） |
| `src/compiler/parser/parser.rs` | 语法分析器 |
| `src/compiler/lexer/token_kind.rs` | Token 类型定义 |

---

*最后更新: 2026-02-10*  
*版本: 2.9*
