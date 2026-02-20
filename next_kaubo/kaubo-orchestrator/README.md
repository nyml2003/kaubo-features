# Kaubo Orchestrator

Kaubo 编排引擎 - 管理组件生命周期和流水线执行。

## 架构概述

```
┌─────────────────────────────────────────────────────────────┐
│                    Kaubo Orchestrator                        │
├─────────────────────────────────────────────────────────────┤
│  Orchestrator                                                 │
│  ├── LoaderRegistry    (文件/网络/内存加载器)                  │
│  ├── ConverterRegistry (格式转换器)                           │
│  ├── PassRegistry      (编译阶段)                             │
│  └── EmitterRegistry   (输出目标)                             │
├─────────────────────────────────────────────────────────────┤
│  PipelineEngine                                              │
│  ├── 构建执行链 (从 pipeline 配置)                            │
│  ├── 协调各阶段执行                                          │
│  └── 错误处理和报告                                          │
└─────────────────────────────────────────────────────────────┘
```

## 组件类型

### 1. Loader (加载器)
从各种来源加载原始数据。

```rust
pub trait Loader: Component {
    fn load(&self, source: &Source) -> Result<RawData, LoaderError>;
}
```

**示例**: `FileLoader`, `HttpLoader`, `MemoryLoader`

### 2. Converter (转换器)
在不同中间表示 (IR) 格式间转换。

```rust
pub trait Converter: Component {
    fn can_convert(&self, from: DataFormat, to: DataFormat) -> bool;
    fn convert(&self, input: &IR) -> Result<IR, ConverterError>;
}
```

**IR 格式**: `Source` → `Tokens` → `Ast` → `TypedAst` → `Bytecode` → `Result`

### 3. Pass (处理阶段)
编译流水线中的处理阶段。

```rust
pub trait Pass: Component {
    fn input_format(&self) -> DataFormat;
    fn output_format(&self) -> DataFormat;
    fn run(&self, input: Input, ctx: &PassContext) -> Result<Output, PassError>;
}
```

**示例**: `Lexer`, `Parser`, `TypeChecker`, `CodeGen`

### 4. Emitter (输出器)
将结果输出到目标。

```rust
pub trait Emitter: Component {
    fn emit(&self, output: &SerializedOutput, target: &Target) -> Result<(), EmitterError>;
}
```

**示例**: `FileEmitter`, `StdoutEmitter`, `BinaryEmitter`

## 流水线配置

在 `package.json` 中定义流水线：

```json
{
  "pipeline": {
    "from": "source",
    "to": "bytecode",
    "stages": [
      { "name": "lex", "pass": "lexer" },
      { "name": "parse", "pass": "parser" },
      { "name": "typecheck", "pass": "type_checker" },
      { "name": "codegen", "pass": "codegen" }
    ],
    "emitters": [
      { "stage": "Bytecode", "emitter": "file", "target": "output.kbc" }
    ]
  }
}
```

## 使用示例

```rust
use kaubo_orchestrator::{Orchestrator, VmConfig};

// 创建编排器
let config = VmConfig::default();
let mut orchestrator = Orchestrator::new(config);

// 注册组件
orchestrator.register_loader(Box::new(FileLoader::new()));
orchestrator.register_pass(Box::new(LexerPass::new()));
orchestrator.register_pass(Box::new(ParserPass::new()));

// 执行流水线
let request = ExecutionRequest::from_file("main.kaubo")?;
let result = orchestrator.run(request)?;
```

## 设计原则

1. **统一组件接口**: 所有组件实现 `Component` trait，提供元数据和能力声明
2. **类型安全**: 使用 IR 类型系统确保阶段间数据兼容性
3. **可扩展**: 通过注册表动态添加新组件
4. **可配置**: 流水线通过 JSON 配置定义
5. **错误隔离**: 每个组件独立错误处理

## 迁移计划

- ✅ `kaubo-orchestrator` 基础结构
- ✅ 组件 trait 定义
- ✅ 注册表系统
- ✅ 流水线引擎
- 🔄 迁移现有 kaubo-core 功能到独立 passes
- 🔄 更新 kaubo-cli 使用 orchestrator
- 🔄 删除旧的 kaubo-api
