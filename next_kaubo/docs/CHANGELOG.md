# Kaubo 变更日志

> 记录项目所有重大变更

## [3.0] - 2026-02-10 (架构重构)

### 新增

- **日志系统** - 基于 `tracing` 的分阶段日志
  - 支持 `trace/debug/info/warn/error` 五级
  - 可独立控制 lexer/parser/compiler/vm 日志级别
  - 支持 Pretty/JSON/Compact 输出格式
  - 函数级 span 追踪

- **配置系统** - 全局单例配置管理
  - `Config` 结构统一管理日志、限制、编译选项
  - `once_cell` 实现线程安全单例
  - CLI 参数与配置自动映射

- **API 层** - 高层封装供库用户使用
  - `compile()` - 仅编译
  - `execute()` - 仅执行
  - `compile_and_run()` - 完整流程
  - 各阶段独立调用支持

- **CLI 重构** - 基于 `clap` 的现代化 CLI
  - `-v/-vv/-vvv` 快速日志级别
  - `--log-*` 分阶段控制
  - `--format` 输出格式选择
  - `--compile-only` + `--dump-bytecode`

### 重构

- **main.rs** - 仅负责 CLI 解析和调度
- **lib.rs** - 清晰的库 API 导出
- **日志替换** - 全部 `eprintln!` 替换为结构化日志
- **测试框架** - 支持配置化日志级别

### 技术栈

| 组件 | 选型 | 说明 |
|------|------|------|
| 日志 | `tracing` | 结构化、异步、span 支持 |
| CLI | `clap` | 声明式、功能全 |
| 配置 | `once_cell` | 线程安全单例 |
| 错误 | `thiserror` | 类型安全 |

---

## [2.9] - 2026-02-10

### 新增

- **标准库支持** - 完整的 `std` 模块实现
  - `std.print()`, `std.assert()`, `std.type()`, `std.to_string()`
  - `std.sqrt()`, `std.sin()`, `std.cos()`, `std.floor()`, `std.ceil()`
  - `std.PI`, `std.E`

- **显式导入** - `import std;` 语法支持
- **测试框架** - 180+ 测试 (vm_tests, stdlib_tests)
- **`not` 操作符** - 逻辑取非
- **`!=` 操作符** - 不等于比较
- **变参函数** - `assert()` 支持 1-2 参数

### 修改

- 移除 `print` 关键字，改用 `std.print()`

---

## [2.7] - 2026-02-08

### 新增

- **模块静态化** - ShapeID 系统
- **模块访问语法** - `module.field` 支持
- `ModuleGet` 指令

### 重构

- `ObjModule` 使用 `Box<[Value]>`

---

## [2.0-2.6] - 早期开发

### 核心功能

- 词法分析器、语法分析器、字节码编译器、栈式虚拟机
- 闭包、协程基础
- 变量、函数、条件、循环、列表、JSON

---

## 版本规则

- `主版本.次版本`
- 主版本：架构级变更
- 次版本：功能迭代

*格式基于 [Keep a Changelog](https://keepachangelog.com/)*

---

*最后更新: 2026-02-10*
