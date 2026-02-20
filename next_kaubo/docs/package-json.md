# Kaubo 配置与代码分离设计

## 文件类型区分

| 后缀名 | 用途 | 约束 |
|--------|------|------|
| `.kaubo-config` | 编译配置 | **禁止 `runtime`**，**top-level 必须是编译期** |
| `.kaubo` | 业务代码 | 完整语法，top-level 可以是 runtime |

---

## 核心规则

### .kaubo-config 文件约束

1. **禁止 `runtime` 关键字**
2. **top-level 语句必须是编译期可求值**
3. **只允许编译期操作**：纯计算、配置读取、条件分支

**允许：**
- `val` 编译期常量
- `struct` 编译期结构体
- `import` / `export` 导入导出
- `if` 编译期条件
- 纯计算表达式

**禁止：**
- `runtime` 关键字
- 运行时操作（IO、网络、随机数等）

### .kaubo 文件

完整语法，无特殊限制。

---

## 编排架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          编译编排流程                                         │
│                                                                              │
│  Input ──▶ [Converter] ──▶ [Stage] ──▶ [Stage] ──▶ [Serializer] ──▶ Output │
│    │            │           │         │              │            │        │
│    │            │           │         │              │            │        │
│  Reader      Converter    Parser   Checker       Serializer     Writer     │
│  Adapter     Adapter      Stage    Stage        Adapter        Adapter     │
│                                                                              │
│  .kaubo ──▶  source  ──▶  ast  ──▶ typed_ast ──▶  json    ──▶ console    │
│    │            │           │         │              │            │        │
│    │            │           │         │              │            │        │
│  file-       kaubo-      parser   checker       ast-to-      console-      │
│  reader      source                           json          writer         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 配置结构

```json
{
  "name": "my-app",
  "version": "0.1.0",
  "compiler": {
    "pipeline": {
      "from": "source",
      "to": "run"
    },
    "adapters": {
      "readers": [
        { "name": "file" },
        { "name": "stdin" }
      ],
      "converters": [
        { "name": "kaubo-source", "inputs": ["file", "stdin"], "outputs": ["source"] },
        { "name": "ast-json", "inputs": ["file"], "outputs": ["ast"] },
        { "name": "bytecode-binary", "inputs": ["file"], "outputs": ["bytecode"] }
      ],
      "stages": [
        { "name": "parser", "inputs": ["source"], "outputs": ["ast"] },
        { "name": "checker", "inputs": ["ast"], "outputs": ["typed_ast"] },
        { "name": "generator", "inputs": ["typed_ast"], "outputs": ["bytecode"] },
        { "name": "runner", "inputs": ["bytecode"], "outputs": ["result"] }
      ],
      "serializers": [
        { "name": "ast-to-json", "inputs": ["ast"], "outputs": ["json"] },
        { "name": "bytecode-to-binary", "inputs": ["bytecode"], "outputs": ["binary"] },
        { "name": "bytecode-to-asm", "inputs": ["bytecode"], "outputs": ["text"] }
      ],
      "writers": [
        { "name": "console", "inputs": ["json", "binary", "text"] },
        { "name": "file", "inputs": ["json", "binary", "text"] }
      ]
    },
    "input": {
      "reader": "file",
      "path": "main.kaubo",
      "converter": "kaubo-source"
    },
    "emit": {
      "ast": {
        "serializer": "ast-to-json",
        "writer": "console"
      },
      "bytecode": {
        "serializer": "bytecode-to-binary",
        "writer": "file"
      }
    }
  }
}
```

---

## 编排组件详解

### `pipeline` - 执行范围

```json
{
  "pipeline": {
    "from": "source",
    "to": "run"
  }
}
```

| 阶段 | 说明 |
|------|------|
| `source` | 源代码输入 |
| `ast` | 抽象语法树 |
| `typed_ast` | 带类型的 AST |
| `bytecode` | 字节码 |
| `result` | 执行结果 |

### `adapters` - 适配器注册

#### `readers` - 输入读取

| 适配器 | 说明 | 选项 |
|--------|------|------|
| `file` | 从文件读取 | `path`: 文件路径 |
| `stdin` | 从标准输入读取 | `encoding`: 编码 |

#### `converters` - 格式转换

| 适配器 | 输入 | 输出 | 说明 |
|--------|------|------|------|
| `kaubo-source` | `file`, `stdin` | `source` | 读取源码 |
| `ast-json` | `file` | `ast` | JSON 转 AST |
| `bytecode-binary` | `file` | `bytecode` | 读取字节码 |

#### `stages` - 编译阶段

| 适配器 | 输入 | 输出 | 说明 |
|--------|------|------|------|
| `parser` | `source` | `ast` | 词法+语法分析 |
| `checker` | `ast` | `typed_ast` | 语义+类型检查 |
| `generator` | `typed_ast` | `bytecode` | 字节码生成 |
| `runner` | `bytecode` | `result` | VM 执行 |

#### `serializers` - 序列化

| 适配器 | 输入 | 输出 | 说明 |
|--------|------|------|------|
| `ast-to-json` | `ast` | `json` | AST 转 JSON |
| `bytecode-to-binary` | `bytecode` | `binary` | 字节码二进制 |
| `bytecode-to-asm` | `bytecode` | `text` | 反汇编 |

#### `writers` - 输出目标

| 适配器 | 输入 | 说明 | 选项 |
|--------|------|------|------|
| `console` | `json`, `binary`, `text` | 输出到控制台 | `stream`: stdout/stderr |
| `file` | `json`, `binary`, `text` | 输出到文件 | `path`: 路径模板 |

### `input` - 输入配置

```json
{
  "input": {
    "reader": "file",
    "path": "main.kaubo",
    "converter": "kaubo-source"
  }
}
```

### `emit` - 输出配置

```json
{
  "emit": {
    "<stage>": {
      "serializer": "<name>",
      "writer": "<name>",
      "options": {}
    }
  }
}
```

---

## .kaubo-config 配置代码

### features.kaubo-config

```kaubo-config
// 编译期特性配置
val DEBUG = cfg.DEBUG;
val PLATFORM = cfg.OS;

// 派生配置
val NETWORKING = cfg.ENABLE_NETWORKING or DEBUG;
val DATABASE = cfg.ENABLE_DATABASE;

// 条件配置
val LOG_LEVEL = if (DEBUG) { "verbose" } else { "info" };
val PATH_SEP = if (PLATFORM == "windows") { "\\" } else { "/" };

// 复杂配置
val SETTINGS = {
    buffer_size: 1024 * cfg.CPU_COUNT,
    max_connections: if (DEBUG) { 10 } else { 1000 }
};

export { DEBUG, NETWORKING, LOG_LEVEL, PATH_SEP, SETTINGS };
```

### platform.kaubo-config

```kaubo-config
val OS = cfg.OS;
val ARCH = cfg.ARCH;

val IS_WINDOWS = OS == "windows";
val IS_UNIX = OS == "linux" or OS == "macos";
val IS_64BIT = ARCH == "x86_64" or ARCH == "arm64";

export { OS, ARCH, IS_WINDOWS, IS_UNIX, IS_64BIT };
```

---

## 完整示例

### 场景 1：标准编译

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "compiler": {
    "pipeline": { "from": "source", "to": "run" },
    "adapters": {
      "readers": [{ "name": "file" }],
      "converters": [{ "name": "kaubo-source", "inputs": ["file"], "outputs": ["source"] }],
      "stages": [
        { "name": "parser", "inputs": ["source"], "outputs": ["ast"] },
        { "name": "checker", "inputs": ["ast"], "outputs": ["typed_ast"] },
        { "name": "generator", "inputs": ["typed_ast"], "outputs": ["bytecode"] },
        { "name": "runner", "inputs": ["bytecode"], "outputs": ["result"] }
      ],
      "serializers": [
        { "name": "bytecode-to-binary", "inputs": ["bytecode"], "outputs": ["binary"] }
      ],
      "writers": [{ "name": "file", "inputs": ["binary"] }]
    },
    "input": {
      "reader": "file",
      "path": "src/main.kaubo",
      "converter": "kaubo-source"
    },
    "emit": {
      "bytecode": {
        "serializer": "bytecode-to-binary",
        "writer": "file",
        "options": { "path": "dist/app.kaubod" }
      }
    }
  }
}
```

### 场景 2：调试 AST 输出

```json
{
  "name": "debug-ast",
  "compiler": {
    "pipeline": { "from": "source", "to": "parse" },
    "adapters": {
      "serializers": [
        { "name": "ast-to-json", "inputs": ["ast"], "outputs": ["json"] }
      ],
      "writers": [{ "name": "file", "inputs": ["json"] }]
    },
    "emit": {
      "ast": {
        "serializer": "ast-to-json",
        "writer": "file",
        "options": { "path": "debug/ast.json" }
      }
    }
  }
}
```

### 场景 3：从 AST 开始编译

```json
{
  "name": "from-ast",
  "compiler": {
    "pipeline": { "from": "ast", "to": "bytecode" },
    "adapters": {
      "readers": [{ "name": "file" }],
      "converters": [{ "name": "ast-json", "inputs": ["file"], "outputs": ["ast"] }],
      "stages": [
        { "name": "checker", "inputs": ["ast"], "outputs": ["typed_ast"] },
        { "name": "generator", "inputs": ["typed_ast"], "outputs": ["bytecode"] }
      ]
    },
    "input": {
      "reader": "file",
      "path": "debug/ast.json",
      "converter": "ast-json"
    }
  }
}
```

---

## 项目结构示例

```
my-project/
├── kaubo.json              # 编排配置（JSON格式）
├── config/
│   ├── features.kaubo-config    # 特性配置
│   └── platform.kaubo-config    # 平台配置
├── src/
│   └── main.kaubo               # 业务入口
└── dist/
    └── app.kaubod               # 输出
```

---

## 验证规则

```
验证流程
    │
    ├── 1. 验证 .kaubo-config 文件
    │      ├── 无 runtime 关键字？
    │      └── top-level 纯编译期？
    │
    ├── 2. 验证适配器链
    │      ├── converter.outputs 匹配 pipeline.from？
    │      ├── stages 链完整？
    │      └── emit 配置有效？
    │
    └── 3. 验证代码
           ├── .kaubo-config 编译期求值
           └── .kaubo 编译期+运行时分离
```

---

## 相关文档

- [语言语法参考](./20-language/spec/syntax.md)
- [开发指南](../DEVELOPMENT.md)
