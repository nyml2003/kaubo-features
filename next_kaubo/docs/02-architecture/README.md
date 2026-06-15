# 架构总览

## Crate 布局

```
kaubo-ir          (零依赖, ~3500 行)
  ↑
  ├── kaubo-compiler  (Lexer/Parser/TypeChecker/Codegen, ~6000 行)
  └── kaubo-runtime   (VM/Stdlib/二进制格式, ~8000 行)
  ↑
kaubo-cli          (CLI 胶水, ~130 行)

辅助 crates:
  kaubo-pipeline  (Stage trait + Pipeline 组合器, ~200 行)
  kaubo-log       (结构化日志, WASM/no_std 兼容, ~2350 行)
  kaubo-config    (纯数据配置, ~150 行)
  kaubo-vfs       (虚拟文件系统, ~1500 行)
  kaubo-wasm      (WASM 绑定, ~427 行)
```

## 文档导航

| 文档 | 内容 |
|------|------|
| [架构全景](./overview.md) | 数据流、设计决策、扩展点 |
| [编译器流水线](./compiler-pipeline.md) | Lexer → Parser → TypeChecker → Codegen |
| [运行时](./runtime.md) | VM 栈式架构、内联缓存、协程、Stdlib |
| [模块系统](./module-system.md) | import/export、多文件编译、二进制格式 |
| [WASM 绑定](./wasm-bindings.md) | lex/diagnose/hover/compile/run 接口 |
