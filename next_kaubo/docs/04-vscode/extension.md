# VSCode 扩展

## 安装

```bash
cd vscode-extension
bash build-wasm.sh      # 构建 WASM（Node.js target）
npm run package          # 打包 .vsix
code --install-extension kaubo-0.1.0.vsix
```

## 功能

### 语法高亮

基于 TextMate 语法的 `.kaubo` 文件高亮，覆盖全部 22 个关键字、3 种字面量、运算符和注释。

**文件：** `syntaxes/kaubo.tmLanguage.json`

### 代码片段

17 个常用代码片段：

| 触发词 | 内容 |
|--------|------|
| `var` | `var ${1:name} = ${2:value};` |
| `vart` | 带类型标注的变量声明 |
| `if` / `ife` / `ifel` | if / if-else / if-elif-else |
| `while` | while 循环 |
| `for` | for-in 循环 |
| `fn` / `fnt` | Lambda / 带类型 Lambda |
| `struct` | struct 定义 |
| `impl` | impl 方法块 |
| `call` | 函数调用 |
| `ret` | return 语句 |
| `import` / `importa` / `from` | 模块导入 |
| `print` | print 语句 |

### 实时诊断

打开 `.kaubo` 文件 → WASM `diagnose()` 自动运行 → 错误标红。

**触发时机：**
- 打开文件
- 编辑文件
- 保存文件

**实现文件：** `src/extension.js`

### 语言配置

- 行注释：`//`
- 块注释：`/* */`
- 括号配对：`{}` / `[]` / `()`
- 自动闭合：`{` `[` `(` `"` `'`
- 缩进规则：`{` 后增加缩进

**文件：** `language-configuration.json`

## 扩展结构

```
vscode-extension/
├── package.json                    # 扩展清单
├── language-configuration.json     # 括号/注释/缩进配置
├── syntaxes/kaubo.tmLanguage.json  # TextMate 语法高亮
├── snippets/kaubo.json             # 代码片段
├── src/extension.js                # WASM 诊断激活
├── wasm/                           # Node.js WASM 构建产物
├── build-wasm.sh                   # 构建脚本
├── tests/grammar.test.js           # 79 tests
└── kaubo-0.1.0.vsix                # 打包产物
```

## 开发

### 构建 WASM

```bash
bash build-wasm.sh
```

等价于：

```bash
cd ../next_kaubo
wasm-pack build crates/kaubo-wasm \
  --target nodejs \
  --out-dir ../vscode-extension/wasm \
  --out-name kaubo_wasm
```

### 运行测试

```bash
node --test tests/grammar.test.js
```

### 打包

```bash
npm run package  # 生成 kaubo-0.1.0.vsix
```

## 与 Web Playground 的关系

两者共享同一个 WASM 核心 `kaubo-wasm`：

| 功能 | 构建方式 | 输出目标 |
|------|---------|---------|
| Web Playground | `--target web` | ES 模块，浏览器加载 |
| VSCode 扩展 | `--target nodejs` | CommonJS，Node.js 加载 |

WASM 导出的 `diagnose()` 函数在两个平台提供一致的结构化错误输出。
