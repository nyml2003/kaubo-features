# Kaubo 开发 TODO

## 当前阶段: Phase 2.6 模块系统（基础已完成）

### ✅ 已完成

- [x] 模块定义语法 (`module name { ... }`)
- [x] `pub` 导出关键字
- [x] 单文件模块编译
- [x] 导出表管理 (ModuleInfo, Export)
- [x] 更新所有文档

---

## 🔥 高优先级（本周）

### 1. break/continue 支持
**难度**: ⭐（简单）  
**价值**: ⭐⭐⭐（非常有用）

```kaubo
// 目标语法
while (true) {
    if (condition) {
        break;
    }
    if (other) {
        continue;
    }
}
```

**实现思路**:
- 添加 `JumpBack` 指令
- 维护循环栈（break/continue 跳转位置）
- 类似函数跳转的处理

### 2. 浮点数解析修复
**难度**: ⭐（简单）  
**价值**: ⭐⭐（重要）

当前问题: `3.14` 可能解析不正确

### 3. 模块访问指令
**难度**: ⭐⭐（中等）  
**价值**: ⭐⭐⭐（核心功能）

```kaubo
// 目标语法
import math;
print math.PI;  // 需要 GetExport 指令
```

**实现思路**:
- 添加 `GetModule` 指令
- 添加 `GetExport` 指令
- 或者修改 `GetGlobal` 支持模块路径

---

## ⭐ 中优先级（本月）

### 4. 标准库注册表
**难度**: ⭐⭐（中等）  
**价值**: ⭐⭐⭐（重要）

```kaubo
// 目标用法
import std.core;
std.core.print("Hello");

import std.math;
var x = std.math.sqrt(16);
```

**实现思路**:
- 创建 `std/` 目录
- 内置模块注册表（HashMap）
- `std.core` 包含基础函数（print, assert, type）

### 5. Result/Option 构造函数
**难度**: ⭐⭐（中等）  
**价值**: ⭐⭐（有用）

```kaubo
// 目标语法
var ok = Ok(42);
var err = Err("failed");
var some = Some(100);
var none = None();

// 方法
if (ok.is_ok()) {
    print ok.unwrap();
}
```

**依赖**: 需要先完成 #3（模块访问）

### 6. 边界测试（50+）
**难度**: ⭐⭐（中等）  
**价值**: ⭐⭐⭐（质量保证）

详见 [TEST_PLAN.md](docs/TEST_PLAN.md)

---

## 🌙 低优先级（后续）

### 7. typeof 运算符
**难度**: ⭐（简单）  
**价值**: ⭐（有用）

```kaubo
var x = 5;
print typeof(x);  // "int"
```

### 8. 字符串插值
**难度**: ⭐⭐（中等）  
**价值**: ⭐⭐（方便）

```kaubo
var name = "World";
print "Hello, {name}!";  // Hello, World!
```

### 9. 多文件模块系统
**难度**: ⭐⭐⭐（复杂）  
**价值**: ⭐⭐⭐（重要）

```kaubo
// 文件结构
// src/math.kaubo
module math {
    pub var PI = 3.14;
}

// main.kaubo
import "src/math";  // 从文件导入
```

### 10. match 表达式
**难度**: ⭐⭐⭐（复杂）  
**价值**: ⭐⭐⭐（核心特性）

```kaubo
var result = Ok(42);
match result {
    Ok(value) => print value,
    Err(msg) => print msg,
}
```

### 11. 错误传播 `?`
**难度**: ⭐⭐（中等）  
**价值**: ⭐⭐（有用）

```kaubo
var content = read_file("file.txt")?;
// 如果 read_file 返回 Err，函数直接返回该 Err
```

---

## 技术债务

### 需要清理的警告
- [ ] 未使用的导入
- [ ] 未使用的变量
- [ ] 未使用的函数

### 代码重构
- [ ] 拆分过大的 parser.rs
- [ ] 拆分过大的 compiler.rs
- [ ] 统一错误处理

### 性能优化
- [ ] 字符串使用 Rc/Arc 减少复制
- [ ] HashMap 查询缓存
- [ ] 指令分发优化

---

## 近期目标

### 本周目标（2026-02-10 至 2026-02-16）

1. **功能实现**
   - [ ] 模块静态化：ShapeID 系统 Phase 1
     - [ ] ObjModule 改为固定长度布局
     - [ ] 新增 ModuleGet 指令
     - [ ] 编译器生成 ShapeID 和 ModuleGet
     - [ ] VM 实现 ModuleGet
   - [ ] 修复浮点数解析（简单）
   - [ ] 实现 `break` 支持（可选，延后）

2. **测试增强**
   - [ ] 添加 20+ 边界测试
   - [ ] 添加 JSON 嵌套测试

3. **文档**
   - [ ] 更新语言规范
   - [ ] 添加更多示例

### 月度目标（2 月）Phase 2.x 完成

- [ ] 完成模块静态化（ShapeID 系统）
- [ ] 浮点数解析修复
- [ ] break/continue 支持
- [ ] 测试覆盖率达到 85%
- [ ] 标准库基础（std.core, std.math）
- [ ] 发布 v0.2.0 预览版

### 季度目标（Q1）Phase 3.0 启动

- [ ] 结构体语法（纯数据布局）
- [ ] Interface/Trait 系统
- [ ] impl 实现语法
- [ ] 类型标注（可选）
- [ ] 发布 v0.3.0 alpha

---

## 优先级矩阵

| 功能 | 阶段 | 难度 | 价值 | 优先级 |
|------|------|------|------|--------|
| 模块静态化 | 2.7 | ⭐⭐⭐ | ⭐⭐⭐ | 🔥 P0 |
| break/continue | 2.8 | ⭐ | ⭐⭐⭐ | ⭐ P1 |
| 浮点数解析修复 | 2.7 | ⭐ | ⭐⭐ | ⭐ P1 |
| 边界测试 | 2.8 | ⭐⭐ | ⭐⭐⭐ | ⭐ P1 |
| 标准库 | 2.9 | ⭐⭐ | ⭐⭐⭐ | ⭐ P1 |
| **结构体 (struct)** | 3.0 | ⭐⭐⭐ | ⭐⭐⭐ | ⭐ P1 |
| **Interface 系统** | 3.1 | ⭐⭐⭐ | ⭐⭐⭐ | ⭐ P1 |
| **impl 实现语法** | 3.2 | ⭐⭐⭐⭐ | ⭐⭐⭐ | 🌙 P2 |
| Result 方法 | 3.2 | ⭐⭐ | ⭐⭐ | 🌙 P2 |
| typeof | 3.2 | ⭐ | ⭐ | 🌙 P2 |
| 字符串插值 | 3.3 | ⭐⭐ | ⭐⭐ | 🌙 P2 |
| match | 3.4 | ⭐⭐⭐ | ⭐⭐⭐ | 🌙 P3 |
| 泛型 | 4.0 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | 🌙 P3 |
| 错误传播 `?` | 4.0 | ⭐⭐⭐ | ⭐⭐ | 🌙 P3 |
| 多文件模块 | 5.0 | ⭐⭐⭐ | ⭐⭐⭐ | 🌙 P3 |

---

*最后更新: 2026-02-10*  
*维护者: Kaubo 开发团队*
