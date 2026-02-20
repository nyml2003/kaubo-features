# VFS 中间件路线图

**状态**: 待评审  
**目标**: 梳理所有中间件，确定实现优先级

---

## 中间件全景图

```
请求: read_file("/std/list.kaubo")
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  Stage 100: Outer（日志/监控）                                │
│  - LoggedLayer（记录操作日志）✅ P0                           │
│  - TracedLayer（分布式追踪）⏸️ P2                             │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  Stage 200: PreProcess（前置处理）                            │
│  - AuthLayer（权限检查）⏸️ P1                                 │
│  - RateLimitLayer（限流）⏸️ P2                                │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  Stage 300: Mapping（路径映射）                               │
│  - MappedLayer（静态路径映射）✅ P0                           │
│  - OverlayLayer（多目录合并）⏸️ P1                            │
│  - RelativeLayer（相对路径解析）⏸️ P1                         │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  Stage 400: Caching（缓存）                                   │
│  - CachedLayer（内容缓存）✅ P0                               │
│  - MetadataCachedLayer（元数据缓存）✅ P0                     │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  Stage 500: Core（实际存储）                                  │
│  - NativeFileSystem / MemoryFileSystem                        │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  Stage 600: PostProcess（后置处理）                           │
│  - WatchLayer（文件监听）⏸️ P2                                │
│  - IndexLayer（索引更新）⏸️ P2                                │
└─────────────────────────────────────────────────────────────┘
```

---

## P0 - 必须实现（多模块编译必需）

### 1. LoggedLayer
```rust
pub struct LoggedLayer {
    logger: Arc<Logger>,
    level: Level,  // 日志级别
}

impl LoggedLayer {
    pub fn new(logger: impl Into<Arc<Logger>>) -> Self;
    
    /// 设置慢操作阈值（默认 100ms）
    pub fn with_slow_threshold(self, threshold: Duration) -> Self;
}
```

**功能**:
- 记录所有 VFS 操作（read/write/exists）
- 记录操作耗时
- 慢操作警告

**示例输出**:
```
[DEBUG] vfs.read("/std/list.kaubo") = 2ms (hit)
[DEBUG] vfs.read("/vendor/foo.kaubo") = 15ms (miss)
[WARN]  slow operation: vfs.read("/network/file.kaubo") = 1.2s
```

---

### 2. MappedLayer
```rust
pub struct MappedLayer {
    mappings: Vec<(String, PathBuf)>,
}

impl MappedLayer {
    pub fn new() -> Self;
    
    /// 添加映射规则
    pub fn map(&mut self, prefix: &str, target: impl AsRef<Path>);
    
    /// 批量添加（便利方法）
    pub fn with_mappings<I>(mappings: I) -> Self
    where I: IntoIterator<Item = (impl AsRef<str>, impl AsRef<Path>)>;
}
```

**功能**:
- 静态路径映射（/std → /opt/kaubo/std）
- 支持多个映射规则
- 前缀匹配，第一个匹配生效

**决策项**: 
- [ ] **Q1**: 是否支持通配符映射？如 `/vendor/*` → `./vendor/*/src`
- [ ] **Q2**: 映射冲突时（多个规则匹配），报错还是按顺序优先？

---

### 3. CachedLayer
```rust
pub struct CachedLayer {
    content_cache: RwLock<LruCache<PathBuf, ContentEntry>>,
    ttl: Duration,
}

struct ContentEntry {
    content: Vec<u8>,
    loaded_at: Instant,
}

impl CachedLayer {
    pub fn new() -> Self;
    
    /// 设置缓存容量（默认 1000 条目）
    pub fn with_capacity(self, capacity: usize) -> Self;
    
    /// 设置 TTL（默认 60s）
    pub fn with_ttl(self, ttl: Duration) -> Self;
    
    /// 手动失效缓存
    pub fn invalidate(&self, path: &Path);
}
```

**功能**:
- LRU 缓存文件内容
- TTL 自动过期
- 支持手动失效

**决策项**:
- [ ] **Q3**: 缓存是否持久化到磁盘？（重启后恢复）
- [ ] **Q4**: 是否需要缓存写操作（write-through/write-behind）？

---

### 4. MetadataCachedLayer
```rust
pub struct MetadataCachedLayer {
    meta_cache: RwLock<HashMap<PathBuf, MetadataEntry>>,
}

struct MetadataEntry {
    size: u64,
    modified: SystemTime,
    is_file: bool,
}
```

**功能**:
- 缓存文件元数据（size、modified）
- 用于增量编译（检查文件是否修改）
- 与 CachedLayer 分离，因为元数据变化更频繁

**决策项**:
- [ ] **Q5**: MetadataCachedLayer 是否独立？还是合并到 CachedLayer？
  - 合并：简单，但粒度粗
  - 独立：灵活，可单独配置元数据 TTL（如 5s）vs 内容 TTL（如 60s）

---

## P1 - 重要（提升体验）

### 5. OverlayLayer
```rust
pub struct OverlayLayer {
    layers: Vec<PathBuf>,  // 上层优先
}

impl OverlayLayer {
    /// 创建 Overlay，后面的层优先
    pub fn new(layers: impl IntoIterator<Item = impl AsRef<Path>>) -> Self;
}
```

**功能**:
- 合并多个目录（类似 OverlayFS）
- 读操作：从上往下找，第一个存在即返回
- 写操作：默认写入最上层

**场景**: 
```rust
// 用户自定义 std 覆盖系统 std
OverlayLayer::new(&[
    "/home/user/.kaubo/std",  // 用户层（可写）
    "/opt/kaubo/std",          // 系统层（只读）
])
```

**决策项**:
- [ ] **Q6**: 是否必须 Phase 1 实现？还是等用户有需求再做？
- [ ] **Q7**: 写操作默认写入最上层，还是可配置？

---

### 6. RelativeLayer
```rust
pub struct RelativeLayer;

impl RelativeLayer {
    /// 从 base 目录解析相对路径
    pub fn with_base(base: impl AsRef<Path>) -> Self;
}
```

**功能**:
- 支持相对路径（`./utils`, `../common`）
- 将相对路径转换为绝对路径

**场景**:
```rust
// main.kaubo 中写 import "./utils"
// 从 /src/main.kaubo 导入，解析为 /src/utils.kaubo
```

**决策项**:
- [ ] **Q8**: 是否支持相对导入？Phase 1 还是 Phase 2？

---

### 7. AuthLayer
```rust
pub struct AuthLayer {
    allowed_prefixes: Vec<PathBuf>,  // 允许访问的路径
}

impl AuthLayer {
    /// 创建沙盒，只允许访问指定目录
    pub fn sandbox(allowed: impl IntoIterator<Item = impl AsRef<Path>>) -> Self;
}
```

**功能**:
- 路径白名单
- 防止路径逃逸（如 `../../../etc/passwd`）

**决策项**:
- [ ] **Q9**: 是否必须？当前用户代码都是本地可信代码

---

## P2 - 可选（未来扩展）

### 8. WatchLayer
- 文件监听、热重载
- 文件修改时触发回调

### 9. CompressionLayer
- 透明压缩（Zstd、Snappy）
- 适合大文件存储

### 10. EncryptionLayer
- 透明加密
- 适合敏感文件

### 11. RemoteLayer
- HTTP/S3 远程文件系统
- 延迟加载

### 12. TracedLayer
- OpenTelemetry 分布式追踪
- 性能分析

### 13. RateLimitLayer
- 限流（防止过多 IO）

---

## 阶段定义

| Stage | 名称 | 用途 | 当前中间件 |
|-------|------|------|-----------|
| 100 | Outer | 日志、追踪、监控 | LoggedLayer |
| 200 | PreProcess | 权限、限流、重试 | AuthLayer |
| 300 | Mapping | 路径映射、Overlay、相对路径 | MappedLayer, OverlayLayer |
| 400 | Caching | 内容缓存、元数据缓存 | CachedLayer, MetadataCachedLayer |
| 500 | Core | 实际存储 | Native/Memory |
| 600 | PostProcess | 监听、索引 | WatchLayer |

---

## 实现计划

### Phase 1（1-2 周）
实现 P0 中间件，支撑多模块编译：
1. **LoggedLayer** - 日志记录
2. **MappedLayer** - 路径映射  
3. **CachedLayer** - 内容缓存
4. **MetadataCachedLayer** - 元数据缓存（或与 CachedLayer 合并）

### Phase 2（后续）
根据需求实现 P1：
- OverlayLayer（如用户需要覆盖 std）
- RelativeLayer（如需要相对导入）
- AuthLayer（如需要安全沙盒）

### Phase 3（未来）
根据场景实现 P2：
- WatchLayer（热重载）
- RemoteLayer（远程依赖）
- 其他...

---

## 待决策项汇总

| 编号 | 问题 | 选项 | 建议 |
|------|------|------|------|
| Q1 | MappedLayer 通配符支持 | A. 支持 `/vendor/*` → `./vendor/*/src`<br>B. 仅前缀匹配 | **B**（简单优先） |
| Q2 | 映射冲突处理 | A. 按顺序优先（第一个匹配）<br>B. 报错 | **A** |
| Q3 | 缓存持久化 | A. 支持磁盘持久化<br>B. 仅内存 | **B**（Phase 1） |
| Q4 | 写缓存策略 | A. write-through（同步写）<br>B. write-behind（异步写）<br>C. 不写缓存 | **C**（简单） |
| Q5 | MetadataCachedLayer | A. 独立中间件<br>B. 合并到 CachedLayer | **A**（更灵活） |
| Q6 | OverlayLayer 优先级 | A. Phase 1（必须）<br>B. Phase 2（重要） | **B**（多模块编译不依赖） |
| Q7 | OverlayLayer 写策略 | A. 固定最上层<br>B. 可配置写入层 | **A**（简单） |
| Q8 | 相对导入支持 | A. Phase 1<br>B. Phase 2<br>C. 不支持 | **B**（延后） |
| Q9 | AuthLayer 必要性 | A. 必须（安全）<br>B. 延后（可信环境） | **B** |

---

## 下一步

1. **评审决策项**（Q1-Q9）
2. **确认 Phase 1 范围**
3. **开始实现**：先实现中间件框架，再逐个实现中间件
