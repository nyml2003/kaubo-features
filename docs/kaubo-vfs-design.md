# kaubo-vfs — 虚拟文件系统抽象

## 动机

Phase 3b 模块系统需要 `ModuleLoader` 读取源文件。CLI 用 `std::fs`，WASM 用内存 `HashMap`，测试用注入。如果现在不把文件 IO 抽成独立 crate，未来每个平台都要单独适配。

**目标**：一个极简的 VFS trait + 两个内置实现，~60 行。

## 接口

```rust
// crates/kaubo-vfs/src/lib.rs

/// 虚拟文件系统。
/// 读取源码文件，不涉及写操作、目录遍历、缓存。
trait VirtualFileSystem {
    /// 读取文件内容。
    /// `path` 是调用方提供的路径（可能包含相对路径）。
    /// 返回文件内容，或错误（文件不存在 / 权限等）。
    fn read(&self, path: &str) -> Result<String, VfsError>;
}

#[derive(Debug)]
enum VfsError {
    NotFound { path: String },
    IoError { path: String, reason: String },
}
```

## 内置实现

### `FsVfs` — 文件系统后端（CLI）

```rust
struct FsVfs {
    root: PathBuf,
}

impl FsVfs {
    fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }
}

impl VirtualFileSystem for FsVfs {
    fn read(&self, path: &str) -> Result<String, VfsError> {
        let full = self.root.join(path).canonicalize()
            .map_err(|e| VfsError::NotFound { path: path.to_string() })?;
        std::fs::read_to_string(&full)
            .map_err(|e| VfsError::IoError { path: path.to_string(), reason: e.to_string() })
    }
}
```

### `MemVfs` — 内存后端（WASM / 测试）

```rust
struct MemVfs {
    files: HashMap<String, String>,
}

impl MemVfs {
    fn new() -> Self {
        Self { files: HashMap::new() }
    }

    fn insert(&mut self, path: &str, source: &str) {
        self.files.insert(path.to_string(), source.to_string());
    }
}

impl VirtualFileSystem for MemVfs {
    fn read(&self, path: &str) -> Result<String, VfsError> {
        self.files.get(path)
            .cloned()
            .ok_or_else(|| VfsError::NotFound { path: path.to_string() })
    }
}
```

## 与 `ModuleLoader` 的关系

`ModuleLoader` 是 Phase 3b 模块系统定义的接口（路径解析 + 读取）。`kaubo-vfs` 是更底层的纯 IO 抽象（只读取，不解析路径）。

模块系统的 `FileLoader` 和 `MemLoader` 内部各持有一个 `Box<dyn VirtualFileSystem>`：

```rust
// Phase 3b ModuleLoader 实现
struct FileLoader {
    vfs: Box<dyn VirtualFileSystem>,
}

impl ModuleLoader for FileLoader {
    fn resolve(&self, from: &str, import_path: &str) -> Result<(String, String)> {
        let base = Path::new(from).parent().unwrap_or("".as_ref());
        let resolved = normalize_path(base.join(import_path));
        let source = self.vfs.read(&resolved)?;  // ★ 委托给 VFS
        Ok((resolved, source))
    }
}
```

## Cargo.toml

```toml
[package]
name = "kaubo-vfs"
version.workspace = true
edition.workspace = true

[dependencies]
# 零依赖（std 即可）
```

## 使用场景

| 场景 | VFS 实现 | 注入方式 |
|------|---------|---------|
| CLI | `FsVfs::new("./")` | 直接构造 |
| WASM Playground | `MemVfs` + 预填内置示例 | 启动时 `insert` |
| 测试 | `MemVfs` + `insert("main.kb", "const x = 42;")` | 测试 fixture |
| 未来：HTTP 远程模块 | `HttpVfs { base_url }` | 实现 trait |

## 不改什么

- ❌ 不写文件（只读）
- ❌ 不列目录
- ❌ 不缓存（缓存是 Coordinator 的职责）
- ❌ 不引入异步

## 实施状态

✅ **已实现**（Phase 3b 同期创建）。`kaubo-vfs` crate 已存在，`ModuleLoader` 的 `FileLoader`/`MemLoader` 内部各持有一个 `Box<dyn VirtualFileSystem>`。
