# 快速开始

## 安装

### 从源码构建

```bash
git clone https://github.com/kaubo-lang/kaubo.git
cd kaubo
cargo build --release
```

### CLI 使用

```bash
# 编译并运行
./target/release/kaubo examples/hello/main.kaubo

# 仅编译
./target/release/kaubo compile examples/hello/main.kaubo

# 词法分析
./target/release/kaubo lex examples/hello/main.kaubo

# 语法分析
./target/release/kaubo parse examples/hello/main.kaubo

# 类型检查
./target/release/kaubo check examples/hello/main.kaubo
```

### Web Playground

```bash
cd gui/packages/app
pnpm install
pnpm dev
# 打开 http://localhost:3000
```

需要先构建 WASM：

```bash
cd ../..
wasm-pack build crates/kaubo-wasm --target web --out-dir gui/packages/wasm/pkg --out-name kaubo_wasm
```

### VSCode 扩展

```bash
cd vscode-extension
bash build-wasm.sh
npm run package
code --install-extension kaubo-0.1.0.vsix
```

## 第一个程序

创建文件 `hello.kaubo`：

```kaubo
print "Hello, World!";
```

运行：

```bash
kaubo hello.kaubo
```

输出：

```
Hello, World!
```

## 下一步

- [语法参考](./01-language/syntax.md) — 完整的 Kaubo 语法
- [编程指南](./01-language/programming-guide.md) — 变量、函数、控制流
- [Playground 开发](./03-playground/development.md) — 本地运行与开发
