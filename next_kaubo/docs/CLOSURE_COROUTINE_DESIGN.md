# Kaubo 高级特性设计方案

> 本文档讨论闭包、协程、生成器/迭代器、异常处理的实现方案选型。
> 
> **目标**: 在保持 Phase 2 字节码 VM 架构基础上，支持闭包、单线程协程（基于迭代器/生成器）和 Rust 风格异常处理。

---

## 设计决策摘要

| 特性 | 决策 | 说明 |
|------|------|------|
| **变量捕获** | 按引用捕获 | 闭包内修改影响外部变量 |
| **Result 类型** | 需显式标注 | 类型安全，行为一致 |
| **? 操作符** | Phase 2.5 后实现 | 先跑通基础，再丰富语法糖 |
| **panic** | 暂不实现 | 仅用 Result 处理错误 |
| **错误传播** | 作为值产出 | yield Err(value) / resume Err(value) |
| **协程栈** | 动态增长 | 初始小栈，按需扩容 |
| **迭代器协议** | 统一协议 | 所有可迭代对象统一接口 |

---

## 1. 闭包实现方案

### 1.1 选定方案：Lua 风格 Upvalue 简化版

**核心机制**：
```rust
// 开放 upvalue 指向栈上变量，关闭后移到堆上
struct ObjUpvalue {
    location: *mut Value,   // 指向栈或堆（关闭后指向自己）
    closed: Cell<Value>,    // 关闭后的值存储
    next: *mut ObjUpvalue,  // GC 链表
}

struct ObjClosure {
    header: ObjHeader,
    function: *ObjFunction,
    upvalues: Vec<Gc<ObjUpvalue>>,  // 捕获的变量列表
}
```

**变量捕获语义（按引用）**：
```kaubo
var x = 5;
var f = || { 
    x = x + 1;  // 修改外部 x
    return x;
};
f();  // 返回 6
print(x);  // 输出 6，外部 x 也被修改
```

**指令设计**：
```rust
Closure(u8 const_idx, u8 upvalue_count)   // 创建闭包
GetUpvalue(u8 idx)                        // 读取第 idx 个 upvalue
SetUpvalue(u8 idx)                        // 写入第 idx 个 upvalue
CloseUpvalues(u8 slot_from)               // 关闭 slot_from 以上的 upvalue
```

**简化点**（相比完整 Lua）：
1. 不使用 open upvalue 链表，变量离开作用域时立即关闭
2. 用 `Vec` 存储 upvalues，无需变长数组技巧
3. 用 `Cell` 实现内部可变性，避免 `RefCell` 的运行时检查

---

## 2. 异常处理：Result 类型

### 2.1 核心设计

**显式类型标注，行为一致**：
```kaubo
// 返回 Result 的函数必须标注
fun may_fail() -> Result<Int, String> {
    if some_condition {
        return Err("error message");
    }
    return Ok(42);
}

// 使用 match 处理
match may_fail() {
    Ok(v) => { print(v); }
    Err(e) => { print("error: " + e); }
}

// 暂时不支持 ? 操作符，Phase 2.5 后添加
// var x = may_fail()?;  // 暂未实现
```

### 2.2 VM 实现：Result 作为 Tagged Value

```rust
// 扩展现有 NaN Boxing
const TAG_RESULT_OK: u64 = 3 << 48;   // Ok 值
const TAG_RESULT_ERR: u64 = 4 << 48;  // Err 值

struct Value {
    // ... 原有字段
    
    pub fn is_result(&self) -> bool {
        self.is_boxed() && (
            (self.0 & TAG_MASK) == TAG_RESULT_OK ||
            (self.0 & TAG_MASK) == TAG_RESULT_ERR
        )
    }
    
    pub fn is_ok(&self) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == TAG_RESULT_OK
    }
    
    pub fn is_err(&self) -> bool {
        self.is_boxed() && (self.0 & TAG_MASK) == TAG_RESULT_ERR
    }
}
```

**指令设计**：
```rust
BuildOk        // 构造 Ok(value)
BuildErr       // 构造 Err(value)
UnwrapOk       // 解包 Ok（运行时检查）
UnwrapErr      // 解包 Err（运行时检查）
IsOk           // 检查是否是 Ok
IsErr          // 检查是否是 Err
MatchResult(jump_if_ok, jump_if_err)  // match 表达式优化
```

---

### 2.3 错误传播语义（作为值产出）

在生成器和协程中，Err 作为普通值产出，不中断执行：

```kaubo
var gen = || -> Result<Int, String> {
    yield may_fail();        // 如果返回 Err，作为值 yield，继续执行
    yield another_fail();    // 同上
    return Ok(0);
};

// 使用
var iter = gen();
loop {
    match iter.next() {
        Some(Ok(v)) => print("value: " + v),
        Some(Err(e)) => print("error: " + e),  // 错误被产出，不终止迭代
        None => break,
    }
}
```

**语义说明**：
- `return Err(x)`：终止生成器/协程，最终结果为 Err
- `yield Err(x)`：产出错误值，继续执行
- 调用者通过 `match` 区分 Ok/Err

---

## 3. 协程与迭代器

### 3.1 协程结构（动态增长栈）

```rust
struct Coroutine {
    stack: Vec<Value>,           // 值栈，动态增长
    call_stack: Vec<CallFrame>,  // 调用帧栈
    state: CoroState,            // Suspended/Running/Dead
    parent: Option<Gc<Coroutine>>, // 调用者
    
    // 动态增长配置
    initial_stack_size: usize,   // 初始 256
    max_stack_size: usize,       // 最大 64KB
}

enum CoroState {
    Suspended,  // 可恢复
    Running,    // 正在执行
    Dead,       // 已结束
}
```

**栈增长策略**：
```rust
fn ensure_stack_capacity(&mut self, needed: usize) {
    if self.stack.len() + needed > self.stack.capacity() {
        let new_cap = (self.stack.capacity() * 2).min(self.max_stack_size);
        self.stack.reserve(new_cap - self.stack.capacity());
    }
}
```

---

### 3.2 统一迭代器协议

所有可迭代对象实现 `Iterator` trait：

```rust
trait Iterator {
    fn next(&mut self) -> Option<Value>;
    fn is_done(&self) -> bool;
}

// 内置类型的迭代器实现
struct ListIterator { list: Gc<ObjList>, index: usize }
struct RangeIterator { current: i32, end: i32, step: i32 }
struct StringIterator { string: Gc<ObjString>, byte_index: usize }
struct GeneratorIterator { coroutine: Gc<Coroutine> }
```

**for-in 语法统一**：
```kaubo
// 所有类型统一使用 in
for (x) in [1, 2, 3] { ... }        // 列表
for (i) in 0..10 { ... }            // 范围
for (ch) in "hello" { ... }         // 字符串
for (item) in my_generator() { ... } // 生成器

// 底层编译为迭代器协议
var iter = [1, 2, 3].iter();
loop {
    match iter.next() {
        Some(x) => { ... }
        None => break,
    }
}
```

**指令设计**：
```rust
GetIterator        // 获取对象的迭代器
IteratorNext       // 调用 next()，返回 Option<Value>
IteratorIsDone     // 检查是否结束
```

---

### 3.3 Yield/Resume 指令

```rust
Yield              // 挂起协程，产出栈顶值
Resume(co)         // 恢复协程，将值压入协程栈
ResumeWith(co)     // 恢复并传入值（双向通信）
```

**双向通信示例**：
```kaubo
var gen = || {
    var x = yield 1;   // 产出 1，接收外部传入的 x
    var y = yield x + 1;  // 产出 x+1，接收 y
    return y * 2;
};

var iter = gen();
var a = iter.next();      // a = Some(1)
iter.send(10);            // x = 10
var b = iter.next();      // b = Some(11)
iter.send(5);             // y = 5
var c = iter.next();      // c = None，生成器结束，返回值 10
```

---

## 4. 内存管理

### 4.1 标记-清除 GC（基础版）

```rust
struct ObjHeader {
    ty: ObjType,
    marked: bool,
    next: *mut ObjHeader,  // GC 链表
}

struct Heap {
    objects: *mut ObjHeader,  // 所有分配的对象链表
    bytes_allocated: usize,
    next_gc: usize,           // 触发 GC 的阈值
}
```

**GC 根节点**：
1. 全局变量表
2. 当前执行协程的栈
3. open upvalues（尚未关闭的捕获变量）
4. 常量池中的对象

**GC 流程**：
```rust
fn gc(&mut self) {
    // 1. 标记阶段
    self.mark_roots();
    self.trace_references();
    
    // 2. 清除阶段
    self.sweep();
    
    // 3. 调整阈值
    self.next_gc = self.bytes_allocated * 2;
}
```

---

## 5. 实现迭代计划

### Phase 2.2: 变量系统与控制流（4周）

**目标**: 支持变量读写和 if/while/for 控制流

| 周次 | 任务 | 产出 |
|------|------|------|
| W1 | 局部变量表设计 | 栈帧结构、LoadLocal/StoreLocal 指令 |
| W2 | 变量编译器 | 支持 var/const 声明、变量读写、赋值 |
| W3 | 控制流编译 | if/elif/else、while 循环、break/continue |
| W4 | for-in 基础 | 列表迭代、范围迭代、集成测试 |

**验收标准**:
```kaubo
var x = 5;
var y = x + 3;
if (y > 7) {
    x = x * 2;
} else {
    x = 0;
}
while (x > 0) {
    x = x - 1;
}
for (i) in 0..10 {
    print(i);
}
```

---

### Phase 2.3: 闭包支持（3周）

**目标**: 支持 Lambda 捕获外部变量

| 周次 | 任务 | 产出 |
|------|------|------|
| W1 | Upvalue 机制 | ObjUpvalue、按引用捕获、关闭逻辑 |
| W2 | 闭包对象 | ObjClosure、编译期捕获分析、Closure 指令 |
| W3 | 集成与测试 | Lambda 全功能测试、捕获生命周期测试 |

**验收标准**:
```kaubo
var x = 5;
var f = || { x = x + 1; return x; };
assert(f() == 6);
assert(x == 6);  // 外部变量被修改

// 嵌套闭包
var make_adder = |n| {
    return |x| { return x + n; };
};
var add5 = make_adder(5);
assert(add5(3) == 8);
```

---

### Phase 2.4: 协程与迭代器（4周）

**目标**: 支持 yield、动态栈、统一迭代器协议

| 周次 | 任务 | 产出 |
|------|------|------|
| W1 | 协程结构体 | Coroutine、动态增长栈、状态管理 |
| W2 | Yield/Resume | 指令实现、双向通信、状态切换 |
| W3 | 统一迭代器协议 | Iterator trait、列表/范围/字符串迭代器 |
| W4 | 生成器语法 | || 生成器函数、for-in 统一、集成测试 |

**验收标准**:
```kaubo
// 协程
var co = coro || {
    yield 1;
    yield 2;
    return 3;
};
assert(co.next() == Some(1));
assert(co.next() == Some(2));
assert(co.next() == None);

// 生成器
var gen = || {
    for (i) in 0..3 {
        yield i * i;
    }
};
for (x) in gen() {
    print(x);  // 0, 1, 4
}
```

---

### Phase 2.5: Result 类型与错误处理（3周）

**目标**: 支持 Rust 风格 Result 类型

| 周次 | 任务 | 产出 |
|------|------|------|
| W1 | Result 值类型 | TAG_RESULT_OK/ERR、BuildOk/BuildErr 指令 |
| W2 | 类型系统扩展 | 函数返回类型标注、Result 类型检查 |
| W3 | match 表达式 | match 编译、模式匹配基础、集成测试 |

**验收标准**:
```kaubo
fun divide(a: Int, b: Int) -> Result<Int, String> {
    if (b == 0) {
        return Err("division by zero");
    }
    return Ok(a / b);
}

match divide(10, 2) {
    Ok(v) => print("result: " + v),
    Err(e) => print("error: " + e),
}

// 生成器中 yield Err
var gen = || -> Result<Int, String> {
    yield Ok(1);
    yield Err("oops");
    yield Ok(2);
};
```

---

### Phase 2.6: GC 与优化（2周）

**目标**: 实现基础 GC，解决内存泄漏

| 周次 | 任务 | 产出 |
|------|------|------|
| W1 | 标记-清除 GC | Heap 管理、GC 根节点、标记清除流程 |
| W2 | 集成与优化 | 闭包 Upvalue GC、协程栈 GC、触发策略 |

**验收标准**:
- 循环引用对象被正确回收
- 长时间运行不 OOM
- GC 暂停时间 < 10ms（小对象）

---

## 6. 技术风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 协程栈动态增长复杂度 | 中 | 先做固定大小，再迁移到动态增长 |
| Upvalue 生命周期管理 | 中 | 详细测试用例，参考 Lua 实现 |
| Result 类型系统复杂度 | 中 | 先做运行时检查，Phase 3 做静态类型检查 |
| GC 与闭包/协程交互 | 高 | 保守扫描，确保不泄露 |

---

## 7. 待决策事项

1. **Phase 2.5 是否实现 ? 操作符**？
   - 建议：先不实现，用 `match` 替代
   - 待验证 Result 稳定性后再添加

2. **是否支持多变量捕获的解构**？
   ```kaubo
   var (a, b) = (1, 2);  // 元组解构
   for ((k, v)) in map { ... }  // 迭代器解构
   ```
   - 建议：Phase 2.4 后评估

3. **异步/await 是否纳入 Phase 2**？
   - 建议：不纳入，Phase 3 专门处理 async/await

---

*文档版本: 1.2*  
*最后更新: 2026-02-08*  
*计划总工期: 16 周（4+3+4+3+2）*
