# Kaubo 二进制模块系统设计

> 状态：设计文档 v1.0 | 支持 Debug/Release 双模式、Source Map、动态链接预留

---

## 1. 概述

### 1.1 目标

- **Debug 模式**：快速编译，保留完整调试信息，Source Map 支持
- **Release 模式**：优化体积和加载速度，可选剥离调试信息
- **动态链接预留**：ABI 稳定，支持未来运行时加载

### 1.2 文件扩展名

| 扩展名 | 含义 | 用途 |
|--------|------|------|
| `.kaubo` | 源码文件 | 开发时编辑 |
| `.kaubod` | Debug 编译产物 | 开发调试 |
| `.kaubor` | Release 编译产物 | 生产部署 |
| `.kpk` | Kaubo Package | 链接后的可执行包 |
| `.kmap` | Source Map | 源码映射（可选分离）|

---

## 2. 文件格式 (.kaubod / .kaubor)

### 2.1 整体结构

```
┌─────────────────────────────────────────────────────────────┐
│                      File Header (128 bytes)                 │
├─────────────────────────────────────────────────────────────┤
│                     Section Directory                        │
│  (变长，记录各 section 的 offset 和 size)                      │
├─────────────────────────────────────────────────────────────┤
│  String Pool Section  │  全局字符串池（去重）                  │
├─────────────────────────────────────────────────────────────┤
│  Module Table Section │  模块元数据表                         │
├─────────────────────────────────────────────────────────────┤
│  Chunk Data Section   │  字节码和常量池                       │
├─────────────────────────────────────────────────────────────┤
│  Shape Table Section  │  Struct shape 定义                    │
├─────────────────────────────────────────────────────────────┤
│  Export Table Section │  导出符号表                           │
├─────────────────────────────────────────────────────────────┤
│  Import Table Section │  导入依赖表                           │
├─────────────────────────────────────────────────────────────┤
│  Relocation Section   │  重定位信息（动态链接预留）             │
├─────────────────────────────────────────────────────────────┤
│  Debug Info Section   │  调试信息（Release 可选剥离）           │
├─────────────────────────────────────────────────────────────┤
│  Source Map Section   │  源码映射（可选分离到 .kmap）           │
├─────────────────────────────────────────────────────────────┤
│  Signature Section    │  签名和校验（未来扩展）                 │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Header 详细定义 (128 bytes)

```rust
#[repr(C, packed)]
pub struct FileHeader {
    // Magic & Version (8 bytes)
    pub magic: [u8; 4],           // "KAUB"
    pub version_major: u8,        // 文件格式主版本
    pub version_minor: u8,        // 文件格式次版本
    pub version_patch: u8,        // 文件格式修订版本
    pub _reserved1: u8,           // 对齐
    
    // Build Info (8 bytes)
    pub build_mode: BuildMode,    // Debug = 0x01, Release = 0x02
    pub target_arch: Arch,        // x86_64, aarch64, wasm32...
    pub target_os: OS,            // windows, macos, linux...
    pub _reserved2: u8,
    pub build_timestamp: u32,     // Unix timestamp
    
    // Feature Flags (8 bytes)
    pub flags: u32,               // 见 FeatureFlags
    pub _reserved3: u32,
    
    // Section Directory Info (16 bytes)
    pub section_count: u16,       // section 数量
    pub section_dir_offset: u32,  // section directory 偏移
    pub section_dir_size: u32,    // section directory 大小
    pub _reserved4: u16,
    
    // Entry Point (16 bytes)
    pub entry_module_idx: u16,    // 入口模块索引
    pub entry_chunk_idx: u16,     // 入口 chunk 索引
    pub _reserved5: u32,
    pub _reserved6: u64,
    
    // Source Info (32 bytes)
    pub source_hash: [u8; 16],    // 源码内容哈希 (用于增量编译)
    pub source_map_offset: u32,   // Source Map 偏移（0 = 无）
    pub source_map_size: u32,     // Source Map 大小
    pub source_map_external: u8,  // 1 = 外部 .kmap 文件
    pub _reserved7: [u8; 11],
    
    // Checksum (32 bytes)
    pub blake3_hash: [u8; 32],    // 整个文件的 Blake3 哈希（除本字段）
}

pub enum BuildMode { Debug = 0x01, Release = 0x02 }
pub enum Arch { x86_64 = 1, Aarch64 = 2, Wasm32 = 3 }
pub enum OS { Windows = 1, MacOS = 2, Linux = 3, Unknown = 0 }

bitflags! {
    pub struct FeatureFlags: u32 {
        const HAS_DEBUG_INFO = 0x0001;      // 包含调试信息
        const HAS_SOURCE_MAP = 0x0002;      // 包含 Source Map
        const SOURCE_MAP_EXTERNAL = 0x0004; // Source Map 在独立文件
        const HAS_RELOCATIONS = 0x0008;     // 包含重定位信息（动态链接）
        const IS_EXECUTABLE = 0x0010;       // 可执行（非库）
        const IS_STDLIB = 0x0020;           // 标准库模块
        const HAS_CHECKSUM = 0x0040;        // 包含签名/校验
        const STRIP_SOURCE = 0x0080;        // Release: 剥离源码路径
    }
}
```

### 2.3 Section Directory

```rust
pub struct SectionDirectory {
    pub entries: Vec<SectionEntry>,
}

pub struct SectionEntry {
    pub kind: SectionKind,    // 1 byte
    pub _padding: u8,         // 1 byte
    pub flags: u16,           // 2 bytes
    pub offset: u32,          // 4 bytes（从文件头开始）
    pub size: u32,            // 4 bytes
    pub compressed_size: u32, // 4 bytes（0 = 未压缩）
}

pub enum SectionKind {
    StringPool = 0x01,
    ModuleTable = 0x02,
    ChunkData = 0x03,
    ShapeTable = 0x04,
    ExportTable = 0x05,
    ImportTable = 0x06,
    Relocation = 0x07,
    DebugInfo = 0x08,
    SourceMap = 0x09,
    Signature = 0x0A,
}
```

---

## 3. Source Map 设计 (.kmap)

### 3.1 用途

- **错误堆栈还原**：将字节码位置映射回源码行列
- **调试器支持**：断点设置、单步执行
- **Profiling**：性能分析时显示源码级热点

### 3.2 格式（VLQ 编码，类似 JS Source Map v3）

```
KMap Header (32 bytes)
├── Magic: "KMAP" (4 bytes)
├── Version: 1 (4 bytes)
├── Source File Count: u32
├── Mapping Entry Count: u32
├── Mappings Data Offset: u32
└── Sources List Offset: u32

Sources List
├── Source File 1: path + hash
├── Source File 2: path + hash
└── ...

Mappings Data (VLQ 编码)
├── 每个 bytecode offset 对应一个映射
├── 格式: (generated_line, generated_column, 
│          source_index, original_line, original_column,
│          name_index)
└── 使用 VLQ 变长编码压缩
```

### 3.3 分离式 Source Map

```bash
# Debug 模式：内嵌 Source Map
kaubo build main.kaubo -o main.kaubod
# 生成单个文件 main.kaubod（包含 source map）

# Release 模式：分离 Source Map
kaubo build main.kaubo --release -o main.kaubor
# 生成 main.kaubor（无 source map）
# 可选生成 main.kmap（供调试器使用）
```

---

## 4. Debug vs Release 差异

### 4.1 Debug 模式 (`.kaubod`)

| 特性 | 说明 |
|------|------|
| 编译速度 | 优先快速编译，-O0 |
| 调试信息 | 完整：局部变量名、源码位置、行号表 |
| Source Map | 内嵌或同目录 .kmap |
| 断言 | 启用所有运行时断言 |
| 边界检查 | 完整数组/索引检查 |
| 体积 | 较大，可接受 |
| 回溯 | 完整堆栈，显示源码片段 |

### 4.2 Release 模式 (`.kaubor`)

| 特性 | 说明 |
|------|------|
| 编译速度 | 优化编译，-O2 |
| 调试信息 | 可选剥离，或精简版 |
| Source Map | 分离到 .kmap 文件 |
| 断言 | 禁用 debug_assert |
| 边界检查 | 信任模式（unsafe 区块）|
| 体积 | 最小化，压缩 bytecode |
| 回溯 | 简化，可能无源码位置 |

### 4.3 Chunk 差异示例

```rust
// Debug Chunk
pub struct DebugChunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub line_info: LineInfoTable,  // 每个指令对应源码行列
    pub local_names: Vec<String>,  // 局部变量名（用于调试）
    pub source_file: String,       // 源文件路径
}

// Release Chunk
pub struct ReleaseChunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub line_info: CompressedLineInfo, // 稀疏编码，仅关键位置
    // 无 local_names
    // source_file 可能为 "<stripped>"
}
```

---

## 5. 动态链接预留设计

### 5.1 ABI 稳定性

```rust
/// 动态链接兼容的 Chunk 结构
/// 所有指针使用相对偏移而非绝对地址
#[repr(C)]
pub struct DynamicChunk {
    pub abi_version: u32,           // ABI 版本号
    pub code_offset: u32,           // 代码偏移（相对本结构）
    pub code_size: u32,
    pub const_table_offset: u32,    // 常量表偏移
    pub const_count: u32,
    pub relocation_table_offset: u32, // 重定位表偏移
    pub relocation_count: u32,
    // ...
}

/// 重定位条目（用于动态链接时修正）
#[repr(C)]
pub struct RelocationEntry {
    pub offset: u32,                // 需要修正的位置（在代码中的偏移）
    pub kind: RelocationKind,       // 重定位类型
    pub symbol_index: u32,          // 符号表索引
    pub addend: i32,                // 修正值
}

pub enum RelocationKind {
    // 模块内部重定位
    LocalConstIndex = 1,           // 常量池索引
    LocalJumpOffset = 2,           // 跳转偏移
    
    // 跨模块重定位（动态链接）
    ImportModuleIndex = 0x10,      // 导入模块索引
    ImportFunctionIndex = 0x11,    // 导入函数索引
    ImportGlobalAddr = 0x12,       // 导入全局变量地址
}
```

### 5.2 动态加载器接口（预留）

```rust
/// 动态模块接口（未来实现）
pub trait DynamicModule: Send + Sync {
    /// ABI 版本
    fn abi_version(&self) -> u32;
    
    /// 模块名
    fn name(&self) -> &str;
    
    /// 解析所有导入
    fn resolve_imports(&mut self, resolver: &dyn ImportResolver) -> Result<(), LinkError>;
    
    /// 执行重定位
    fn relocate(&mut self) -> Result<(), LinkError>;
    
    /// 获取导出的 Chunk
    fn get_export(&self, name: &str) -> Option<&Chunk>;
    
    /// 获取导出函数的指针（JIT 预留）
    fn get_function_ptr(&self, name: &str) -> Option<*const u8>;
}
```

---

## 6. 构建流程

### 6.1 Debug 构建

```bash
# 编译单个模块
kaubo build math.kaubo --debug -o math.kaubod

# 编译项目（自动处理依赖）
kaubo build main.kaubo --debug --out-dir ./build/
# 输出:
#   ./build/main.kaubod
#   ./build/math.kaubod
#   ./build/utils.kaubod
#   ./build/main.kmap  (Source Map)

# 链接成可执行包（可选）
kaubo link ./build/*.kaubod -o app.kpk
```

### 6.2 Release 构建

```bash
# 编译项目
kaubo build main.kaubo --release --out-dir ./dist/
# 输出:
#   ./dist/main.kaubor
#   ./dist/main.kmap   (分离的 Source Map)

# 链接并优化（死代码消除、常量折叠）
kaubo link ./dist/*.kaubor --optimize -o app.kpk --strip-debug

# 验证文件
kaubo inspect app.kpk
# 输出:
#   Format: KPK (Kaubo Package)
#   Entry: main
#   Modules: 5
#   Size: 45KB
#   Debug: stripped
```

### 6.3 运行时加载

```bash
# 源码模式（开发）
kaubo run main.kaubo

# Debug 模式（快速加载）
kaubo run main.kaubod

# Release 模式（生产）
kaubo run app.kpk

# 加载时自动检查依赖和版本
```

---

## 7. 实现规划

### Phase 1.1: 基础格式 (2 周)

| 任务 | 说明 |
|------|------|
| File Header | 实现 Header 序列化/反序列化 |
| Section 系统 | Section Directory 管理 |
| String Pool | 全局字符串去重和编码 |
| Chunk 编码 | Chunk -> bytes 基础实现 |

### Phase 1.2: Debug 模式 (1 周)

| 任务 | 说明 |
|------|------|
| Line Info | 源码行列 -> bytecode offset 映射 |
| Local Names | 局部变量名存储 |
| CLI build | `kaubo build --debug` 命令 |

### Phase 1.3: Release 模式 (1 周)

| 任务 | 说明 |
|------|------|
| 压缩 | Bytecode 压缩（如 zstd）|
| 精简 | 剥离调试信息选项 |
| CLI release | `kaubo build --release` 命令 |

### Phase 1.4: Source Map (1 周)

| 任务 | 说明 |
|------|------|
| VLQ 编码 | Variable Length Quantity 实现 |
| 映射表 | 生成和解析映射数据 |
| 错误还原 | 堆栈跟踪映射回源码 |

### Phase 1.5: 链接器 (1 周)

| 任务 | 说明 |
|------|------|
| 符号表 | 跨模块符号解析 |
| KPK 格式 | 打包多个模块 |
| 静态链接 | 合并到单一文件 |

### Phase 1.6: 运行时加载器 (1 周)

| 任务 | 说明 |
|------|------|
| 格式检测 | 自动识别 .kaubo/.kaubod/.kaubor/.kpk |
| 缓存管理 | 编译产物缓存 |
| 版本检查 | 版本兼容性验证 |

---

## 8. 文件结构

```
kaubo-core/
├── compiler/
│   └── emitter/
│       ├── mod.rs
│       ├── encoder.rs          # Chunk -> binary
│       ├── decoder.rs          # binary -> Chunk
│       └── compress.rs         # 压缩/解压
├── linker/
│   ├── mod.rs
│   ├── linker.rs               # 静态链接
│   ├── symbol_table.rs         # 符号解析
│   └── kpk.rs                  # KPK 打包格式
├── runtime/
│   └── loader/
│       ├── mod.rs
│       ├── binary_loader.rs    # .kaubod/.kaubor 加载
│       ├── source_loader.rs    # .kaubo 编译加载
│       └── cache.rs            # 编译缓存管理
└── debug/
    ├── mod.rs
    ├── source_map.rs           # Source Map 生成/解析
    └── line_info.rs            # 行号表管理
```

---

## 9. 关键决策总结

| 决策 | 选择 | 理由 |
|------|------|------|
| **双格式** | .kaubod (Debug) + .kaubor (Release) | 开发vs生产不同需求 |
| **Source Map** | 支持内嵌/分离 | 灵活，Release 可分离减小体积 |
| **压缩** | zstd (Release) | 高压缩率，快速解压 |
| **Hash** | Blake3 | 快速，安全，适合增量编译 |
| **ABI** | 预留 32 位版本字段 | 保证未来动态链接兼容性 |
| **Relocation** | 相对偏移 | 支持 PIC，适合动态加载 |

---

*文档版本：1.0 | 最后更新：2026-02-17*
