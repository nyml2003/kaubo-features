# 配置系统设计

> 运行时行为配置

## 设计原则

1. **显式执行目标**：CLI 必须明确指定入口文件，配置文件不控制执行目标
2. **项目级配置**：配置文件置于项目根目录，自动识别运行时行为
3. **预设默认值**：内置 profile 提供合理的运行时默认值
4. **运行时专注**：仅影响执行期行为，不控制编译目标
5. **简洁优先**：不追求完整，满足 90% 场景即可

## 配置文件

### 文件位置

```
project/
├── kaubo.json          # 运行时配置（自动识别）
├── src/
│   └── main.kaubo
└── tests/
```

### 配置结构

```json
{
    "version": "1.0",
    "profile": "dev",
    
    "compilerOptions": {
        "emitDebugInfo": true
    },
    
    "runtimeOptions": {
        "logging": {
            "level": "info",
            "targets": {
                "lexer": "warn",
                "parser": "warn",
                "compiler": "info",
                "vm": "error"
            }
        },
        "limits": {
            "maxStackSize": 10240,
            "maxRecursionDepth": 256
        },
        "lexer": {
            "bufferSize": 102400
        },
        "vm": {
            "initialStackSize": 256,
            "initialFramesCapacity": 64,
            "inlineCacheCapacity": 64
        },
        "coroutine": {
            "initialStackSize": 256,
            "initialFramesCapacity": 64
        }
    }
}
```

## 配置项详解

### version

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `version` | `string` | 否 | 配置格式版本，当前为 `"1.0"` |

### compilerOptions

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `emitDebugInfo` | `boolean` | `true` | 是否生成调试信息（行号映射等） |

### runtimeOptions.logging

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `level` | `"error" \| "warn" \| "info" \| "debug" \| "trace"` | `"warn"` | 全局日志级别 |
| `targets` | `object` | `{}` | 按组件覆盖日志级别 |
| `targets.lexer` | `string` | 继承 `level` | Lexer 组件日志级别 |
| `targets.parser` | `string` | 继承 `level` | Parser 组件日志级别 |
| `targets.compiler` | `string` | 继承 `level` | Compiler 组件日志级别 |
| `targets.vm` | `string` | 继承 `level` | VM 组件日志级别 |

### runtimeOptions.limits

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `maxStackSize` | `number` | `10240` | 最大栈大小（字节） |
| `maxRecursionDepth` | `number` | `256` | 最大递归深度 |

### runtimeOptions.lexer

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `bufferSize` | `number` | `102400` | 输入缓冲区大小（字节） |

### runtimeOptions.vm

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `initialStackSize` | `number` | `256` | 初始栈容量（槽位数） |
| `initialFramesCapacity` | `number` | `64` | 初始调用帧容量 |
| `inlineCacheCapacity` | `number` | `64` | 内联缓存容量 |

### runtimeOptions.coroutine

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `initialStackSize` | `number` | `256` | 初始栈容量（槽位数） |
| `initialFramesCapacity` | `number` | `64` | 初始调用帧容量 |

## 内置 Profile

Profile 仅提供**未指定字段**的默认值。

| Profile | 日志级别 | targets | limits | vm 初始栈 | 用途 |
|---------|---------|---------|--------|----------|------|
| `silent` | `error` | 全部 `error` | 标准 | 标准 | 生产运行 |
| `default` | `warn` | 全部 `warn` | 标准 | 标准 | 日常使用 |
| `dev` | `info` | 全部 `info` | 标准 | 标准 | 开发调试 |
| `debug` | `debug` | 全部 `debug` | 2倍 | 2倍 | 深度排查 |
| `trace` | `trace` | 全部 `trace` | 4倍 | 4倍 | 性能分析 |

**标准默认值**：
- `limits.maxStackSize`: 10240
- `limits.maxRecursionDepth`: 256
- `vm.initialStackSize`: 256
- `vm.initialFramesCapacity`: 64

## CLI 使用

### 显式文件要求

**CLI 必须指定入口文件**，配置文件仅控制运行时行为。

```bash
# ✅ 正确：显式指定文件
kaubo src/main.kaubo
kaubo src/test.kaubo

# ❌ 错误：未指定文件
kaubo
# 报错：缺少入口文件参数
```

### 配置加载规则

```bash
# 自动查找 kaubo.json 作为运行时配置
kaubo src/main.kaubo

# 显式指定运行时配置
kaubo --config ./configs/prod.json src/main.kaubo

# 使用预设 profile（不使用任何配置文件）
kaubo --profile=debug src/main.kaubo
```

### CLI 参数互斥

```bash
# ❌ 错误：--config 和 --profile 不能同时使用
kaubo --config kaubo.json --profile=debug src/main.kaubo

# ✅ 正确二选一
kaubo --config kaubo.json src/main.kaubo
kaubo --profile=debug src/main.kaubo
```

### CLI 编译参数

```bash
# 仅编译，不执行（编译时行为，与配置无关）
kaubo src/main.kaubo --compile-only

# 输出生成的字节码
kaubo src/main.kaubo --dump-bytecode

# 显示执行步骤
kaubo src/main.kaubo --show-steps

# 显示源代码
kaubo src/main.kaubo --show-source
```

## 配置继承与优先级

### 优先级

```
代码硬编码默认值
    ↓
--profile=xxx（选择默认值模板，仅当无配置文件时）
    ↓
kaubo.json（自动查找或 --config 指定）
    ↓
CLI 参数（最终覆盖）
```

### 合并规则

**对象类型**：深度合并（profile 默认值 → 配置文件覆盖）

```json
// profile: dev (默认 targets.vm = "info")
{
    "runtimeOptions": {
        "logging": {
            "targets": {
                "vm": "trace"  // 只覆盖 vm，其他用 profile 默认值
            }
        }
    }
}
```

**标量类型**：完全覆盖

```json
{
    "profile": "debug",                 // 完全覆盖
    "runtimeOptions": {
        "limits": {
            "maxStackSize": 20480       // 完全覆盖
        }
    }
}
```

## 配置示例

### 示例 1：纯 profile（无配置文件）

```bash
kaubo src/main.kaubo --profile=dev
```

### 示例 2：日常开发配置

```json
{
    "profile": "dev"
}
```

```bash
kaubo src/main.kaubo
```

### 示例 3：排查 Lexer 问题

```json
{
    "profile": "silent",
    "runtimeOptions": {
        "logging": {
            "targets": {
                "lexer": "trace"
            }
        }
    }
}
```

```bash
kaubo src/main.kaubo
```

### 示例 4：大栈性能测试

```json
{
    "profile": "trace",
    "runtimeOptions": {
        "limits": {
            "maxStackSize": 102400,
            "maxRecursionDepth": 1024
        },
        "vm": {
            "initialStackSize": 1024
        }
    }
}
```

```bash
kaubo benchmarks/fib.kaubo
```

### 示例 5：多环境配置

`kaubo.json`（开发环境）：
```json
{
    "profile": "dev",
    "runtimeOptions": {
        "limits": {
            "maxStackSize": 20480,
            "maxRecursionDepth": 512
        }
    }
}
```

`kaubo.prod.json`（生产环境）：
```json
{
    "profile": "silent",
    "runtimeOptions": {
        "logging": {
            "level": "error"
        }
    }
}
```

```bash
# 开发（自动加载 kaubo.json）
kaubo src/main.kaubo

# 生产（指定配置）
kaubo --config kaubo.prod.json src/main.kaubo
```

## 决策记录

| 日期 | 决策 | 理由 |
|------|------|------|
| 2026-02-15 | **不支持 extends** | 保持简单，需要继承时直接 copy-paste |
| 2026-02-15 | **不支持 $schema** | 当前不需要 IDE 支持，减轻认知负担 |
| 2026-02-15 | **CLI 必须显式指定文件** | 执行目标明确，避免隐式行为 |
| 2026-02-15 | **配置文件不控制入口文件** | 运行时行为与执行目标分离 |
| 2026-02-15 | `compilerOptions` 和 `runtimeOptions` 分离 | 编译时 vs 运行时行为清晰 |
| 2026-02-15 | profile 仅作为默认值来源 | 避免配置复杂度，保持单一真相 |
