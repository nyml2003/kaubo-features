# 测试与覆盖率计划

> 测试覆盖率提升计划和已知限制跟踪

## 当前状态 (2026-02-12)

```
测试状态: 187 passed, 0 failed
总分支覆盖率: ~51%
目标: 90%
```

### 各模块分支覆盖率

| 模块 | 当前 | 目标 | 优先级 |
|------|------|------|--------|
| vm.rs | ~36% | 90% | P0 (核心执行) |
| compiler.rs | ~41% | 90% | P0 (核心编译) |
| stdlib/mod.rs | ~39% | 90% | P1 (标准库) |
| object.rs | ~6% | 80% | P1 (对象操作) |
| parser/*.rs | ~60% | 85% | P2 (语法分析) |
| lexer/*.rs | ~70% | 85% | P2 (词法分析) |

## Phase 1: 提升到 60% (VM 核心路径)

### 1.1 VM 执行循环 (vm.rs)
- [ ] 常量加载指令 (LoadConst0-15, LoadConst, LoadConstWide)
- [ ] 栈操作 (Pop, Dup, Swap)
- [ ] 局部变量 (LoadLocal0-7, StoreLocal0-7)
- [ ] 全局变量 (LoadGlobal, StoreGlobal, DefineGlobal)
- [ ] 算术运算 (Add, Sub, Mul, Div, Neg)
- [ ] 比较运算 (Equal, NotEqual, Greater, Less, etc.)

### 1.2 函数调用
- [ ] 普通函数调用 (Call)
- [ ] 闭包创建 (Closure)
- [ ] Upvalue 访问 (GetUpvalue, SetUpvalue)
- [ ] 返回值 (Return, ReturnValue)

### 1.3 控制流
- [ ] 条件跳转 (Jump, JumpIfFalse)
- [ ] 循环跳转 (JumpBack)

## Phase 2: 提升到 75% (边界条件和错误处理)

### 2.1 错误处理
- [ ] 运行时错误 (除零、越界等)
- [ ] 栈溢出
- [ ] 类型错误

### 2.2 复杂数据结构
- [ ] 列表操作 (BuildList, IndexGet, IndexSet)
- [ ] JSON 操作
- [ ] 模块访问

### 2.3 协程
- [ ] 创建协程
- [ ] Yield/Resume
- [ ] 协程状态

## Phase 3: 提升到 90% (边缘情况)

### 3.1 编译器分支
- [ ] 所有表达式类型的编译
- [ ] 所有语句类型的编译
- [ ] 优化路径

### 3.2 标准库
- [ ] 所有 std 函数的参数校验
- [ ] 错误返回值

### 3.3 对象操作
- [ ] 所有 Value 类型
- [ ] 内存分配失败路径

## 高优先级开发任务

来自 TODO 的高优先级任务：

### 错误处理完善 ⭐⭐⭐

- [ ] 添加调用堆栈追踪
- [ ] 保留源码位置信息在字节码中
- [ ] 常见错误提示（如未导入的模块）

### 字节码版本号 ⭐⭐⭐

- [ ] 魔数 "KAUB" 头
- [ ] 主/次版本号 (u16)
- [ ] 版本兼容性检查

## 中优先级功能

### 1. 浮点数字面量支持

- [ ] Lexer 添加 `LiteralFloat` Token
- [ ] Parser 支持浮点数解析
- [ ] 更新测试用例

### 2. @ProgramStart 装饰器

- [ ] Parser 支持装饰器语法
- [ ] 编译期检查唯一性
- [ ] 运行时自动调用 `run` 函数

### 3. 垃圾回收

- [ ] 标记-清除 GC 实现
- [ ] 增量回收支持

## 覆盖率提升策略

1. **从高频路径开始** - 先覆盖最常用的指令和代码路径
2. **边界条件优先** - 0、1、最大值、空值等边界
3. **错误路径补充** - runtime error 的分支
4. **避免过度测试** - 相似路径合并，不追求 100%

## 测试文件结构

```
kaubo-core/tests/
├── vm_tests.rs           # 现有 VM 测试
├── compiler_tests.rs     # 编译器测试（新建）
├── stdlib_tests.rs       # 现有标准库测试
├── value_tests.rs        # Value 类型测试（新建）
├── integration_tests.rs  # 现有集成测试
└── coverage/             # 覆盖率专用测试
    ├── vm_coverage.rs    # VM 指令覆盖
    ├── compiler_coverage.rs
    └── edge_cases.rs     # 边界条件
```
