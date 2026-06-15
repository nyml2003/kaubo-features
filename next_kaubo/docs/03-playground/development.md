# Playground 本地开发

## 前置依赖

- Node.js 18+
- pnpm 9+
- wasm-pack（`cargo install wasm-pack`）
- Rust toolchain

## 构建 WASM

```bash
# 从项目根目录
cd next_kaubo
wasm-pack build crates/kaubo-wasm \
  --target web \
  --out-dir gui/packages/wasm/pkg \
  --out-name kaubo_wasm
```

## 启动开发服务器

```bash
cd gui
pnpm install

cd packages/app
pnpm dev
# 打开 http://localhost:3000
```

## 运行测试

```bash
cd gui/packages/app

# 类型检查
pnpm exec tsc --noEmit

# 代码规范
pnpm exec eslint src/ --max-warnings=0

# 单元测试
pnpm exec vitest run

# E2E 测试
cd ../../e2e
pnpm exec playwright test
```

## 代码质量检查（一键）

```bash
cd gui/packages/app
./node_modules/.bin/tsc --noEmit && \
./node_modules/.bin/eslint src/ --max-warnings=0 && \
./node_modules/.bin/vitest run
```

## 构建生产版本

```bash
cd gui/packages/app
pnpm build
# 产物：dist/
```

产物包括：
- `index.html` — 入口
- `assets/kaubo_wasm_bg-*.wasm` — WASM 二进制
- `assets/index-*.css` — 样式
- `assets/index-*.js` — 应用代码

## 部署

生产环境使用 nginx 托管静态文件：

```
server {
    root /var/www/kaubo/dist;
    location / {
        try_files $uri /index.html;
    }
}
```

WASM MIME 类型必须在 nginx 中配置：

```
types {
    application/wasm wasm;
}
```

## 项目结构

```
gui/packages/app/
├── src/
│   ├── main.tsx                    # 入口
│   ├── App.tsx                     # 根组件（主题/布局）
│   ├── store/app.ts                # 全局状态（信号 + WASM 调用）
│   ├── hooks/useKaubo.ts          # WASM 加载钩子
│   ├── editor/                     # CodeMirror 扩展
│   │   ├── kauboLang.ts            # 高亮 + lint + 补全 + 悬停
│   │   ├── kauboAutocomplete.ts    # 补全源
│   │   ├── kauboLang.test.ts       # 30 tests
│   │   └── kauboAutocomplete.test.ts # 14 tests
│   ├── themes/                     # 主题系统
│   │   ├── types.ts                # 类型定义
│   │   ├── presets.ts              # 5 个预设
│   │   ├── apply.ts                # CSS 变量应用
│   │   ├── types.test.ts
│   │   ├── presets.test.ts
│   │   └── apply.test.ts
│   ├── components/
│   │   ├── Editor/                 # CodeMirror 编辑器
│   │   ├── Toolbar/                # 工具栏
│   │   ├── OutputPanel/            # 输出面板
│   │   ├── ErrorOverlay/           # 错误弹窗
│   │   ├── Examples/               # 示例侧边栏
│   │   └── Settings/               # 配置抽屉
│   ├── examples.ts                 # 8 个内置示例
│   ├── examples.test.ts            # 34 tests
│   └── lib/logger.ts               # 调试日志
├── package.json
├── tsconfig.json
├── vite.config.ts
├── vitest.config.ts
└── eslint.config.mjs
```
