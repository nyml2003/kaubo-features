# Kaubo 测试文档

> 测试框架、分层测试策略和调试技巧

## 目录

1. [测试架构](#1-测试架构)
2. [运行测试](#2-运行测试)
3. [分层测试](#3-分层测试)
4. [日志与调试](#4-日志与调试)
5. [编写测试](#5-编写测试)

---

## 1. 测试架构

### 1.1 测试分层

```
tests/
├── api_tests.rs           # 【新增】API 层集成测试
├── lexer_tests.rs         # 【新增】Lexer 单元测试
├── parser_tests.rs        # 【新增】Parser 单元测试
├── compiler_tests.rs      # 【新增】Compiler 单元测试
├── vm_tests.rs            # VM 执行测试
├── stdlib_tests.rs        # 标准库测试
├── integration/           # 场景测试
│   ├── arithmetic/        # 算术运算
│   ├── control_flow/      # 控制流
│   ├── functions/         # 函数
│   └── stdlib/            # 标准库使用
└── common/
    └── mod.rs             # 共享工具
```

### 1.2 测试策略

| 层级 | 范围 | 速度 | 稳定性 | 数量 |
|------|------|------|--------|------|
| **单元测试** | 单个函数/模块 | 快 | 高 | 多 (>200) |
| **API 测试** | 各阶段独立调用 | 快 | 高 | 中 (50+) |
| **集成测试** | 完整执行链 | 中 | 中 | 中 (30+) |
| **端到端** | CLI + 文件 | 慢 | 中 | 少 (10+) |

---

## 2. 运行测试

### 2.1 基础命令

```bash
# 运行所有测试
cargo test

# 运行特定测试文件
cargo test --test api_tests
cargo test --test vm_tests
cargo test --test stdlib_tests

# 运行特定测试
cargo test test_addition
cargo test test_std_sqrt
```

### 2.2 调试测试

```bash
# 显示测试输出
cargo test -- --nocapture
cargo test test_name -- --nocapture

# 单线程运行（便于调试）
cargo test -- --test-threads=1

# 详细日志
cargo test -- --nocapture 2>&1 | less
```

### 2.3 环境变量

```bash
# 启用详细日志
RUST_LOG=kaubo::vm=trace cargo test

# 特定测试启用调试
RUST_LOG=kaubo::parser=debug cargo test test_parse

# 保存日志到文件
cargo test 2> test.log
```

---

## 3. 分层测试

### 3.1 单元测试（内联）

```rust
// src/runtime/value.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smi_creation() {
        let v = Value::smi(42);
        assert!(v.is_smi());
        assert_eq!(v.as_smi(), Some(42));
    }
}
```

### 3.2 API 测试（独立阶段）

```rust
// tests/lexer_tests.rs

mod common;
use kaubo::api::lex;

#[test]
fn test_lex_simple() {
    let tokens = lex("var x = 5;").unwrap();
    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[0].kind, KauboTokenKind::Var);
}
```

### 3.3 集成测试（完整执行）

```rust
// tests/vm_tests.rs

mod common;
use common::{get_int, run_code};

#[test]
fn test_arithmetic() {
    let result = run_code("return 1 + 2;").unwrap();
    assert_eq!(get_int(&result), Some(3));
}
```

### 3.4 共享工具（common/mod.rs）

```rust
//! 测试共享工具

use kaubo::{compile_and_run, init, Config, LogConfig, LogLevel};

/// 标准测试配置（低日志级别）
pub fn run_code(source: &str) -> Result<ExecResult, KauboError> {
    let config = Config {
        log: LogConfig {
            global: LogLevel::WARN,  // 测试时安静
            ..Default::default()
        },
        ..Default::default()
    };
    
    init(config);
    compile_and_run(source)
}

/// 调试配置（详细日志）
pub fn run_code_debug(source: &str) -> Result<ExecResult, KauboError> {
    let config = Config {
        log: LogConfig {
            global: LogLevel::DEBUG,
            lexer: Some(LogLevel::TRACE),
            parser: Some(LogLevel::DEBUG),
            vm: Some(LogLevel::DEBUG),
        },
        ..Default::default()
    };
    
    init(config);
    compile_and_run(source)
}

/// 断言辅助函数
pub fn assert_int(result: &ExecResult, expected: i32) {
    let actual = result.return_value
        .as_ref()
        .and_then(|v| v.as_int())
        .expect("Expected integer");
    assert_eq!(actual, expected);
}
```

---

## 4. 日志与调试

### 4.1 测试中启用日志

```rust
#[test]
fn test_with_logging() {
    // 使用调试配置查看日志
    let result = run_code_debug(r#"
        import std;
        var x = 5;
        std.print(x);
    "#);
    
    // 日志输出到 stderr，测试失败时可见
    assert!(result.is_ok());
}
```

### 4.2 捕获特定阶段日志

```rust
#[test]
fn test_lexer_specific() {
    // 只关注 Lexer
    let config = Config {
        log: LogConfig {
            global: LogLevel::OFF,
            lexer: Some(LogLevel::TRACE),  // 只开 Lexer
        },
        ..Default::default()
    };
    
    init(config);
    
    // 运行测试...
    let tokens = kaubo::lex("var x = 5;").unwrap();
}
```

### 4.3 失败测试调试

```bash
# 查看失败测试的输出
cargo test failing_test -- --nocapture

# 保存完整日志
cargo test failing_test -- --nocapture 2> debug.log

# 使用 RUST_BACKTRACE
RUST_BACKTRACE=1 cargo test failing_test
```

---

## 5. 编写测试

### 5.1 测试命名规范

```rust
// 好：清晰描述测试内容
#[test]
fn test_addition_with_negative_numbers() { }

#[test]
fn test_lexer_handles_unicode() { }

#[test]
fn test_vm_stack_overflow_graceful() { }

// 不好：过于笼统
#[test]
fn test1() { }
#[test]
fn test_math() { }
```

### 5.2 测试结构

```rust
#[test]
fn test_feature() {
    // Arrange：准备
    let source = "var x = 5; return x;";
    
    // Act：执行
    let result = run_code(source);
    
    // Assert：验证
    assert!(result.is_ok());
    assert_int(&result.unwrap(), 5);
}
```

### 5.3 表驱动测试

```rust
#[test]
fn test_arithmetic_operators() {
    let cases = vec![
        ("return 1 + 2;", 3),
        ("return 5 - 3;", 2),
        ("return 4 * 5;", 20),
        ("return 20 / 4;", 5),
    ];
    
    for (code, expected) in cases {
        let result = run_code(code).unwrap();
        assert_int(&result, expected, "Failed for: {}", code);
    }
}
```

### 5.4 错误测试

```rust
#[test]
fn test_undefined_variable() {
    let result = run_code("return x;");
    
    assert!(result.is_err());
    
    // 验证错误类型
    match result.unwrap_err() {
        KauboError::Compile(msg) => {
            assert!(msg.contains("undefined"));
        }
        _ => panic!("Expected compile error"),
    }
}
```

### 5.5 条件测试

```rust
// 某些平台特有测试
#[test]
#[cfg(target_os = "linux")]
fn test_linux_specific() { }

// 需要特定特性的测试
#[test]
#[cfg(feature = "gc")]
fn test_gc_behavior() { }
```

---

## 附录：测试检查清单

提交代码前检查：

- [ ] `cargo test` 全部通过
- [ ] 新功能有对应测试
- [ ] 边界情况被覆盖
- [ ] 错误路径被测试
- [ ] 测试命名清晰
- [ ] 没有 `println!` 残留（改用日志）

---

*最后更新: 2026-02-10*  
*版本: 3.0 (架构重构)*
