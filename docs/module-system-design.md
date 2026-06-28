# Phase 3b：模块系统 — 设计文档

## 1. 目标与范围

```kaubo
// math.kb
export const PI = 3.14159;
export const add = |a: Int64, b: Int64| -> Int64 { return a + b; };

// main.kb
import { PI, add } from "./math.kb";
const r = add(2, 3);
print(r.to_string());   // 5
```

**本期做**：单层 import/export、跨模块类型推断、CPS 链接、缓存失效、循环依赖检测。
**本期不做**：通配符 import、re-export、包管理器、平台无关文件 IO（预留 `kaubo-vfs`）。

## 2. 核心数据结构

### 2.1 导出表 (Export Table)

每个模块编译完成后产生导出表——被导入方只暴露这张表，不暴露内部细节。

```rust
/// 单个导出项——按种类区分，携带完整的跨模块信息。
enum ExportEntry {
    Const {
        name: String,
        ty: Type,
        /// 源模块 const_idx
        const_idx: usize,
    },
    Function {
        name: String,
        ty: Type,
        /// 源模块 func_idx
        func_idx: usize,
    },
    Struct {
        name: String,
        /// ★ 完整的字段列表（跨模块边界不丢失）
        fields: Vec<(String, Type)>,
        /// 源模块 struct_id（LinkStage 重映射为全局 ID）
        struct_id: usize,
    },
    Interface {
        name: String,
        methods: Vec<(String, Vec<(String, Type)>, Option<Type>)>,
    },
}

/// 整个模块的导出表
struct ExportTable {
    /// 源模块路径
    source_path: String,
    /// 导出项列表（按声明顺序）
    entries: Vec<ExportEntry>,
    /// 本模块的导入表（LinkStage 用它解析本模块的 CallExternal）
    import_table: ImportTable,
    /// 源模块的 CpsModule（LinkStage 需要）
    cps_module: CpsModule,
}
```

### 2.2 导入表 (Import Table)

Parser 解析 `import` 语句时产生原始导入请求。Infer 阶段将其解析为具体引用。

```rust
/// 原始导入请求（Parser 产物，不做路径解析）
struct RawImport {
    /// 导入名（源码中的名字）
    local_name: String,
    /// 来源路径（源码字面量，未解析）
    source_path: String,
}

/// 解析后的导入引用（Infer 阶段产物）
struct ResolvedImport {
    /// 本地名
    local_name: String,
    /// 来源路径
    source_path: String,
    /// 被导入条目（从 ExportTable 复制）
    entry: ExportEntry,
}

/// 整个模块的导入表——双重索引
struct ImportTable {
    /// 按句柄索引（CPS/Link 阶段用）：handle → ResolvedImport
    entries: Vec<ResolvedImport>,
    /// 按本地名查找（Infer 阶段注入类型时用）：local_name → handle
    by_name: HashMap<String, usize>,
}
```

### 2.3 符号表扩展

`SemanticArtifact` 新增 `exports` 字段——记录本模块哪些符号是公开的。

```rust
struct SemanticArtifact {
    // ... 现有字段 ...
    /// 本模块的导出符号集合
    exports: HashSet<SymbolId>,
}
```

## 3. 构建流程

```
                    ┌─────────────┐
                    │  Entry File │
                    └──────┬──────┘
                           │ parse
                    ┌──────▼──────┐
                    │  RawImports │  ["PI", "add"] from "./math.kb"
                    └──────┬──────┘
                           │ resolve paths, recursively build deps
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌─────────┐  ┌─────────┐  ┌─────────┐
        │ math.kb │  │ lib.kb  │  │ ...     │
        │ Build   │  │ Build   │  │         │
        └────┬────┘  └────┬────┘  └────┬────┘
             │            │            │
        ┌────▼────┐  ┌────▼────┐       │
        │ExportTbl│  │ExportTbl│       │
        └────┬────┘  └────┬────┘       │
             │            │            │
             └────────────┼────────────┘
                          │ resolve imports against export tables
                   ┌──────▼──────┐
                   │ ImportTable │
                   └──────┬──────┘
                          │ inject types into env, infer main module
                   ┌──────▼──────┐
                   │ Semantic    │
                   │ (with types │
                   │  of imports)│
                   └──────┬──────┘
                          │ CPS Build (leaves "holes" for imports)
                   ┌──────▼──────┐
                   │  CpsModule  │
                   │  (per-module│
                   │   unlinked) │
                   └──────┬──────┘
                          │
              ┌───────────┼───────────┐
              │  LinkStage            │
              │  - 合并函数表          │
              │  - 重映射 func_idx     │
              │  - 重映射 struct_id    │
              │  - 解析外部调用        │
              └───────────┬───────────┘
                          │
                   ┌──────▼──────┐
                   │  Linked     │
                   │  CpsModule  │  → VM Execute
                   └─────────────┘
```

## 4. 各阶段详细设计

### 4.1 两阶段分离架构

**核心原则：图发现（语法）和图执行（编译）是独立阶段。** 图发现只做轻量 Parser 提取 import 语句——不涉及类型检查、CPS、缓存。图执行利用拓扑序按固定顺序编译——零递归，签名唯一。

#### 阶段 1：`ModuleGraph` — 纯图构建

```rust
struct ModuleGraph {
    order: Vec<String>,                           // 拓扑序（叶子→根）
    sources: HashMap<String, String>,              // 路径 → 源码
    imports: HashMap<String, Vec<RawImport>>,      // 路径 → 导入列表
    deps: HashMap<String, Vec<String>>,            // 路径 → 直接依赖
}

impl ModuleGraph {
    fn build(entry: &str, loader: &dyn ModuleLoader) -> Result<Self> {
        let mut graph = Self { order: vec![], sources: HashMap::new(),
            imports: HashMap::new(), deps: HashMap::new() };
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        graph.dfs(entry, loader, &mut visited, &mut stack)?;
        graph.order.reverse();  // 后序反转 = 叶子在前
        Ok(graph)
    }

    fn dfs(&mut self, path: &str, loader: &dyn ModuleLoader,
           visited: &mut HashSet<String>, stack: &mut Vec<String>) -> Result<()> {
        if stack.contains(&path.to_string()) {
            return Err(CircularImport { cycle: stack[stack.iter()
                .position(|p| p == path).unwrap()..].to_vec() }.into());
        }
        if visited.contains(path) { return Ok(()); }
        stack.push(path.to_string());

        let source = loader.read(path)?;
        let module = FrontendStage.execute(&source)?;  // ★ 仅语法解析
        let raw = collect_raw_imports(&module);

        self.sources.insert(path.to_string(), source);
        self.imports.insert(path.to_string(), raw.clone());

        for imp in &raw {
            let (dep, _) = loader.resolve(path, &imp.source_path)?;
            self.deps.entry(path.to_string()).or_default().push(dep.clone());
            self.dfs(&dep, loader, visited, stack)?;
        }

        stack.pop();
        visited.insert(path.to_string());
        self.order.push(path.to_string());
        Ok(())
    }
}
```

#### 阶段 2：`ModuleCompiler` — 按序编译

```rust
struct ModuleCompiler {
    built: HashMap<String, ModuleCacheEntry>,
    loader: Box<dyn ModuleLoader>,
    ctx: BuildContext<'static>,  // ★ 事件系统 + 缓存上下文
}

impl ModuleCompiler {
    fn compile_all(&mut self, graph: &ModuleGraph) -> Result<CpsModule> {
        // ★ 按拓扑序编译——每个模块编译时依赖已全部就绪
        for path in &graph.order {
            let source = &graph.sources[path];
            let raw = graph.imports.get(path).map_or(&[][..], |v| v);

            if self.is_cache_fresh(path, source, raw)? { continue; }

            let import_table = self.resolve_imports(path, raw, &self.built)?;
            let module = FrontendStage.execute(source, &self.ctx)?;  // 只此一次
            let (semantic, _) = infer_module_with_imports(&module, &import_table)?;
            let cps = CpsBuildStage::build_with_symbol_map(&module, &semantic)?;
            let export_table = build_export_table(path, &semantic, &cps, import_table)?;

            self.built.insert(path.to_string(), ModuleCacheEntry {
                export_table, content_hash: sha256(source.as_bytes()),
                dep_hashes: self.snapshot_dep_hashes(path, raw)?,
            });
        }

        // LinkStage 内部自己构建 remap 表——ModuleCompiler 不关心全局索引
        LinkStage::link(&self.built, &graph.order)
    }

    /// 收集所有直接依赖的 content_hash 快照（用于缓存失效判断）
    fn snapshot_dep_hashes(&self, path: &str, raw: &[RawImport]) -> Result<HashMap<String, String>> {
        let mut hashes = HashMap::new();
        for imp in raw {
            let (dep, _) = self.loader.resolve(path, &imp.source_path)?;
            hashes.insert(dep, self.built[&dep].content_hash.clone());
        }
        Ok(hashes)
    }

    fn is_cache_fresh(&self, path: &str, source: &str, raw: &[RawImport]) -> Result<bool> {
        let hash = sha256(source.as_bytes());
        let Some(entry) = self.built.get(path) else { return Ok(false) };
        if entry.content_hash != hash { return Ok(false); }
        for imp in raw {
            let (dep, _) = self.loader.resolve(path, &imp.source_path)?;
            if entry.dep_hashes.get(&dep) != Some(&self.built[&dep].content_hash) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
```

**为什么分离能根治**：

| 旧问题 | 分离后 |
|--------|--------|
| 重复解析 | 图构建解析一次（语法），编译阶段复用 cached `Module` |
| 签名不一致 | 只有一个 `resolve_imports`，依赖全在 `built` 中 |
| `deps` 不完整 | DFS **总是**走完，`deps` 永完整 |
| `remap_struct_ids` 时机 | `LinkStage` 内部两遍扫描——第一遍建表，第二遍重映射 |
| 重复计算 | 不存在——`remap` 表只有 `LinkStage` 一个生产者 |

### 4.2 边界契约

| 边界 | 生产者 | 消费者 | 数据契约 |
|:---|:---|:---|:---|
| **图发现→编译** | `ModuleGraph` | `ModuleCompiler` | `order` + `sources` + `imports` |
| **编译→链接** | `ModuleCompiler` | `LinkStage` | `built` (per-module `ExportTable`) + `order` |

**核心约束**：
- `ModuleGraph` 不产生类型/IR 信息，只做语法解析提取 `import`
- `ModuleCompiler` 不分配全局索引，不计算 `remap` 表——只产出每个模块的局部 CPS
- `LinkStage` 是全局索引映射的**唯一生产者**——`func_remap`/`struct_remap`/`const_remap` 在 `LinkStage` 内部从零构建，`ModuleCompiler` 完全不碰

### 4.3 Infer 改动

`infer_module` 新增 `imports` 参数。在 Pass 2（stdlib 注入）之后、Pass 3（推断）之前，将导入符号注入类型环境。

```rust
fn infer_module_with_imports(
    module: &Module,
    imports: &ImportTable,    // ★ 新增
) -> InferResult<(TypeEnv, HashMap<usize, Vec<(String, Type)>>, HashSet<SymbolId>)> {
    // ... Pass 1, Pass 2 不变 ...

    // ★ 注入导入符号
    for resolved in &imports.entries {
        match &resolved.entry {
            ExportEntry::Const { name, ty, .. } | ExportEntry::Function { name, ty, .. } => {
                env.insert(name.clone(), Scheme::monomorphic(ty.clone()));
            }
            ExportEntry::Struct { name, fields, struct_id, .. } => {
                // ★ 将结构体字段列表注入本地 struct_fields（分配新本地 ID 避免冲突）
                let local_id = fresh_struct_id();  // 或直接用 source 模块的全局 ID
                env.insert(name.clone(), Scheme::monomorphic(Type::Record(local_id, fields.clone())));
                struct_fields.insert(local_id, fields.clone());
                // 记录重映射：source_struct_id → local_id（后续 CPS/Link 需要）
                struct_id_remap.insert(*struct_id, local_id);
            }
            ExportEntry::Interface { name, methods } => {
                env.insert(name.clone(), Scheme::monomorphic(Type::Null));
                // 注入接口方法签名到 interface_registry
                interface_registry.insert(name.clone(), methods.clone());
            }
        }
    }

    // Pass 3 不变，但遇到 ExportStmt 时需要标记符号为公开
    for stmt in &module.stmts {
        match stmt {
            Stmt::ExportStmt(inner) => {
                // 推断内部声明，并标记为 exported
                // 例如 export const x = 42 → 推断 x: Int64，加入 exports 集合
            }
            // ...
        }
    }
}

/// 从 AST 收集原始导入请求（纯语法，不做路径解析）
fn collect_raw_imports(module: &Module) -> Vec<RawImport> {
    module.stmts.iter().filter_map(|stmt| {
        if let Stmt::Import { path, names, .. } = stmt {
            Some(RawImport {
                names: names.clone(),
                source_path: path.clone(),
            })
        } else {
            None
        }
    }).collect()
}
```

### 4.4 CPS Build 改动

每个模块独立编译为 `CpsModule`。导入的函数调用在 CPS 层标记为外部调用。

**关键设计决策：CPS 层就用整数句柄，不存字符串**。

```rust
// CpsTerminator 新增变体
enum CpsTerminator {
    // ... 现有变体 ...
    /// 静态导入的外部调用。
    /// import_handle = ImportTable.entries 中的索引。
    /// CPS 层就把字符串固化为整数，LinkStage 直接 O(1) 取条目。
    CallExternal {
        import_handle: usize,    // ★ 整数句柄，非字符串
        args: Vec<usize>,
        ret_block: usize,
    },
    /// 动态导入的外部调用（保留到运行时，Phase 3b 预留，本期不生成）
    CallExternalDynamic {
        module_expr: String,
        func_name: String,
        args: Vec<usize>,
        ret_block: usize,
    },
}
```

**为什么不用字符串**：
- 同名冲突在 `ImportTable` 构建时已处理，不会"静默绑定错"
- LinkStage 直接 `import_table.entries[handle]` —— O(1)，零字符串查找
- 整个编译链路在 CPS 层完成"字符串→整数"的固化

当 `build_call` 遇到对导入函数的调用时：
1. 查 local func_map → 未命中
2. 查 `import_table.by_name` → 命中，拿到 `handle`
3. 生成 `CallExternal { import_handle: handle, args, ret_block }`
4. 若路径不是字符串字面量（未来动态 import）→ 生成 `CallExternalDynamic`

### 4.5 LinkStage（核心新增）

每个模块的 `CpsModule` 用局部索引（func_idx、struct_id、const_idx）。LinkStage 把多个 `CpsModule` 缝合成一个全局 `CpsModule`。

```rust
struct LinkStage;

struct LinkedModule {
    cps: CpsModule,
    /// 全局符号表：export_name → (global_func_idx | global_struct_id | global_const_idx)
    global_exports: HashMap<String, GlobalRef>,
}

enum GlobalRef {
    Func(usize),    // global func_idx
    Struct(usize),  // global struct_id
    Const(usize),   // global const_idx
}

impl LinkStage {
    fn link(
        built: &HashMap<String, ExportTable>,
        order: &[String],
    ) -> Result<CpsModule> {
        // ★ func_remap / struct_remap / const_remap 由 LinkStage 内部构建
        let (func_remap, struct_remap, const_remap) = Self::build_remap_tables(built, order);

    // ── 内部：构建全局索引映射表 ──
    fn build_remap_tables(
        built: &HashMap<String, ExportTable>,
        order: &[String],
    ) -> (HashMap<(String, usize), usize>, HashMap<(String, usize), usize>, HashMap<(String, usize), usize>) {
        let mut func_remap = HashMap::new();
        let mut struct_remap = HashMap::new();
        let mut const_remap = HashMap::new();
        let mut func_offset = 0;
        let mut struct_offset = 0;
        let mut const_offset = 0;
        for path in order {
            let cps = &built[path].cps_module;
            for i in 0..cps.functions.len() { func_remap.insert((path.clone(), i), func_offset + i); }
            for (i, sd) in cps.structs.iter().enumerate() { struct_remap.insert((path.clone(), sd.id), struct_offset + i); }
            for i in 0..cps.constants.len() { const_remap.insert((path.clone(), i), const_offset + i); }
            func_offset += cps.functions.len();
            struct_offset += cps.structs.len();
            const_offset += cps.constants.len();
        }
        (func_remap, struct_remap, const_remap)
    }

        // 2. 构建全局 struct 表 ★ 必须在重映射之前
        let mut global_structs = Vec::new();
        for path in &order {
            let export = &built[path];
            let offset = global_structs.len();
            for (i, sd) in export.cps_module.structs.iter().enumerate() {
                struct_remap.insert((path.clone(), sd.id), offset + i);
                let mut new_sd = sd.clone();
                new_sd.id = offset + i;
                global_structs.push(new_sd);
            }
        }

        // 3. 构建全局函数索引映射
        let mut total_funcs = 0;
        for path in &order {
            let n = built[path].cps_module.functions.len();
            for i in 0..n {
                func_remap.insert((path.clone(), i), total_funcs + i);
            }
            total_funcs += n;
        }

        // 4. ★ 遍历所有模块，重映射 CallExternal + struct_id（此时两个 remap 都已就绪）
        let mut linked_funcs: Vec<Option<CpsFunction>> = vec![None; total_funcs];

        for path in &order {
            let export = &built[path];
            let import_table = &export.import_table;
            let mut module_cps = (*export.cps_module).clone();

            for func in &mut module_cps.functions {
                for block in &mut func.blocks {
                    // 重映射 CallExternal → Call(global_idx)
                    if let CpsTerminator::CallExternal { import_handle, args, ret_block } = &block.term {
                        let resolved = import_table.entries.get(*import_handle)
                            .ok_or_else(|| BuildError::LinkError {
                                module: path.clone(),
                                message: format!("invalid import handle {import_handle}"),
                            })?;
                        let global_idx = match &resolved.entry {
                            ExportEntry::Function { func_idx, .. } =>
                                func_remap[&(resolved.source_path.clone(), *func_idx)],
                            _ => return Err(BuildError::LinkError {
                                module: path.clone(),
                                message: "import handle is not a function".into(),
                            }.into()),
                        };
                        block.term = CpsTerminator::Call(global_idx, args.clone(), *ret_block);
                    }
                }
            }

            // ★ 重映射 struct_id（struct_remap 已就绪）
            for func in &mut module_cps.functions {
                for block in &mut func.blocks {
                    remap_struct_ids(block, &struct_remap, path);
                }
            }

            // 写入全局函数表
            for (i, func) in module_cps.functions.into_iter().enumerate() {
                let global_idx = func_remap[&(path.clone(), i)];
                linked_funcs[global_idx] = Some(func);
            }
        }

        // 5. 合并 vtable、常量表
        // TODO: vtable 中的 func_idx 也需要经过 func_remap 重映射

        // 6. 返回链接后的 CpsModule
        Ok(CpsModule {
            functions: global_functions,
            structs: global_structs,
            enums: vec![],
            vtables: /* 合并后 */ vec![],
            constants: /* 合并后 */ vec![],
            symbol_map: /* (module, name) → global_idx */ HashMap::new(),
            func_owners: /* 每个函数归属模块 */ vec![],
        })
    }
}
```

**LinkStage 的核心职责（修正后）**：
1. 全局索引映射 — `(module_path, local_func_idx) → global_func_idx`
2. **遍历所有模块**，逐个解析各自的 `CallExternal`（不仅入口）
3. 将重映射后的函数写入全局表的正确位置
4. 全局 struct 表拼接 + struct_id 重映射
5. 常量表、vtable 合并 + 重映射

## 5. 文件结构

```
kaubo-driver/src/
├── protocol.rs
├── event.rs
├── coordinator.rs               # 单文件 Coordinator（不变）
├── module_graph.rs              # ★ ModuleGraph：纯语法 DFS + 拓扑排序
├── module_compiler.rs           # ★ ModuleCompiler：按序编译 + 缓存失效
├── module_loader.rs             # ★ ModuleLoader trait + FileLoader + MemLoader
├── link_stage.rs                # ★ LinkStage：多模块链接
├── export_table.rs              # ★ ExportTable / ImportTable 数据结构
└── stages.rs                    # 现有 Stage（不变）

kaubo-infer/src/
└── infer.rs                     # infer_module 接收 ImportTable 参数

kaubo-ir/src/
├── cps_build.rs                 # build_call 生成 CallExternal
├── cps/lib.rs                   # CpsTerminator::CallExternal 变体
└── cps/link.rs                  # (可选) 链接逻辑如果在 ir 层
```

## 6. 改动规模

| 文件 | 内容 | 预估行数 |
|------|------|---------|
| `export_table.rs` (新) | ExportEntry (enum per kind), ExportTable, ImportTable, ResolvedImport | ~100 |
| `link_stage.rs` (新) | LinkStage: 遍历所有模块重映射 + 常量合并 + symbol_map 填充 | ~140 |
| `module_graph.rs` (新) | ModuleGraph: 纯语法 DFS + 拓扑排序 + 循环检测 | ~60 |
| `module_compiler.rs` (新) | ModuleCompiler: 按序编译 + 缓存失效 + resolve_imports | ~80 |
| `module_loader.rs` (新) | ModuleLoader trait + FileLoader + MemLoader | ~50 |
| `infer.rs` | 接收 ImportTable, 注入 Struct 字段/Interface 方法, 标记 exports | ~60 |
| `cps_build.rs` | build_call 生成 CallExternal, symbol_cps_map 填充 | ~40 |
| `kaubo-cps/src/lib.rs` | CpsTerminator::CallExternal + CallExternalDynamic + CpsModule 新字段 | ~30 |
| `coordinator.rs` | 适配 ModuleGraph + ModuleCompiler | ~20 |
| 测试 | 跨模块 struct 字段访问 + 常量引用 + 缓存失效 + 循环依赖 | ~80 |

**总计：~720 行**（含缓存失效传递逻辑 + symbol_cps_map + compile_module + remap_struct_ids + normalize_path）

## 7. 单文件路径不变

```rust
// 单文件（Coordinator）——现有行为不变
Coordinator::new().run("const x = 42;");

// 多文件（ModuleGraph + ModuleCompiler）
let graph = ModuleGraph::build("main.kb", &loader)?;
let linked = ModuleCompiler::new(loader).compile_all(&graph)?;
VmExecStage.execute(linked)?;
```

单文件编译不触发任何模块逻辑——`import` 语句不存在时 `ImportTable` 为空，`LinkStage` 退化为透传。

## 8. 设计约束：为动态 import 预留兼容性

静态链接（Phase 3b）如果"压得太平"——把所有跨模块调用替换为硬编码 `Call(global_idx)` 并丢弃符号元数据——未来动态 import 就无法在运行时查找符号。设计上需要区分**静态边界**和**动态边界**。

### 8.1 两种导入边界

| 导入类型 | 示例 | 编译时可知路径？ | Link 策略 | VM 执行 |
|---------|------|:---:|---------|--------|
| **静态导入** | `import { add } from "./math.kb"` | ✅ | 压平为 `Call(global_idx)` | 直接跳转 |
| **动态导入** | `import(expr)` 或 `import "./lib.kb"`（运行时） | ❌/⚠ | 保留 `CallExternalDynamic` | 运行时查符号表 |

### 8.2 Phase 3b 必须遵守的约束

1. **保留符号映射，不丢弃**：
   `CpsModule` 增加 `symbol_map: HashMap<(String, String), usize>` 字段——记录 `(module_path, func_name) → global_func_idx` 映射。静态调用会用到它，动态加载时新模块通过函数名字符串查找。

2. **禁止跨模块内联**：
   LinkStage 只做**薄链接**（索引重映射 + 函数表拼接），不跨模块内联函数体。每个模块的函数体保持独立，未来动态加载器才能安全追加新模块。

3. **区分内部索引和外部符号**：
   `CallExternalDynamic` 是 CPS 层的预留变体——Phase 3b 不生成它，但定义了契约。LinkStage 遇到**路径不是字符串字面量**的 import 时保留该变体，不做压平。

4. **保留函数归属信息**：
   `CpsModule` 新增 `func_owners: Vec<ModuleId>` 字段——记录每个函数的原始模块归属。动态加载模块时检查符号冲突。

### 8.3 约束对数据结构的影响

```rust
struct CpsModule {
    functions: Vec<CpsFunction>,
    structs: Vec<StructDef>,
    // ... 现有字段 ...

    /// ★ Phase 3b 新增：符号映射表
    /// key: (module_path, export_name) → global_func_idx
    symbol_map: HashMap<(String, String), usize>,

    /// ★ Phase 3b 新增：每个函数的归属模块
    /// func_owners[i] = 函数 i 的原始模块路径
    func_owners: Vec<String>,
}
```

### 8.4 平台无关 IO：`kaubo-vfs` 预留

当前 `ModuleLoader` trait 直接对接文件系统。未来引入 `kaubo-vfs` 提供虚拟文件系统抽象（文件系统 / WASM virtual FS / 测试 mock / 网络资源），`ModuleLoader` trait 签名不变：

```rust
/// Phase 3b：简单文件 IO
struct FileLoader { root: PathBuf }
impl ModuleLoader for FileLoader { /* 直接读文件 */ }

/// 未来：kaubo-vfs
struct VfsLoader { vfs: Arc<dyn VirtualFileSystem> }
impl ModuleLoader for VfsLoader { /* 通过 VFS 读取 */ }
```

**本期同期创建** `kaubo-vfs` crate（~60 行），`ModuleLoader` 的 `FileLoader`/`MemLoader` 直接依赖它。详见 [kaubo-vfs 设计文档](kaubo-vfs-design.md)。

## 9. 细节补全

### 9.1 SymbolId → CPS 索引映射 + 填充路径

`build_export_table` 需要知道每个导出符号在 CPS 中的位置。`CpsBuildStage` 在构建过程中填充 `symbol_cps_map`：

```rust
struct CpsModule {
    // ... 现有字段 ...
    /// SymbolId → CPS 局部索引
    symbol_cps_map: HashMap<SymbolId, CpsRef>,
}

enum CpsRef {
    Func(usize),   // local func_idx
    Struct(usize),  // local struct_id
    Const(usize),   // local const_idx  ★ 常量也需要
}
```

**填充路径**——`CpsBuildStage::build_with_symbol_map`：

1. 函数：`build_top_stmt` 遇到 `ExportStmt(ConstDecl { name, value: Lambda { .. } })` 时，从 `SemanticArtifact.symbols` 中找到 `name` 对应的 `SymbolId`，`build_lambda_as_function` 返回 `func_idx`，写入 `symbol_cps_map.insert(sym_id, CpsRef::Func(func_idx))`
2. 常量：同理，`add_const` 返回 `const_idx`，写入 `CpsRef::Const(const_idx)`
3. 结构体：`build_top_stmt` 遇到 `ExportStmt(StructDef { .. })` 时，记录 `struct_id` → `CpsRef::Struct(struct_id)`

### 9.3 `build_export_table` 完整定义（含 import_table）

```rust
fn build_export_table(
    path: &str,
    semantic: &SemanticArtifact,
    cps: &CpsModule,
    import_table: ImportTable,  // ★ 4 参数——ExportTable 需要 import_table 字段
) -> ExportTable {
    let mut entries = Vec::new();

    for sym_id in &semantic.exports {
        let sym_name = &semantic.symbol_names[sym_id];
        let sym_def = &semantic.symbols[sym_id];

        let cps_ref = cps.symbol_cps_map.get(sym_id)
            .expect("exported symbol has no CPS mapping");

        let entry = match (sym_def.kind, cps_ref) {
            (SymbolKind::Function, CpsRef::Func(idx)) => ExportEntry::Function {
                name: sym_name.clone(),
                ty: sym_def.ty.clone(),
                func_idx: *idx,
            },
            (SymbolKind::Const, CpsRef::Const(idx)) => ExportEntry::Const {
                name: sym_name.clone(),
                ty: sym_def.ty.clone(),
                const_idx: *idx,
            },
            (SymbolKind::Struct, CpsRef::Struct(id)) => {
                let fields = semantic.struct_fields[id].clone();
                ExportEntry::Struct {
                    name: sym_name.clone(),
                    fields,
                    struct_id: *id,
                }
            }
            (SymbolKind::Interface, _) => {
                let methods = /* from interface_registry */ vec![];
                ExportEntry::Interface {
                    name: sym_name.clone(),
                    methods,
                }
            }
            _ => panic!("export entry kind/CPS ref mismatch for {sym_name}"),
        };
        entries.push(entry);
    }

    ExportTable {
        source_path: path.to_string(),
        entries,
        import_table: /* 当前模块的导入表 */,
        cps_module: cps.clone(),
    }
}
```

### 9.4 `resolve_imports` 完整实现（含导出检查 + 冲突检测）

```rust
fn resolve_imports(
    &self,
    path: &str,
    raw: &[RawImport],
    built: &HashMap<String, ModuleCacheEntry>,
) -> Result<ImportTable> {
    let mut entries = Vec::new();
    let mut by_name = HashMap::new();

    for raw_imp in raw {
        let (dep_path, _) = self.loader.resolve(path, &raw_imp.source_path)?;
        let dep = built.get(&dep_path)
            .ok_or_else(|| BuildError::ImportNotFound {
                path: dep_path.clone(),
                name: raw_imp.local_name.clone(),
            })?;

        // ★ 在被导入模块的导出表中按名查找
        let entry = dep.export_table.find_export(&raw_imp.local_name)
            .cloned()
            .ok_or_else(|| BuildError::ExportNotFound {
                name: raw_imp.local_name.clone(),
                path: dep_path.clone(),
            })?;

        // ★ 冲突检测：同模块不能重复导入同名符号
        if let Some(existing_handle) = by_name.get(&raw_imp.local_name) {
            let existing = &entries[*existing_handle];
            return Err(BuildError::SymbolConflict {
                name: raw_imp.local_name.clone(),
                path1: existing.source_path.clone(),
                path2: dep_path.clone(),
            });
        }

        by_name.insert(raw_imp.local_name.clone(), entries.len());
        entries.push(ResolvedImport {
            local_name: raw_imp.local_name.clone(),
            source_path: dep_path,
            entry,
        });
    }

    Ok(ImportTable { entries, by_name })
}

// ExportTable 辅助方法
impl ExportTable {
    fn find_export(&self, name: &str) -> Option<&ExportEntry> {
        self.entries.iter().find(|e| e.export_name() == name)
    }
}

impl ExportEntry {
    fn export_name(&self) -> &str {
        match self {
            ExportEntry::Const { name, .. } => name,
            ExportEntry::Function { name, .. } => name,
            ExportEntry::Struct { name, .. } => name,
            ExportEntry::Interface { name, .. } => name,
        }
    }
}
```

### 9.5 `remap_struct_ids` 详细逻辑

`LinkStage` 重映射每个函数的所有 struct 引用指令：

```rust
fn remap_struct_ids(
    block: &mut CpsBlock,
    struct_remap: &HashMap<(String, usize), usize>,
    module_path: &str,
) {
    for instr in &mut block.instrs {
        match instr {
            CpsInstr::NewStruct(dst, sid, fields) => {
                if let Some(&global_id) = struct_remap.get(&(module_path.to_string(), *sid)) {
                    *sid = global_id;
                }
            }
            CpsInstr::GetField(_, _, field_idx) => {
                // field_idx 不变——struct 重映射后字段顺序保持一致
            }
            CpsInstr::SetField(_, _, field_idx, _) => {
                // 同上
            }
            _ => {}
        }
    }
}
```

### 9.5 `func_remap` lookup 链路（整数句柄驱动）

重映射 `CallExternal` 时，lookup 为纯整数路径：

```
CallExternal.import_handle          (3)
  → import_table.entries[3]         → ResolvedImport { source_path: "./math.kb", entry: Function { func_idx: 0 } }
  → ("./math.kb", 0)               → key for func_remap
  → func_remap[(./math.kb, 0)]     → 42 (global func_idx)
  → Call(42, args, ret_block)
```

**零字符串查找，零冲突风险**。同名符号冲突在 `ImportTable` 构建时（`by_name` insert 发现 key 已存在）就报错。

### 9.6 `symbol_map` 统一——去掉冗余 `global_exports`

`symbol_map` 是唯一真相来源。`GlobalRef` 只作为枚举类型存在，不另建独立 HashMap：

```rust
/// 唯一的全局符号表：(module_path, export_name) → GlobalRef
symbol_map: HashMap<(String, String), GlobalRef>,

/// 按名查找缓存（动态导入用）：export_name → [(module_path, GlobalRef)]
symbol_by_name: HashMap<String, Vec<(String, GlobalRef)>>,
```

`LinkStage` 在合并时同步填充两者。

### 9.7 `func_owners` 在 `LinkStage` 中填充

```rust
let mut func_owners = Vec::with_capacity(total_funcs);

for path in &order {
    let n = built[path].cps_module.functions.len();
    for _ in 0..n {
        func_owners.push(path.clone());
    }
}
```

### 9.9 `CallExternalDynamic` 在 LinkStage 的跳过策略

`LinkStage` 遍历所有模块的指令时：

```rust
match &block.term {
    CpsTerminator::CallExternal { import_name, args, ret_block } => {
        // 静态 import：解析并重映射
        let global_idx = resolve_static_import(import_name, &import_table, &func_remap)?;
        block.term = CpsTerminator::Call(global_idx, args.clone(), *ret_block);
    }
    CpsTerminator::CallExternalDynamic { .. } => {
        // 动态 import：原样保留，不做任何修改
        // 运行时由 VM 的 ModuleRegistry 解析
    }
    _ => {}
}
```

### 9.10 LSP 跨文件导航预留

`LspCoordinator` 可持有 `ModuleGraph` 的只读引用：

```rust
struct LspCoordinator {
    // ... 现有字段 ...
    /// 可选的模块图引用（跨文件 goto-def 时使用）
    module_graph: Option<Arc<ModuleGraph>>,
}

impl LspCoordinator {
    fn goto_def_cross_module(&self, name: &str) -> Option<(String, Span)> {
        // 如果 name 是导入符号，查模块图找到定义文件
        let graph = self.module_graph.as_ref()?;
        // graph.built[path].export_table → 查符号来源
        // → 返回 (source_path, definition_span)
        None // TODO
    }
}
```

**本期不做**跨文件导航，仅预留接口。Phase 3b 完成后 LspCoordinator 至少能正确服务单文件内的 hover/goto-def。

### 9.11 常量表达式引用重映射

常量之间可能存在引用（`const TAU = PI * 2`），合并时需重映射内部 `LoadConst` 指令中的 `const_idx`：

```rust
for func in &mut global_functions {
    for block in &mut func.blocks {
        for instr in &mut block.instrs {
            if let CpsInstr::LoadConst(_, idx) = instr {
                if let Some(&global_idx) = const_remap.get(&(module_path, *idx)) {
                    *idx = global_idx;
                }
            }
        }
    }
}
```

### 9.13 `ExportTable` 用 `Arc<CpsModule>` 避免深拷贝

```rust
struct ExportTable {
    source_path: String,
    entries: Vec<ExportEntry>,
    import_table: ImportTable,
    cps_module: Arc<CpsModule>,  // ★ Arc 共享，不 clone
}
```

`LinkStage` 通过 `Arc::clone(&export.cps_module)` 获取引用，零拷贝。

### 9.14 静态/动态 import 的 AST 区分

当前 `Stmt::Import { path: String, .. }` 的 `path` 是字符串字面量（静态）。未来动态导入需要区分：

```rust
enum Stmt {
    ImportStatic { path: String, names: Vec<String> },  // Phase 3b
    // ImportDynamic { expr: Box<Expr> },               // 未来预留
}
```

**本期不改 AST**——Phase 3b 只处理 `ImportStatic`。

### 9.15 `normalize_path` 实现

```rust
fn normalize_path(base: &str, import_path: &str) -> String {
    // 以 base 所在目录为基准，拼接 import_path
    let parent = std::path::Path::new(base).parent().unwrap_or("".as_ref());
    let joined = parent.join(import_path);
    // 标准化：去掉 ./ 和 ../
    let mut components = Vec::new();
    for c in joined.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => { components.pop(); }
            other => components.push(other.as_os_str().to_str().unwrap().to_string()),
        }
    }
    components.join("/")
}
```

```

## 10. 验证场景

```kaubo
// 基础 import
// math.kb: export const answer = 42;
// main.kb: import { answer } from "./math.kb"; const r = answer + 1;  // 43

// 跨模块 struct
// point.kb: export struct Point { x: Int64, y: Int64 };
// main.kb: import { Point } from "./point.kb"; const p = Point { x: 1, y: 2 };

// 跨模块函数调用
// math.kb: export const add = |a: Int64, b: Int64| -> Int64 { return a + b; };
// main.kb: import { add } from "./math.kb"; add(2, 3);  // 5

// 缓存失效：修改 math.kb → main.kb cache miss → 重编译

// 错误：导入未导出的符号 → 报错
// 错误：导入的文件不存在 → 报错
```
