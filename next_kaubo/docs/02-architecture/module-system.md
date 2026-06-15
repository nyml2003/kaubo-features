# 模块系统

## 概述

Kaubo 支持多文件编译。每个 `.kaubo` 文件是一个模块，通过 `import` 导入。

## 导入语法

### 简单导入

```kaubo
import "math";
```

导入模块的全部导出到模块命名空间：

```kaubo
import "math";
print math.add(1, 2);
print math.PI;
```

### 带别名

```kaubo
import "very_long_module_name" as short;
print short.some_function();
```

### 命名导入

```kaubo
from "math" import { add, PI };
print add(1, 2);
print PI;
```

## 导出

使用 `pub` 关键字导出顶级声明：

```kaubo
// math.kaubo
pub var PI = 3.14159;

pub var add = |a, b| -> int {
    return a + b;
};

// 私有 — 外部不可见
var secret = 42;
```

## 模块解析

编译器按以下顺序查找模块：

1. 当前文件所在目录
2. 当前工作目录
3. 通过 `CompileContext` 注册的目录

模块路径：
- `"math"` → 查找 `math.kaubo`
- `"lib/math"` → 查找 `lib/math.kaubo`
- 无 `.kaubo` 扩展名时自动补全

## 多文件编译

```bash
kaubo compile main.kaubo
# 自动追踪 import 链，编译所有依赖
```

依赖图支持：

| 结构 | 说明 |
|------|------|
| 链式导入 | A → B → C |
| 菱形依赖 | A → B, A → C, B → D, C → D |
| 嵌套目录 | `import "dir/module"` |

## 二进制格式

编译后的模块可以输出为二进制：

```
.kaubod  — 调试版（无压缩）
.kaubor  — 发布版（可选压缩）
```

二进制格式包含：

```
Header: "KAUB" + version + checksum
Sections:
  ├── StringPool       — 字符串常量池
  ├── ChunkData        — 字节码
  ├── ModuleTable      — 模块名 → Chunk 映射
  ├── ShapeTable       — 结构体定义
  ├── ExportTable      — 公开导出
  └── ImportTable      — 依赖关系
```

## 多文件模块设计

详细设计参见历史文档：
- `archive/old/30-implementation/design/module-system.md`
- `archive/old/30-implementation/design/multi-module-system-final.md`
- `archive/old/30-implementation/design/binary-module-system.md`
