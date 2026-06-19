# 架构

目标读者：维护编译器、运行时、Web app 或编辑器集成的开发者。

## 当前状态

Kaubo 是一个 monorepo。核心实现位于 Rust workspace，外层有两个主要 UI 适配层：Web Playground 和 VSCode 扩展。

当前类似生产路径的执行链路是线性的，由 `kaubo-driver` 负责。历史上未接入主路径的 pipeline/module/vfs/log 实验 crate 已移除，避免 workspace 同时维护多套入口。

## 分层

期望的依赖方向是：

```text
source text
  -> token/syntax
  -> AST
  -> infer/semantic facts
  -> CPS/IR
  -> VM
  -> adapters
```

适配层应该调用稳定 API 和 DTO，不应该重复实现编译器逻辑。

## 核心 Crate

- `kaubo-token`：token kind 和 token 级数据。
- `kaubo-ast`：源码级语法树数据结构。
- `kaubo-syntax`：lexer 和 parser。
- `kaubo-infer`：类型推断和类型错误。
- `kaubo-cps`：CPS module/function/block/instruction 定义。
- `kaubo-ir`：AST 到 CPS lowering、flatten、二进制编码和 pass。
- `kaubo-vm`：寄存器 VM、堆对象、native 函数和执行逻辑。
- `kaubo-driver`：当前直接 compile/run 编排层。
- `kaubo-language-service`：编辑器侧 semantic tokens 和 completion。
- `kaubo-web-api`：与 WASM-facing 代码共享的 JSON/DTO 辅助逻辑。
- `kaubo-wasm`：wasm-bindgen 导出。

## Ops 工具

发布、部署、覆盖率和 benchmark 统一放在 `next_kaubo/ops/`：

- `ops/release/publish.py`：构建 Web app、打包并发布 GitHub Release。
- `ops/deploy/deploy.py`：从 GitHub Release 下载产物并部署到 nginx。
- `ops/quality/coverage.py`：运行 Rust workspace 覆盖率报告。
- `ops/benchmark/runner.py`：运行 Kaubo/Python/Rust benchmark。

## 适配层边界

Web 和 VSCode 不应该自己解析编译器内部状态。目标形态是：

```text
adapter -> kaubo-wasm -> language service / driver -> compiler/runtime
```

目前 Web app 已经通过 WASM 消费 `semantic_tokens` 和 `complete`。VSCode 当前消费 WASM diagnostics，但还没有暴露与 Web app 相同的 semantic token provider。

## 当前执行路径

`kaubo-driver::compile_source` 当前执行编译路径：

1. 使用 `kaubo_syntax::parser::Parser` 解析源码。
2. 运行 `kaubo_infer::infer_module`。
3. 使用 `kaubo_ir::cps_build::build_module` 构建 CPS。
4. 使用 `kaubo_ir::flatten::flatten_module` 展平 CPS。
5. 通过 `kaubo_ir::pass` 运行常量折叠。

`kaubo-driver::run_module` 随后把 CPS module 加载进 `kaubo-vm`，并把最后一个函数作为入口执行。

## 设计方向

当前缺少的架构层是稳定的语义事实层。它未来应该负责 symbols、scopes、definitions、references、type facts 和 member resolution。`kaubo-language-service`、lowering、diagnostics、编辑器能力都应该消费这层事实，而不是各自重建 heuristic。
