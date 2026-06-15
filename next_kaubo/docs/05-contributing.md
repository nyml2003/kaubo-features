# 贡献指南

## 开发环境

### 前置依赖

- Rust 2021 edition（`rustup` 安装）
- Node.js 18+ / pnpm 9+
- wasm-pack（`cargo install wasm-pack`）

### 仓库结构

```
next_kaubo/
├── crates/                     # Rust 核心 crates
│   ├── kaubo-ir/               # Value/OpCode/Chunk/VM 类型定义
│   ├── kaubo-compiler/         # Lexer/Parser/TypeChecker/Codegen
│   ├── kaubo-runtime/          # VM 执行/Stdlib/二进制格式
│   ├── kaubo-pipeline/         # Stage trait + Pipeline 框架
│   └── kaubo-wasm/             # WASM 绑定（lex/diagnose/hover/compile/run）
├── kaubo-cli/                  # CLI 入口（clap 参数解析）
├── kaubo-log/                  # 结构化日志
├── kaubo-config/               # 配置数据结构
├── kaubo-vfs/                  # 虚拟文件系统
├── gui/packages/app/           # Web Playground（Vite + SolidJS + CodeMirror 6）
├── gui/packages/wasm/          # WASM npm 包包装
├── vscode-extension/           # VSCode 扩展
└── docs/                       # 文档
```

## 本地开发

### Rust 核心

```bash
cargo build                    # 构建
cargo test                     # 测试
cargo check -p kaubo-compiler  # 检查单个 crate
```

### Web Playground

```bash
cd gui/packages/app
pnpm dev                       # 开发服务器

# 代码质量
pnpm tsc --noEmit              # 类型检查
pnpm eslint src/               # 代码规范
pnpm vitest run                # 单元测试
```

### WASM 重建

```bash
# Web Playground 用
wasm-pack build crates/kaubo-wasm --target web \
  --out-dir gui/packages/wasm/pkg --out-name kaubo_wasm

# VSCode 扩展用
wasm-pack build crates/kaubo-wasm --target nodejs \
  --out-dir vscode-extension/wasm --out-name kaubo_wasm
```

## 提交规范

- 提交前通过 `tsc --noEmit && eslint src/ --max-warnings=0 && vitest run`
- Rust 侧通过 `cargo test`
- 提交信息用中文

## 文档

- 新功能必须有对应文档
- 设计决策记录在对应架构文档中
- 历史文档归档在 `docs/archive/old/`
