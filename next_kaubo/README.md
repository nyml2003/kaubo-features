# Kaubo Programming Language

一门静态类型的编译型脚本语言。

## 快速开始

```bash
cargo build --release
./target/release/kaubo examples/hello/main.kaubo
```

## 语法速览

```kaubo
var name = "Kaubo";
var age = 25;

if age >= 18 {
    print "Adult";
}

var add = |a, b| -> int { return a + b; };
print add(2, 3);

struct Point { x: int, y: int }
var p = Point { x: 10, y: 20 };
print p.x;

var items = [1, 2, 3];
for var item in items { print item; }
```

## 文档

完整文档见 [docs/](./docs/README.md)：

| 章节 | 内容 |
|------|------|
| [快速开始](./docs/00-getting-started.md) | 安装、编译、运行 |
| [语言手册](./docs/01-language/README.md) | 语法、类型、内置函数、示例 |
| [架构手册](./docs/02-architecture/README.md) | 编译器、运行时、WASM |
| [Playground](./docs/03-playground/README.md) | Web 编辑器功能与主题 |
| [VSCode 扩展](./docs/04-vscode/extension.md) | 安装与使用 |
| [贡献指南](./docs/05-contributing.md) | 开发环境与规范 |

## Web Playground

```bash
cd gui/packages/app
pnpm dev
# http://localhost:3000
```
