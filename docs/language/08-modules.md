# 模块语法

## 当前状态

模块系统已完整实现（Phase 3b ✅）。`import`/`export` 语义完整，支持跨文件类型推断、CPS 链接和缓存失效。

## import

```kaubo
import { sqrt, sin } from "./math.kb";
```

从其他 Kaubo 模块导入具名导出。路径相对于当前文件解析。

## export

```kaubo
export const answer = 42;
export const add = |a: Int64, b: Int64| -> Int64 { return a + b; };
export struct Point { x: Int64, y: Int64 };
```

`export` 标记顶层声明为公开，可被其他模块导入。

## 实现架构

```
Entry File
  → ModuleGraph::build (DFS + 拓扑排序 + 循环依赖检测)
  → ModuleCompiler::compile_all (按拓扑序编译 + 缓存失效)
  → LinkStage::link (多模块 CPS 链接：函数表合并、func_remap、struct_remap、CallExternal 重映射)
  → VM Execute
```

### 关键组件

| 组件 | 文件 | 职责 |
|------|------|------|
| ModuleGraph | `kaubo-driver/src/module_graph.rs` | DFS + 拓扑排序 + 循环检测 |
| ModuleCompiler | `kaubo-driver/src/module_compiler.rs` | 按序编译 + 传递哈希缓存失效 |
| ModuleLoader | `kaubo-driver/src/module_loader.rs` | 路径解析 + 文件加载（FileLoader/MemLoader） |
| LinkStage | `kaubo-driver/src/link_stage.rs` | 多模块 CPS 链接 |
| ExportTable/ImportTable | `kaubo-driver/src/export_table.rs` | 导出/导入表数据结构 |
| kaubo-vfs | `kaubo-vfs/` | VirtualFileSystem trait + FsVfs + MemVfs |

## 当前限制

- 不支持通配符 import（`import * from "..."`）
- 不支持 re-export（`export { x } from "..."`）
- 没有包管理器
- 没有动态 import（运行时解析路径）

## 设计文档

详见 [架构总览](../architecture/README.md)。
