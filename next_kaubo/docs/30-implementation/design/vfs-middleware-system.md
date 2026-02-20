# VFS 中间件系统设计

**状态**: 设计评审中  
**目标**: 支持无顺序添加中间件，自动编排执行顺序

---

## 核心思想

VFS 保持纯净（只定义基本操作），中间件通过**阶段（Stage）**自动排序，无需用户关心顺序。

```rust
// 用户代码：任意顺序添加中间件
let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(CachedLayer::new())      // 缓存层
    .with(MappedLayer::new())      // 映射层
    .with(LoggedLayer::new())      // 日志层
    .build();

// 系统自动编排执行顺序：
// Logged -> Mapped -> Cached -> Native
```

---

## 1. 阶段定义

```rust
/// 中间件执行阶段
/// 阶段号越小越早执行
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// 最外层（日志、监控）
    Outer = 100,
    /// 前置处理（权限检查、路径验证）
    PreProcess = 200,
    /// 路径映射（/std -> /opt/kaubo/std）
    Mapping = 300,
    /// 缓存层（读缓存、写缓存）
    Caching = 400,
    /// 核心操作（实际读写）
    Core = 500,
    /// 后置处理（缓存回写、日志记录）
    PostProcess = 600,
}

/// 中间件 trait
pub trait Middleware: Send + Sync {
    /// 声明所属阶段
    fn stage(&self) -> Stage;
    
    /// 处理请求
    /// next: 调用下一个中间件
    fn read(&self, path: &Path, next: Next<'_>) -> Result<Vec<u8>>;
    fn exists(&self, path: &Path, next: Next<'_>) -> bool;
    // ... 其他方法
}

/// 下一个中间件的句柄
pub struct Next<'a> {
    inner: &'a dyn Fn(&Path) -> Result<Vec<u8>>,
}

impl<'a> Next<'a> {
    pub fn call(&self, path: &Path) -> Result<Vec<u8>> {
        (self.inner)(path)
    }
}
```

---

## 2. 具体中间件实现

### 2.1 MappedLayer（路径映射）

```rust
pub struct MappedLayer {
    mappings: Vec<(String, PathBuf)>,
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping  // 阶段 300
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> Result<Vec<u8>> {
        // 转换路径
        let real_path = self.resolve(path);
        // 继续下一个
        next.call(&real_path)
    }
    
    fn exists(&self, path: &Path, next: Next<'_>) -> bool {
        let real_path = self.resolve(path);
        next.call_exists(&real_path)
    }
    
    // ... 其他方法
}

impl MappedLayer {
    pub fn map(&mut self, prefix: &str, target: impl AsRef<Path>) {
        self.mappings.push((prefix.to_string(), target.as_ref().to_path_buf()));
    }
    
    fn resolve(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        for (prefix, target) in &self.mappings {
            if let Some(rest) = path_str.strip_prefix(prefix) {
                return target.join(rest.trim_start_matches('/'));
            }
        }
        path.to_path_buf()
    }
}
```

### 2.2 CachedLayer（缓存）

```rust
pub struct CachedLayer {
    cache: RwLock<HashMap<PathBuf, CacheEntry>>,
    ttl: Duration,
}

impl Middleware for CachedLayer {
    fn stage(&self) -> Stage {
        Stage::Caching  // 阶段 400
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> Result<Vec<u8>> {
        // 1. 查缓存
        if let Some(entry) = self.get_cached(path) {
            return Ok(entry.content);
        }
        
        // 2. 未命中，调用下一层
        let content = next.call(path)?;
        
        // 3. 写入缓存
        self.cache.insert(path.to_path_buf(), CacheEntry {
            content: content.clone(),
            modified: Instant::now(),
        });
        
        Ok(content)
    }
}
```

### 2.3 LoggedLayer（日志）

```rust
pub struct LoggedLayer {
    logger: Arc<Logger>,
}

impl Middleware for CachedLayer {
    fn stage(&self) -> Stage {
        Stage::Outer  // 阶段 100，最外层
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> Result<Vec<u8>> {
        let start = Instant::now();
        self.logger.debug(&format!("Reading: {}", path.display()));
        
        let result = next.call(path);
        
        match &result {
            Ok(_) => self.logger.debug(&format!("Read OK in {:?}", start.elapsed())),
            Err(e) => self.logger.error(&format!("Read failed: {}", e)),
        }
        
        result
    }
}
```

---

## 3. 构建器（自动排序）

```rust
pub struct VfsBuilder {
    backend: Box<dyn VirtualFileSystem>,
    middlewares: Vec<Box<dyn Middleware>>,
}

impl VfsBuilder {
    pub fn new(backend: impl VirtualFileSystem + 'static) -> Self {
        Self {
            backend: Box::new(backend),
            middlewares: vec![],
        }
    }
    
    /// 添加中间件（任意顺序）
    pub fn with(mut self, middleware: impl Middleware + 'static) -> Self {
        self.middlewares.push(Box::new(middleware));
        self
    }
    
    /// 构建最终 VFS
    /// 自动按 Stage 排序
    pub fn build(self) -> LayeredVFS {
        // 按阶段排序
        let mut middlewares = self.middlewares;
        middlewares.sort_by_key(|m| m.stage());
        
        LayeredVFS::new(self.backend, middlewares)
    }
}
```

---

## 4. 执行引擎

```rust
/// 层叠 VFS：自动编排中间件
pub struct LayeredVFS {
    backend: Box<dyn VirtualFileSystem>,
    chain: Vec<Box<dyn Middleware>>,
}

impl LayeredVFS {
    fn new(backend: Box<dyn VirtualFileSystem>, chain: Vec<Box<dyn Middleware>>) -> Self {
        Self { backend, chain }
    }
    
    /// 构建调用链
    fn build_read_chain(&self, path: &Path) -> Result<Vec<u8>> {
        self.call_read_at(0, path)
    }
    
    /// 递归调用中间件链
    fn call_read_at(&self, index: usize, path: &Path) -> Result<Vec<u8>> {
        if index >= self.chain.len() {
            // 到达底层，调用实际 VFS
            return self.backend.read_file(path);
        }
        
        let middleware = &self.chain[index];
        let next = Next {
            inner: &|p| self.call_read_at(index + 1, p),
        };
        
        middleware.read(path, next)
    }
}

impl VirtualFileSystem for LayeredVFS {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        self.build_read_chain(path)
    }
    
    // ... 其他方法同理
}
```

---

## 5. 使用示例

### 5.1 基础使用

```rust
// 任意顺序添加，系统自动排序
let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(CachedLayer::new())
    .with(MappedLayer::with_mappings(&[
        ("/std", "/opt/kaubo/std"),
        ("/vendor", "./vendor"),
    ]))
    .with(LoggedLayer::new())
    .build();

// 实际执行顺序（按 Stage 排序）：
// 1. LoggedLayer (Stage::Outer)
// 2. MappedLayer (Stage::Mapping)
// 3. CachedLayer (Stage::Caching)
// 4. NativeFileSystem (Core)
```

### 5.2 自定义中间件

```rust
/// 权限检查中间件
pub struct AuthLayer;

impl Middleware for AuthLayer {
    fn stage(&self) -> Stage {
        Stage::PreProcess  // 阶段 200
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> Result<Vec<u8>> {
        // 检查是否有权限读取
        if !self.check_permission(path) {
            return Err(VfsError::PermissionDenied);
        }
        next.call(path)
    }
}

let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(CachedLayer::new())
    .with(AuthLayer::new())        // 权限检查
    .with(MappedLayer::new())
    .build();

// 执行顺序：
// Mapped (300) -> Auth (200) -> Cached (400)
// 实际排序后：Auth (200) -> Mapped (300) -> Cached (400)
```

---

## 6. 阶段定义建议

| 阶段 | 用途 | 内置中间件 |
|-----|------|-----------|
| 100 Outer | 日志、监控、Tracing | LoggedLayer |
| 200 PreProcess | 权限检查、路径验证 | AuthLayer |
| 300 Mapping | 路径映射、重定向 | MappedLayer |
| 400 Caching | 读写缓存 | CachedLayer |
| 500 Core | 实际存储（不可插入） | NativeFileSystem |
| 600 PostProcess | 缓存回写、索引更新 | - |

---

## 7. 与多模块编译结合

```rust
// 多模块编译场景
let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(CachedLayer::with_ttl(Duration::from_secs(60)))
    .with(MappedLayer::with_mappings(&[
        ("/std", stdlib_path()),
    ]))
    .build();

let core = CoreContext::new(vfs, logger, config);
let ctx = PassContext::from_core(core, Some(PathBuf::from("/src/main.kaubo")));

// MultiModulePass 直接使用 ctx.vfs()
// 自动享受：路径映射 + 缓存 + 日志
```

---

## 8. 优势

1. **无顺序敏感**：任意顺序添加，自动排序
2. **可扩展**：新增中间件只需实现 trait 和 stage
3. **可组合**：中间件可独立测试、独立复用
4. **零开销**：编译期优化，无运行时反射

---

## 9. 待确认

1. **同一阶段多个中间件**：按添加顺序还是随机？
   - 建议：按添加顺序，稳定可预期

2. **短路机制**：中间件能否提前返回（如缓存命中）？
   - 可以：直接返回，不调用 `next`

3. **可变状态**：中间件能否修改请求（如改路径）？
   - 可以：`next.call(&new_path)` 传新路径
