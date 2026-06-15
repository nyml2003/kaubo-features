# 主题配色

## 预设方案（5 种）

### Material Dark（默认）

经典 Material 深色主题，紫色关键字 + 绿色字符串。

| Token | 色值 |
|-------|------|
| 背景 | `#1a1a2e` |
| 关键字 | `#c792ea` |
| 字符串 | `#c3e88d` |
| 注释 | `#676e95` |
| 数字 | `#f78c6c` |
| 原子量 | `#ff5370` |
| 运算符 | `#89ddff` |
| 标识符 | `#a6accd` |

### Nord

冷色调蓝灰北欧风。

| Token | 色值 |
|-------|------|
| 背景 | `#2e3440` |
| 关键字 | `#81a1c1` |
| 字符串 | `#a3be8c` |
| 注释 | `#7b88a1` |
| 数字 | `#b48ead` |
| 原子量 | `#bf616a` |
| 运算符 | `#88c0d0` |
| 标识符 | `#d8dee9` |

### Gruvbox Dark

复古暖色风格。

| Token | 色值 |
|-------|------|
| 背景 | `#282828` |
| 关键字 | `#d3869b` |
| 字符串 | `#b8bb26` |
| 注释 | `#928374` |
| 数字 | `#d65d0e` |
| 原子量 | `#fe8019` |
| 运算符 | `#83a598` |
| 标识符 | `#ebdbb2` |

### Min Light

简洁浅色主题。

| Token | 色值 |
|-------|------|
| 背景 | `#fafafa` |
| 关键字 | `#7c3aed` |
| 字符串 | `#059669` |
| 注释 | `#94a3b8` |
| 数字 | `#b45309` |
| 原子量 | `#dc2626` |
| 运算符 | `#0284c7` |
| 标识符 | `#334155` |

### High Contrast

最高对比度，黑底白字。

| Token | 色值 |
|-------|------|
| 背景 | `#000000` |
| 关键字 | `#c586c0` |
| 字符串 | `#6a9955` |
| 注释 | `#808080` |
| 数字 | `#ce9178` |
| 原子量 | `#d16969` |
| 运算符 | `#4ec9b0` |
| 标识符 | `#d4d4d4` |

## CSS 变量体系

主题通过 `applyTheme(element, theme)` 设置 13 个 CSS 自定义属性到 `document.documentElement`：

| CSS 变量 | 用途 |
|----------|------|
| `--kb-bg` | 编辑器/工具栏背景 |
| `--kb-gutter` | 行号列/侧边栏背景 |
| `--kb-active-line` | 当前行高亮 |
| `--kb-selection` | 选中区域边框色（UI 元素用） |
| `--kb-cursor` | 光标颜色 |
| `--kb-text` | 文本颜色 |
| `--kb-keyword` | 关键字 token |
| `--kb-number` | 数字 token |
| `--kb-string` | 字符串 token |
| `--kb-comment` | 注释 token |
| `--kb-identifier` | 标识符 token |
| `--kb-atom` | 原子量 token |
| `--kb-operator` | 运算符 token |

选中区域使用独立的半透明蓝色蒙层（`rgba(100, 144, 255, 0.35)`），不与主题色耦合。

## 切换主题

Setting 面板 → Color Theme 下拉选择。选择立即生效，通过 localStorage 持久化（key: `kaubo-theme`）。

## 实现文件

| 文件 | 内容 |
|------|------|
| `src/themes/types.ts` | `KauboTheme` 类型定义 |
| `src/themes/presets.ts` | 5 个预设配色数据 |
| `src/themes/apply.ts` | `applyTheme()` 设 CSS 变量 |
| `src/App.tsx` | `createEffect` 中调用 `applyTheme()` |
