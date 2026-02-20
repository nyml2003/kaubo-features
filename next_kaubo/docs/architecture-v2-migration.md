# Kaubo 架构 V2 迁移方案

## 当前项目结构

```
kaubo-workspace/
├── kaubo-config      # 配置处理
├── kaubo-vfs         # 虚拟文件系统
├── kaubo-log         # 日志系统
├── kaubo-core        # 核心：lexer, parser, compiler, VM
├── kaubo-api         # API：执行编排（需要重构）
└── kaubo-cli         # CLI：命令行接口
```

## 目标架构

保留现有 crate，将 `kaubo-api` 升级为编排引擎，将 `kaubo-core` 拆分为组件：

```
kaubo-workspace/
├── kaubo-config              # 保留：配置解析
├── kaubo-vfs                 # 保留：虚拟文件系统
├── kaubo-log                 # 保留：日志系统
│
├── kaubo-orchestrator        # 新增：由 kaubo-api 升级
│   ├── src/
│   │   ├── lib.rs
│   │   ├── component.rs      # 组件基础 trait
│   │   ├── loader.rs         # Loader trait
│   │   ├── converter.rs      # Converter trait
│   │   ├── pass.rs           # Pass trait
│   │   ├── emitter.rs        # Emitter trait
│   │   ├── registry.rs       # 组件注册表
│   │   ├── pipeline.rs       # 流水线执行
│   │   └── context.rs        # 执行上下文
│   └── Cargo.toml
│
├── kaubo-passes/             # 新增：Pass 集合 workspace
│   ├── Cargo.toml            # workspace 定义
│   ├── pass-lexer/           # 从 kaubo-core 拆分
│   ├── pass-parser/          # 从 kaubo-core 拆分
│   ├── pass-checker/         # 从 kaubo-core 拆分
│   ├── pass-generator/       # 从 kaubo-core 拆分
│   └── pass-runner/          # 从 kaubo-core 拆分
│
├── kaubo-loaders/            # 新增：Loader 集合
│   ├── loader-file/          # 文件加载
│   └── loader-stdin/         # 标准输入加载
│
├── kaubo-converters/         # 新增：Converter 集合
│   └── converter-ast-json/   # AST JSON 转换
│
├── kaubo-emitters/           # 新增：Emitter 集合
│   ├── emitter-ast-json/     # AST JSON 输出
│   ├── emitter-bytecode/     # 字节码输出
│   └── emitter-writer/       # 文件/控制台写入
│
└── kaubo-cli                 # 保留：组装所有组件
    └── src/main.rs           # 注册所有组件
```

---

## 迁移步骤

### 第一步：创建 kaubo-orchestrator

由 `kaubo-api` 升级而来：

```rust
// kaubo-orchestrator/src/lib.rs

pub mod component;
pub mod loader;
pub mod converter;
pub mod pass;
pub mod emitter;
pub mod registry;
pub mod pipeline;
pub mod context;
pub mod event;

use std::sync::Arc;

/// 编排引擎
pub struct Orchestrator {
    config: kaubo_config::Config,
    vfs: Arc<kaubo_vfs::Vfs>,
    log: Arc<kaubo_log::Logger>,
    
    // 组件注册表
    loaders: registry::LoaderRegistry,
    converters: registry::ConverterRegistry,
    passes: registry::PassRegistry,
    emitters: registry::EmitterRegistry,
    
    // 流水线
    pipeline: pipeline::PipelineEngine,
}

impl Orchestrator {
    pub fn new(config: kaubo_config::Config) -> Self {
        let vfs = Arc::new(kaubo_vfs::Vfs::new());
        let log = Arc::new(kaubo_log::Logger::new());
        
        Self {
            config,
            vfs: vfs.clone(),
            log: log.clone(),
            loaders: registry::LoaderRegistry::new(),
            converters: registry::ConverterRegistry::new(),
            passes: registry::PassRegistry::new(),
            emitters: registry::EmitterRegistry::new(),
            pipeline: pipeline::PipelineEngine::new(vfs, log),
        }
    }
    
    // 注册组件
    pub fn register_loader(&mut self, loader: Box<dyn loader::Loader>) {
        self.loaders.register(loader);
    }
    
    pub fn register_converter(&mut self, converter: Box<dyn converter::Converter>) {
        self.converters.register(converter);
    }
    
    pub fn register_pass(&mut self, pass: Box<dyn pass::Pass>) {
        self.passes.register(pass);
    }
    
    pub fn register_emitter(&mut self, emitter: Box<dyn emitter::Emitter>) {
        self.emitters.register(emitter);
    }
    
    /// 执行流水线
    pub fn run(&self, request: ExecutionRequest) -> Result<ExecutionResult, Error> {
        self.pipeline.execute(
            request,
            &self.loaders,
            &self.converters,
            &self.passes,
            &self.emitters,
        )
    }
}
```

### 第二步：拆分 kaubo-core 为 Pass 组件

从 `kaubo-core` 中提取 lexer、parser、checker、generator、runner：

```rust
// kaubo-passes/pass-lexer/src/lib.rs

use kaubo_orchestrator::{Pass, ComponentMetadata, ComponentKind, Capabilities};
use kaubo_orchestrator::pass::{PassContext, Input, Output};

pub struct LexerPass;

impl kaubo_orchestrator::Component for LexerPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata {
            name: "lexer",
            version: "0.1.0",
            kind: ComponentKind::Pass,
            description: Some("Tokenize source code into tokens"),
        }
    }
    
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            inputs: vec!["source"],
            outputs: vec!["tokens"],
        }
    }
}

impl Pass for LexerPass {
    fn run(&self, input: Input, ctx: &PassContext) -> Result<Output, PassError> {
        let source = input.as_source()?;
        let tokens = tokenize(source)?;
        Ok(Output::new(tokens))
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, PassError> {
    // 从 kaubo-core 迁移 lexer 逻辑
    todo!()
}

pub fn register() -> Box<dyn Pass> {
    Box::new(LexerPass)
}
```

### 第三步：创建 Loaders

```rust
// kaubo-loaders/loader-file/src/lib.rs

use kaubo_orchestrator::{Loader, ComponentMetadata, ComponentKind, Capabilities};
use kaubo_orchestrator::loader::{Source, RawData};

pub struct FileLoader;

impl kaubo_orchestrator::Component for FileLoader {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata {
            name: "file",
            version: "0.1.0",
            kind: ComponentKind::Loader,
            description: Some("Load source from file system"),
        }
    }
    
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            inputs: vec![],
            outputs: vec!["file-content"],
        }
    }
}

impl Loader for FileLoader {
    fn load(&self, source: &Source) -> Result<RawData, LoaderError> {
        let path = source.path()?;
        let content = std::fs::read_to_string(path)?;
        Ok(RawData::Text(content))
    }
}

pub fn register() -> Box<dyn Loader> {
    Box::new(FileLoader)
}
```

### 第四步：更新 kaubo-cli

```rust
// kaubo-cli/src/main.rs

use kaubo_orchestrator::Orchestrator;

// 加载所有组件
use kaubo_loader_file as file_loader;
use kaubo_loader_stdin as stdin_loader;

use kaubo_pass_lexer as lexer;
use kaubo_pass_parser as parser;
use kaubo_pass_checker as checker;
use kaubo_pass_generator as generator;
use kaubo_pass_runner as runner;

use kaubo_emitter_bytecode as bytecode_emitter;
use kaubo_emitter_writer as writer_emitter;

fn main() {
    let config = load_config();
    
    let mut orchestrator = Orchestrator::new(config);
    
    // 注册 Loaders
    orchestrator.register_loader(file_loader::register());
    orchestrator.register_loader(stdin_loader::register());
    
    // 注册 Passes（从 kaubo-core 拆分出来的）
    orchestrator.register_pass(lexer::register());
    orchestrator.register_pass(parser::register());
    orchestrator.register_pass(checker::register());
    orchestrator.register_pass(generator::register());
    orchestrator.register_pass(runner::register());
    
    // 注册 Emitters
    orchestrator.register_emitter(bytecode_emitter::register());
    orchestrator.register_emitter(writer_emitter::register());
    
    // 执行
    let request = ExecutionRequest::from_args();
    match orchestrator.run(request) {
        Ok(result) => println!("Success: {:?}", result),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

---

## 组件协议

```rust
// kaubo-orchestrator/src/component.rs

/// 组件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Loader,      // 加载器：读取输入
    Converter,   // 转换器：格式转换
    Pass,        // 处理遍：编译处理
    Emitter,     // 生成器：输出
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
    pub inputs: Vec<&'static str>,
    pub outputs: Vec<&'static str>,
}

/// 基础组件接口
pub trait Component: Send + Sync {
    fn metadata(&self) -> ComponentMetadata;
    fn capabilities(&self) -> Capabilities;
}
```

```rust
// kaubo-orchestrator/src/loader.rs

pub trait Loader: Component {
    fn load(&self, source: &Source) -> Result<RawData, LoaderError>;
}

pub struct Source {
    pub kind: SourceKind,
    pub path: Option<std::path::PathBuf>,
}

pub enum SourceKind {
    File,
    Stdin,
}

pub enum RawData {
    Text(String),
    Bytes(Vec<u8>),
}
```

```rust
// kaubo-orchestrator/src/pass.rs

pub trait Pass: Component {
    fn run(&self, input: Input, ctx: &PassContext) -> Result<Output, PassError>;
}

pub struct Input {
    pub data: IR,
    pub metadata: HashMap<String, Value>,
}

pub struct Output {
    pub data: IR,
    pub metadata: HashMap<String, Value>,
}

pub struct PassContext {
    pub config: Arc<kaubo_config::Config>,
    pub vfs: Arc<kaubo_vfs::Vfs>,
    pub log: Arc<kaubo_log::Logger>,
}

pub enum IR {
    Source(String),
    Tokens(Vec<Token>),
    Ast(AstNode),
    TypedAst(TypedAstNode),
    Bytecode(Bytecode),
    Result(ExecutionResult),
}
```

```rust
// kaubo-orchestrator/src/emitter.rs

pub trait Emitter: Component {
    fn emit(&self, output: &Output, target: &Target) -> Result<(), EmitterError>;
}

pub struct Target {
    pub kind: TargetKind,
    pub path: Option<std::path::PathBuf>,
}

pub enum TargetKind {
    File,
    Stdout,
    Stderr,
}
```

---

## 保留的 crate

| Crate | 修改 | 说明 |
|-------|------|------|
| `kaubo-config` | 保留 | 配置解析，可能被 orchestrator 使用 |
| `kaubo-vfs` | 保留 | 虚拟文件系统，被 orchestrator 使用 |
| `kaubo-log` | 保留 | 日志系统，被所有组件使用 |
| `kaubo-core` | **逐步拆分** | lexer/parser/compiler/VM 拆分到 pass-* |
| `kaubo-api` | **重命名为 orchestrator** | 升级为编排引擎 |
| `kaubo-cli` | 保留 | 注册所有组件并执行 |

---

## 迁移顺序建议

1. **创建 kaubo-orchestrator**（从 kaubo-api 复制并扩展）
2. **创建组件 trait**（Loader/Converter/Pass/Emitter）
3. **拆分第一个 Pass**（如 lexer）验证架构
4. **逐步迁移其他 Pass**
5. **创建 Loaders 和 Emitters**
6. **更新 kaubo-cli** 使用新架构
7. **移除旧的 kaubo-core**（所有逻辑迁移完成后）

---

## 兼容性考虑

- 保持 `kaubo-config`、`kaubo-vfs`、`kaubo-log` API 不变
- `kaubo-cli` 入口不变，内部实现改为使用 orchestrator
- 逐步迁移，每个 Pass 可以独立测试
