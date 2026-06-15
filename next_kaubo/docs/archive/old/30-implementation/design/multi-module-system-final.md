# 多模块编译系统设计（最终版）

**状态**: 已确认，待实现  
**目标**: 支持 import 语句的多文件编译

---

## 架构概览

```
用户代码: import "math.utils"
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  ModuleId::parse("math.utils")                                   │
│  -> components: ["math", "utils"]                                │
│  -> vfs_path: "/mod/math.utils"                                  │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  VFS 中间件链（自动排序）                                         │
│                                                                   │
│  1. LoggedLayer (Stage::Outer)                                   │
│     -> 记录: "read /mod/math.utils"                              │
│     │                                                             │
│     ▼                                                             │
│  2. MappedLayer (Stage::Mapping) ⭐ 核心                          │
│     -> 解析 "/mod/math.utils"                                     │
│     -> 转换为: "math/utils.kaubo"                                │
│     -> 在 search_paths 中查找:                                   │
│        ["./src/math/utils.kaubo", "/opt/kaubo/std/math/utils.kaubo"]
│     -> 返回第一个存在的路径                                       │
│     │                                                             │
│     ▼                                                             │
│  3. CachedLayer (Stage::Caching)                                 │
│     -> 查缓存（key: 物理路径）                                    │
│     -> 未命中则继续                                               │
│     │                                                             │
│     ▼                                                             │
│  4. NativeFileSystem (Stage::Core)                               │
│     -> 实际文件读取                                               │
└─────────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  CompileContext                                                  │
│  -> 解析 import 语句                                             │
│  -> 递归编译依赖                                                 │
│  -> 循环依赖检测                                                 │
│  -> 模块缓存（ModuleId -> CompileUnit）                          │
└─────────────────────────────────────────────────────────────────┘
```

---

## 1. 模块标识（ModuleId）

### 职责
- 解析 import 语句（如 `"math.utils"`）
- 生成 VFS 逻辑路径（如 `/mod/math.utils`）

### 实现
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// 模块路径组件
    /// "math.utils" -> ["math", "utils"]
    pub components: Vec<String>,
}

impl ModuleId {
    /// 解析 import 语句
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let components: Vec<String> = s
            .split('.')
            .map(|s| s.to_string())
            .collect();
        
        if components.iter().any(|c| c.is_empty()) {
            return Err(ParseError::InvalidModulePath(s.to_string()));
        }
        
        Ok(Self { components })
    }
    
    /// 转换为 VFS 逻辑路径
    /// ["math", "utils"] -> "/mod/math.utils"
    pub fn to_vfs_path(&self) -> PathBuf {
        PathBuf::from(format!("/mod/{}", self.components.join(".")))
    }
    
    /// 转换为文件系统路径（点号转斜杠）
    /// ["math", "utils"] -> "math/utils.kaubo"
    pub fn to_file_path(&self) -> PathBuf {
        let mut path: PathBuf = self.components.iter().collect();
        path.set_extension("kaubo");
        path
    }
}
```

---

## 2. 映射层（MappedLayer）

### 职责
- 拦截 `/mod/*` 路径
- 按搜索路径顺序查找模块文件
- **本地优先于标准库**

### 搜索路径
```rust
pub struct ModuleContext {
    /// 搜索路径（按优先级排序）
    pub search_paths: Vec<PathBuf>,
}

impl ModuleContext {
    /// 默认搜索路径
    pub fn default() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("./src"),              // 本地优先
                detect_std_path(),                    // 系统标准库
            ],
        }
    }
}

/// 自动检测标准库路径
fn detect_std_path() -> PathBuf {
    // 1. 环境变量 KABO_STD_PATH
    if let Ok(path) = std::env::var("KABO_STD_PATH") {
        return path.into();
    }
    
    // 2. 平台默认
    #[cfg(target_os = "linux")]
    return PathBuf::from("/opt/kaubo/std");
    
    #[cfg(target_os = "macos")]
    return PathBuf::from("/usr/local/lib/kaubo/std");
    
    #[cfg(target_os = "windows")]
    return PathBuf::from(r"C:\Program Files\Kaubo\std");
}
```

### MappedLayer 实现
```rust
pub struct MappedLayer {
    ctx: ModuleContext,
}

impl MappedLayer {
    pub fn new(ctx: ModuleContext) -> Self {
        Self { ctx }
    }
    
    /// 解析 VFS 路径为物理路径
    fn resolve(&self, path: &Path) -> Option<PathBuf> {
        // 检查是否是模块路径
        let module_part = path.to_string_lossy().strip_prefix("/mod/")?;
        
        // 点号转路径分隔符
        // math.utils -> math/utils.kaubo
        let relative: PathBuf = module_part.split('.').collect();
        let file_path = relative.with_extension("kaubo");
        
        // 按优先级查找
        for search_path in &self.ctx.search_paths {
            let full_path = search_path.join(&file_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        // 没找到，返回第一个路径（用于错误信息）
        self.ctx.search_paths.first()
            .map(|p| p.join(&file_path))
    }
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping  // 300
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> VfsResult<Vec<u8>> {
        match self.resolve(path) {
            Some(real_path) => next.call(&real_path),
            None => next.call(path),  // 非模块路径，原样传递
        }
    }
}
```

### 查找示例
```rust
// import "list"
// ModuleId::parse("list") -> components: ["list"]
// to_vfs_path() -> "/mod/list"

// MappedLayer::resolve("/mod/list")
// 1. 解析模块名: "list"
// 2. 转换为文件路径: "list.kaubo"
// 3. 查找:
//    - ./src/list.kaubo          (存在? 返回)
//    - /opt/kaubo/std/list.kaubo (存在? 返回)

// import "math.utils"
// -> 查找 math/utils.kaubo
```

---

## 3. 缓存层（CachedLayer）

### 职责
- 缓存文件内容，避免重复磁盘 I/O
- **缓存 key 是物理路径**（映射后的真实路径）

### 实现要点
```rust
pub struct CachedLayer {
    cache: RwLock<LruCache<PathBuf, Vec<u8>>>,
    ttl: Duration,
}

impl Middleware for CachedLayer {
    fn stage(&self) -> Stage {
        Stage::Caching  // 400
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> VfsResult<Vec<u8>> {
        // 注意：path 已经是 MappedLayer 处理后的物理路径
        
        // 1. 查缓存
        if let Some(cached) = self.get_cached(path) {
            return Ok(cached);
        }
        
        // 2. 未命中，读取文件
        let content = next.call(path)?;
        
        // 3. 写入缓存
        self.insert(path, content.clone());
        
        Ok(content)
    }
}
```

---

## 4. 编译上下文（CompileContext）

### 职责
- 管理多模块编译过程
- 模块缓存（ModuleId -> CompileUnit）
- 循环依赖检测

### 数据结构
```rust
pub struct CompileContext<'a> {
    /// VFS 引用
    vfs: &'a dyn VirtualFileSystem,
    /// 已编译模块缓存
    cache: HashMap<ModuleId, CompileUnit>,
    /// 当前解析栈（循环依赖检测）
    stack: Vec<ModuleId>,
}

pub struct CompileUnit {
    /// 模块 ID
    pub id: ModuleId,
    /// 物理路径
    pub path: PathBuf,
    /// AST
    pub ast: Module,
    /// 依赖的模块
    pub dependencies: Vec<ModuleId>,
}

impl<'a> CompileContext<'a> {
    /// 获取或编译模块
    pub fn get_or_compile(&mut self, id: &ModuleId) -> Result<&CompileUnit, CompileError> {
        // 1. 检查缓存
        if self.cache.contains_key(id) {
            return Ok(&self.cache[id]);
        }
        
        // 2. 检查循环依赖
        if self.stack.contains(id) {
            return Err(CompileError::CircularDependency {
                chain: self.stack.clone(),
            });
        }
        
        // 3. 推入栈
        self.stack.push(id.clone());
        
        // 4. 编译模块
        let unit = self.compile_module(id)?;
        let deps = unit.dependencies.clone();
        
        // 5. 缓存
        self.cache.insert(id.clone(), unit);
        self.stack.pop();
        
        // 6. 递归编译依赖
        for dep_id in deps {
            self.get_or_compile(&dep_id)?;
        }
        
        Ok(&self.cache[id])
    }
    
    /// 编译单个模块
    fn compile_module(&self, id: &ModuleId) -> Result<CompileUnit, CompileError> {
        // 1. 生成 VFS 路径
        let vfs_path = id.to_vfs_path();
        
        // 2. 通过 VFS 读取文件（经过中间件链）
        let content = self.vfs.read_file(&vfs_path)?;
        let source = String::from_utf8(content)?;
        
        // 3. 解析 AST
        let ast = parse(&source)?;
        
        // 4. 提取依赖
        let dependencies = extract_imports(&ast);
        
        Ok(CompileUnit {
            id: id.clone(),
            path: vfs_path,  // 逻辑路径
            ast,
            dependencies,
        })
    }
}
```

---

## 5. 完整流程示例

### 场景：编译 main.kaubo（import math.utils）

```rust
// main.kaubo
import "math.utils";
print math.utils.add(1, 2);

// math/utils.kaubo
pub var add = |a, b| => a + b;
```

### 编译流程

```
MultiModulePass::run()
  │
  ▼
创建 CompileContext { vfs, cache: {}, stack: [] }
  │
  ▼
get_or_compile(ModuleId("main"))
  │
  ├─> 编译 main.kaubo
  │    ├─> vfs.read_file("/mod/main")
  │    │     ├─> LoggedLayer: 记录日志
  │    │     ├─> MappedLayer: 解析为 "./src/main.kaubo"
  │    │     ├─> CachedLayer: 未命中
  │    │     └─> NativeFS: 读取文件
  │    │
  │    ├─> 解析 AST
  │    └─> 提取依赖: [ModuleId(["math", "utils"])]
  │
  ▼
递归编译依赖
  │
  ├─> get_or_compile(ModuleId(["math", "utils"]))
  │    ├─> vfs.read_file("/mod/math.utils")
  │    │     ├─> MappedLayer: 解析为 "math/utils.kaubo"
  │    │     │           查找:
  │    │     │           1. ./src/math/utils.kaubo (存在!)
  │    │     │
  │    │     └─> 返回: "./src/math/utils.kaubo" 内容
  │    │
  │    ├─> 解析 AST
  │    └─> 提取依赖: [] (无依赖)
  │
  ▼
拓扑排序，合并字节码
```

---

## 6. 实现顺序

| 阶段 | 任务 | 文件 | 时间 |
|------|------|------|------|
| 1 | VFS 中间件框架 | `src/middleware/` | 0.5 天 |
| 2 | LoggedLayer | `src/middleware/logged.rs` | 0.5 天 |
| 3 | MappedLayer | `src/middleware/mapped.rs` | 0.5 天 |
| 4 | CachedLayer | `src/middleware/cached.rs` | 0.5 天 |
| 5 | ModuleId | `src/pipeline/module/id.rs` | 0.5 天 |
| 6 | CompileContext | `src/pipeline/module/compile_context.rs` | 1 天 |
| 7 | MultiModulePass 重构 | `src/stages/multi_module.rs` | 1 天 |
| 8 | 集成测试 | `tests/multi_module/` | 0.5 天 |

**总计: 约 5 天**

---

## 7. 关键决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 搜索路径 | `["./src", "/opt/kaubo/std"]` | 本地优先于系统，支持覆盖 |
| 模块路径格式 | `/mod/xxx.yyy` | 统一前缀，易于识别 |
| 文件路径转换 | `xxx.yyy` -> `xxx/yyy.kaubo` | 点号表示层级 |
| 缓存 key | 物理路径 | MappedLayer 之后，缓存看到的是真实路径 |
| 循环依赖检测 | 解析栈 | 简单有效 |

---

## 8. 待扩展（后续）

| 特性 | 说明 | 优先级 |
|------|------|--------|
| Vendor 支持 | 第三方依赖管理 | P1 |
| 相对导入 | `import "./utils"` | P1 |
| 热重载 | 文件监听 | P2 |
| 版本管理 | `import "http@2.0"` | P2 |

---

**确认后进入实现阶段。**
