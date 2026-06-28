# 05 — 模块系统

**管线位置**：Entry file → 依赖发现 → 按序编译 → 链接 → 全局 CpsModule

## 两阶段架构

模块系统分为**图发现**（语法层）和**图执行**（编译层）两个独立阶段：

```
阶段 1：ModuleGraph::build(entry, loader)
  → DFS 遍历 import 链 → 拓扑排序 → 循环依赖检测
  → 产出：order + sources + imports（纯语法，不碰类型/CPS）

阶段 2：ModuleCompiler::compile_all(graph)
  → 按拓扑序逐个编译每个模块（依赖已就绪）
  → LinkStage::link(built, order)
  → 多模块 CPS 链接为全局 CpsModule
```

## 核心组件

| 组件 | 文件 | 行数 | 职责 |
|------|------|------|------|
| `ModuleGraph` | `kaubo-driver/src/module_graph.rs` | ~260 | DFS 发现 + 拓扑排序 + 循环检测 |
| `ModuleCompiler` | `kaubo-driver/src/module_compiler.rs` | ~430 | 按拓扑序编译 + 传递闭包哈希缓存失效 |
| `ModuleLoader` | `kaubo-driver/src/module_loader.rs` | ~190 | trait + `FileLoader` + `MemLoader` |
| `LinkStage` | `kaubo-driver/src/link_stage.rs` | ~400 | 全局索引映射唯一生产者 |
| `ExportTable` | `kaubo-driver/src/export_table.rs` | ~210 | 导出/导入表数据结构 |

## 数据流

```
Entry File (main.kb)
  │
  ▼
ModuleGraph::build          ← 轻量 parser 只提取 import 语句
  ├── DFS("main.kb")        → 发现依赖 "./math.kb" "./types.kb"
  ├── 循环检测               → stack.contains(path) → CircularImport 错误
  └── 拓扑排序               → order: ["types.kb", "math.kb", "main.kb"]
  │
  ▼
ModuleCompiler::compile_all
  ├── for path in order:       ← 叶子在前，依赖已就绪
  │     parse → InferModuleWithImports → CpsBuild → ExportTable
  │     缓存：content_hash ≠ cached_hash → 重编译
  │
  └── LinkStage::link(built, order)
        ├── 构建全局索引映射（func_remap / struct_remap / const_remap）
        ├── 遍历所有模块：CallExternal(handle) → Call(global_idx)
        ├── 重映射 struct_id（所有 NewStruct/GetField/...）
        ├── 合并 vtable / constants / global_structs
        └── 填充 symbol_map: (module, name) → global_idx
  │
  ▼
全局 CpsModule → VM Execute
```

## 缓存失效

传递闭包哈希：模块 Key = `sha256(source) + 所有直接依赖的 content_hash`。依赖变化 → 父 Key 变化 → 自动重编译。仅在被依赖模块变化时触发重编译。

## 虚拟文件系统

`ModuleLoader` 内部持有 `Box<dyn VirtualFileSystem>`（来自 `kaubo-vfs`）：

```rust
pub trait VirtualFileSystem {
    fn read(&self, path: &str) -> Result<String, VfsError>;
}
```

- `FsVfs`：`std::fs` + 路径安全检查（canonicalize 后必须在 root 下）
- `MemVfs`：内存 `HashMap<String, String>` — WASM / 测试

只读不写，不列目录，不缓存（缓存是 ModuleCompiler 的职责）。

## 单文件兼容

单文件路径不触发模块逻辑：`import` 不存在时 `ImportTable` 为空，`LinkStage` 退化为透传。`compile_source` / `run_source` 保持向后兼容。

## 当前限制

- 不支持通配符 import（`import * from "..."`）
- 不支持 re-export（`export { x } from "..."`）
- 没有包管理器
- 不考虑动态 import（运行时路径解析）

## 代码位置

```
kaubo-driver/src/
├── module_graph.rs       ModuleGraph::build + DFS + 拓扑排序
├── module_compiler.rs    ModuleCompiler::compile_all + 缓存失效
├── module_loader.rs      ModuleLoader trait + FileLoader + MemLoader
├── link_stage.rs         LinkStage::link（全局索引映射 + CallExternal 重映射）
└── export_table.rs       ExportTable / ImportTable / ExportEntry / ResolvedImport

kaubo-vfs/src/
└── lib.rs                VirtualFileSystem trait + FsVfs + MemVfs (~140 行)
```
