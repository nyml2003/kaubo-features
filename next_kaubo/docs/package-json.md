# Kaubo 配置 (package.json)

## 目录结构

```
my-project/
├── package.json          # 项目配置
├── main.kaubo            # 入口文件
└── lib/
    ├── math.kaubo
    └── utils.kaubo
```

## package.json

```json
{
  "name": "my-project",
  "version": "0.1.0",
  "entry": "main.kaubo",
  "description": "项目描述",
  "compiler": {}
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 项目名称 |
| `version` | string | 否 | 版本号 |
| `entry` | string | 否 | 入口文件（默认 `main.kaubo`） |
| `description` | string | 否 | 项目描述 |
| `compiler` | object | 否 | 编译器配置 |

### compiler 配置

```json
{
  "compiler": {
    "emit_binary": true,
    "mode": "source"
  }
}
```

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `emit_binary` | bool | false | 是否输出 `.kaubod` 二进制文件 |
| `mode` | string | "auto" | 执行模式：`auto` / `source` / `binary` |

## CLI 使用

```bash
# 运行项目
kaubo

# 运行指定配置
kaubo path/to/package.json

# 编译但不运行
kaubo --compile-only

# 输出二进制文件
kaubo --emit-binary

# 查看字节码
kaubo --dump-bytecode

# 运行二进制文件
kaubo --mode binary path/to/output.kaubod
```

## 模块解析

模块路径相对于 `package.json` 所在目录解析：

```kaubo
// main.kaubo
import lib.math;
import lib.utils as util;

var result = math.add(1, 2);
util.print(result);
```

> `import lib.math` 对应文件 `./lib/math.kaubo`

---

*最后更新：2026-06-11*
