# Kaubo 文档中心

## 快速入口

- **[快速开始](./00-getting-started.md)** — 安装、编译、运行第一个 Kaubo 程序
- **[语言手册](./01-language/README.md)** — 语法、类型、内置函数、编程指南
- **[架构手册](./02-architecture/README.md)** — 编译器流水线、运行时 VM、WASM 绑定
- **[Playground 手册](./03-playground/README.md)** — Web 编辑器功能、主题、配置
- **[VSCode 扩展](./04-vscode/extension.md)** — 安装、语法高亮、实时诊断
- **[贡献指南](./05-contributing.md)** — 如何参与开发

## 目录

| 章节 | 内容 |
|------|------|
| `01-language/` | 语法参考、类型系统、内置函数、示例、编程指南 |
| `02-architecture/` | Crate 布局、编译器流水线、VM 运行时、模块系统、WASM 接口 |
| `03-playground/` | 编辑器功能、主题配色、配置面板、本地开发 |
| `04-vscode/` | VSCode 扩展安装与使用 |
| `archive/old/` | 历史文档归档（47 个原始文件） |

## 项目入口

| 入口 | 路径 |
|------|------|
| Web Playground | `gui/packages/app/`（`pnpm dev`） |
| VSCode 扩展 | `vscode-extension/` |
| CLI 编译器 | `kaubo-cli/` |
| Rust 核心 crates | `crates/` |
| WASM 绑定 | `crates/kaubo-wasm/` |
