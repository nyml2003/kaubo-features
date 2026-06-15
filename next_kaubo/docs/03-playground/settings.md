# 配置面板

## 入口

Toolbar 最右侧齿轮图标（⚙）→ 右侧抽屉面板滑出。

## 可配置项

| 选项 | 控件 | 默认值 | localStorage key |
|------|------|--------|-----------------|
| 主题 | 5 项下拉 | `material-dark` | `kaubo-theme` |
| 缩进宽度 | 2 / 4 toggle | `4` | `kaubo-tabsize` |
| 字号 | 12px / 14px / 16px toggle | `14px` | `kaubo-fontsize` |

## 重置

"Restore Defaults" 按钮一键恢复所有选项到默认值，同时清空 localStorage。

## 实现

| 文件 | 内容 |
|------|------|
| `src/components/Settings/Settings.tsx` | 右侧抽屉面板组件 |
| `src/components/Settings/Settings.module.css` | 滑入动画 + 暗色适配样式 |
| `src/store/app.ts` | `settingsOpen` / `fontSize` signals + localStorage |

## 持久化

所有配置通过 `localStorage` 存储，页面刷新后自动恢复。

```typescript
// 主题
localStorage.setItem("kaubo-theme", "material-dark");

// 缩进
localStorage.setItem("kaubo-tabsize", "4");

// 字号
localStorage.setItem("kaubo-fontsize", "14");
```
