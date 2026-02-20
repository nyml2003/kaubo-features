# MappedLayer 路径映射层详细设计

**状态**: 设计中  
**用途**: 将逻辑路径映射到物理路径，支持多模块编译中的标准库/依赖路径解析

---

## 核心问题

为什么需要路径映射？

```rust
// 用户代码写：
import "std.list";

// 实际文件可能在：
// - Linux:   /opt/kaubo/std/list.kaubo
// - macOS:   /usr/local/lib/kaubo/std/list.kaubo
// - Windows: C:\Program Files\Kaubo\std\list.kaubo
// - 项目本地: ./vendor/kaubo-stdlib/list.kaubo

// 甚至可能是虚拟路径：
// - 网络: https://cdn.kaubo.dev/std/list.kaubo
// - 内存: (内置标准库)
```

**目标**: 代码中的 `import "std.list"` 与实际文件位置解耦

---

## 映射模型

### 1. 前缀匹配模型

```rust
// 映射表（按顺序匹配，第一个命中生效）
[
    ("/std", "/opt/kaubo/std"),
    ("/vendor", "./vendor"),
    ("/project", "/home/user/myproject/src"),
]

// 转换示例
"/std/list.kaubo"           → "/opt/kaubo/std/list.kaubo"
"/vendor/http.kaubo"        → "./vendor/http.kaubo"
"/vendor/internal/util"     → "./vendor/internal/util"
"/project/main.kaubo"       → "/home/user/myproject/src/main.kaubo"
"/other/file.kaubo"         → "/other/file.kaubo" (无匹配，原样返回)
```

### 2. 与 ModuleId 的配合

```rust
// ModuleId 生成逻辑路径（统一格式）
let id = ModuleId::parse("std.list")?;
// id.namespace = Std
// id.components = ["list"]

let logical_path = id.to_vfs_path();
// logical_path = "/std/list.kaubo"

// MappedLayer 将逻辑路径映射到物理路径
let physical_path = mapped_layer.resolve(&logical_path);
// physical_path = "/opt/kaubo/std/list.kaubo"
```

**分层职责**:
- `ModuleId`: 语言层，处理 import 语法
- `to_vfs_path()`: 协议层，生成统一逻辑路径
- `MappedLayer`: 存储层，映射到实际文件系统

---

## 实现设计

### 基础版（P0）

```rust
pub struct MappedLayer {
    /// 映射规则（前缀 -> 目标目录）
    /// 按优先级排序，第一个匹配生效
    mappings: Vec<Mapping>,
}

struct Mapping {
    /// 前缀（如 "/std"）
    prefix: String,
    /// 目标目录（如 "/opt/kaubo/std"）
    target: PathBuf,
}

impl MappedLayer {
    pub fn new() -> Self {
        Self { mappings: vec![] }
    }
    
    /// 添加映射规则
    /// 后添加的优先级更高（插入到队首）
    pub fn map(&mut self, prefix: &str, target: impl AsRef<Path>) {
        self.mappings.insert(0, Mapping {
            prefix: prefix.to_string(),
            target: target.as_ref().to_path_buf(),
        });
    }
    
    /// 批量添加（便利方法）
    pub fn with_mappings<I>(mut self, mappings: I) -> Self
    where
        I: IntoIterator<Item = (impl AsRef<str>, impl AsRef<Path>)>,
    {
        for (prefix, target) in mappings {
            self.map(prefix.as_ref(), target);
        }
        self
    }
    
    /// 路径解析（核心方法）
    fn resolve(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        
        for mapping in &self.mappings {
            // 前缀匹配：/std/list 匹配 /std
            if let Some(rest) = path_str.strip_prefix(&mapping.prefix) {
                // 处理边界：/stdlist 不应该匹配 /std
                // 要求匹配后紧跟 "/" 或者是完全匹配
                if rest.is_empty() || rest.starts_with('/') {
                    return mapping.target.join(rest.trim_start_matches('/'));
                }
            }
        }
        
        // 无匹配，原样返回
        path.to_path_buf()
    }
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping  // 300
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> VfsResult<Vec<u8>> {
        let real_path = self.resolve(path);
        next.call(&real_path)
    }
    
    fn exists(&self, path: &Path, next: Next<'_>) -> bool {
        let real_path = self.resolve(path);
        next.call_exists(&real_path)
    }
    
    // ... 其他方法同理
}
```

### 使用示例

```rust
let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(MappedLayer::new()
        .with_mappings(&[
            ("/std", "/opt/kaubo/std"),
            ("/vendor", "./vendor"),
        ])
    )
    .build();

// 使用逻辑路径访问
let content = vfs.read_file(Path::new("/std/list.kaubo"))?;
// 实际读取: /opt/kaubo/std/list.kaubo
```

---

## 边界情况处理

### 1. 精确匹配 vs 前缀匹配

```rust
// 映射表
[
    ("/std", "/opt/kaubo/std"),
    ("/stdlib", "/opt/kaubo/stdlib"),
]

// 边界情况
"/std/list.kaubo"      → "/opt/kaubo/std/list.kaubo"    ✅ 匹配 /std
"/stdlib/math.kaubo"   → "/opt/kaubo/stdlib/math.kaubo" ✅ 匹配 /stdlib（不是 /std）
"/stdlist"             → (无匹配，原样返回)              ✅ 不是 /std 的子路径
```

**关键规则**: 前缀匹配要求匹配后是路径结尾（空）或路径分隔符（/）

### 2. 多层嵌套映射

```rust
// 映射表（优先级从高到低）
[
    ("/std/internal", "/home/user/custom-std"),
    ("/std", "/opt/kaubo/std"),
]

// 转换
"/std/internal/debug.kaubo" → "/home/user/custom-std/debug.kaubo"  ✅ 优先匹配长的
"/std/list.kaubo"           → "/opt/kaubo/std/list.kaubo"
```

**规则**: 长的前缀优先（/std/internal 优先于 /std）

### 3. 尾部斜杠处理

```rust
// 用户可能写：
map("/std", "/opt/kaubo/std");
map("/std/", "/opt/kaubo/std");  // 多了斜杠

// 应该等价处理，内部标准化
```

**实现**: 内部统一去除尾部斜杠

### 4. 空映射（无匹配）

```rust
// 无映射规则时
let layer = MappedLayer::new();
let path = layer.resolve(Path::new("/any/path"));
// path = "/any/path" (原样返回)
```

**行为**: 无匹配时原样返回，不报错（透明的）

---

## 高级功能（P1/P2 考虑）

### 1. 动态映射（回调函数）

```rust
pub type MappingFn = Box<dyn Fn(&Path) -> Option<PathBuf> + Send + Sync>;

pub struct MappedLayer {
    static_mappings: Vec<Mapping>,
    dynamic_mapping: Option<MappingFn>,
}

impl MappedLayer {
    /// 设置动态映射函数
    pub fn with_dynamic<F>(mut self, f: F) -> Self
    where
        F: Fn(&Path) -> Option<PathBuf> + Send + Sync + 'static,
    {
        self.dynamic_mapping = Some(Box::new(f));
        self
    }
}

// 使用场景：根据环境变量动态决定路径
let layer = MappedLayer::new()
    .with_dynamic(|path| {
        if path.starts_with("/std") {
            let custom_std = env::var("KABO_STD_PATH").ok()?;
            Some(PathBuf::from(custom_std))
        } else {
            None
        }
    });
```

**决策**: Phase 1 不做，需要时再加

### 2. 反向映射（物理→逻辑）

```rust
impl MappedLayer {
    /// 将物理路径转换为逻辑路径（用于错误信息）
    pub fn reverse(&self, physical: &Path) -> Option<PathBuf> {
        for mapping in &self.mappings {
            if let Ok(stripped) = physical.strip_prefix(&mapping.target) {
                return Some(PathBuf::from(&mapping.prefix).join(stripped));
            }
        }
        None
    }
}

// 使用：错误信息更友好
// 原：Error: file not found /opt/kaubo/std/missing.kaubo
// 改：Error: file not found /std/missing.kaubo
```

**决策**: Phase 1 不做，错误信息优化后续处理

### 3. 映射缓存

```rust
pub struct MappedLayer {
    mappings: Vec<Mapping>,
    /// 缓存解析结果（路径 -> 结果）
    resolve_cache: RwLock<HashMap<PathBuf, PathBuf>>,
}
```

**决策**: 不需要，路径解析是简单的字符串操作，足够快

---

## 配置来源

映射规则可以从多个来源加载：

### 1. 硬编码（默认）

```rust
// kaubo-cli 中硬编码标准库路径
let std_path = if cfg!(target_os = "linux") {
    "/opt/kaubo/std"
} else if cfg!(target_os = "macos") {
    "/usr/local/lib/kaubo/std"
} else {
    "C:\\Program Files\\Kaubo\\std"
};
```

### 2. 配置文件（package.json）

```json
{
  "name": "my-project",
  "dependencies": {
    "http": "^1.0.0"
  },
  "vfs": {
    "mappings": {
      "/vendor/http": "./vendor/http/src",
      "/vendor/json": "./vendor/json-parser/lib"
    }
  }
}
```

### 3. 环境变量

```bash
export KABO_STD_PATH=/home/user/custom-std
export KABO_VENDOR_PATH=./my-vendor
```

### 4. 命令行参数

```bash
kaubo run --map /std=/opt/kaubo/std --map /vendor=./vendor main.kaubo
```

**优先级**（从高到低）：
1. 命令行参数
2. 环境变量
3. 配置文件
4. 硬编码默认值

---

## 与其他中间件的配合

### 执行顺序

```
请求: "/std/list.kaubo"
    │
    ▼
┌─────────────────┐
│ LoggedLayer     │ 记录: "read /std/list.kaubo"
│ Stage::Outer    │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ MappedLayer     │ 转换: "/std/list.kaubo" → "/opt/kaubo/std/list.kaubo"
│ Stage::Mapping  │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ CachedLayer     │ 查缓存（key 是映射后的路径）
│ Stage::Caching  │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ NativeFileSystem│ 实际读取: /opt/kaubo/std/list.kaubo
│ Stage::Core     │
└─────────────────┘
```

### 关键问题：缓存 key 用逻辑路径还是物理路径？

```rust
// 选项 A: 逻辑路径作为 key
CachedLayer { key: "/std/list.kaubo" }
// 优点：修改映射后缓存仍然有效（只要内容不变）
// 缺点：同一个物理文件有两个逻辑路径，会缓存两份

// 选项 B: 物理路径作为 key  
CachedLayer { key: "/opt/kaubo/std/list.kaubo" }
// 优点：唯一对应一个文件
// 缺点：修改映射需要清空缓存
```

**决策**: 选项 B（物理路径作为 key）
- 原因：MappedLayer 在 CachedLayer 之前执行，CachedLayer 看到的是物理路径
- 修改映射是低频操作，清空缓存可接受

---

## 测试策略

### 1. 单元测试

```rust
#[test]
fn test_basic_mapping() {
    let layer = MappedLayer::new()
        .with_mappings(&[("/std", "/opt/kaubo/std")]);
    
    assert_eq!(
        layer.resolve(Path::new("/std/list.kaubo")),
        PathBuf::from("/opt/kaubo/std/list.kaubo")
    );
}

#[test]
fn test_no_mapping() {
    let layer = MappedLayer::new();
    
    assert_eq!(
        layer.resolve(Path::new("/other/file.kaubo")),
        PathBuf::from("/other/file.kaubo")
    );
}

#[test]
fn test_prefix_boundary() {
    let layer = MappedLayer::new()
        .with_mappings(&[("/std", "/opt/kaubo/std")]);
    
    // /stdlist 不应该匹配 /std
    assert_eq!(
        layer.resolve(Path::new("/stdlist")),
        PathBuf::from("/stdlist")
    );
}

#[test]
fn test_longest_prefix_wins() {
    let layer = MappedLayer::new()
        .with_mappings(&[
            ("/std", "/opt/kaubo/std"),
            ("/std/internal", "/home/user/custom"),
        ]);
    
    assert_eq!(
        layer.resolve(Path::new("/std/internal/debug.kaubo")),
        PathBuf::from("/home/user/custom/debug.kaubo")
    );
}
```

### 2. 集成测试

```rust
#[test]
fn test_mapped_vfs() {
    let vfs = VfsBuilder::new(MemoryFileSystem::with_files(&[
        ("/real/std/list.kaubo", b"pub var x = 1;"),
    ]))
    .with(MappedLayer::new()
        .with_mappings(&[("/std", "/real/std")])
    )
    .build();
    
    // 用逻辑路径读取
    let content = vfs.read_file(Path::new("/std/list.kaubo")).unwrap();
    assert_eq!(content, b"pub var x = 1;");
}
```

---

## 决策项

| 编号 | 问题 | 选项 | 建议 |
|------|------|------|------|
| M1 | 是否支持动态映射（回调函数）？ | A. Phase 1 支持<br>B. 后续再加 | **B** |
| M2 | 是否支持反向映射（物理→逻辑）？ | A. Phase 1 支持<br>B. 后续再加 | **B** |
| M3 | 配置来源优先级？ | A. 命令行 > 环境变量 > 配置 > 硬编码<br>B. 其他顺序 | **A** |
| M4 | 是否支持通配符 `*` 匹配？ | A. 支持 `/vendor/*`<br>B. 仅前缀匹配 | **B** |
| M5 | 映射失败（目标目录不存在）如何处理？ | A. 静默忽略（后续操作会报错）<br>B. 构建时检查并报错 | **A** |

---

## 总结

**MappedLayer 核心职责**:
- 将逻辑路径（`/std/list.kaubo`）映射到物理路径（`/opt/kaubo/std/list.kaubo`）
- 前缀匹配，最长优先
- 无匹配时原样返回

**Phase 1 实现范围**:
- 静态映射（prefix → target）
- 基础边界处理（前缀边界、尾部斜杠）
- 不涉及动态映射、反向映射、通配符
