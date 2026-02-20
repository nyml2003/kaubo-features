# 多模块编译架构设计方案 V2

**状态**: 设计评审中  
**作者**: Kimi Code  
**日期**: 2026-02-20  
**相关模块**: `kaubo-orchestrator`, `kaubo-vfs`

---

## 1. 背景与问题

当前 `kaubo-orchestrator` 的多模块编译存在架构问题：

1. **路径信息丢失**: `MultiModulePass` 需要文件路径来解析 `import`，但 Orchestrator 流程导致路径在传递到 Pass 时丢失
2. **VFS 重复创建**: `MultiFileCompiler` 自行创建 VFS 实例，与 `PassContext` 中的 VFS 重复
3. **模块标识不统一**: 使用 `String` 表示模块路径，缺乏类型安全

---

## 2. 设计目标

- ✅ **依赖注入**: 通过 `PassContext` 传递 VFS，避免全局状态
- ✅ **类型安全**: 使用结构化的 `ModuleId` 替代字符串
- ✅ **模块缓存**: 支持 diamond dependency 场景，避免重复编译
- ✅ **可测试**: 可以使用 `MemoryFileSystem` 测试多模块编译

---

## 3. 总体架构

```
┌─────────────────────────────────────────────────────────────┐
│  CLI / Orchestrator                                         │
│  └── 创建 CoreContext { vfs, logger, config }               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  PassContext { core, source_path, options, output }         │
│  └── 每次 run() 时创建，包含入口文件路径                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  MultiModulePass                                            │
│  └── 创建 CompileContext { vfs, cache, stack }              │
│      └── 使用 ModuleId 解析 import                          │
│          └── 通过 VFS 读取模块文件                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. 关键数据结构

### 4.1 CoreContext（核心基础设施）

```rust
/// 可复用的核心上下文（Clone 成本低，都是 Arc）
#[derive(Clone)]
pub struct CoreContext {
    pub vfs: Arc<dyn VirtualFileSystem + Send + Sync>,
    pub logger: Arc<Logger>,
    pub config: Arc<VmConfig>,
}

impl CoreContext {
    pub fn new(
        vfs: impl VirtualFileSystem + Send + Sync + 'static,
        logger: impl Into<Arc<Logger>>,
        config: impl Into<Arc<VmConfig>>,
    ) -> Self {
        Self {
            vfs: Arc::new(vfs),
            logger: logger.into(),
            config: config.into(),
        }
    }
}
```

**职责**: 提供全局共享的基础设施（VFS、日志、配置）

### 4.2 PassContext（Pass 执行上下文）

```rust
pub struct PassContext {
    /// 核心基础设施（Clone 即可）
    pub core: CoreContext,
    /// 源文件路径（多模块编译必需）
    pub source_path: Option<PathBuf>,
    /// 编译选项
    pub options: PassOptions,
    /// 输出缓冲区
    pub output: OutputHandle,
    /// 上一步的元数据
    pub previous_metadata: HashMap<String, Value>,
}

impl PassContext {
    /// 从 CoreContext 创建
    pub fn from_core(core: CoreContext, source_path: Option<PathBuf>) -> Self;
    
    /// 获取 VFS（便利方法）
    pub fn vfs(&self) -> &dyn VirtualFileSystem;
    
    /// 获取 logger（便利方法）
    pub fn logger(&self) -> &Logger;
}
```

**职责**: 为单次编译会话提供上下文，包含入口文件路径

### 4.3 ModuleId（结构化模块标识）

```rust
/// 模块标识符（替代 String）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// 命名空间（包）
    pub namespace: Namespace,
    /// 模块路径（如 ["math"] 或 ["math", "utils"]）
    pub components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Namespace {
    Std,    // 标准库：import "std.list";
    Local,  // 当前项目：import "math";
}

impl ModuleId {
    /// 从 import 语句解析
    /// "math"       -> Local, ["math"]
    /// "std.list"   -> Std, ["list"]
    pub fn parse(s: &str) -> Result<Self, ParseError>;
    
    /// 转换为 VFS 路径
    /// Local(["math"])       -> "/math.kaubo"
    /// Std(["list"])         -> "/std/list.kaubo"
    pub fn to_vfs_path(&self) -> PathBuf;
    
    /// 获取字符串表示（用于缓存 key）
    pub fn to_string(&self) -> String;
}
```

**职责**: 类型化模块标识，支持不同命名空间，统一路径转换

### 4.4 CompileContext（编译上下文）

```rust
/// 多模块编译上下文
/// 生命周期：单次 MultiModulePass::run() 期间
pub struct CompileContext<'a> {
    /// VFS 引用（不拥有所有权）
    vfs: &'a dyn VirtualFileSystem,
    /// 已编译模块缓存（ModuleId -> CompileUnit）
    cache: HashMap<ModuleId, CompileUnit>,
    /// 当前解析栈（循环依赖检测）
    stack: Vec<ModuleId>,
    /// 入口文件目录（用于解析相对路径）
    root_dir: PathBuf,
}

impl<'a> CompileContext<'a> {
    pub fn new(vfs: &'a dyn VirtualFileSystem, root_dir: impl AsRef<Path>) -> Self;
    
    /// 获取或编译模块（核心方法）
    pub fn get_or_compile(&mut self, id: &ModuleId) -> Result<&CompileUnit, CompileError>;
    
    /// 编译单个模块
    fn compile_module(&mut self, id: &ModuleId) -> Result<CompileUnit, CompileError>;
    
    /// 提取 import 语句
    fn extract_imports(&self, ast: &Module) -> Vec<ModuleId>;
}
```

**职责**: 管理多模块编译过程，包括缓存、循环依赖检测

---

## 5. 核心流程

### 5.1 模块解析流程

```
import "math"
    │
    ▼
ModuleId::parse("math") 
    │
    ▼
ModuleId { Local, ["math"] }
    │
    ▼
to_vfs_path() -> "/math.kaubo"
    │
    ▼
vfs.read("/math.kaubo")
    │
    ▼
解析 AST
    │
    ▼
提取 imports -> 递归编译
```

### 5.2 缓存机制

```rust
pub fn get_or_compile(&mut self, id: &ModuleId) -> Result<&CompileUnit, CompileError> {
    // 1. 检查缓存（已编译）
    if let Some(unit) = self.cache.get(id) {
        return Ok(unit);
    }
    
    // 2. 检查循环依赖
    if self.stack.contains(id) {
        return Err(CompileError::CircularDependency { ... });
    }
    
    // 3. 推入栈，编译，缓存，弹出
    self.stack.push(id.clone());
    let unit = self.compile_module(id)?;
    self.stack.pop();
    self.cache.insert(id.clone(), unit);
    
    Ok(self.cache.get(id).unwrap())
}
```

---

## 6. 改动范围

### 6.1 新增文件

| 文件 | 说明 |
|------|------|
| `src/context.rs` | 新增 `CoreContext`，重构 `PassContext` |
| `src/pipeline/module/module_id.rs` | 结构化模块标识 |
| `src/pipeline/module/compile_context.rs` | 编译上下文 |

### 6.2 修改文件

| 文件 | 改动 |
|------|------|
| `src/pipeline/module/mod.rs` | 导出新增模块 |
| `src/stages/multi_module.rs` | 使用新架构重写 |
| `src/lib.rs` | 导出 `CoreContext` 和 `ModuleId` |
| `src/orchestrator.rs` | 创建 `CoreContext`，传递 `source_path` |

### 6.3 废弃文件

- `src/pipeline/module/multi_file.rs` -> 功能合并到 `compile_context.rs`
- `src/pipeline/module/resolver.rs` -> 功能合并（可选保留简化版）

---

## 7. 实现步骤

### Phase 1: 基础设施（1-2 天）

1. 创建 `CoreContext`
2. 重构 `PassContext` 包含 `CoreContext`
3. 更新 `Orchestrator` 创建 `CoreContext`

### Phase 2: 模块标识（0.5-1 天）

1. 创建 `ModuleId` 和 `Namespace`
2. 实现 `parse()` 和 `to_vfs_path()`
3. 添加单元测试

### Phase 3: 编译上下文（2-3 天）

1. 创建 `CompileContext`
2. 实现 `get_or_compile()` 核心逻辑
3. 整合循环依赖检测
4. 添加单元测试

### Phase 4: 集成（1-2 天）

1. 重写 `MultiModulePass`
2. 使用 `CompileContext` 替代 `MultiFileCompiler`
3. 集成测试

---

## 8. 测试策略

### 8.1 单元测试

```rust
#[test]
fn test_module_id_parse() {
    assert_eq!(
        ModuleId::parse("math").unwrap(),
        ModuleId { namespace: Local, components: vec!["math".to_string()] }
    );
    
    assert_eq!(
        ModuleId::parse("std.list").unwrap(),
        ModuleId { namespace: Std, components: vec!["list".to_string()] }
    );
}

#[test]
fn test_module_id_to_vfs_path() {
    let id = ModuleId::parse("math").unwrap();
    assert_eq!(id.to_vfs_path(), PathBuf::from("/math.kaubo"));
    
    let id = ModuleId::parse("std.fs").unwrap();
    assert_eq!(id.to_vfs_path(), PathBuf::from("/std/fs.kaubo"));
}
```

### 8.2 集成测试

```rust
#[test]
fn test_multi_module_compilation() {
    // 使用 MemoryFileSystem，无需真实文件
    let fs = MemoryFileSystem::with_files([
        ("/main.kaubo", b"import math; print math.add(1, 2);".to_vec()),
        ("/math.kaubo", b"pub var add = |a, b| => a + b;".to_vec()),
    ]);
    
    let core = CoreContext::new(fs, Logger::new(Level::Info), VmConfig::default());
    let ctx = PassContext::from_core(core, Some(PathBuf::from("/main.kaubo")));
    
    let pass = MultiModulePass::new();
    let output = pass.run(input, &ctx).unwrap();
    
    assert!(matches!(output.data, IR::Bytecode(_)));
}
```

### 8.3 循环依赖测试

```rust
#[test]
fn test_circular_dependency_detection() {
    let fs = MemoryFileSystem::with_files([
        ("/a.kaubo", b"import b;".to_vec()),
        ("/b.kaubo", b"import c;".to_vec()),
        ("/c.kaubo", b"import a;".to_vec()), // 循环
    ]);
    
    // 应该返回 CircularDependency 错误
}
```

---

## 9. VFS 中间件集成

多模块编译将使用 [VFS 中间件系统](./vfs-middleware-system.md) 实现路径映射和缓存：

```rust
// 创建带中间件的 VFS
let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(CachedLayer::with_ttl(Duration::from_secs(60)))
    .with(MappedLayer::with_mappings(&[
        ("/std", stdlib_path()),         // 标准库映射
        ("/vendor", "./vendor".into()),  // 依赖映射
    ]))
    .with(LoggedLayer::new())
    .build();

// ModuleId 生成逻辑路径
let id = ModuleId::parse("std.list")?;  // -> "/std/list.kaubo"

// VFS 中间件自动处理映射和缓存
let content = vfs.read_file(&id.to_vfs_path())?;
```

**无需**在 `CompileContext` 中处理路径映射，完全由 VFS 中间件层负责。

---

## 10. 待确认问题

### 10.1 相对导入支持

当前设计只支持绝对导入（如 `import "math"`），是否需要支持相对导入？

```rust
// 相对导入示例
import "./utils";      // 同级目录
import "../common";   // 上级目录
```

**建议**：Phase 1 先不支持，后续通过 `MappedLayer` 或 `RelativeImportLayer` 实现。

### 10.2 错误处理细化

`CompileError` 是否需要细化错误类型？

```rust
pub enum CompileError {
    ModuleNotFound { id: ModuleId, tried: Vec<PathBuf> },
    SyntaxError { id: ModuleId, line: usize, message: String },
    CircularDependency { chain: Vec<ModuleId> },
    ReadError { id: ModuleId, source: VfsError },
}
```

---

## 11. 参考

- [模块系统设计](./module-system.md)
- [VFS 文档](../../20-language/guide/README.md)
- [架构原则](../../00-principles/README.md)
