# 运行时

## VM 架构

栈式虚拟机，核心数据结构：

```
┌─────────────────────┐
│  Operand Stack      │  ← 操作数栈（push/pop）
├─────────────────────┤
│  Call Frames        │  ← 调用栈（函数调用帧）
│  ├─ closure         │
│  ├─ ip (指令指针)    │
│  └─ locals          │
├─────────────────────┤
│  Upvalues           │  ← 闭包捕获的外部变量
├─────────────────────┤
│  Globals            │  ← 全局变量表
├─────────────────────┤
│  Constants Pool     │  ← 从 Chunk 加载的常量
└─────────────────────┘
```

## OpCode（字节码指令）

146 种变体，包括：

| 类别 | 示例 |
|------|------|
| 栈操作 | `Push`, `Pop`, `LoadLocal`, `StoreLocal` |
| 算术 | `Add`, `Sub`, `Mul`, `Div`, `Mod` |
| 比较 | `Eq`, `Neq`, `Lt`, `Gt`, `Lte`, `Gte` |
| 逻辑 | `And`, `Or`, `Not` |
| 跳转 | `Jump`, `JumpIfFalse`, `Loop` |
| 函数 | `Call`, `Return`, `Closure` |
| 访问 | `GetField`, `SetField`, `GetIndex`, `SetIndex` |
| 模块 | `GetModule`, `ModuleGet`, `GetModuleExport` |
| 协程 | `Yield`, `Resume`, `CreateCoroutine` |
| 结构体 | `NewStruct` |
| 列表 | `NewList` |
| 类型转换 | `AsInt`, `AsFloat`, `AsString`, `AsBool` |

## Value 表示（NaN-boxed）

使用 NaN-boxing 技术，在 8 字节内区分：

| 类型 | 编码 |
|------|------|
| SMI（小整数） | 低位标记 |
| Float | IEEE 754 f64 |
| Pointer | 指向堆对象的指针 |

## 内联缓存（Inline Cache）

方法调用使用**多态内联缓存**加速：

```
GetField / SetField / LoadMethod
  → 检查对象形状（Shape）
    → hit: 直接跳转到缓存的偏移
    → miss: 运行时查找，更新缓存
```

每个调用点维护 hit/miss 计数。

## 协程

```
CreateCoroutine(fn) → Coroutine 对象
  ↓
Resume(coroutine)
  → 执行 fn 直到 yield
  → 保存 IP + 栈帧状态
  ↓
Resume(coroutine)
  → 从上次 yield 处恢复执行
```

## Stdlib（标准库）

30 个内置函数，包括：

| 类别 | 函数 |
|------|------|
| 输出 | `print` |
| 类型 | `type`, `to_string` |
| 数学 | `sqrt`, `sin`, `cos`, `floor`, `ceil` |
| 列表 | `len`, `push`, `is_empty`, `range`, `clone` |
| 字符串 | `length`, `substring`, `trim`, `split`, `join` 等 |
| 协程 | `create_coroutine`, `resume` |
| 随机 | `random`, `random_int` |

## 二进制格式

### Header

```
Magic:   "KAUB" (4 bytes)
Version: 0.1.0
Checksum
```

### Sections

```
StringPool      — 字符串常量
ChunkData       — 字节码 + 常量表 + 行号表
ModuleTable     — 模块名 → Chunk 索引
ShapeTable      — 结构体形状定义
ExportTable     — 公开导出
ImportTable     — 导入依赖
```

## 当前未完成

| 项目 | 状态 |
|------|------|
| panic 消除 | 53 处 expect 待替换 |
| GC | 无 GC，手动 `Box::into_raw` |
| SourceMap | 结构存在但编译器未填充 |
| CoroutineStatus handler | OpCode 已定义但未实现 |
