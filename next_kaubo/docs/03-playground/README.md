# Playground 总览

Web Playground 是基于 Vite + SolidJS + CodeMirror 6 的在线编辑器。

**路径：** `gui/packages/app/`

## 文档导航

| 文档 | 内容 |
|------|------|
| [编辑器功能](./editor-features.md) | 语法高亮、补全、悬停、折叠、诊断 |
| [主题配色](./themes.md) | 5 种预设、CSS 变量体系 |
| [配置面板](./settings.md) | 主题/缩进/字号/重置 + localStorage 持久化 |
| [本地开发](./development.md) | pnpm/vite/wasm-pack 构建流程 |

## 技术栈

| 技术 | 用途 |
|------|------|
| Vite 6 | 构建工具 / 开发服务器 |
| SolidJS 1.9 | 响应式 UI |
| CodeMirror 6 | 代码编辑器 |
| TypeScript 5.7 | 类型安全 |
| pnpm 9+ | 包管理 |
| Vitest 4 | 单元测试 |
| Playwright 1.60 | E2E 测试 |

## 布局

```
┌─────────────────────────────────────────────┐
│ Toolbar  [☰] Kaubo  Compile Run  [ready] [⚙] │
├────────┬────────────────────────────────────┤
│        │                                    │
│ Examples│         Editor                     │
│  (8)   │    (CodeMirror 6)                 │
│        │                                    │
│        ├────────────────────────────────────┤
│        │         Output                     │
└────────┴────────────────────────────────────┘
                            Settings drawer →
```
