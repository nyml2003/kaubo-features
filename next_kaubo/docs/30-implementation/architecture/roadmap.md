# Kaubo 迭代路线图

> 项目分阶段推进，当前处于 **Phase 2**

---

## 概览

| 阶段 | 名称 | 状态 | 核心目标 |
|------|------|------|----------|
| Phase 0 | 基础设施 | ✅ 完成 | Lexer、Parser、基本VM |
| Phase 1 | 模块系统 | ✅ 完成 | VFS、多文件编译、模块导入 |
| Phase 2 | 泛型类型系统 | 🚧 进行中 | 泛型函数、泛型结构体、类型推导 |
| Phase 3 | JIT编译器 | 📋 规划中 | Cranelift集成、热点编译 |
| Phase 4 | 热重载 | 📋 规划中 | 状态保持、代码热更新 |

---

## Phase 0 - 基础设施（✅ 完成）

**时间**：2025-02 至 2025-Q2

### 已完成

- [x] 流式 Lexer（支持增量输入）
- [x] 递归下降 Parser
- [x] AST 定义
- [x] 字节码设计
- [x] 基本 VM（栈机实现）

### 关键决策

- 使用 NaN-boxing 的 Value 表示
- 栈与局部变量分离设计
- Upvalue 机制（类似 Lua）

---

## Phase 1 - 模块系统与二进制格式（🚧 进行中）

**时间**：2026-02-17 开始，预计 6 周

### Phase 1.1: 源文件模块系统 ✅ (完成)

**1. 虚拟文件系统 (VFS)**
- [x] 新增 `kaubo-vfs` crate
- [x] `VirtualFileSystem` trait 抽象
- [x] `MemoryFileSystem` - 内存文件系统（测试用）
- [x] `NativeFileSystem` - 原生文件系统（生产用）
- [x] 跨平台路径处理（Windows/Unix）

**2. 模块系统改造**
- [x] 删除 `module` 关键字（废弃并返回错误）
- [x] 单文件即单模块设计
- [x] `pub var` 导出机制
- [x] `import path.to.module;` 导入语法

**3. 模块解析器 (ModuleResolver)**
- [x] 路径解析: `std.list` → `std/list.kaubo`
- [x] 模块缓存（避免重复加载）
- [x] 循环依赖检测

**4. 多文件编译器 (MultiFileCompiler)**
- [x] 拓扑排序编译（依赖优先）
- [x] 传递依赖解析（A→B→C 链）
- [x] 菱形依赖处理（共享模块只加载一次）

**测试覆盖：** 486 tests ✅

---

### Phase 1.2: 二进制格式（当前）

**目标**：实现 Debug/Release 双模式二进制编译产物

**文件格式：**

| 扩展名 | 模式 | 说明 |
|--------|------|------|
| `.kaubod` | Debug | 完整调试信息，内嵌 Source Map |
| `.kaubor` | Release | zstd 压缩，可选剥离调试信息 |
| `.kmap` | 可选 | Source Map 文件（VLQ 编码）|
| `.kpk` | 可执行 | 链接后的包格式 |

**核心设计：**

```
Kaubo Binary Module
├── Header (128 bytes) - Magic/Version/ABI/Flags
├── Section Directory - 各 section 偏移和大小
├── String Pool - 全局字符串去重
├── Module Table - 模块元数据
├── Chunk Data - 字节码和常量池
├── Shape Table - Struct shape 定义
├── Export/Import Tables - 符号表
├── Relocation Table - 重定位信息（动态链接预留）
├── Debug Info - 调试信息（Release 可选剥离）
├── Source Map - 源码映射（可选分离到 .kmap）
└── Signature - Blake3 哈希校验
```

**Debug vs Release：**

| 特性 | Debug (.kaubod) | Release (.kaubor) |
|------|-----------------|-------------------|
| 编译速度 | 优先快速编译 | 优化编译 |
| 调试信息 | 完整行号表、局部变量名 | 精简或剥离 |
| Source Map | 内嵌或同目录 .kmap | 分离或省略 |
| 压缩 | 无 | zstd |
| 断言 | 启用 | 禁用 |
| 体积 | 较大 | 最小化 |

**任务列表：**

- [ ] Header 设计与序列化
- [ ] Section 管理系统
- [ ] Chunk Encoder/Decoder
- [ ] Source Map (VLQ 编码)
- [ ] zstd 压缩集成
- [ ] CLI `kaubo build --debug/--release`

---

### Phase 1.3: 链接器（待开始）

**目标**：将多个模块链接成可执行包

**功能：**
- [ ] 符号表构建与解析
- [ ] 跨模块引用重定位
- [ ] KPK 打包格式
- [ ] CLI `kaubo link`

---

### Phase 1.4: 运行时加载器（待开始）

**目标**：支持加载二进制格式

**功能：**
- [ ] 格式自动识别（.kaubo/.kaubod/.kaubor/.kpk）
- [ ] 版本兼容性检查
- [ ] Blake3 校验
- [ ] 编译产物缓存

---

### Phase 1.5: 动态链接预留（待开始）

**目标**：为未来动态库加载预留能力

**设计：**
- [ ] ABI 版本字段（32 位）
- [ ] 重定位表（相对偏移）
- [ ] `DynamicModule` trait 接口

---

### 进入 Phase 2 的条件

- [ ] Debug/Release 双模式编译
- [ ] Source Map 支持
- [ ] KPK 可执行包格式
- [ ] 运行时二进制加载
- [ ] 版本兼容性检查

---

## Phase 2 - 泛型类型系统（📋 规划中）

**时间**：Phase 1 完成后开始

### 目标

实现完整的编译时泛型系统，支持泛型函数、泛型结构体、类型推导。

### 核心功能

| 功能 | 示例 | 状态 |
|------|------|------|
| 泛型匿名函数 | `\|[T] x: T\| -> T { return x; }` | 📋 待实现 |
| 泛型 struct | `struct Box[T] { value: T }` | 📋 待实现 |
| 泛型 impl | `impl[T] Box[T] { ... }` | 📋 待实现 |
| 类型推导 | `identity(42)` → `\|int\| -> int` | 📋 待实现 |
| 多类型参数 | `\|[T, U] x: T, y: U\|` | 📋 待实现 |
| 嵌套泛型 | `Box[List[T]]` | 📋 待实现 |

### 语法规范

统一使用 `[]` 表示泛型参数，避免 `<>` 与小于运算符冲突：

```kaubo
// 类型定义
struct Box[T] { value: T }
impl[T] Box[T] { 
    pub var get = || { return self.value; };
}

// 表达式
|[T] x: T| -> T { return x; }

// 类型标注
var b: Box[int] = Box[int] { value: 42 };
var list: List[List[string]] = [];
```

### 技术方案

**1. 类型系统扩展**

```rust
// TypeExpr 扩展
pub enum TypeExpr {
    Named(NamedType),                    // int, string, bool
    TypeParam(TypeParam),                // NEW: 类型参数 T
    List(Box<TypeExpr>),                 // List[T]
    Tuple(Vec<TypeExpr>),                // Tuple[T, U]
    Function(FunctionType),              // |T| -> U
    GenericInstance(GenericInstance),    // NEW: Box[int]
}
```

**2. 单态化 (Monomorphization)**

编译期展开泛型为具体类型：

```kaubo
// 源码
var identity = |[T] x: T| -> T { return x; };
var a = identity(42);        // T = int
var b = identity("hello");   // T = string

// 编译后（概念上）
var identity$int = |x: int| -> int { return x; };
var identity$str = |x: string| -> string { return x; };
var a = identity$int(42);
var b = identity$str("hello");
```

**3. 类型推导算法**

基于 Hindley-Milner 的简化版本：
- 从函数参数类型推导
- 从返回值上下文推导
- 约束求解与统一

### 实施步骤

| 步骤 | 任务 | 产出 |
|------|------|------|
| 1 | 类型参数语法解析 | Parser 支持 `[T]` 语法 |
| 2 | TypeExpr 扩展 | 支持 TypeParam、GenericInstance |
| 3 | 泛型 Lambda 编译 | 单态化实现 |
| 4 | 泛型 Struct 定义 | struct 语句支持类型参数 |
| 5 | 泛型 Impl | impl 语句支持类型参数 |
| 6 | 类型推导 | 自动推导泛型参数 |
| 7 | 测试覆盖 | 泛型相关测试 > 50 个 |

### 设计文档

- `docs/30-implementation/design/generic-type-system.md`

### 进入 Phase 3 的条件

- [ ] 泛型 Lambda 完整实现
- [ ] 泛型 Struct 完整实现
- [ ] 泛型 Impl 完整实现
- [ ] 基础类型推导
- [ ] 50+ 泛型相关测试
- [ ] 示例项目使用泛型

---

## Phase 3 - JIT编译器（📋 规划中）

**时间**：待定（预计 Phase 2 完成后 1-2 个月）

### 目标

实现基于 Cranelift 的 JIT 编译器，将热点函数编译为机器码。

### 关键任务

- [ ] Cranelift 依赖集成
- [ ] 热点检测机制
- [ ] AST → Cranelift IR 转换
- [ ] 解释器 ↔ JIT 切换
- [ ] 降级机制（JIT失败时回退到解释器）

### 性能目标

| 指标 | 目标值 |
|------|--------|
| JIT编译单函数 | < 100ms |
| JIT代码执行 | 比解释器快 5-10x |

### 依赖

- Phase 2 完成
- Cranelift 0.1xx+ 版本评估

---

## Phase 4 - 热重载（📋 规划中）

**时间**：待定（预计 Phase 3 完成后 2-3 个月）

### 目标

实现开发时的代码热更新，不丢失运行状态。

### 关键任务

- [ ] 状态序列化/反序列化
- [ ] 函数级代码替换
- [ ] `@hot` 注解设计与实现
- [ ] 安全点机制
- [ ] 版本检查与迁移

### 性能目标

| 指标 | 目标值 |
|------|--------|
| 热重载延迟 | < 1秒 |
| 状态恢复成功率 | > 99% |

### 依赖

- Phase 3 JIT 完成
- 确定性 Shape ID（已预留）
- 统一栈帧 ABI（已预留）

---

## 明确不做（复杂度预算外）

参见 [复杂度预算](../10-constraints/complexity-budget.md)

- 完整 GC（使用 Arena 足够）
- 多线程并行
- 跨平台支持（初期）
- 包管理器
- IDE/LSP 协议

---

## 决策记录

| 日期 | 决策 | 理由 |
|------|------|------|
| 2025-02 | 分阶段推进 | 控制复杂度，逐步验证 |
| 2026-02 | 完成 Phase 1 模块系统 | VFS + 多文件编译 + 模块导入 |
| 2026-02 | 开始 Phase 2 泛型 | 核心基础设施就绪，可支持泛型实现 |
| 2026-02 | JIT推迟到Phase 3 | 优先实现泛型，为JIT生成代码做准备 |
| 2026-02 | 热重载推迟到Phase 4 | 依赖JIT，且需要更多架构预留 |

---

## 参考文档

- [模块系统设计](../design/module-system.md)
- [泛型类型系统设计](../design/generic-type-system.md)
- [复杂度预算](../10-constraints/complexity-budget.md)

---

*最后更新：2026-02-17*
