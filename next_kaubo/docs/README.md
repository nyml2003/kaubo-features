# Kaubo 编译器 - 项目文档

## 1. 项目概述

**Kaubo** 是一个现代编程语言的编译器前端实现，采用 Rust 编写，包含完整的词法分析器和语法分析器。

| 属性 | 值 |
|------|-----|
| 名称 | next_kaubo |
| 版本 | 0.1.0 |
| 语言 | Rust (Edition 2024) |
| 架构 | 状态机驱动的 Lexer + Pratt Parser |

### 1.1 项目结构

```
src/
├── main.rs                 # 程序入口
├── compiler/               # 编译器模块
│   ├── lexer/              # Kaubo 语言专用词法分析器
│   │   ├── builder.rs      # Lexer 构建器
│   │   └── token_kind.rs   # Token 类型定义
│   ├── parser/             # 语法分析器
│   │   ├── parser.rs       # Pratt 解析器实现
│   │   ├── expr.rs         # 表达式 AST
│   │   ├── stmt.rs         # 语句 AST
│   │   ├── module.rs       # 模块 AST
│   │   ├── error.rs        # 错误类型
│   │   └── utils.rs        # 工具函数
│   └── ir/                 # 中间表示（预留）
└── kit/                    # 通用工具包
    ├── lexer/              # 可复用的词法分析框架
    │   ├── c_lexer.rs      # 通用 Lexer 实现
    │   ├── types.rs        # Token/Coordinate 类型
    │   └── state_machine/  # 状态机框架
    └── ring_buffer/        # 环形缓冲区
```

---

## 2. 技术架构

### 2.1 词法分析器

采用**状态机驱动**架构，支持流式输入和 UTF-8。

**核心流程**:
```
输入字节流 → 环形缓冲区 → UTF-8解码 → 状态机竞争匹配 → Token输出
```

**支持的 Token 类型**:

| 类别 | Token |
|------|-------|
| 关键字 | `var`, `if`, `else`, `elif`, `while`, `for`, `return`, `true`, `false`, `null`, `and`, `or`, `not`, `async`, `await` 等 26 个 |
| 字面量 | `LiteralInteger`, `LiteralString` |
| 运算符 | `==`, `!=`, `>=`, `<=`, `>`, `<`, `+`, `-`, `*`, `/`, `=`, `.`, `\|` 等 |
| 分隔符 | `(`, `)`, `{`, `}`, `[`, `]`, `,`, `;`, `:` |

### 2.2 语法分析器

采用 **Pratt 解析算法**（Top-down operator precedence）。

**优先级表**（从高到低）:

| 优先级 | 运算符 | 说明 |
|--------|--------|------|
| 450 | `not` | 一元逻辑非 |
| 400 | `.` | 成员访问 |
| 300 | `*`, `/` | 乘除 |
| 200 | `+`, `-` | 加减 |
| 100 | `==`, `!=`, `>`, `<`, `>=`, `<=` | 比较 |
| 80 | `and` | 逻辑与 |
| 70 | `\|` | 管道 |
| 60 | `or` | 逻辑或 |
| 50 | `=` | 赋值 |

**AST 结构**:

```rust
// 表达式
pub enum ExprKind {
    LiteralInt(LiteralInt),       // 42
    LiteralString(LiteralString), // "hello"
    LiteralTrue, LiteralFalse, LiteralNull,
    LiteralList(LiteralList),     // [1, 2, 3]
    Binary(Binary),               // a + b
    Unary(Unary),                 // -a, not b
    Grouping(Grouping),           // (a + b)
    VarRef(VarRef),               // x
    FunctionCall(FunctionCall),   // foo(a, b)
    Assign(Assign),               // x = 5
    Lambda(Lambda),               // |x| { x + 1 }
    MemberAccess(MemberAccess),   // obj.field
}

// 语句
pub enum StmtKind {
    Expr(ExprStmt),               // a + b;
    Empty,                        // ;
    Block(BlockStmt),             // { ... }
    VarDecl(VarDeclStmt),         // var x = 5;
    If(IfStmt),                   // if/elif/else
    While(WhileStmt),             // while (cond) { ... }
    For(ForStmt),                 // for (i) in (list) { ... }
    Return(ReturnStmt),           // return value;
}
```

---

## 3. 支持的语言特性

### 3.1 当前支持

```kaubo
// 变量声明
var x = 5;
var s = "hello";
var flag = true;

// 算术表达式
var a = 1 + 2 * 3;
var b = (10 - 5) / 2;

// 比较和逻辑
var c = a > b and b < 10;
var d = not c;

// 条件语句
if (a > b) {
    print("a is bigger");
} elif (a == b) {
    print("equal");
} else {
    print("b is bigger");
}

// 循环
while (i < 10) {
    i = i + 1;
}

for (item) in (list) {
    print(item);
}

// 列表
var nums = [1, 2, 3];

// 函数（Lambda）
var add = |x, y| {
    return x + y;
};

// 成员访问
var len = list.length();
```

### 3.2 已知限制

| 限制 | 说明 | 状态 |
|------|------|------|
| 字符串无转义 | `"hello\n"` 中的 `\n` 不会转义 | 待修复 |
| 赋值受限 | 只支持 `x = 5`，不支持 `obj.x = 5` | 待修复 |
| 无索引访问 | 不支持 `arr[0]` | 待修复 |
| 无浮点数 | 只支持整数 | 待添加 |

---

## 4. 测试策略

采用**混合测试方案**：内联单元测试 + 集成测试分离。

### 4.1 内联单元测试（主要）

位于 `src/` 各模块底部，测试私有函数和内部逻辑。

```rust
// src/compiler/lexer/builder.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keyword_machine() {
        // 直接测试内部函数
    }
}
```

**适用场景**:
- 状态机的内部转换逻辑
- Pratt 解析器的优先级计算
- 边界情况（空输入、超长 token 等）
- 需要访问私有函数的工具方法

### 4.2 集成测试（辅助）

位于 `tests/` 目录，测试公共 API 和端到端场景。

```rust
// tests/integration_tests.rs
#[test]
fn test_parse_hello_world() {
    let code = r#"var x = "hello";"#;
    let ast = parse(code).unwrap();
    // 验证 AST 结构
}
```

**适用场景**:
- 完整文件解析
- 端到端编译流程
- 示例程序验证

### 4.3 测试目录结构

```
tests/
├── integration_tests.rs      # 主要集成测试
└── fixtures/                 # 测试代码文件
    ├── basic.kaubo
    ├── expressions.kaubo
    └── statements.kaubo
```

**决策理由**:
- Lexer/Parser 内部逻辑复杂，需要测试私有函数
- 内联测试方便修改时同步更新
- 集成测试验证整体功能正确性
- 符合 Rust 社区惯例

---

## 5. 开发路线图

### Phase 1: 测试与修复 (当前 - 进行中)

**目标**: 建立完整测试套件，修复所有已知 Bug，确保前端稳定可靠。

**本周任务清单**:
```markdown
- [ ] 创建 tests/ 目录结构
- [ ] 补全 Lexer 测试（所有 Token 类型识别）
- [ ] 补全 Parser 测试（表达式、语句、边界情况）
- [ ] 修复 parser.rs:336 无效 check 调用
- [ ] 修复 get_associativity 硬编码返回 true
- [ ] 修复 whitespace_machine 错误匹配换行
```

**Phase 1 完成标准**:
- [x] 测试覆盖率 (行覆盖) > 80% → **85.43%** ✅
  > 注: cargo-tarpaulin 分支覆盖率尚未实现，仅支持行覆盖率
- [x] `cargo test` 全绿 → **105 个测试通过** ✅
- [ ] 无编译器警告
- [ ] CI 通过

**Phase 2 启动条件**: Phase 1 完成 + 字节码方案设计评审

### Phase 2: 字节码后端（设计中）

**方向**: 基于栈的字节码虚拟机（Stack-based VM），而非 Tree-walking Interpreter。

**设计原则**:
- 二进制字节码，可序列化/缓存
- 属性访问按索引偏移（类似结构体 field offset）
- 为后续优化（JIT、AOT）预留空间
- 替换成本低于 Tree-walking → Bytecode 的迁移

**核心组件（预留）**:
```
src/
└── runtime/
    ├── bytecode/         # 字节码定义
    │   ├── mod.rs        # Opcode 枚举
    │   ├── encoder.rs    # 字节码编码
    │   └── decoder.rs    # 字节码解码
    ├── compiler.rs       # AST → Bytecode 编译器
    ├── vm.rs             # 虚拟机执行引擎
    ├── value.rs          # 运行时值（对象头 + 数据）
    └── heap.rs           # 内存管理/GC 预留
```

**关键设计决策**（待详细设计）:
| 决策项 | 方向 | 说明 |
|--------|------|------|
| 属性访问 | 索引偏移 | 编译期确定 field index，运行时直接偏移访问 |
| 调用约定 | 栈帧 | 基于栈的参数传递和返回值 |
| 值表示 |  tagged pointer 或 NaN boxing | 预留 |

> **当前状态**: 方案预留，具体设计待 Phase 1 完成后讨论确定。

**Phase 2 启动条件**:
- [ ] Phase 1 测试覆盖 > 80%
- [ ] 所有 P0 Bug 修复
- [ ] 字节码方案设计评审通过

### Phase 3: 功能迭代

在闭环基础上按需添加功能：

| 功能 | 工作量 | 优先级 |
|------|--------|--------|
| 字符串转义 | 1 天 | 高 |
| 成员/索引赋值 | 2 天 | 高 |
| 浮点数 | 2 天 | 中 |
| 闭包 | 3 天 | 中 |
| 复合赋值 | 1 天 | 低 |
| 错误位置报告 | 3 天 | 中 |

---

## 5. 已知问题

### 5.1 Bug 清单

| 问题 | 位置 | 影响 | 修复方案 |
|------|------|------|---------|
| 无效 check 调用 | `parser.rs:336` | 无实际效果 | 删除或改为 expect |
| 结合性硬编码 | `utils.rs:23` | 所有运算符左结合 | 根据运算符返回 |
| whitespace 冲突 | `builder.rs:125` | 换行被 whitespace 匹配 | 改用 `c == ' '` |

### 5.2 技术债务

| 问题 | 说明 | 优化方案 |
|------|------|---------|
| 动态分发 | `Box<dyn Fn(char) -> bool>` | 改为函数指针或枚举 |
| 内存分配 | 每个 Token 都新建 String | 考虑字符串 interning |
| 无错误位置 | ParserError 无行列号 | 添加 Coordinate |

---

## 6. 使用方式

### 6.1 构建

```bash
cargo build --release
```

### 6.2 运行

```bash
# 解析文件
cargo run -- assets/a.txt

# 运行测试
cargo test
```

### 6.3 示例代码

```kaubo
// assets/a.txt
var a = 3;

var func = |x|{
    return x;
};

a = 5;
```

---

## 7. 未来演进

```
Phase 1: 测试与修复（当前）
    ↓
Phase 2: 字节码 VM（设计中）
    ↓
Phase 3: 功能完善（闭包、浮点、转义、GC 等）
    ↓
Phase 4: 性能优化（JIT、内联缓存等）
    ↓
Phase 5: 可选 - WASM/LLVM 后端
```

**后端方案演进**:
- **Phase 2** 直接采用字节码 VM，跳过 Tree-walking 阶段
- 属性访问采用索引偏移，为后续优化打基础
- 预留 JIT/AOT 扩展接口

---

## 8. 参考资料

- [Pratt Parsing](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html)
- [Crafting Interpreters](https://craftinginterpreters.com/)

---

*文档版本: 1.0*  
*最后更新: 2026-02-07*
