# 架构全景

## 数据流

```
源码 (.kaubo)
    │
    ▼
┌──────────────┐
│    Lexer     │  词法分析 → Token 流（39 种 token）
└──────────────┘
    │
    ▼
┌──────────────┐
│   Parser     │  语法分析 → AST Module
└──────────────┘
    │
    ▼
┌──────────────┐
│ TypeChecker  │  类型检查 → 带类型的 AST（已实现，默认不启用）
└──────────────┘
    │
    ▼
┌──────────────┐
│   Codegen    │  代码生成 → Chunk（字节码，146 种 OpCode）
└──────────────┘
    │
    ▼
┌──────────────┐
│  VM Runtime  │  解释执行 → 输出
└──────────────┘
```

## 输出格式

```
.kaubo → .kaubod (debug, 无压缩)
       → .kaubor (release, 可选压缩)
```

二进制格式 Header：
- Magic: `"KAUB"` (4 bytes)
- Version: `0.1.0`
- Checksum

Section 结构：
- `StringPool` — 字符串常量池
- `ChunkData` — 字节码数据
- `ModuleTable` — 模块表
- `ShapeTable` — 结构体形状表
- ...

## Crate 职责

### `kaubo-ir`（零依赖）
- `Value` — NaN-boxed 值（SMI/float/pointer）
- `OpCode` — 146 种字节码指令
- `Chunk` — 编译产物容器
- `Object` — 运行时对象（Closure/Function/Shape/String/List 等）
- `VM` — VM 状态定义

### `kaubo-compiler`
- `lexer/` — `Lexer`（v2）, `KauboTokenKind`（39 种）, `KauboScanner`
- `parser/` — Pratt 解析框架, `Module`, `Stmt`, `Expr`, `TypeChecker`
- `codegen/` — AST → Chunk, `Compiler` 结构
- `hir/` — 实验性：HIR Lowering/Optimizer/Codegen
- `stages/` — `ParseStage`, `CheckStage`, `CodegenStage`
- `module/` — 多文件模块解析器、编译上下文

### `kaubo-runtime`
- `vm/execution.rs` — 主解释循环（~1778 行）
- `vm/` — 栈帧、内联缓存、形状、运算符
- `stdlib.rs` — 30 个内置函数注册
- `binary/` — BinaryReader/Writer/Loader

### `kaubo-wasm`
- `lex(source)` → JSON token 数组
- `diagnose(source)` → JSON 诊断数组
- `hover(source, offset)` → JSON 悬停信息
- `compile(source)` → 字节码指令数
- `run(bytes)` → stdout 输出

## 扩展点

| 扩展点 | 说明 |
|--------|------|
| `Stage<In, Out>` trait | 流水线阶段的抽象接口 |
| `Pipeline` | `.then()/.then_fn()/.observe()` 组合器 |
| `Platform` trait | WASM/Native 平台抽象（I/O 注入） |
| `OptimizationPass` trait | 优化 pass 框架（预留） |
| `SourceRepo` | 源码仓库抽象（预留） |
| `LogSink` | 日志输出抽象 |
| `Allocator` | 内存分配器抽象（预留） |
