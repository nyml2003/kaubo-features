# MappedLayer V3 - 简化版（无 Vendor）

**状态**: Phase 1 实现版  
**核心变更**: 仅支持 Std + Local，Vendor 延后

---

## 简化设计

### 仅支持两种来源

```rust
/// 模块命名空间（Phase 1 仅支持两种）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// 标准库（系统安装）
    /// import "std.list"
    Std,
    
    /// 当前项目（本地模块）
    /// import "math"
    Local,
}

impl Namespace {
    /// 解析 import 路径
    pub fn parse(path: &str) -> Result<(Self, Vec<String>), ParseError> {
        let parts: Vec<&str> = path.split('.').collect();
        
        match parts.as_slice() {
            // 以 std. 开头 -> 标准库
            ["std", rest @ ..] if !rest.is_empty() => {
                Ok((Self::Std, rest.iter().map(|s| s.to_string()).collect()))
            }
            // 不以 std. 开头 -> 本地模块
            [first, rest @ ..] if !first.is_empty() => {
                let components = parts.into_iter().map(|s| s.to_string()).collect();
                Ok((Self::Local, components))
            }
            _ => Err(ParseError::InvalidModulePath(path.to_string())),
        }
    }
}
```

---

## ModuleContext（极简）

```rust
/// 模块上下文（Phase 1：仅 std + workspace）
pub struct ModuleContext {
    /// 标准库路径
    /// 如：/opt/kaubo/std
    pub std_path: PathBuf,
    
    /// 项目 workspace 路径
    /// 如：/home/user/project/src
    pub workspace_path: PathBuf,
}

impl ModuleContext {
    /// 创建（仅用于测试）
    pub fn new(std_path: impl AsRef<Path>, workspace_path: impl AsRef<Path>) -> Self {
        Self {
            std_path: std_path.as_ref().to_path_buf(),
            workspace_path: workspace_path.as_ref().to_path_buf(),
        }
    }
    
    /// 自动检测标准库路径
    pub fn detect() -> Result<Self, Error> {
        let std_path = detect_std_path()?;
        let workspace_path = std::env::current_dir()?.join("src");
        
        Ok(Self {
            std_path,
            workspace_path,
        })
    }
}

/// 自动检测标准库路径
fn detect_std_path() -> Result<PathBuf, Error> {
    // 1. 环境变量覆盖
    if let Ok(path) = std::env::var("KABO_STD_PATH") {
        return Ok(path.into());
    }
    
    // 2. 按平台默认
    #[cfg(target_os = "linux")]
    return Ok(PathBuf::from("/opt/kaubo/std"));
    
    #[cfg(target_os = "macos")]
    return Ok(PathBuf::from("/usr/local/lib/kaubo/std"));
    
    #[cfg(target_os = "windows")]
    return Ok(PathBuf::from(r"C:\Program Files\Kaubo\std"));
}
```

---

## MappedLayer 实现

```rust
pub struct MappedLayer {
    ctx: ModuleContext,
}

impl MappedLayer {
    pub fn new(ctx: ModuleContext) -> Self {
        Self { ctx }
    }
    
    /// 解析 VFS 路径为 ModuleId
    pub fn parse_path(&self, path: &Path) -> Option<ModuleId> {
        let path_str = path.to_string_lossy();
        
        // /std/xxx
        if let Some(rest) = path_str.strip_prefix("/std/") {
            let components = rest
                .trim_end_matches(".kaubo")
                .split('/')
                .map(|s| s.to_string())
                .collect();
            return Some(ModuleId {
                namespace: Namespace::Std,
                components,
            });
        }
        
        // /local/xxx
        if let Some(rest) = path_str.strip_prefix("/local/") {
            let components = rest
                .trim_end_matches(".kaubo")
                .split('/')
                .map(|s| s.to_string())
                .collect();
            return Some(ModuleId {
                namespace: Namespace::Local,
                components,
            });
        }
        
        None
    }
    
    /// 模块 ID 解析为物理路径
    pub fn resolve(&self, id: &ModuleId) -> PathBuf {
        match id.namespace {
            Namespace::Std => {
                let mut path = self.ctx.std_path.clone();
                for comp in &id.components {
                    path.push(comp);
                }
                path.set_extension("kaubo");
                path
            }
            Namespace::Local => {
                let mut path = self.ctx.workspace_path.clone();
                for comp in &id.components {
                    path.push(comp);
                }
                path.set_extension("kaubo");
                path
            }
        }
    }
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> VfsResult<Vec<u8>> {
        if let Some(id) = self.parse_path(path) {
            let real_path = self.resolve(&id);
            next.call(&real_path)
        } else {
            next.call(path)
        }
    }
}
```

---

## 使用示例

### 1. 基本使用

```rust
let ctx = ModuleContext::detect()?;  // 自动检测

let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(MappedLayer::new(ctx))
    .build();

// 读取标准库
let content = vfs.read_file(Path::new("/std/list.kaubo"))?;

// 读取本地模块
let content = vfs.read_file(Path::new("/local/math.kaubo"))?;
```

### 2. 测试使用

```rust
let ctx = ModuleContext::new(
    "/opt/kaubo/std",      // 标准库路径
    "./src",                // 项目路径
);

let vfs = VfsBuilder::new(MemoryFileSystem::with_files(&[
    ("/opt/kaubo/std/list.kaubo", b"pub var x = 1;"),
    ("./src/math.kaubo", b"pub var y = 2;"),
]))
.with(MappedLayer::new(ctx))
.build();
```

---

## 模块解析流程

```rust
// 用户代码：import "std.list";

// 1. 解析为 ModuleId
let id = ModuleId::parse("std.list")?;
// -> ModuleId { namespace: Std, components: ["list"] }

// 2. 生成 VFS 路径
let vfs_path = id.to_vfs_path();
// -> "/std/list.kaubo"

// 3. VFS 读取（MappedLayer 拦截）
let content = vfs.read_file(&vfs_path)?;
// MappedLayer 解析 -> Namespace::Std
// -> resolve() 返回 /opt/kaubo/std/list.kaubo
// -> 实际读取文件
```

---

## Phase 1 范围

| 特性 | 支持 | 说明 |
|------|------|------|
| Std 命名空间 | ✅ | `import "std.xxx"` |
| Local 命名空间 | ✅ | `import "xxx"` |
| Vendor 命名空间 | ❌ | 后续支持 |
| 相对导入 | ❌ | 不支持 `./utils` |
| 动态映射 | ❌ | 仅静态配置 |

---

## 后续扩展（Vendor）

```rust
// Phase 2 支持 Vendor
pub enum Namespace {
    Std,
    Local,
    Vendor(String),  // 新增
}

// import "vendor.http"
// -> ModuleId { Vendor("http"), ["client"] }

// 需要设计：
// 1. vendor 目录结构（vendor/http/ 还是 vendor/http@2.0/）
// 2. 版本管理
// 3. 依赖解析（vendor 的依赖怎么处理）
```

---

## 确认

Phase 1 仅实现：
- ✅ Namespace::Std（标准库）
- ✅ Namespace::Local（本地模块）
- ❌ Namespace::Vendor（第三方依赖，后续支持）

**确认后我开始实现。**
