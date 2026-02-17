# 内置类型方法支持设计文档（Phase 1: List 方法）

> 状态：Phase 1 已确认 | 目标：支持 `a.push(100)` 语法

---

## 1. 设计决策过程

### 1.1 背景：当前现状

Kaubo 已支持 Struct 方法调用：

```kaubo
struct Point {
    x: float,
    y: float
}

impl Point {
    move: |self, dx: float, dy: float| -> void {
        self.x = self.x + dx;
        self.y = self.y + dy;
    }
}

var p = Point { x: 0.0, y: 0.0 };
p.move(1.0, 2.0);  // ✅ 当前已支持
```

但内置类型（List、String、JSON）**不支持**方法调用：

```kaubo
var list = [1, 2, 3];
list.push(4);      // ❌ 当前不支持
std.push(list, 4); // ✅ 只能这样用
```

### 1.2 核心问题：是否支持方法值？

**什么是方法值？**

```kaubo
var list = [1, 2, 3];
var f = list.push;   // 把方法作为值提取出来
f(4);                // 稍后调用
```

#### 方案对比

| 特性 | 支持方法值（Python 式） | 不支持方法值（Rust 式） |
|------|----------------------|----------------------|
| 语法 | `var f = list.push` | `var f = \|x\| list.push(x)` |
| 直接调用性能 | 有 BoundMethod 分配开销 | 零开销 |
| 间接调用 | 自动绑定 receiver | 需显式闭包 |
| 实现复杂度 | 高（需 BoundMethod 类型） | 低（复用现有指令） |
| 心智负担 | 低（语法糖） | 中（需理解闭包） |

#### JavaScript 的教训

```js
var obj = {
    items: [],
    push: function(x) { this.items.push(x); }
};

var f = obj.push;
f(4);  // ❌ TypeError: this.items is undefined
       // this 绑定丢失了！
```

JS 的 `obj.method` 返回**裸函数**，receiver 绑定丢失，是常见的坑。

#### Python 的实现成本

Python 的 `obj.method` 返回 **bound method**（receiver 已绑定）：

```python
lst = []
f = lst.append    # 每次访问都创建 BoundMethod 对象
f(1)              # 但性能有开销
```

**代价**：即使直接调用 `lst.append(1)`，也要创建 BoundMethod。

#### Kaubo 的决策

**选择：不支持方法值（Phase 1）**

理由：
1. **性能优先**：和 Kaubo "迭代速度优先" 原则一致，高频调用场景（如游戏循环）不能有额外开销
2. **显式优于隐式**：需要闭包时用户自己写 `\|x\| list.push(x)`，意图明确，无隐藏开销
3. **简单性**：减少心智负担，和 Rust/C++ 保持一致
4. **可扩展**：Phase 2 如果强烈需求，可以添加，不破坏现有代码

**参考语言**：
- ✅ **Rust**：`Vec::push` 不是值，必须 `\|x\| v.push(x)`
- ✅ **C++**：方法不是一等值
- ⚠️ **Go**：支持方法值，但有分配开销
- ⚠️ **Python**：支持，但方法访问比属性访问慢
- ❌ **JavaScript**：支持但易出错（this 绑定问题）

---

## 2. 最终方案：统一栈布局（方案 A）

### 2.1 核心设计

```
用户代码: a.push(100)
     ↓
字节码:  LoadLocal(a)      # receiver 入栈
         LoadMethod(0)     # 方法索引（编译期确定）
         LoadConst(100)    # 参数
         Call(2)           # 调用，2 个参数（receiver + 1 个显式参数）
     ↓
VM 执行: LoadMethod: peek receiver, push encoded_method_id
         Call: pop method_id, pop args, dispatch
```

**关键特性**：
- `LoadMethod`：**peek** receiver（不弹出），压入编码方法标识
- `Call`：统一从栈上取 receiver，内置类型走原生分发
- **零堆分配**：无 BoundMethod 对象

### 2.2 类型索引常量

```rust
// kaubo-core/src/core/builtin_methods.rs

pub mod builtin_types {
    pub const LIST: u8 = 0;
    pub const STRING: u8 = 1;
    pub const JSON: u8 = 2;
}

pub mod list_methods {
    pub const PUSH: u8 = 0;
    pub const LEN: u8 = 1;
    pub const REMOVE: u8 = 2;
    pub const CLEAR: u8 = 3;
    pub const IS_EMPTY: u8 = 4;
    
    pub const COUNT: usize = 5;
}

pub mod string_methods {
    pub const LEN: u8 = 0;
    pub const IS_EMPTY: u8 = 1;
    // ...
}
```

### 2.3 编码方案（SMI）

```rust
/// 编码: 高 4 位 = 类型，低 4 位 = 方法索引
/// 范围: 0x0100 ~ 0x01FF（避开普通小整数）
pub fn encode_method(type_tag: u8, method_idx: u8) -> i32 {
    0x0100 + ((type_tag as i32) << 4) + (method_idx as i32)
}

pub fn decode_method(value: Value) -> Option<(u8, u8)> {
    let n = value.as_int()?;
    if n >= 0x0100 && n < 0x0200 {
        let type_tag = ((n - 0x0100) >> 4) as u8;
        let method_idx = ((n - 0x0100) & 0xF) as u8;
        Some((type_tag, method_idx))
    } else {
        None
    }
}
```

### 2.4 数据结构

```rust
/// 原生方法函数类型
pub type BuiltinMethodFn = fn(receiver: Value, args: &[Value]) -> Result<Value, String>;

/// 静态方法表（编译期确定）
pub static LIST_METHODS: [BuiltinMethodFn; list_methods::COUNT] = [
    list_push,      // idx 0
    list_len,       // idx 1
    list_remove,    // idx 2
    list_clear,     // idx 3
    list_is_empty,  // idx 4
];

/// 编译期方法名解析
pub fn resolve_list_method(name: &str) -> Option<u8> {
    match name {
        "push" => Some(list_methods::PUSH),
        "len" => Some(list_methods::LEN),
        "remove" => Some(list_methods::REMOVE),
        "clear" => Some(list_methods::CLEAR),
        "is_empty" => Some(list_methods::IS_EMPTY),
        _ => None,
    }
}
```

### 2.5 运行时实现

**LoadMethod 指令**（扩展）：

```rust
LoadMethod => {
    let method_idx = read_byte(vm);
    let receiver = stack::peek(vm, 0);  // peek，不弹出
    
    // 检查是否为内置类型
    if receiver.is_list() {
        let encoded = encode_method(builtin_types::LIST, method_idx);
        vm.stack.push(Value::smi(encoded));
    } else if receiver.is_string() {
        let encoded = encode_method(builtin_types::STRING, method_idx);
        vm.stack.push(Value::smi(encoded));
    } else if receiver.is_json() {
        let encoded = encode_method(builtin_types::JSON, method_idx);
        vm.stack.push(Value::smi(encoded));
    } else if let Some(struct_ptr) = receiver.as_struct() {
        // Struct：现有逻辑
        let shape = unsafe { (*struct_ptr).shape };
        if let Some(method) = unsafe { (*shape).get_method(method_idx) } {
            vm.stack.push(Value::function(method));
        } else {
            return RuntimeError("Method not found");
        }
    } else {
        return RuntimeError("Type has no methods");
    }
}
```

**Call 指令**（扩展）：

```rust
Call => {
    let arg_count = read_byte(vm);
    let callee = vm.stack.pop();
    
    // 检查是否为内置方法编码
    if let Some((type_tag, method_idx)) = decode_method(callee) {
        // 收集参数（包含 receiver）
        let mut args = Vec::with_capacity(arg_count as usize);
        for _ in 0..arg_count {
            args.push(vm.stack.pop().expect("Stack underflow"));
        }
        args.reverse();  // [receiver, arg1, arg2...]
        
        // 直接调用原生方法
        let result = match type_tag {
            builtin_types::LIST => LIST_METHODS[method_idx as usize](args[0], &args[1..]),
            builtin_types::STRING => STRING_METHODS[method_idx as usize](args[0], &args[1..]),
            builtin_types::JSON => JSON_METHODS[method_idx as usize](args[0], &args[1..]),
            _ => return RuntimeError("Unknown builtin type"),
        };
        
        match result {
            Ok(v) => vm.stack.push(v),
            Err(e) => return RuntimeError(e),
        }
    } else if let Some(closure_ptr) = callee.as_closure() {
        // 现有闭包调用逻辑...
    } else if let Some(func_ptr) = callee.as_function() {
        // 现有函数调用逻辑...
    } else {
        return RuntimeError("Not callable");
    }
}
```

---

## 3. 编译器实现

### 3.1 类型推导

```rust
fn infer_method_receiver_type(&self, receiver: &Expr) -> Option<Type> {
    match receiver {
        Expr::Variable(name) => self.get_var_type(name),
        Expr::ListLiteral(_) => Some(Type::List(Box::new(Type::Any))),
        Expr::StringLiteral(_) => Some(Type::String),
        Expr::JsonLiteral(_) => Some(Type::Json),
        _ => None,
    }
}
```

### 3.2 方法调用编译

```rust
fn compile_method_call(
    &mut self,
    receiver: &Expr,
    method_name: &str,
    args: &[Expr],
) -> CompileResult {
    // 1. 编译 receiver
    self.compile_expr(receiver)?;
    
    // 2. 根据 receiver 类型确定方法索引
    let receiver_type = self.infer_type(receiver)
        .ok_or_else(|| format!("Cannot infer type of receiver"))?;
    
    let method_idx = match &receiver_type {
        Type::List(_) => resolve_list_method(method_name)
            .ok_or_else(|| format!("List has no method '{}'", method_name))?,
        Type::String => resolve_string_method(method_name)
            .ok_or_else(|| format!("String has no method '{}'", method_name))?,
        Type::Struct(name) => self.get_struct_method_index(name, method_name)
            .ok_or_else(|| format!("Struct '{}' has no method '{}'", name, method_name))?,
        _ => return Err(format!("Type '{}' does not support methods", receiver_type)),
    };
    
    // 3. 生成 LoadMethod
    self.emit_u8(OpCode::LoadMethod as u8, method_idx);
    
    // 4. 编译参数
    for arg in args {
        self.compile_expr(arg)?;
    }
    
    // 5. 生成 Call（参数个数 = receiver + 显式参数）
    let total_args = args.len() + 1;
    self.emit_u8(OpCode::Call as u8, total_args as u8);
    
    Ok(())
}
```

---

## 4. 支持的方法清单

### 4.1 List 方法

| 方法 | 签名 | 说明 | 索引 |
|------|------|------|------|
| `push` | `\|self, item: T\| -> List<T>` | 末尾添加元素 | 0 |
| `len` | `\|self\| -> int` | 返回元素个数 | 1 |
| `remove` | `\|self, index: int\| -> T` | 移除并返回指定索引元素 | 2 |
| `clear` | `\|self\| -> List<T>` | 清空列表 | 3 |
| `is_empty` | `\|self\| -> bool` | 检查是否为空 | 4 |

### 4.2 String 方法（Phase 2）

| 方法 | 签名 | 说明 |
|------|------|------|
| `len` | `\|self\| -> int` | 返回字符数 |
| `is_empty` | `\|self\| -> bool` | 检查是否为空 |

### 4.3 JSON 方法（Phase 2）

| 方法 | 签名 | 说明 |
|------|------|------|
| `len` | `\|self\| -> int` | 返回键值对数 |
| `is_empty` | `\|self\| -> bool` | 检查是否为空 |

---

## 5. 实现步骤

| 序号 | 任务 | 文件 | 预估工时 |
|------|------|------|---------|
| 1 | 创建 `builtin_methods.rs` 模块 | `kaubo-core/src/core/builtin_methods.rs` | 1h |
| 2 | 添加 SMI 编码/解码函数 | `kaubo-core/src/core/builtin_methods.rs` | 30m |
| 3 | VM 集成 BuiltinMethodTable | `kaubo-core/src/core/vm.rs` | 30m |
| 4 | 扩展 `LoadMethod` 支持内置类型 | `kaubo-core/src/runtime/vm/execution.rs` | 1h |
| 5 | 扩展 `Call` 支持内置方法 | `kaubo-core/src/runtime/vm/execution.rs` | 1h |
| 6 | 编译器支持方法调用 | `kaubo-core/src/runtime/compiler/expr.rs` | 1.5h |
| 7 | 回归测试 | `kaubo-core/src/runtime/vm/tests.rs` | 1h |
| 8 | 更新语法规范 | `docs/20-current/spec/syntax.md` | 30m |

**总计**：7 小时

---

## 6. 验证示例

```kaubo
// test_list_methods.kaubo

var a: List<int> = [123, 1];

// push
a.push(100);
print(a);  // 期望: [123, 1, 100]

// len
print(a.len());  // 期望: 3

// is_empty
print(a.is_empty());  // 期望: false

// remove
var removed = a.remove(0);
print(removed);  // 期望: 123
print(a);        // 期望: [1, 100]

// clear
a.clear();
print(a);        // 期望: []
print(a.len());  // 期望: 0

// 链式调用（push 返回 receiver）
var b: List<int> = [];
b.push(1).push(2).push(3);
print(b);  // 期望: [1, 2, 3]
```

---

## 7. 性能特征

| 指标 | 数值 | 说明 |
|------|------|------|
| 直接调用开销 | 4 条指令 | 和 Struct 方法一致 |
| 内存分配 | 0 | 无 BoundMethod 对象 |
| 方法查找 | O(1) | 数组索引，编译期确定 |
| 与 `std.push` 对比 | 相同 | 无额外开销 |

---

## 8. 后续扩展（Phase 2+）

### Phase 2: 更多内置方法
- String: `substring`, `contains`, `starts_with`, `ends_with`
- JSON: `keys`, `has`

### Phase 3: 方法值支持（待定）
- 触发条件：明确需求 + 性能测试证明当前方案不足
- 实现：添加 `CallMethod` 指令优化直接调用，`LoadMethod` 保持现有行为

---

*最后更新：2026-02-16*  
*决策：不支持方法值（Rust 式），统一栈布局*  
*状态：待实施*
