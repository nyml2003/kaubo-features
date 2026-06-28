# 01 — 词法与语法分析

**管线位置**：Source text → Tokens → AST

## 输入 / 输出

```
source: &str → Lexer.tokenize() → Vec<Token> → Parser.parse() → Module (AST)
```

## 核心类型

| 类型 | 所在 | 说明 |
|------|------|------|
| `Lexer` | `kaubo-syntax/src/lexer.rs:7` | 逐字符扫描，生成 token 流 |
| `Token` | `kaubo-syntax/src/token.rs` | token 种类 + 词面 + 位置 |
| `Parser` | `kaubo-syntax/src/parser.rs:13` | 递归下降，`Vec<Token>` → `Module` |
| `Module` | `kaubo-ast/src/lib.rs` | 顶层 AST 根节点：`stmts: Vec<Stmt>` |
| `Stmt` | `kaubo-ast/src/lib.rs` | 语句枚举（Const/Struct/Fn/Import/Export/…） |
| `Expr` | `kaubo-ast/src/lib.rs` | 表达式枚举（Literal/Binary/Call/Lambda/…） |

## 关键 API

```rust
// kaubo-syntax/src/parser.rs
impl Parser {
    pub fn new(source: &str) -> Self;           // 内部调用 Lexer
    pub fn register_struct_name(&mut self, name: &str);  // 跨模块 struct 识别
    pub fn parse(&mut self) -> Result<Module, ParseError>;
}
```

## 设计要点

- **递归下降**，无 parser generator。每个 `Stmt` / `Expr` 有对应的 `parse_*` 方法。
- `Parser` 在构造时预处理 struct/variant 名称集合，避免解析时反复回溯。
- `register_struct_name()` 是模块系统的侵入点——导入 struct 后 parser 能识别跨文件的 `Name { ... }` 字面量。
- AST 节点（`Stmt`/`Expr`）定义在独立 crate `kaubo-ast` 中，与 parser 解耦。

## 代码位置

```
kaubo-syntax/src/
├── lib.rs          # re-export
├── lexer.rs        ~700 行
├── parser.rs       ~2400 行
├── token.rs        token 种类定义
└── ast.rs          re-export kaubo-ast 类型

kaubo-ast/src/
└── lib.rs          Module / Stmt / Expr / Pattern 等 AST 节点
```
