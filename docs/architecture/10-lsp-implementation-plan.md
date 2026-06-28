# Phase 2 实施计划：LSP 编排层独立化

## 目标

把 `kaubo-language-service` 从纯 token 扫描升级为基于 `SemanticArtifact` 的语义感知 LSP 层。

## 当前状态 vs 目标

| 能力 | 当前 | 目标 |
|------|------|------|
| `semantic_tokens` | token 分类（keyword/number/type/…） | AST 节点类型 + token fallback |
| `completions` | 内置方法表 + struct 字段 | `SemanticArtifact.symbols` + 已有逻辑 |
| `hover` | token 种类描述 | `SemanticArtifact.symbols` → 类型信息 |
| `go_to_definition` | 无 | `SemanticArtifact.references` → 跳转到定义 |
| 编排层 | 无（每次调用独立解析） | `LspCoordinator`：Frontend→Semantic，缓存结果 |

## 架构

```
kaubo-language-service
├── lsp_coordinator.rs    ★ 新增：LspCoordinator
├── semantic_tokens.rs    重写：AST 节点 → token role
├── hover.rs              ★ 新增：类型信息 hover
├── goto_def.rs           ★ 新增：跳转到定义
├── completions.rs        重写：语义补全 + token fallback
└── lib.rs                导出新 API，保留旧 API 兼容
```

### LspCoordinator

```rust
struct LspCoordinator {
    source: String,
    module: Option<Module>,
    semantic: Option<SemanticArtifact>,
}

impl LspCoordinator {
    fn on_change(&mut self, source: &str) -> Result<(), BuildError>;
    fn hover(&self, offset: usize) -> Option<HoverInfo>;
    fn goto_def(&self, offset: usize) -> Option<Span>;
    fn complete(&self, offset: usize) -> Vec<CompletionItem>;
    fn semantic_tokens(&self, source: &str) -> Vec<SemanticToken>;
}
```

`on_change` 跑 Frontend→Semantic，其他方法只做查询。与编译器 Coordinator 共享协议层（`Stage` trait），但接线独立。

## 各能力详细设计

### 1. Go-to-definition

```
输入: source + 光标位置
  1. FrontendStage → Module
  2. SemanticStage → SemanticArtifact (含 references: HashMap<Span, SymbolKey>)
  3. 根据光标位置在 references 中查找 → 得到 SymbolKey
  4. 在 symbols 中查找 SymbolKey → 得到 SymbolDef (含定义 span)
输出: 定义的源码位置 (line, col, span)
```

Fallback：AST 中找不到时返回 null。

### 2. Hover

```
输入: source + 光标位置
  1. 同上拿到 SemanticArtifact
  2. 根据光标位置查 references → SymbolKey → SymbolDef
  3. 返回 SymbolDef.kind + SymbolDef.ty
输出: HoverInfo { kind: SymbolKind, type: String, description: String }
```

Fallback：语义结果不可用时降级到现有 token 分类描述。

### 3. Completion 增强

```
输入: source + 光标位置
  1. 现有 token 补全（struct 字段、builtin 方法）保留
  2. 增补 SemanticArtifact.symbols 中的同作用域变量/函数
  3. 如果光标在 Member 访问后（`expr.`），查 expr 类型 → 列出可用方法/字段
输出: Vec<CompletionItem>（合并语义补全 + token 补全）
```

### 4. Semantic Tokens

```
输入: source
  1. 现有 token 扫描（classify_token）保留作为 fallback
  2. SemanticArtifact.symbols → 按 span 转换为 token role
  3. AST 覆盖优先：符号的 span 区域用 SymbolKind 映射 → role
  4. 未被覆盖的区域用 token fallback
输出: Vec<SemanticToken>
```

SymbolKind → role 映射：

| SymbolKind | Token Role |
|------------|-----------|
| Const | `identifier` |
| Var | `identifier` |
| Function | `function` |
| Struct | `type` |
| Interface | `type` |

## WASM 适配

`kaubo-wasm` 新增导出：

```rust
#[wasm_bindgen]
pub fn goto_def(source: &str, offset: usize) -> String;
```

`hover` 改为调用 `LspCoordinator::hover`，`semantic_tokens` 和 `complete` 改为走 `LspCoordinator`。

## 改动范围

| 文件 | 改动 | 行数 |
|------|------|------|
| `kaubo-language-service/src/lsp_coordinator.rs` | 新增 | ~60 |
| `kaubo-language-service/src/semantic_tokens.rs` | 重写 | ~40 |
| `kaubo-language-service/src/hover.rs` | 新增 | ~30 |
| `kaubo-language-service/src/goto_def.rs` | 新增 | ~30 |
| `kaubo-language-service/src/completions.rs` | 重写 | ~40 |
| `kaubo-language-service/src/lib.rs` | 导出 + 旧 API 兼容 | ~20 |
| `kaubo-language-service/Cargo.toml` | 依赖 kaubo-driver | ~5 |
| `kaubo-wasm/src/lib.rs` | 新增 goto_def 导出；hover/complete/semantic 改用 LspCoordinator | ~30 |
| Tests | LspCoordinator 单元测试 + WASM 集成测试 | ~50 |

**合计 ~300 行**。不改编译器核心。

## 不改什么

- 编译器 Coordinator
- Parser、Infer、CPS、VM
- `kaubo-driver` 的协议层（只消费，不改）
- 现有 token-based 逻辑（保留为 fallback）

## 执行步骤

1. **Cargo.toml** — `kaubo-language-service` 加 `kaubo-driver` 依赖
2. **LspCoordinator** — 新建，`on_change` → `FrontendStage` + `SemanticStage`
3. **goto_def** — 基于 `SemanticArtifact.references`
4. **hover** — 基于 `SemanticArtifact.symbols`
5. **completions** — 语义 + token 合并
6. **semantic_tokens** — AST 覆盖 + token fallback
7. **lib.rs** — 导出新 API，保留旧 API
8. **WASM** — 新增 `goto_def`，切换 hover/complete/semantic 到 LspCoordinator
9. **Tests** — 全量回归 + 新增 LSP 测试

## 验收标准

- `python kaubo-ops ci` 全绿
- `goto_def("const x = 42; x;", 位置指向x)` 返回第一个 x 的位置
- `hover("const x: Int64 = 42;", 位置指向x)` 返回 `{ kind: "const", type: "Int64" }`
- `complete("const p = Point { x: 1, y: 2 }; p.", 光标在"."后)` 返回 `["x", "y"]`
- WASM 层 `goto_def` / `hover` 返回有效 JSON
