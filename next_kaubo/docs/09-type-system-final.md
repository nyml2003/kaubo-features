# Kaubo 类型系统设计（最终版）

> Rust 风格的静态类型脚本语言，编译时确定内存布局

---

## 核心原则

1. **静态类型检查**：编译时捕获类型错误，运行时无类型检查开销
2. **统一函数语法**：函数类型和 Lambda 统一使用 `|params| -> R` 风格
3. **渐进类型**：类型标注可选，优先类型推导
4. **设计前瞻**：为泛型、JIT 预留扩展点，未来迭代成本低
5. **脚本友好**：无 `fn name()` 语法，只用匿名函数

---

## 基础类型

```kaubo
// 标量类型（值类型，Copy）
var b: bool = true;
var i: int = 42;          // i64
var f: float = 3.14;      // f64

// 字符串（只读引用）
var s: string = "hello";

// 类型推导
var x = 42;               // 推导为 int
var y = "hello";          // 推导为 string
```

### 内存布局

| 类型 | 大小 | 对齐 | 说明 |
|------|------|------|------|
| `bool` | 1 | 1 | 0 或 1 |
| `int` | 8 | 8 | i64 |
| `float` | 8 | 8 | f64 |
| `string` | 16 | 8 | (ptr, len)，不可变 |

---

## 复合类型

### 1. List<T> - 动态同类型序列

目前 `T` 作为语法占位符，实际为具体类型（如 `List<int>`、`List<string>`），**暂不支持用户定义泛型**。

```kaubo
// 定义（T 只能是具体类型）
var nums: List<int> = [1, 2, 3];
var names: List<string> = ["a", "b"];

// 自动增长
nums.push(4);        // [1, 2, 3, 4]
nums.pop();          // 返回 4

// 访问和修改
var first = nums[0];
nums[1] = 10;

// 长度
var len = nums.len();

// 遍历
for (var n in nums) {
    std.print(n);
}
```

**内存布局**：
```rust
struct List<T> {
    ptr: *mut T,     // 堆指针
    len: int,        // 当前长度
    cap: int,        // 容量
}
// 总大小：24 bytes
```

---

### 2. Tuple - 异构定长结构

```kaubo
// 定义（目前只支持具体类型）
var point: Tuple<int, int> = (10, 20);
var person: Tuple<string, int, bool> = ("Alice", 30, true);

// 索引访问
var name = person.0;
var age = person.1;

// 解构
var [x, y] = point;

// 函数返回多值
fn getUser() -> Tuple<string, int> {
    return ("Alice", 30);
}
var [name, age] = getUser();

// 空 tuple（unit 类型）
var unit: Tuple<> = ();
```

---

### 3. Struct - 具名字段结构

```kaubo
// 定义
struct Point {
    x: float,
    y: float,
}

struct User {
    name: string,
    age: int,
    scores: List<int>,
}

// 实例化
var p = Point { x: 1.0, y: 2.0 };
var u = User {
    name: "Alice",
    age: 30,
    scores: [85, 90],
};

// 访问字段
var x = p.x;
p.x = 2.0;
```

---

## 函数和闭包

### 统一的函数语法

```kaubo
// 函数类型：|参数类型| -> 返回类型
var callback: |int| -> void;
var binaryOp: |int, int| -> int;
var constructor: || -> Point;

// Lambda 表达式：|参数| -> 返回类型 { 函数体 }
var add = |a: int, b: int| -> int {
    return a + b;
};

// 简化（单行）
var double = |x: int| -> int { x * 2 };

// 无参
var getTimestamp = || -> int {
    return std.now();
};

// 捕获环境
var factor = 2;
var multiply = |x: int| -> int {
    return x * factor;
};
```

---

## 内置类型的方法分派

### 对象头 + 方法表设计

所有堆分配对象都有统一的对象头，为将来 Interface 虚表分派打基础：

```rust
// 对象头（8 bytes）
struct ObjectHeader {
    type_id: u32,      // 类型标识
    flags: u32,        // GC 标记等
}

// 类型描述符（全局唯一，运行时只读）
struct TypeDescriptor {
    type_id: u32,
    name: &'static str,
    size: usize,
    method_table: *const MethodTable,
    // 预留：为未来泛型扩展
    generic_info: Option<GenericInfo>,  // None for now
}

// 方法表
struct MethodTable {
    drop_fn: fn(*mut ()),
    method_count: u16,
    methods: [*const ()],  // 方法指针数组
}
```

### 方法调用编译

```kaubo
// 源代码
var list: List<int> = [1, 2, 3];
list.push(4);
var n = list.pop();
```

```
; 编译后的字节码
load_local 0           ; 加载 list 对象指针
dup                    ; 复制指针
load_method 0          ; 从方法表加载 push（索引 0）
load_const 4           ; 参数
call_method 1          ; 调用（1 个参数）
pop                    ; 丢弃返回值

load_local 0           ; 加载 list
dup
load_method 1          ; 加载 pop（索引 1）
call_method 0          ; 调用（0 个参数）
store_local 1          ; 存储到 n
```

### 内置类型的方法表

```rust
// List<int> 的方法表（编译期生成）
static LIST_INT_METHODS: MethodTable = MethodTable {
    drop_fn: list_int_drop,
    method_count: 5,
    methods: [
        list_push_int as *const (),    // 0: push
        list_pop_int as *const (),     // 1: pop
        list_len_int as *const (),     // 2: len
        list_get_int as *const (),     // 3: get
        list_set_int as *const (),     // 4: set
    ],
};

// 类型描述符表（运行时全局）
static TYPE_TABLE: &[TypeDescriptor] = &[
    TypeDescriptor { type_id: 1, name: "int", size: 8, method_table: ptr::null(), ... },
    TypeDescriptor { type_id: 2, name: "List<int>", size: 24, method_table: &LIST_INT_METHODS, ... },
    TypeDescriptor { type_id: 3, name: "List<string>", size: 24, method_table: &LIST_STRING_METHODS, ... },
    // ... 更多类型
];
```

### 编译期方法解析

```rust
pub struct TypeChecker {
    // 内置类型的方法签名表
    builtin_methods: HashMap<Type, HashMap<String, (MethodIndex, Signature)>>,
}

impl TypeChecker {
    fn resolve_method_call(&self, obj_type: Type, method_name: &str) -> MethodIndex {
        // 编译期确定方法索引
        let (index, _) = self.builtin_methods[&obj_type][method_name];
        index
    }
}
```

---

## Interface（特质系统）

### 定义 Interface

```kaubo
interface Reader {
    read: |self| -> string;
    close: |self| -> void;
}

interface Comparable {
    compare: |self, self| -> int;
}

// 组合 interface
interface ReadWriter: Reader {
    write: |string| -> void;
}
```

### 实现 Interface

```kaubo
struct FileReader {
    path: string,
    handle: int,
}

impl Reader for FileReader {
    read: |self| -> string {
        return std.read_file(self.path);
    },
    close: |self| -> void {
        std.close(self.handle);
        self.handle = -1;
    },
}
```

### 虚表分派

```rust
// Interface 值（胖指针）
struct InterfaceValue {
    data_ptr: *mut (),           // 指向实际对象
    vtable_ptr: *const VTable,   // 指向虚表
}

// 虚表
struct VTable {
    type_descriptor: *const TypeDescriptor,
    method_count: u16,
    methods: [*const ()],  // Interface 方法指针
}

// Reader interface 的虚表
struct ReaderVTable {
    base: VTable,
    read: fn(*const ()) -> string,
    close: fn(*mut ()),
}
```

```kaubo
// 使用 Interface
fn process(reader: Reader) -> string {
    var content = reader.read();  // 虚表分派
    reader.close();
    return content;
}
```

```
; 虚表分派的字节码
load_local 0           ; reader (InterfaceValue)
load_vtable_method 0   ; 从虚表加载 read 方法
load_interface_data    ; 加载 data_ptr 作为 self
call_indirect          ; 间接调用
```

---

## 泛型预留设计（暂不实现）

### 设计目标

**现在**：不支持用户定义泛型，但内置类型（`List<T>`、`Tuple<T>`）可用具体类型实例化  
**未来**：添加用户泛型时，改动最小，复用现有基础设施

### 预留的扩展点

#### 1. 类型描述符预留字段

```rust
struct TypeDescriptor {
    type_id: u32,
    name: &'static str,
    size: usize,
    method_table: *const MethodTable,
    
    // === 预留：泛型扩展 ===
    is_generic: bool,                    // 是否是泛型定义
    generic_params: Option<&'static [TypeParam]>,  // 泛型参数列表
    concrete_instances: Option<HashMap<TypeArgs, TypeId>>,  // 已实例化的类型
}

struct TypeParam {
    name: &'static str,
    constraints: Option<&'static [InterfaceId]>,
}
```

#### 2. 方法表预留动态扩展

```rust
struct MethodTable {
    drop_fn: fn(*mut ()),
    method_count: u16,
    methods: [*const ()],
    
    // === 预留：泛型方法单态化缓存 ===
    generic_instances: Option<HashMap<TypeArgs, *const MethodTable>>,
}
```

#### 3. 字节码预留指令

```rust
enum OpCode {
    // 基础指令
    LoadLocal(u8),
    StoreLocal(u8),
    LoadMethod(u8),
    CallMethod(u8),
    
    // === 预留：泛型相关指令（暂不实现）===
    // LoadGeneric(u8),       // 加载泛型参数
    // CallGeneric(u16, u8),  // 调用泛型函数
    // TypeCheck(TypeId),     // 运行时类型检查
    
    // Interface 调用（Phase 3 实现）
    LoadVTableMethod(u8),
    LoadInterfaceData,
    CallIndirect,
}
```

#### 4. 编译器预留阶段

```rust
pub struct Compiler {
    // 现有阶段
    lexer: Lexer,
    parser: Parser,
    type_checker: TypeChecker,
    bytecode_gen: BytecodeGen,
    
    // === 预留：泛型阶段（暂不启用）===
    // generic_monomorphizer: Option<GenericMonomorphizer>,
    // jit_compiler: Option<JitCompiler>,
}
```

### 未来添加泛型的改动量

| 组件 | 现有代码 | 添加泛型时需要 |
|------|---------|---------------|
| Parser | 解析 `List<int>` | + 解析 `fn identity<T>(x: T)` |
| TypeChecker | 检查具体类型 | + 泛型约束检查 |
| 字节码 | 单态化代码 | + 泛型描述符生成 |
| 运行时 | 固定类型表 | + 动态类型注册 |
| **改动比例** | **100%** | **+ 约 30%** |

---

## 内存布局总结

| 类型 | 内存布局 | 大小 |
|------|---------|------|
| `bool` | 直接值 | 1 byte |
| `int` | 直接值 | 8 bytes |
| `float` | 直接值 | 8 bytes |
| `string` | ObjectHeader + (ptr, len) | 24 bytes |
| `List<T>` | ObjectHeader + (ptr, len, cap) | 32 bytes |
| `Tuple<T1, T2>` | ObjectHeader + 字段 | 按类型而定 |
| `struct` | ObjectHeader + 字段 | 按类型而定 |
| `Interface` | (data_ptr, vtable_ptr) | 16 bytes |

---

## 语法速查表

| 特性 | 语法 | 示例 |
|------|------|------|
| 变量 | `var name: Type = value` | `var x: int = 42` |
| 推导 | `var name = value` | `var x = 42` |
| List | `List<T>`（T 为具体类型） | `var list: List<int> = [1, 2, 3]` |
| Tuple | `Tuple<T1, T2>` | `var t: Tuple<int, string> = (1, "a")` |
| Struct | `struct Name { fields }` | `struct Point { x: float, y: float }` |
| 函数类型 | `|T| -> R` | `var f: |int| -> int` |
| Lambda | `|params| -> R { body }` | `|x: int| -> int { x * 2 }` |
| 方法调用 | `obj.method()` | `list.push(4)` |
| Interface | `interface Name { methods }` | `interface Reader { read: \|self\| -> string }` |
| 实现 | `impl Interface for Type { }` | `impl Reader for File { ... }` |
| 方法 | `impl Type { methods }` | `impl Point { new: \|float, float\| -> Point { ... } }` |

---

## 实施路线图

### Phase 1：基础类型系统（2 周）

**目标**：类型标注、推导、检查可用

1. **新增 Token**：`->`
2. **类型表达式解析**：
   - `int`, `string`, `bool`, `float`
   - `List<T>`（T 必须是具体类型）
   - `Tuple<T1, T2>`
   - `|T| -> R` 函数类型
3. **变量声明扩展**：`var x: Type = value`
4. **Lambda 扩展**：`|x: T| -> R { }`
5. **类型检查器基础**：
   - 类型推导
   - 类型兼容性检查
   - 错误报告

**验收标准**：
```kaubo
var x: int = 42;
var add = |a: int, b: int| -> int { a + b };
var list: List<int> = [1, 2, 3];
// list.push("4");  // 编译错误：类型不匹配
```

---

### Phase 2：内置类型布局 + 方法分派（2 周）

**目标**：`list.push()` 编译为方法表索引调用

1. **对象头设计**：`ObjectHeader { type_id, flags }`
2. **全局类型表**：`TYPE_TABLE: [TypeDescriptor]`
3. **方法表生成**：
   - 为每个内置类型实例生成方法表
   - `List<int>`、`List<string>` 各自有独立方法表
4. **字节码指令**：
   - `LoadMethod(u8)` - 加载方法（编译期确定索引）
   - `CallMethod(u8)` - 调用方法
5. **编译期方法解析**：
   - `list.push(4)` -> `LoadMethod(0)`
   - `list.pop()` -> `LoadMethod(1)`

**验收标准**：
```kaubo
var list: List<int> = [1, 2, 3];
list.push(4);        // 字节码：LoadMethod(0); CallMethod(1)
var n = list.pop();  // 字节码：LoadMethod(1); CallMethod(0)
```

---

### Phase 3：Struct + Interface（3 周）

**目标**：用户自定义类型和接口多态

1. **Struct 定义和实例化**
2. **impl 块**：结构体方法（显式 self）
3. **Interface 定义**
4. **impl Interface for Type**：显式实现
5. **虚表生成**：
   - 为每个 (Type, Interface) 对生成虚表
6. **虚表分派**：
   - `reader.read()` -> `LoadVTableMethod(0); CallIndirect`

**验收标准**：
```kaubo
struct Point { x: float, y: float }
impl Point {
    new: |x: float, y: float| -> Point { Point { x, y } },
    distance: |self, other: Point| -> float { ... },
}

interface Drawable { draw: |self| -> void; }
impl Drawable for Point {
    draw: |self| -> void { ... },
}

fn render(item: Drawable) { item.draw(); }  // 虚表分派
render(Point::new(1.0, 2.0));
```

---

### Phase 4：泛型与 JIT（预留，暂不实现）

**目标**：设计完成，未来低代价实现

1. **预留语法解析**：`<T>` 解析但不通过类型检查
2. **预留数据结构**：
   - TypeDescriptor 添加 `is_generic`、`generic_params` 字段
   - MethodTable 添加 `generic_instances` 字段
3. **预留字节码指令**：
   - `LoadGeneric`、`CallGeneric` 等指令码预留，但不实现
4. **设计文档**：
   - JIT 编译策略
   - AOT 编译策略
   - 单态化 vs 共享代码策略

**暂不实现**：
- 用户定义泛型函数
- 用户定义泛型结构体
- JIT 编译器
- AOT 编译器

---

## 设计决策总结

| 决策 | 选择 | 理由 |
|------|------|------|
| 函数定义 | 只支持 Lambda | 简化语法，统一概念 |
| 函数语法 | `|T| -> R` | 与 Lambda 语法统一 |
| 列表 | `List<T>`（T 为具体类型） | 预留泛型语法，暂不实现用户泛型 |
| 数组 | ❌ 删除 `[T; N]` | 用 `List<T>` 替代，减少复杂度 |
| 借用 | ❌ 删除 `&` / `&mut` | 简化内存模型，值语义为主 |
| 元组 | `Tuple<T1, T2>` | 泛型语法统一，预留扩展 |
| 方法分派 | 对象头 + 方法表 | 为 Interface 虚表打基础，预留泛型方法缓存 |
| Interface | 显式 `impl` + 显式 `self` | Rust trait 风格，清晰明确 |
| 泛型 | **暂不实现**，但预留扩展点 | 现在成本高，未来迭代成本低 |
| JIT | **暂不实现**，预留设计 | Phase 4 再考虑 |
| 动态类型 | 仅 JSON | 限制动态特性，保证性能 |

---

## 关键预留扩展点

1. **TypeDescriptor.generic_params**：未来添加泛型参数信息
2. **MethodTable.generic_instances**：未来缓存泛型方法单态化版本
3. **OpCode 预留指令码**：未来添加泛型指令无需改现有指令
4. **类型 ID 分配策略**：预留范围给泛型实例化类型
