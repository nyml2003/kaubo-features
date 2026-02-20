# MappedLayer V4 - 搜索路径模型（极简）

**状态**: Phase 1 实现版  
**核心洞察**: 标准库和本地模块没有本质区别，只是查找路径不同

---

## 核心设计

### 没有 Namespace，只有搜索路径

```rust
/// 模块上下文（搜索路径列表）
pub struct ModuleContext {
    /// 搜索路径（按优先级排序）
    /// 前面的路径优先于后面的
    pub search_paths: Vec<PathBuf>,
}

impl ModuleContext {
    /// 默认搜索路径
    pub fn default() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("./src"),              // 本地优先
                detect_std_path().unwrap_or_default(), // 系统标准库
            ],
        }
    }
    
    /// 从环境变量/配置构建
    pub fn from_env() -> Self {
        let mut paths = vec![];
        
        // 1. 本地路径
        paths.push(PathBuf::from("./src"));
        
        // 2. 环境变量 KABO_PATH（类似 PYTHONPATH）
        if let Ok(path_str) = std::env::var("KABO_PATH") {
            for p in path_str.split(':') {
                paths.push(PathBuf::from(p));
            }
        }
        
        // 3. 系统标准库
        if let Ok(std_path) = detect_std_path() {
            paths.push(std_path);
        }
        
        Self { search_paths: paths }
    }
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
    
    /// 解析模块路径为物理路径
    /// 
    /// 逻辑路径格式：/mod/xxx.yyy
    /// - /mod/ 是虚拟前缀
    /// - xxx.yyy 是模块名（点号表示子目录）
    /// 
    /// 转换示例：
    /// /mod/list.kaubo -> 在 search_paths 中查找 list.kaubo
    /// /mod/math.utils -> 在 search_paths 中查找 math/utils.kaubo
    pub fn resolve(&self, path: &Path) -> Option<PathBuf> {
        let path_str = path.to_string_lossy();
        
        // 检查是否是模块路径（以 /mod/ 开头）
        let module_part = path_str.strip_prefix("/mod/")?;
        
        // 将点号转换为路径分隔符
        // list.utils -> list/utils
        let relative_path: PathBuf = module_part
            .split('.')
            .collect();
        
        // 在搜索路径中查找（按优先级）
        for search_path in &self.ctx.search_paths {
            let full_path = search_path.join(&relative_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        // 没找到，返回第一个搜索路径（用于错误信息）
        self.ctx.search_paths.first()
            .map(|p| p.join(&relative_path))
    }
}

impl Middleware for MappedLayer {
    fn stage(&self) -> Stage {
        Stage::Mapping
    }
    
    fn read(&self, path: &Path, next: Next<'_>) -> VfsResult<Vec<u8>> {
        if let Some(real_path) = self.resolve(path) {
            next.call(&real_path)
        } else {
            // 非模块路径，原样传递
            next.call(path)
        }
    }
    
    fn exists(&self, path: &Path, next: Next<'_>) -> bool {
        if let Some(real_path) = self.resolve(path) {
            next.call_exists(&real_path)
        } else {
            next.call_exists(path)
        }
    }
}
```

---

## ModuleId（简化）

```rust
/// 模块标识（仅用于内部表示）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// 模块路径组件
    /// "math.utils" -> ["math", "utils"]
    pub components: Vec<String>,
}

impl ModuleId {
    /// 解析 import 路径
    /// "list"       -> ["list"]
    /// "math.utils" -> ["math", "utils"]
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        if s.is_empty() {
            return Err(ParseError::EmptyModulePath);
        }
        
        // 检查合法性（只能是字母数字下划线点号）
        if !s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Err(ParseError::InvalidModulePath(s.to_string()));
        }
        
        let components: Vec<String> = s.split('.').map(|s| s.to_string()).collect();
        
        if components.iter().any(|c| c.is_empty()) {
            return Err(ParseError::EmptyComponent(s.to_string()));
        }
        
        Ok(Self { components })
    }
    
    /// 转换为 VFS 逻辑路径
    /// ["math", "utils"] -> /mod/math.utils
    pub fn to_vfs_path(&self) -> PathBuf {
        let name = self.components.join(".");
        PathBuf::from(format!("/mod/{}", name))
    }
}
```

---

## 使用示例

### 1. 默认搜索路径

```rust
// 默认：["./src", "/opt/kaubo/std"]
let ctx = ModuleContext::default();

let vfs = VfsBuilder::new(NativeFileSystem::new())
    .with(MappedLayer::new(ctx))
    .build();

// 查找 list.kaubo
// 1. ./src/list.kaubo （优先）
// 2. /opt/kaubo/std/list.kaubo （备选）
```

### 2. 覆盖标准库

```rust
// 用户在自己的项目中创建 ./src/list.kaubo
// 自动覆盖系统的标准库
```

### 3. 添加自定义搜索路径

```rust
// 通过环境变量
export KABO_PATH="./vendor:./libs"

// 搜索顺序：
// 1. ./src/
// 2. ./vendor/
// 3. ./libs/
// 4. /opt/kaubo/std/
```

---

## 导入语法

```rust
// 所有 import 语法一致
import "list";           // 查找 list.kaubo
import "math.utils";    // 查找 math/utils.kaubo
import "http.client";   // 查找 http/client.kaubo

// 没有特殊前缀，都是点号分隔的模块名
```

---

## 与 V3 对比

| 特性 | V3（Namespace） | V4（搜索路径） |
|------|----------------|---------------|
| 模块分类 | Std / Local / Vendor | 无分类，只有搜索顺序 |
| import 语法 | `std.list` vs `list` | 都是 `list` |
| 覆盖标准库 | 需要特殊机制 | 直接在 `./src/` 放文件即可 |
| 第三方依赖 | Vendor namespace | 添加搜索路径即可 |
| 配置复杂度 | 需要配置 std_path, workspace_path | 只需要 search_paths 列表 |
| 灵活性 | 低（固定 2-3 种） | 高（任意路径） |

---

## 示例场景

### 场景 1：标准库被覆盖

```
项目结构：
my-project/
├── src/
│   ├── main.kaubo
│   └── list.kaubo      # 用户自定义 list 模块
└── package.json

main.kaubo:
import "list";  // 使用 ./src/list.kaubo，而非系统标准库
```

### 场景 2：使用第三方库

```bash
# 下载第三方库到 vendor/
mkdir -p vendor/http
cp http-client.kaubo vendor/http/client.kaubo

# 添加搜索路径
export KABO_PATH="./vendor"
```

```rust
// main.kaubo
import "http.client";  // 查找 vendor/http/client.kaubo
```

### 场景 3：标准库分发

```bash
# 将标准库复制到项目（vendoring）
cp -r /opt/kaubo/std ./vendor/std

# 项目使用本地版本，不依赖系统安装
```

---

## Phase 1 范围

| 特性 | 支持 | 说明 |
|------|------|------|
| 搜索路径 | ✅ | 列表形式，按优先级查找 |
| 默认路径 | ✅ | `./src` + 系统标准库 |
| 环境变量 | ✅ | `KABO_PATH` 添加额外路径 |
| 覆盖机制 | ✅ | 本地优先于系统 |
| 子模块 | ✅ | `math.utils` -> `math/utils.kaubo` |

---

## 确认

V4 设计：**无 Namespace，只有搜索路径**

- 所有模块统一对待
- 搜索路径决定查找顺序
- 本地 `./src/` 优先于系统标准库
- 通过 `KABO_PATH` 添加第三方路径

**是否确认这个设计？**
