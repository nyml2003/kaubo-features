# 当前语法（v2025.02）

> ⚠️ 此文件随时可能重写。这是当前实验状态，不是承诺。

## 设计方向

- 静态类型，但类型推导优先于显式标注
- 表达式导向，减少 ceremony
- 支持热重载的语义（状态保持友好）

## 基础语法（待定）

```kaubo
// 变量声明
var x = 1
var y: int = 2

// 函数（lambda语法）
add = |a: int, b: int| -> int { a + b }

// 简写形式（如果单行）
add = |a, b| a + b

// 结构体
struct Point {
    x: float,
    y: float
}

// 方法（通过impl块）
impl Point {
    distance = |self, other: Point| -> float {
        dx = self.x - other.x
        dy = self.y - other.y
        sqrt(dx * dx + dy * dy)
    }
}

// 热重载保留的函数标记
@hot
update = |dt: float| {
    // 此函数在热重载时保持状态
}
```

## 待定问题

| 问题 | 选项 | 倾向 |
|------|------|------|
| 分号是否可选 | A) 可选 B) 强制 | 倾向A，需验证歧义 |
| 类型标注位置 | A) 后置 `:int` B) 前置 `int x` | 倾向A，更流畅 |
| 错误处理 | A) Result类型 B) 异常 C) 断言+panic | 倾向C，简化 |
| 模块系统 | A) 文件即模块 B) 显式import | 倾向A |

## 实验记录

- 2025-02-14: 初始语法草稿，参考Rust+Kotlin

---

*此文件是工作区，不代表最终设计*
