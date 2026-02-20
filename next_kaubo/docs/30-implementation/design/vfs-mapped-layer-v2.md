# MappedLayer V2 - 基于 Namespace 的结构化映射

**状态**: 重新设计中  
**核心变更**: 从字符串前缀匹配 → 基于 Namespace 的结构化映射

---

## 问题分析

### V1 的问题

```rust
// V1: 字符串前缀匹配，容易混淆
[
    ("/std/internal", "/home/user/custom-std"),  // 这是啥？
    ("/std", "/opt/kaubo/std"),
    ("/vendor", "./vendor"),
]

// 问题：
// 1. "/std/internal" 是一个特殊覆盖？还是标准库子模块？
// 2. 用户怎么知道该用什么前缀？
// 3. 相对路径 "./vendor" 相对于谁？
```

### V2 的核心思想

```rust
// 模块来源是固定的几种，枚举全了：
// 1. Std      -> 系统 kaubo 安装目录
// 2. Local    -> 当前项目 workspace
// 3. Vendor   -> 第三方依赖（vendor 目录）

// 没有相对路径，没有任意字符串匹配
```

---

## 模块来源枚举

```rust
/// 模块命名空间（来源）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// 标准库（系统安装）
    /// import "std.list" -> Namespace::Std
    Std,
    
    /// 当前项目（本地模块）
    /// import "math" -> Namespace::Local
    Local,
    
    /// 第三方依赖
    /// import "vendor.http" -> Namespace::Vendor("http")
    Vendor(String),
}

impl Namespace {
    /// 解析 import 路径
    /// "std.list"      -> Std
    /// "math"          -> Local
    /// "vendor.http"   -> Vendor("http")
    /// "vendor.react.dom" -> Vendor("react")
    pub fn parse(path: &str) -> Result<(Self, Vec<String>), ParseError> {
        let parts: Vec<&str> = path.split('.').collect();
        
        match parts.as_slice() {
            ["std", rest @ ..] if !rest.is_empty() => {
                Ok((Self::Std, rest.iter().map(|s| s.to_string()).collect()))
            }
            ["vendor", name, rest @ ..] => {
                let vendor_name = name.to_string();
                let components = rest.iter().map(|s| s.to_string()).collect();
                Ok((Self::Vendor(vendor_name), components))
            }
            [first, rest @ ..] if !first.is_empty() => {
                // 不以 std. 或 vendor. 开头的，都是本地模块
                let components = parts.into_iter().map(|s| s.to_string()).collect();
                Ok((Self::Local, components))
            }
            _ => Err(ParseError::InvalidModulePath(path.to_string())),
        }
    }
}

/// 模块完整标识
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    pub namespace: Namespace,
    pub components: Vec<String>,
}

impl ModuleId {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let (namespace, components) = Namespace::parse(s)?;
        Ok(Self { namespace, components })
    }
    
    /// 转换为 VFS 路径（逻辑路径）
    /// Std(["list"])    -> "/std/list.kaubo"
    /// Local(["math"])  -> "/local/math.kaubo"
    /// Vendor("http", ["client"]) -> "/vendor/http/client.kaubo"
    pub fn to_vfs_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        
        match &self.namespace {
            Namespace::Std => {
                path.push("std");
            }
            Namespace::Local => {
                path.push("local");
            }
            Namespace::Vendor(name) => {
                path.push("vendor");
                path.push(name);
            }
        }
        
        for comp in &self.components {
            path.push(comp);
        }
        
        path.set_extension("kaubo");
        path
    }
}
```

---

## 结构化映射层

```rust
/// 模块上下文（映射配置）
pub struct ModuleContext {
    /// 标准库路径（系统安装目录）
    /// 如：/opt/kaubo/std 或 C:\Program Files\Kaubo\std
    pub std_path: PathBuf,
    
    /// 项目 workspace 路径
    /// 如：/home/user/my-project/src
    pub workspace_path: PathBuf,
    
    /// vendor 目录路径
    /// 如：/home/user/my-project/vendor
    pub vendor_path: PathBuf,
}

/// 映射层（V2）
pub struct MappedLayer {
    ctx: ModuleContext,
}

impl MappedLayer {
    pub fn new(ctx: ModuleContext) -> Self {
        Self { ctx }
    }
    
    /// 核心映射逻辑（无字符串匹配，直接匹配 namespace）
    pub fn resolve(&self, id: &ModuleId) -> PathBuf {
        match &id.namespace {
            Namespace::Std => {
                // 标准库 -> 系统安装目录
                let mut path = self.ctx.std_path.clone();
                for comp in &id.components {
                    path.push(comp);
                }
                path.set_extension("kaubo");
                path
            }
            Namespace::Local => {
                // 本地模块 -> 项目 workspace
                let mut path = self.ctx.workspace_path.clone();
                for comp in &id.components {
                    path.push(comp);
                }
                path.set_extension("kaubo");
                path
            }
            Namespace::Vendor(name) => {
                // 第三方依赖 -> vendor/{name}/
                let mut path = self.ctx.vendor_path.clone();
                path.push(name);
                for comp in &id.components {
                    path.push(comp);
                }
                path.set_extension("kaubo");
                path
            }
        }
    }
    
    /// 从 VFS 逻辑路径解析 ModuleId（反向解析）
    pub fn parse_vfs_path(&self, path: &Path) -> Option<ModuleId> {
        let path_str = path.to_string_lossy();
        
        // /std/xxx
        if let Some(rest) = path_str.strip_prefix("/std/") {
            let components: Vec<String> = rest
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
            let components: Vec<String> = rest
                .trim_end_matches(".kaubo")
                .split('/')
                .map(|s| s.to_string())
                .collect();
            return Some(ModuleId {
                namespace: Namespace::Local,
                components,
            });
        }
        
        // /vendor/{name}/xxx
        if let Some(rest) = path_str.strip_prefix("/vendor/") {
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() >= 1 {
                let name = parts[0].to_string();
                let components: Vec<String> = parts
                    .get(1)
                    .map(|s| s.trim_end_matches(".kaubo").split('/').map(|s| s.to_string()).collect())
                    .unwrap_or_default();
                return Some(ModuleId {
                    namespace: Namespace::Vendor(name),
                    components,
                });
            }
        }
        
        None
    }
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> VfsResult<Vec<u8>> {
        // 尝试解析为 ModuleId
        if let Some(id) = self.parse_vfs_path(path) {
            let real_path = self.resolve(&id);
            next.call(&real_path)
        } else {
            // 不是模块路径，原样传递
            next.call(path)
        }
    }
}
```

---

## 使用示例

### 1. 配置 ModuleContext

```rust
// 自动检测系统标准库路径
let std_path = detect_std_path()?;
// Linux: /opt/kaubo/std
// macOS: /usr/local/lib/kaubo/std
// Windows: C:\Program Files\Kaubo\std

// 从 package.json 或命令行参数获取项目路径
let workspace_path = PathBuf::from("./src");
let vendor_path = PathBuf::from("./vendor");

let ctx = ModuleContext {
    std_path,
    workspace_path,
    vendor_path,
};

let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(MappedLayer::new(ctx))
    .build();
```

### 2. 模块解析流程

```rust
// 用户代码：import "std.list";

// 1. ModuleId::parse("std.list")
//    -> ModuleId { namespace: Std, components: ["list"] }

// 2. id.to_vfs_path()
//    -> "/std/list.kaubo"

// 3. vfs.read_file("/std/list.kaubo")
//    -> MappedLayer 拦截

// 4. MappedLayer 解析 VFS 路径
//    -> ModuleId { namespace: Std, components: ["list"] }

// 5. MappedLayer::resolve(id)
//    -> ctx.std_path + "list.kaubo"
//    -> "/opt/kaubo/std/list.kaubo"

// 6. 实际读取文件
```

### 3. 不同来源的模块

```rust
// 标准库
import "std.list";           // -> /opt/kaubo/std/list.kaubo
import "std.fs.path";        // -> /opt/kaubo/std/fs/path.kaubo

// 本地模块
import "math";               // -> ./src/math.kaubo
import "utils.helper";       // -> ./src/utils/helper.kaubo

// 第三方依赖
import "vendor.http.client"; // -> ./vendor/http/client.kaubo
import "vendor.react";       // -> ./vendor/react.kaubo
```

---

## 路径来源总结

| import 语句 | Namespace | 物理路径 | 说明 |
|------------|-----------|---------|------|
| `import "std.xxx"` | Std | `{std_path}/xxx.kaubo` | 系统安装的标准库 |
| `import "xxx"` | Local | `{workspace_path}/xxx.kaubo` | 项目本地模块 |
| `import "vendor.xxx.yyy"` | Vendor("xxx") | `{vendor_path}/xxx/yyy.kaubo` | 第三方依赖 |

**清晰、无歧义、枚举全了**。

---

## 配置来源

```rust
/// ModuleContext 构建器
pub struct ModuleContextBuilder {
    std_path: Option<PathBuf>,
    workspace_path: Option<PathBuf>,
    vendor_path: Option<PathBuf>,
}

impl ModuleContextBuilder {
    pub fn new() -> Self {
        Self {
            std_path: None,
            workspace_path: None,
            vendor_path: None,
        }
    }
    
    /// 自动检测标准库路径
    pub fn detect_std(mut self) -> Result<Self, Error> {
        self.std_path = Some(detect_std_path()?);
        Ok(self)
    }
    
    /// 从环境变量读取
    pub fn from_env(mut self) -> Self {
        if let Ok(path) = env::var("KABO_STD_PATH") {
            self.std_path = Some(path.into());
        }
        if let Ok(path) = env::var("KABO_WORKSPACE") {
            self.workspace_path = Some(path.into());
        }
        self
    }
    
    /// 从 package.json 读取
    pub fn from_package_json(mut self, path: &Path) -> Result<Self, Error> {
        let package = read_package_json(path)?;
        self.workspace_path = Some(path.parent().unwrap().join("src"));
        self.vendor_path = Some(path.parent().unwrap().join("vendor"));
        Ok(self)
    }
    
    pub fn build(self) -> Result<ModuleContext, Error> {
        Ok(ModuleContext {
            std_path: self.std_path.ok_or(Error::MissingStdPath)?,
            workspace_path: self.workspace_path.ok_or(Error::MissingWorkspace)?,
            vendor_path: self.vendor_path.unwrap_or_else(|| PathBuf::from("./vendor")),
        })
    }
}

// 使用
let ctx = ModuleContextBuilder::new()
    .detect_std()?                    // 自动检测
    .from_env()                       // 环境变量覆盖
    .from_package_json("./package.json")?  // 配置文件
    .build()?;
```

---

## 与 V1 对比

| 特性 | V1（字符串前缀） | V2（结构化 Namespace） |
|------|-----------------|----------------------|
| 映射规则 | 任意字符串前缀 | 固定的 3 种 Namespace |
| 配置方式 | 前缀 -> 目标 | 标准库路径 + 项目路径 + vendor 路径 |
| 相对路径 | 支持（有歧义） | 不支持（清晰） |
| 可扩展性 | 任意扩展 | 新增 Namespace 类型（Std/Local/Vendor） |
| 用户理解 | 需要知道前缀规则 | import 语法即规则 |
| 实现复杂度 | 中等（前缀匹配） | 简单（直接 match） |

---

## 决策项

| 编号 | 问题 | 建议 |
|------|------|------|
| V2-1 | 是否支持自定义 Namespace？ | **否**（Phase 1），Std/Local/Vendor 够用 |
| V2-2 | Vendor 是否支持版本？如 vendor.http@2.0 | **否**（Phase 1），用 vendor/http-2.0/ 目录 |
| V2-3 | 本地模块是否支持子目录映射？ | **否**，workspace 下按 components 层级 |
| V2-4 | 标准库路径检测策略？ | A. 编译期硬编码<br>B. 运行时检测（推荐） |

---

这个设计是否更合理？更清晰？
