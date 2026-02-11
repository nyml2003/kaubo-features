# Kaubo 变更日志

> 记录项目所有重大变更

## [4.0] - 2026-02-12 (Workspace 架构)

### 修复

- **闭包 Upvalue 内存安全 Bug** - 修复 Y 组合子测试用例随机失败问题
  - 问题：`close()` 后 `get()` 仍使用栈指针读取已释放内存
  - 修复：`get()/set()` 优先使用 `closed` 字段
  - 影响：`factorial(5)` 等递归闭包现在可稳定执行

### 新增

- **Workspace 架构** - 4 个 crate 分层
  - `kaubo-config` - 纯配置数据结构
  - `kaubo-core` - 核心编译器（纯逻辑，无全局状态）
  - `kaubo-api` - 执行编排层（含全局单例）
  - `kaubo-cli` - CLI 平台实现

- **标准库扩展**
  - 协程函数: `create_coroutine`, `resume`, `coroutine_status`
  - 列表操作: `len`, `push`, `is_empty`, `range`, `clone`
  - 文件系统: `read_file`, `write_file`, `exists`, `is_file`, `is_dir`

- **AGENTS.md 开发指南** - 面向 AI 助手的开发文档

### 重构

- **配置分层** - 明确的配置分层架构
  - CLI 层: LogConfig
  - API 层: RunConfig + GLOBAL_CONFIG
  - Core 层: 参数传递，无全局状态
  - Config 层: 纯数据结构

- **文档更新** - 所有文档与现状同步

---

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

## 未来计划

### 高优先级

- [ ] 错误处理完善（调用堆栈追踪）
- [ ] 字节码版本号（魔数 + 版本头）

### 中优先级

- [ ] 浮点数字面量支持
- [ ] @ProgramStart 装饰器
- [ ] 垃圾回收实现

### 低优先级

- [ ] LSP 支持
- [ ] REPL 增强
- [ ] 更多标准库函数

---

## 历史 Bug 修复

### 闭包 Upvalue 内存安全 (4.0)

**问题**: Y 组合子测试用例运行时随机失败  
**根因**: `ObjUpvalue::close()` 后 `get()` 仍通过 `location` 指针访问栈内存  
**修复**: 优先使用 `closed` 字段，关闭后将 `location` 设为 null  
**验证**: `factorial(5)` 现在可稳定输出 120

---

*最后更新: 2026-02-12*
