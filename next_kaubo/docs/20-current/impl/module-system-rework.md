# Kaubo 模块系统改造设计

> 状态：设计文档 v1.0 | 目标：单文件单模块 + 插件化 Std + 虚拟文件系统

---

## 1. 设计目标（一句话总结）

1. **语法简化**：一个 `.kaubo` 文件 = 一个模块，删除 `module` 关键字
2. **Std 插件化**：标准库是独立 Rust 包，通过 trait 自动注册，零编译器依赖
3. **虚拟文件系统**：统一文件操作接口，支持 Windows/Mac/Linux/Web

---

## 2. 语法改造（单文件单模块）

### 2.1 删除 `module` 关键字

```kaubo
// ❌ 旧语法（删除）
module math {
    pub fn add(a: int, b: int) -> int {
        return a + b;
    }
}

// ✅ 新语法：文件即模块
// math.kaubo
pub fn add(a: int, b: int) -> int {
    return a + b;
}
```

### 2.2 导入语法保持不变

```kaubo
import math;                    // 导入 math.kaubo
from math import add;           // 选择性导入
import std.math as m;          // 别名
```

### 2.3 模块解析规则

| 导入路径 | 解析结果 | 示例 |
|----------|----------|------|
| `./utils` | 相对路径 | `./utils.kaubo` |
| `../helpers` | 相对路径 | `../helpers.kaubo` |
| `std.math` | std 内置 | `std/math.kaubo` |
| `external.lib` | 第三方库 | `modules/external/lib.kaubo` |

---

## 3. 插件化 Std 设计

### 3.1 核心原则

- **独立 crate**：`kaubo-std` 是独立包，不依赖 `kaubo-core`
- **自动包装**：编译器通过宏自动将 Rust 函数包装为 Kaubo 可调用
- **插件化**：任何 crate 实现 `StdModule` trait 即可注册

### 3.2 最小 API (kaubo-std-api)

```rust
// kaubo-std-api 包，零依赖

/// Kaubo 值的简化表示
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    String(String),
    List(ListHandle),  // 不透明句柄
}

/// 标准模块 trait
pub trait StdModule: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn exports(&self) -> &[FunctionExport];
    fn call(&self, name: &str, args: &[Value]) -> Result<Value>;
}

/// 函数导出描述
pub struct FunctionExport {
    pub name: String,
    pub arity: Arity,  // Exact(u8) 或 Variadic
    pub doc: String,
}
```

### 3.3 Std 实现示例 (kaubo-std)

```rust
// kaubo-std/src/math.rs

use kaubo_std_api::{std_module, export, Value, Result};

#[std_module(name = "math", version = "1.0.0")]
pub struct MathModule;

impl MathModule {
    #[export(name = "sqrt", arity = 1)]
    fn sqrt(x: f64) -> Result<f64> {
        if x < 0.0 {
            Err("sqrt domain error".to_string())
        } else {
            Ok(x.sqrt())
        }
    }
    
    #[export(name = "sin", arity = 1)]
    fn sin(x: f64) -> f64 {
        x.sin()
    }
    
    // 常量
    #[export(name = "PI", arity = 0)]
    fn pi() -> f64 {
        std::f64::consts::PI
    }
}
```

### 3.4 编译器集成

**只需新增一个文件**：`kaubo-core/src/native_registry.rs`

```rust
pub struct NativeRegistry {
    modules: HashMap<String, Box<dyn StdModule>>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        let mut registry = Self { modules: HashMap::new() };
        
        // 加载 kaubo-std 的所有模块
        for module in kaubo_std::modules() {
            registry.register(module);
        }
        
        registry
    }
    
    pub fn register(&mut self, module: Box<dyn StdModule>) {
        // 自动注册所有导出函数到 VM 的 globals
        for export in module.exports() {
            let full_name = format!("{}.{}", module.name(), export.name);
            // 注册到 VM
        }
        self.modules.insert(module.name().to_string(), module);
    }
}
```

**你的 `kaubo-core/src/` 结构保持不变**，只新增 `native_registry.rs`。

---

## 4. 虚拟文件系统（VFS）

### 4.1 目的

- 统一文件操作接口
- 支持 Web/WASM（浏览器无原生文件系统）
- 支持分层加载（std 内置 + 用户代码）

### 4.2 接口

```rust
pub trait VirtualFileSystem: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, VfsError>;
    fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), VfsError>;
    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
}
```

### 4.3 多平台实现

| 平台 | 实现 | 说明 |
|------|------|------|
| Windows/Mac/Linux | `NativeFileSystem` | 封装 `std::fs` |
| Web/WASM | `MemoryFileSystem` | IndexedDB / 内存 |
| 测试 | `MemoryFileSystem` | 纯内存，无 IO |

### 4.4 模块解析器使用 VFS

```rust
pub struct ModuleResolver {
    vfs: Arc<dyn VirtualFileSystem>,
    cache: HashMap<ModuleId, Arc<Module>>,
}

impl ModuleResolver {
    pub fn load(&self, id: &ModuleId) -> Result<Arc<Module>, LoadError> {
        // 检查缓存
        if let Some(cached) = self.cache.get(id) {
            return Ok(cached.clone());
        }
        
        // 通过 VFS 读取文件（支持所有平台）
        let content = self.vfs.read_file(&id.path())?;
        let source = String::from_utf8(content)?;
        
        // 解析
        let module = parse_module(&source)?;
        let module = Arc::new(module);
        self.cache.insert(id.clone(), module.clone());
        
        Ok(module)
    }
}
```

---

## 5. 实施计划

### Phase 1：语法改造（2周）

| 周次 | 任务 | 产出 |
|------|------|------|
| 1 | 删除 `module` 语句，文件即模块 | Parser 更新 |
| 1 | 更新模块解析逻辑 | 基于文件路径 |
| 2 | 多文件导入测试 | import 正常工作 |

**代码改动**：`parser.rs`, `compiler.rs`

### Phase 2：插件化 Std（3周）

| 周次 | 任务 | 产出 |
|------|------|------|
| 3 | kaubo-std-api crate | Value, StdModule trait |
| 3 | kaubo-std-macros crate | #[std_module], #[export] |
| 4 | kaubo-std crate | math, core 模块迁移 |
| 5 | 编译器 NativeRegistry | 新增 `native_registry.rs` |

**代码改动**：新增 3 个 crate，编译器新增 1 个文件

### Phase 3：虚拟文件系统（2周）

| 周次 | 任务 | 产出 |
|------|------|------|
| 6 | VFS trait + MemoryFileSystem | `vfs.rs` |
| 6 | NativeFileSystem | 封装 std::fs |
| 7 | ModuleResolver 集成 VFS | 模块加载走 VFS |

**代码改动**：新增 `vfs.rs`，修改模块加载逻辑

### Phase 4：整合测试（1周）

| 周次 | 任务 | 产出 |
|------|------|------|
| 8 | 端到端测试 | 多文件项目可运行 |
| 8 | 性能测试 | 无显著劣化 |

---

## 6. 项目结构（最终状态）

```
workspace/
├── kaubo-core/              # 你的编译器（最小改动）
│   ├── src/
│   │   ├── lib.rs
│   │   ├── compiler/
│   │   ├── core/
│   │   ├── kit/
│   │   ├── runtime/
│   │   └── native_registry.rs   # ← 新增（仅此文件）
│   └── Cargo.toml
│
├── kaubo-std/               # 新增：标准库（独立包）
│   ├── src/
│   │   ├── lib.rs
│   │   ├── core.rs          # print, assert, type
│   │   ├── math.rs          # sqrt, sin, cos
│   │   └── list.rs          # 列表方法
│   └── Cargo.toml
│
└── kaubo-std-api/           # 新增：最小接口
    ├── src/lib.rs
    └── Cargo.toml
```

---

## 7. 关键决策

| 决策 | 选择 | 理由 |
|------|------|------|
| **模块语法** | 删除 `module`，文件即模块 | 简化概念，与其他语言一致 |
| **Std 位置** | 独立 `kaubo-std` crate | 可独立开发发布，无需重新编译编译器 |
| **Std 依赖** | 零依赖，通过 trait 通信 | 编译器升级不破坏 std |
| **文件系统** | VFS 抽象 | 支持 Web，统一接口 |
| **第三方模块** | 同样 trait 接口 | 未来可动态加载 |

---

## 8. 注意事项

### 8.1 无 `fn` 声明

Std 提供的是 **Rust 原生函数**，不是 Kaubo 函数：

```rust
// ✅ 正确：Rust 函数，#[export] 包装
#[export(name = "sqrt")]
fn sqrt(x: f64) -> f64 { x.sqrt() }

// ❌ 错误：不支持 Kaubo 函数声明语法
fn sqrt(x: float) -> float { return x.sqrt(); }
```

### 8.2 与泛型的结合

Std 中的泛型方法接收单态化后的结果：

```kaubo
// 用户代码
nums.map(|[U] x| -> int { return x * 2; })

// 编译期展开后调用（单态化完成）
list_map$int$int(nums, __lambda_0)
```

Std 侧看到的是**具体类型**的函数调用。

---

*文档版本：1.0 | 最后更新：2026-02-16*
