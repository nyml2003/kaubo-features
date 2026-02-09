# Kaubo 设计文档

本文档描述 Kaubo 语言的设计决策、字节码规范和待实现特性。

---

## 目录

1. [字节码设计](#1-字节码设计)
2. [值表示（NaN Boxing）](#2-值表示na-n-boxing)
3. [变量解析](#3-变量解析)
4. [闭包设计](#4-闭包设计)
5. [模块系统设计](#5-模块系统设计)
6. [待实现计划](#6-待实现计划)

---

## 1. 字节码设计

### 1.1 指令集

```rust
pub enum OpCode {
    // 常量加载 (0x00-0x1F)
    LoadConst0 = 0x00, LoadConst1, ..., LoadConst15,
    LoadConst,           // 0x10 + u8
    LoadConstWide,       // 0x11 + u16
    LoadNull = 0x18, LoadTrue, LoadFalse, LoadZero, LoadOne,

    // 栈操作 (0x20-0x2F)
    Pop = 0x20, Dup, Swap,

    // 局部变量 (0x30-0x47)
    LoadLocal0 = 0x30, ..., LoadLocal7,
    LoadLocal,           // 0x38 + u8
    StoreLocal0 = 0x40, ..., StoreLocal7,
    StoreLocal,          // 0x48 + u8

    // 算术运算 (0x60-0x6F)
    Add = 0x60, Sub, Mul, Div, Neg,

    // 比较运算 (0x70-0x77)
    Equal = 0x70, NotEqual, Greater, GreaterEqual, Less, LessEqual,

    // 逻辑运算 (0x78-0x7B)
    Not = 0x78,

    // 控制流 (0x80-0x8F)
    Jump = 0x80, JumpIfFalse, JumpBack,

    // 函数 (0x90-0x9F)
    Call = 0x90, Return, ReturnValue,
    Closure,              // 创建闭包/函数对象
    GetUpvalue,           // 读取 upvalue（预留）
    SetUpvalue,           // 设置 upvalue（预留）

    // 模块 (0xA0-0xAF)
    ImportBuiltin = 0xA0, // + u8 模块名索引
    ImportModule,         // + u8 用户模块索引
    GetModuleMember,      // + u8 成员名索引

    // 列表 (0xB0-0xBF)
    BuildList = 0xB0,     // + u8 元素个数
    IndexGet,             // 列表索引读取

    // 调试 (0xF0-0xFF)
    Print = 0xF0, Invalid = 0xFF,
}
```

### 1.2 调用约定

```
栈帧布局:
┌─────────────────────────────┐ ← 栈顶
│          操作数栈            │
├─────────────────────────────┤
│  局部变量 0 (slot_base)      │
│  局部变量 1                  │
│  ...                        │
├─────────────────────────────┤
│      返回地址 / 原 FP         │
└─────────────────────────────┘
```

---

## 2. 值表示（NaN Boxing）

基于 IEEE 754 double 的 Quiet NaN 空间，采用 7-bit Tag。

### 2.1 位布局

```
64-bit 布局:
[63] [62-52] [51] [50-44] [43-0]
  │    │      │     │       └── Payload (44 bits)
  │    │      │     └── Tag (7 bits)
  │    │      └── QNAN 标志 (1 bit)
  │    └── Exponent (11 bits, 0x7FF)
  └── Sign (1 bit)

完整位模式: 0x7FF8_0000_0000_0000 | (Tag << 44) | Payload
```

### 2.2 类型标签分配

| Tag | 类型 | 说明 |
|-----|------|------|
| 0 | QNAN | 语言级 NaN |
| 1 | null | 直接位比较 |
| 2 | true | 直接位比较 |
| 3 | false | 直接位比较 |
| 4 | SMI | 小整数，Payload 低 31 位存储值 (-2^30 ~ 2^30-1) |
| 5-7 | 预留 | 未来特殊值 |
| 8-23 | InlineInt | 内联整数 -8~+7，值 = Tag-16，零 Payload |
| 24-31 | 预留 | 内联值扩展 |
| 32 | Heap | 通用堆对象指针 |
| 33 | String | 字符串对象 |
| 34 | Function | 函数对象 |
| 35 | List | 列表对象 |
| 36 | Iterator | 迭代器对象 |
| 37 | Closure | 闭包对象（预留）|
| 38-127 | 预留 | Map/Set/Date/Error 等堆类型 |

### 2.3 整数编码策略

```rust
// 自动选择最优编码
Value::int(n) 匹配:
  -8..=7   → InlineInt (Tag 编码，零空间)
  SMI 范围 → SMI (31-bit Payload)
  其他     → 溢出（未来用堆 BigInt）
```

### 2.4 关键常量

```rust
const QNAN: u64 = 0x7FF8_0000_0000_0000;  // 基础 NaN
const TAG_MASK: u64 = 0x7F << 44;          // bits 50-44
const PAYLOAD_MASK: u64 = 0xFFFFFFFFFFF;   // bits 43-0 (44位)
```

---

## 3. 变量解析

### 3.1 无全局变量模式

Kaubo 采用**无全局变量**设计，所有变量来源必须显式声明。

**6 种变量来源**（按解析优先级）：

| # | 来源 | 例子 | 说明 |
|---|------|------|------|
| 1 | 局部变量 | `var x = 5` | 当前函数内声明 |
| 2 | Upvalue | `\|\| { return x; }` | 外层函数变量，闭包捕获 |
| 3 | 模块变量 | 模块级 `var x` | 当前模块内声明 |
| 4 | 用户模块导入 | `math.PI` | `import math` |
| 5 | Builtin 导入 | `std.core.print` | `import std.core` |
| 6 | 未定义 | - | 编译错误 |

### 3.2 导入语法

```kaubo
// 方式 A：模块前缀（推荐）
import std.core;
import std.math;

fun demo() {
    std.core.print("Hello");
    var pi = std.math.PI;
}

// 方式 B：选择性导入
from std.core import print, assert;
from std.math import sqrt;

fun demo() {
    print("Hello");  // 直接使用
}

// 方式 C：重命名
from std.core import print as log;
```

### 3.3 编译时解析

```rust
enum Variable {
    Local(u8),
    Upvalue(u8),
    Module(u8),
    Import { module: u8, name: u8 },
    Builtin { module: u8, name: u8 },
    Undefined,
}

impl Compiler {
    fn resolve_variable(&mut self, name: &str) -> Variable {
        // 1. 局部变量
        if let Some(idx) = self.find_local(name) {
            return Variable::Local(idx);
        }
        
        // 2. Upvalue（递归向外查找）
        if let Some(idx) = self.resolve_upvalue(name) {
            return Variable::Upvalue(idx);
        }
        
        // 3. 当前模块变量
        if let Some(idx) = self.find_module_var(name) {
            return Variable::Module(idx);
        }
        
        // 4. 显式导入（用户模块或 builtin）
        if let Some(var) = self.find_import(name) {
            return var;
        }
        
        // 5. 未定义
        self.error(format!("undefined variable: {}", name));
        Variable::Undefined
    }
}
```

---

## 4. 闭包设计

### 4.1 核心数据结构

```rust
/// Upvalue 对象 - 表示对外部变量的引用（Lua 风格）
pub struct ObjUpvalue {
    /// 指向外部变量的指针（栈上或已关闭）
    pub location: *mut Value,
    /// 如果变量离开栈，转储到这里
    pub closed: Option<Value>,
}

impl ObjUpvalue {
    pub fn new(location: *mut Value) -> Self;
    pub fn get(&self) -> Value;
    pub fn set(&mut self, value: Value);
    pub fn close(&mut self);  // 将栈值复制到 closed
}

/// 闭包对象 - 包含函数和捕获的 upvalues
pub struct ObjClosure {
    pub function: *mut ObjFunction,
    pub upvalues: Vec<*mut ObjUpvalue>,
}

impl ObjClosure {
    pub fn new(function: *mut ObjFunction) -> Self;
    pub fn add_upvalue(&mut self, upvalue: *mut ObjUpvalue);
    pub fn get_upvalue(&self, index: usize) -> Option<*mut ObjUpvalue>;
}
```
```

### 4.2 捕获策略

- **按引用捕获**（Lua 风格）：闭包内外共享同一变量
- **立即堆分配**：创建 upvalue 时即分配堆内存
- **写时关闭**：当外部函数返回时，将栈上的值复制到 upvalue 的 `closed` 字段

### 4.3 编译时 Upvalue 解析

```rust
/// Upvalue 描述（编译时）
struct UpvalueDescriptor {
    name: String,
    index: u8,        // 在该层的索引
    is_local: bool,   // true=局部变量, false=继承的 upvalue
}

impl Compiler {
    /// 递归解析 upvalue
    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        let parent_idx = self.scope.parent?;
        
        // 在父作用域查找局部变量
        if let Some((local_idx, _)) = self.scopes[parent_idx].find_local(name) {
            self.scopes[parent_idx].mark_captured(local_idx);
            return Some(self.add_upvalue(UpvalueDescriptor {
                name: name.to_string(),
                index: local_idx,
                is_local: true,
            }));
        }
        
        // 递归查找更外层
        if let Some(upvalue_idx) = self.resolve_upvalue_recursive(name, parent_idx) {
            return Some(self.add_upvalue(UpvalueDescriptor {
                name: name.to_string(),
                index: upvalue_idx,
                is_local: false,
            }));
        }
        
        None
    }
}
```

### 4.4 内存布局示例

```
外部函数栈帧:
┌─────────────┐
│ local x: 5  │ ← slot 0
└─────────────┘
      ↑
      │ 引用
┌─────────────┐     ┌─────────────┐
│ Upvalue     │────→│ location    │────→ slot 0 (栈上)
│ { location, │     │ closed: None│
│   closed }  │     └─────────────┘
└─────────────┘
      ↑
      │ 包含
┌─────────────┐
│ Closure     │
│ { function, │
│   upvalues: │
│   [upvalue] }│
└─────────────┘
```

### 4.5 VM 中的 Upvalue 管理

```rust
pub struct VM {
    // ... 其他字段
    open_upvalues: Vec<*mut ObjUpvalue>,  // 打开的 upvalues（按地址排序）
}

impl VM {
    /// 捕获 upvalue（复用已存在的或创建新的）
    fn capture_upvalue(&mut self, location: *mut Value) -> *mut ObjUpvalue {
        // 从后向前查找是否已有指向相同位置的 upvalue
        for &upvalue in self.open_upvalues.iter().rev() {
            if unsafe { (*upvalue).location == location } {
                return upvalue;  // 复用
            }
        }
        // 创建新的 upvalue
        let upvalue = Box::into_raw(Box::new(ObjUpvalue::new(location)));
        self.open_upvalues.push(upvalue);
        upvalue
    }

    /// 关闭从指定槽位开始的所有 upvalues
    fn close_upvalues(&mut self, slot: usize) {
        // 关闭所有地址 >= 指定位置的 upvalue
        // 将值从栈复制到 closed 字段
    }
}
```

### 4.6 指令实现

**Closure** 指令格式：`Closure | const_idx | upvalue_count | (is_local, index)...`

```rust
Closure => {
    let const_idx = read_byte();
    let upvalue_count = read_byte();
    let func = constants[const_idx].as_function();
    let mut closure = ObjClosure::new(func);
    
    for _ in 0..upvalue_count {
        let is_local = read_byte() != 0;
        let index = read_byte();
        
        if is_local {
            // 捕获当前帧的局部变量
            let location = current_local_ptr(index);
            closure.add_upvalue(capture_upvalue(location));
        } else {
            // 继承当前闭包的 upvalue
            let upvalue = current_closure().get_upvalue(index);
            closure.add_upvalue(upvalue);
        }
    }
    push(Value::closure(closure));
}

GetUpvalue => {
    let idx = read_byte();
    let upvalue = current_closure().get_upvalue(idx);
    push(upvalue.get());
}

SetUpvalue => {
    let idx = read_byte();
    let value = peek(0);
    let upvalue = current_closure().get_upvalue(idx);
    upvalue.set(value);
}

CloseUpvalues => {
    let slot = read_byte();
    close_upvalues(slot);
}
```

### 4.7 验收代码

```kaubo
// 基础捕获
var x = 5;
var f = || { return x; };
assert(f() == 5);

// 修改外部变量
var y = 10;
var g = || { y = y + 1; return y; };
assert(g() == 11);
assert(y == 11);

// 多变量捕获
var a = 1;
var b = 2;
var h = || { return a + b; };
assert(h() == 3);

// 嵌套闭包
var outer = 100;
var f1 = || {
    var inner = 10;
    var f2 = || { return outer + inner; };
    return f2();
};
assert(f1() == 110);
```

---

## 5. 模块系统设计

### 5.1 设计原则

- **无全局变量**：所有变量必须显式声明来源
- **显式导入**：Builtin 模块也需要 `import`
- **文件即模块**：`math.kaubo` 文件对应 `math` 模块

### 5.2 模块定义

```kaubo
// math.kaubo
module math {
    // 默认 private
    var PI = 3.14;
    
    // pub 导出
    pub fun add(a, b) { return a + b; }
    pub fun square(x) { return x * x; }
}
```

### 5.3 模块使用

```kaubo
// main.kaubo
import math;              // 导入用户模块
import std.core;          // 导入 builtin

print math.add(1, 2);
std.core.print("Hello");

// 选择性导入
from math import square;
print square(5);
```

### 5.4 Builtin 模块

```
std.core      // 核心：print, assert, panic, typeof
std.math      // 数学：sin, cos, sqrt, PI
std.string    // 字符串：len, concat, slice
std.io        // IO：read_line, write_file
std.collections // 集合：List, Map, Set 类型
```

### 5.5 运行时模块对象

```rust
pub struct ObjModule {
    name: String,
    exports: HashMap<String, Value>,
    variables: Vec<Value>,
    imports: Vec<Gc<ObjModule>>,
}

pub struct CallFrame {
    chunk: Chunk,
    ip: *const u8,
    locals: Vec<Value>,
    module: Gc<ObjModule>,  // 当前模块（用于访问模块变量）
    upvalues: Option<Vec<Gc<ObjUpvalue>>>,
}
```

---

## 6. 待实现计划

### Phase 2.3：闭包支持 ✅ 已完成

**已完成**:
- ✅ `ObjUpvalue` / `ObjClosure` 结构体 (`src/runtime/object.rs`)
- ✅ `Value::closure()` 及类型判断方法 (`src/runtime/value.rs`, Tag 37)
- ✅ `GetUpvalue(u8)` / `SetUpvalue(u8)` / `CloseUpvalues(u8)` 指令
- ✅ VM：闭包调用、upvalue 捕获与关闭 (`src/runtime/vm.rs`)
- ✅ 编译器：变量解析与捕获分析
  - 作用域链跟踪（编译时维护嵌套函数层次）
  - 变量解析：区分 Local / Upvalue / Module / Import
  - 递归 Upvalue 解析（嵌套闭包捕获）
  - Upvalue 描述表：每个函数维护 upvalue 索引映射

**验收代码**:
```kaubo
var x = 5;
var f = || { return x; };
assert(f() == 5);  // ✅ 通过

// 多变量捕获
var a = 1;
var b = 2;
var g = || { return a + b; };
assert(g() == 3);  // ✅ 通过

// 可修改捕获
var c = 10;
var h = || { c = c + 1; return c; };
assert(h() == 11);  // ✅ 通过
assert(c == 11);    // ✅ 外部变量同步更新

// Y 组合子（高阶闭包嵌套）
var Y = |f|{
    return |x|{ return f(|n|{ return x(x)(n); }); }
           (|x|{ return f(|n|{ return x(x)(n); }); });
};
var factorial = Y(|f|{
    return |n|{ if (n == 0) { return 1; } else { return n * f(n - 1); } };
});
assert(factorial(5) == 120);  // ✅ 通过
```

**问题修复**: 修复了闭包 upvalue 内存安全 bug（详见 `docs/issues/closure-upvalue-bug.md`）

### Phase 2.4：协程与迭代器 🚧 当前阶段

- [ ] 栈式协程（独立调用栈）
- [ ] Yield/Resume 双向通信
- [ ] 生成器函数（yield 语法）
- [ ] 用户自定义迭代器

### Phase 2.5：Result 类型与错误处理 ⏳

- [ ] `Result<T, E>` 类型
- [ ] `Option<T>` 类型（替换 null）
- [ ] match 表达式
- [ ] 错误传播机制

### Phase 2.6：模块系统与标准库 ⏳

- [ ] 单文件内模块语法
- [ ] `import` / `from...import` 语法
- [ ] `pub` 导出关键字
- [ ] Builtin 模块注册表（`std.core`, `std.math` 等）
- [ ] 多文件模块加载（文件系统）

### Phase 2.7：严格类型系统 ⏳

- [ ] 类型标注语法 (`var x: Int`)
- [ ] 函数签名标注
- [ ] 类型推断
- [ ] 类型检查器

### Phase 2.8：GC 与优化 ⏳

- [ ] 标记-清除 GC
- [ ] 对象生命周期管理
- [ ] 循环引用处理

### Phase 3：包管理 ⏳

- [ ] 包配置格式
- [ ] 依赖解析
- [ ] 包发布/安装

### Phase 4：性能优化 ⏳

- [ ] JIT 编译（基线 JIT）
- [ ] 内联缓存
- [ ] 逃逸分析

---

*文档版本: 2.0*  
*最后更新: 2026-02-09*  
*状态: Phase 2.4 进行中*
