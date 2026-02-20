# Kaubo 架构设计 V2：编排引擎 + 分类组件

## 架构概览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           用户层 (User Layer)                                │
│                                                                              │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                    │
│   │  kaubo-cli  │    │  kaubo-lsp  │    │  kaubo-ide  │                    │
│   │  (命令行)    │    │  (语言服务)  │    │  (IDE 插件)  │                    │
│   └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                    │
│          │                   │                   │                          │
└──────────┼───────────────────┼───────────────────┼──────────────────────────┘
           │                   │                   │
           └───────────────────┴───────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         编排引擎层 (Orchestrator)                            │
│                                                                              │
│   kaubo-orchestrator crate                                                   │
│   ======================                                                     │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Core Orchestrator                                                  │   │
│   │  - 配置解析 (kaubo.json + .kaubo-config)                            │   │
│   │  - 依赖图构建                                                        │   │
│   │  - 阶段调度 (Pipeline Execution)                                     │   │
│   │  - 事件总线                                                          │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│   │  Component      │  │  Pipeline       │  │  Event          │             │
│   │  Registry       │  │  Engine         │  │  Bus            │             │
│   │  (组件注册表)    │  │  (流水线引擎)    │  │  (事件总线)      │             │
│   └────────┬────────┘  └─────────────────┘  └─────────────────┘             │
│            │                                                                 │
└────────────┼────────────────────────────────────────────────────────────────┘
             │
             │ 统一组件协议 (Component Protocol)
             │
             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          组件层 (Components)                                 │
│                                                                              │
│   源码导入的组件 crate（Rust 源码依赖）                                       │
│                                                                              │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│   │   Loader    │  │  Converter  │  │    Pass     │  │   Emitter   │       │
│   │   (加载器)   │  │   (转换器)   │  │   (处理遍)   │  │   (生成器)   │       │
│   ├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤       │
│   │  file       │  │ kaubo-source│  │   parser    │  │  json       │       │
│   │  stdin      │  │  ast-json   │  │   checker   │  │  binary     │       │
│   │  network    │  │  bytecode   │  │  generator  │  │  file-writer│       │
│   │  ...        │  │   ...       │  │   runner    │  │  console    │       │
│   └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘       │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                     统一组件协议 (Component Protocol)                 │   │
│   │                                                                      │   │
│   │  trait Component {                                                  │   │
│   │      fn metadata(&self) -> Metadata;                                │   │
│   │      fn capabilities(&self) -> Capabilities;                       │   │
│   │  }                                                                 │   │
│   │                                                                      │   │
│   │  trait Loader: Component { fn load(&self, source) -> Data; }       │   │
│   │  trait Converter: Component { fn convert(&self, input) -> Data; }  │   │
│   │  trait Pass: Component { fn run(&self, input, ctx) -> Data; }      │   │
│   │  trait Emitter: Component { fn emit(&self, output, target); }      │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 组件分类

| 组件类型 | 职责 | 示例 | 命名规范 |
|---------|------|------|---------|
| **Loader** | 从外部来源读取原始数据 | 文件、stdin、网络 | `kaubo-loader-file` |
| **Converter** | 格式转换（反序列化） | source→AST、AST→bytecode | `kaubo-converter-ast-json` |
| **Pass** | 编译处理（中间表示转换） | parser、checker、generator | `kaubo-pass-parser` |
| **Emitter** | 输出到目标 | 文件、stdout、网络 | `kaubo-emitter-file` |

---

## 术语定义

### Loader（加载器）

从外部来源读取原始字节或文本数据。

```rust
pub trait Loader: Component {
    /// 从来源加载数据
    fn load(&self, source: &Source) -> Result<RawData, Error>;
}

// 示例
pub struct FileLoader;
pub struct StdinLoader;
pub struct NetworkLoader;
```

### Converter（转换器）

将原始数据转换为结构化的中间表示（IR）。

```rust
pub trait Converter: Component {
    /// 输入格式
    fn input_format(&self) -> &str;
    /// 输出格式
    fn output_format(&self) -> &str;
    /// 执行转换
    fn convert(&self, input: RawData) -> Result<IR, Error>;
}

// 示例
pub struct KauboSourceConverter;  // 源码文本 → AST
pub struct AstJsonConverter;      // JSON → AST
pub struct BytecodeBinaryConverter; // Binary → Bytecode
```

### Pass（处理遍）

对中间表示（IR）进行转换的编译处理阶段。

```rust
pub trait Pass: Component {
    /// 输入 IR 类型
    fn input_ir(&self) -> IRKind;
    /// 输出 IR 类型
    fn output_ir(&self) -> IRKind;
    /// 执行处理
    fn run(&self, ir: IR, ctx: &Context) -> Result<IR, Error>;
}

// 示例
pub struct ParserPass;      // Source → AST
pub struct CheckerPass;     // AST → TypedAST
pub struct GeneratorPass;   // TypedAST → Bytecode
pub struct RunnerPass;      // Bytecode → Result
```

### Emitter（生成器）

将中间表示序列化并输出到目标。

```rust
pub trait Emitter: Component {
    /// 接受的 IR 类型
    fn accepts(&self) -> Vec<IRKind>;
    /// 输出格式
    fn output_format(&self) -> &str;
    /// 发射输出
    fn emit(&self, output: Output, target: &Target) -> Result<(), Error>;
}

// 示例
pub struct JsonEmitter;      // AST → JSON → target
pub struct BinaryEmitter;    // Bytecode → binary → target
pub struct FileWriter;       // 写入文件
pub struct ConsoleWriter;    // 写入 stdout/stderr
```

---

## 数据流

```
┌─────────┐    ┌─────────────┐    ┌─────────┐    ┌─────────┐    ┌──────────┐
│  Source │───▶│    Loader   │───▶│Converter│───▶│   IR    │───▶│   Pass   │
│ (文件等) │    │ (原始数据)   │    │(结构化)  │    │ (AST等) │    │(处理转换) │
└─────────┘    └─────────────┘    └─────────┘    └─────────┘    └────┬─────┘
                                                                      │
                                           ┌──────────────────────────┘
                                           │
                                           ▼
┌─────────┐    ┌─────────────┐    ┌─────────┐    ┌─────────┐    ┌──────────┐
│  Target │◀───│   Emitter   │◀───│Serializer│◀───│   IR    │◀───│   Pass   │
│(文件/控制台)│   │  (输出)     │    │ (序列化) │    │ (Bytecode)│    │(处理转换) │
└─────────┘    └─────────────┘    └─────────┘    └─────────┘    └──────────┘
```

---

## Crate 结构

### 1. kaubo-orchestrator（编排引擎）

```rust
// orchestrator/src/lib.rs

pub mod config;      // 配置解析
pub mod pipeline;    // 流水线管理
pub mod component;   // 组件协议定义
pub mod loader;      // Loader trait
pub mod converter;   // Converter trait
pub mod pass;        // Pass trait
pub mod emitter;     // Emitter trait
pub mod event;       // 事件系统
pub mod context;     // 执行上下文

// 核心编排器
pub struct Orchestrator {
    config: Config,
    loaders: LoaderRegistry,
    converters: ConverterRegistry,
    passes: PassRegistry,
    emitters: EmitterRegistry,
    pipeline_engine: PipelineEngine,
    event_bus: EventBus,
}

impl Orchestrator {
    pub fn new(config: Config) -> Self;
    
    // 注册各类组件
    pub fn register_loader(&mut self, loader: Box<dyn Loader>);
    pub fn register_converter(&mut self, converter: Box<dyn Converter>);
    pub fn register_pass(&mut self, pass: Box<dyn Pass>);
    pub fn register_emitter(&mut self, emitter: Box<dyn Emitter>);
    
    pub fn run(&self) -> Result<ExecutionResult, Error>;
}
```

### 2. 组件 Crate

```rust
// loader-file/src/lib.rs

use kaubo_orchestrator::{Loader, ComponentMetadata, RawData};

pub struct FileLoader;

impl Component for FileLoader {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata {
            name: "file",
            version: "1.0.0",
            kind: ComponentKind::Loader,
        }
    }
}

impl Loader for FileLoader {
    fn load(&self, source: &Source) -> Result<RawData, Error> {
        let path = source.path()?;
        let content = std::fs::read_to_string(path)?;
        Ok(RawData::Text(content))
    }
}

pub fn register() -> Box<dyn Loader> {
    Box::new(FileLoader)
}
```

```rust
// pass-parser/src/lib.rs

use kaubo_orchestrator::{Pass, ComponentMetadata, IR, Context};

pub struct ParserPass;

impl Component for ParserPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata {
            name: "parser",
            version: "1.0.0",
            kind: ComponentKind::Pass,
        }
    }
}

impl Pass for ParserPass {
    fn input_ir(&self) -> IRKind { IRKind::Source }
    fn output_ir(&self) -> IRKind { IRKind::Ast }
    
    fn run(&self, ir: IR, ctx: &Context) -> Result<IR, Error> {
        let source = ir.as_source()?;
        let ast = parse(source)?;
        Ok(IR::Ast(ast))
    }
}

pub fn register() -> Box<dyn Pass> {
    Box::new(ParserPass)
}
```

### 3. kaubo-cli（命令行工具）

```rust
// cli/src/main.rs

use kaubo_orchestrator::Orchestrator;

// 导入所有组件
use kaubo_loader_file as file_loader;
use kaubo_loader_stdin as stdin_loader;
use kaubo_converter_kaubo_source as source_converter;
use kaubo_pass_parser as parser;
use kaubo_pass_checker as checker;
use kaubo_pass_generator as generator;
use kaubo_emitter_file as file_emitter;

fn main() {
    let config = load_config();
    let mut orchestrator = Orchestrator::new(config);
    
    // 注册 Loaders
    orchestrator.register_loader(file_loader::register());
    orchestrator.register_loader(stdin_loader::register());
    
    // 注册 Converters
    orchestrator.register_converter(source_converter::register());
    
    // 注册 Passes
    orchestrator.register_pass(parser::register());
    orchestrator.register_pass(checker::register());
    orchestrator.register_pass(generator::register());
    
    // 注册 Emitters
    orchestrator.register_emitter(file_emitter::register());
    
    orchestrator.run().unwrap();
}
```

---

## 统一组件协议

```rust
// orchestrator/src/component/mod.rs

/// 组件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Loader,      // 加载器
    Converter,   // 转换器
    Pass,        // 处理遍
    Emitter,     // 生成器
}

/// 组件元数据
pub struct ComponentMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub kind: ComponentKind,
    pub description: Option<&'static str>,
}

/// 能力声明
pub struct Capabilities {
    /// 输入格式
    pub inputs: Vec<&'static str>,
    /// 输出格式
    pub outputs: Vec<&'static str>,
}

/// 基础组件接口
pub trait Component: Send + Sync {
    fn metadata(&self) -> ComponentMetadata;
    fn capabilities(&self) -> Capabilities;
}
```

---

## 配置结构

```json
{
  "name": "my-app",
  "version": "0.1.0",
  "compiler": {
    "pipeline": {
      "from": "source",
      "to": "run"
    },
    "components": {
      "loaders": [
        { "name": "file" },
        { "name": "stdin" }
      ],
      "converters": [
        { "name": "kaubo-source", "from": ["file", "stdin"], "to": "source" },
        { "name": "ast-json", "from": "file", "to": "ast" }
      ],
      "passes": [
        { "name": "parser", "from": "source", "to": "ast" },
        { "name": "checker", "from": "ast", "to": "typed_ast" },
        { "name": "generator", "from": "typed_ast", "to": "bytecode" },
        { "name": "runner", "from": "bytecode", "to": "result" }
      ],
      "emitters": [
        { "name": "json", "from": "ast", "format": "json" },
        { "name": "binary", "from": "bytecode", "format": "binary" },
        { "name": "file-writer", "format": "binary" },
        { "name": "console-writer", "format": "text" }
      ]
    },
    "input": {
      "loader": "file",
      "path": "main.kaubo",
      "converter": "kaubo-source"
    },
    "emit": {
      "bytecode": {
        "emitter": "binary",
        "writer": "file-writer",
        "options": { "path": "dist/app.kaubod" }
      }
    }
  }
}
```

---

## 项目结构

```
kaubo-project/
├── Cargo.toml
├── crates/
│   ├── orchestrator/           # 编排引擎
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── component.rs    # 组件基础 trait
│   │       ├── loader.rs       # Loader trait
│   │       ├── converter.rs    # Converter trait
│   │       ├── pass.rs         # Pass trait
│   │       ├── emitter.rs      # Emitter trait
│   │       └── ...
│   │
│   ├── loaders/                # 加载器组件
│   │   ├── file/
│   │   └── stdin/
│   │
│   ├── converters/             # 转换器组件
│   │   ├── kaubo-source/
│   │   └── ast-json/
│   │
│   ├── passes/                 # 处理遍组件
│   │   ├── parser/
│   │   ├── checker/
│   │   ├── generator/
│   │   └── runner/
│   │
│   ├── emitters/               # 生成器组件
│   │   ├── json/
│   │   ├── binary/
│   │   ├── file-writer/
│   │   └── console-writer/
│   │
│   └── cli/                    # 命令行工具
│       └── src/main.rs
│
└── docs/
    └── architecture-v2.md
```

---

## 优势

| 方面 | 优势 |
|------|------|
| **语义精准** | 每种组件类型有准确的术语，符合编译器领域惯例 |
| **职责清晰** | Loader/Converter/Pass/Emitter 各司其职 |
| **可扩展** | 新功能只需添加对应类型的组件 crate |
| **可组合** | 不同类型组件可以灵活组合 |
| **类型安全** | Rust trait 保证接口一致性 |

---

## 术语对照表

| 概念 | 术语 | 说明 |
|------|------|------|
| 输入读取 | **Loader** | 从外部来源读取原始数据 |
| 格式解析 | **Converter** | 原始数据 ↔ 结构化 IR |
| 编译处理 | **Pass** | IR → IR 的转换（parser/checker/generator）|
| 输出生成 | **Emitter** | IR → 输出格式 → 目标 |

这样设计术语是否更清晰合理？
