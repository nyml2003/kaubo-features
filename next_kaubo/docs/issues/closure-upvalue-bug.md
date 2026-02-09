# Issue: 闭包 Upvalue 内存安全 Bug

## 问题描述

Y 组合子（Y Combinator）测试用例 `assets/test_2.txt` 运行时随机失败，出现 "Can only call functions" 错误。

## 测试用例

```kaubo
var Y = |f|{
    return |x|{ 
        return f(|n|{ return x(x)(n); });
    } (|x|{ return f(|n|{ return x(x)(n); }); });
};

var factorial = Y(|f|{
    return |n|{
        if (n == 0) { return 1; } 
        else { return n * f(n - 1); }
    };
});

print factorial(5);  // 期望: 120
```

## 现象

- 偶尔能成功执行（输出 120）
- 大部分情况下失败："❌ 运行时错误: Can only call functions"
- 调试发现 `GetUpvalue` 返回了 `SMI(4)` 而不是预期的 `Closure`

## 根因分析

### 根本原因

`ObjUpvalue::close()` 只是将值复制到 `closed` 字段，但 `get()` 和 `set()` 仍然使用 `location` 指针访问栈内存。

```rust
// 修复前（buggy）
pub fn get(&self) -> Value {
    unsafe { *self.location }  // 始终使用栈指针！
}

pub fn close(&mut self) {
    if self.closed.is_none() {
        self.closed = Some(self.get());  // 复制到 closed
        // 但 location 仍指向已释放的栈内存！
    }
}
```

当递归调用 `f(n-1)` 时：
1. 外层函数返回，其栈帧被销毁
2. 外层的 upvalue 被 close，值被复制到 `closed`
3. 但 `get()` 仍然通过 `location` 指针读取栈内存
4. 新的函数调用覆盖了相同的栈位置，导致 upvalue 读取到错误的值

### 调试过程

1. **初始怀疑栈未清理**：尝试在 `ReturnValue` 中截断栈到 `caller_stack_base`，但问题依旧
2. **添加调试输出**：发现 `GetUpvalue` 返回了 `SMI(4)` 而非 `Closure`
3. **定位 upvalue 实现**：发现 `close()` 后仍使用 `location` 指针
4. **修复 get/set**：优先使用 `closed` 字段

## 修复方案

```rust
// src/runtime/object.rs

impl ObjUpvalue {
    /// 获取当前值（优先使用 closed，否则使用 location）
    pub fn get(&self) -> Value {
        match self.closed {
            Some(value) => value,
            None => unsafe { *self.location },
        }
    }

    /// 设置值（优先写入 closed，否则写入 location）
    pub fn set(&mut self, value: Value) {
        match self.closed {
            Some(_) => self.closed = Some(value),
            None => unsafe { *self.location = value; }
        }
    }

    /// 关闭 upvalue：将栈上的值复制到 closed
    pub fn close(&mut self) {
        if self.closed.is_none() {
            self.closed = Some(unsafe { *self.location });
            self.location = std::ptr::null_mut();  // 不再使用
        }
    }
}
```

## 验证

```bash
$ cargo run -- assets/test_2.txt
✅ 执行成功!
factorial(5) = 120

$ cargo test
test result: ok. 227 passed
```

## 经验教训

1. **指针安全**：当数据从栈移动到堆时，必须更新所有访问路径
2. **close 语义**：Lua 风格的 upvalue close 必须确保后续访问使用堆上数据
3. **随机性**：内存布局的随机性导致问题间歇性出现，增加调试难度

## 相关代码

- `src/runtime/object.rs` - `ObjUpvalue` 结构体和实现
- `src/runtime/vm.rs` - `close_upvalues()` 和 `capture_upvalue()`
- `assets/test_2.txt` - Y 组合子测试用例
