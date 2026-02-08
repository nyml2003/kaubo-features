# 字节码后端设计方案

> 本文档记录 Kaubo 字节码 VM 的设计决策和实现细节。
> 
> **设计决策** (已确定):
> - 值表示: **NaN Boxing + SMI 优化**
> - 指令格式: **定长指令**
> - 字符串编码: **UTF-8**

---

## 1. 值表示 (NaN Boxing + SMI)

### 1.1 基本设计

利用 IEEE 754 double 的 NaN 空间存储非浮点值：

```
64-bit 布局:
[63] [62-52] [51-0]
  │    │       └── Payload (52 bits)
  │    └─────────── Exponent (11 bits)
  └──────────────── Sign (1 bit)

- 正常浮点数: Exponent != 0x7FF
- NaN:          Exponent == 0x7FF
  - Quiet NaN:   Payload highest bit = 1
  - Signaling:   Payload highest bit = 0
```

我们使用 **Quiet NaN** 的子类型来存储其他值：

```
Bit 63 = 1, Bits 62-52 = 0x7FF (Quiet NaN marker)
Bit 51 = 1 (我们的自定义标记，避免与标准 NaN 冲突)
Bits 50-48 = 类型标签 (3 bits)
Bits 47-0  = 值 (48 bits)
```

### 1.2 类型标签分配

| 类型 | 标签 (bits 50-48) | Payload 含义 | 范围/限制 |
|------|------------------|--------------|-----------|
| **SMI (小整数)** | `000` | 31-bit signed int | -2^30 ~ 2^30-1 |
| **Heap Object** | `001` | 对象指针 (48-bit) | 现代 OS 足够 |
| **特殊值** | `010` | 枚举: null, true, false, undefined | - |
| **保留** | `011` - `111` | 未来扩展 | - |

**特殊值编码** (bits 47-0):
- `0`: null
- `1`: true  
- `2`: false
- `3`: undefined (预留)

### 1.3 SMI (Small Integer) 优化

SMI 范围: **-1,073,741,824 ~ 1,073,741,823** (±10亿)

超出范围的整数自动装箱为 Heap Object (Int64)。

### 1.4 Rust 实现草图

```rust
/// NaN-boxed 值 (64-bit)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Value(u64);

const QNAN: u64 = 0x7FF8_0000_0000_0000;  // Quiet NaN 基础值
const TAG_SMI: u64 = 0;      // 000
const TAG_HEAP: u64 = 1;     // 001  
const TAG_SPECIAL: u64 = 2;  // 010

const SHIFT: u64 = 48;
const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

impl Value {
    // SMI 构造: SMI << 3 | TAG_SMI，然后或到 QNAN
    pub fn smi(n: i32) -> Self {
        let bits = QNAN | (TAG_SMI << SHIFT) | ((n as u64) & PAYLOAD_MASK);
        Self(bits)
    }
    
    // 浮点数: 直接存储 IEEE 754 表示
    pub fn float(f: f64) -> Self {
        Self(f.to_bits())
    }
    
    // 堆对象: 指针必须 8-byte 对齐 (低 3 位为 0)
    pub fn object<T>(ptr: *mut T) -> Self {
        let addr = ptr as u64;
        debug_assert!(addr & 0x7 == 0); // 对齐检查
        let bits = QNAN | (TAG_HEAP << SHIFT) | (addr >> 3);
        Self(bits)
    }
    
    // 类型判断
    pub fn is_float(&self) -> bool {
        (self.0 & 0x7FF0_0000_0000_0000) != 0x7FF0_0000_0000_0000
    }
    
    pub fn is_smi(&self) -> bool {
        self.0 & (0x7 << SHIFT) == QNAN | (TAG_SMI << SHIFT)
    }
    
    pub fn is_heap(&self) -> bool {
        self.0 & (0x7 << SHIFT) == QNAN | (TAG_HEAP << SHIFT)
    }
    
    // 解包
    pub fn as_smi(&self) -> Option<i32> {
        if self.is_smi() {
            Some((self.0 as i64) as i32) // 符号扩展
        } else {
            None
        }
    }
    
    pub fn as_float(&self) -> f64 {
        f64::from_bits(self.0)
    }
    
    pub fn as_object<T>(&self) -> Option<*mut T> {
        if self.is_heap() {
            Some(((self.0 & PAYLOAD_MASK) << 3) as *mut T)
        } else {
            None
        }
    }
}

// 特殊值常量
impl Value {
    pub const NULL: Value = Value(QNAN | (TAG_SPECIAL << SHIFT) | 0);
    pub const TRUE: Value = Value(QNAN | (TAG_SPECIAL << SHIFT) | 1);
    pub const FALSE: Value = Value(QNAN | (TAG_SPECIAL << SHIFT) | 2);
}
```

### 1.5 Heap Object 类型

```rust
/// 堆对象头
pub struct ObjHeader {
    pub ty: ObjType,      // 对象类型
    pub flags: u8,        // GC 标记等
    pub size: u32,        // 对象大小 (用于 GC)
}

pub enum ObjType {
    Int64,        // 超出 SMI 范围的大整数
    Float64,      // 需要装箱的浮点数 (极少见)
    String,       // UTF-8 字符串
    List,         // 动态数组
    Function,     // 函数字节码
    Closure,      // 闭包 (函数 + 捕获环境)
    Class,        // 类定义 (未来)
    Instance,     // 类实例 (未来)
}

pub struct ObjString {
    pub header: ObjHeader,
    pub len: usize,
    pub chars: [u8],  // Flexible array member
}

pub struct ObjList {
    pub header: ObjHeader,
    pub capacity: usize,
    pub len: usize,
    pub items: [Value],  // Flexible array
}

pub struct ObjFunction {
    pub header: ObjHeader,
    pub arity: u8,           // 参数个数
    pub upvalue_count: u8,   // 捕获变量数
    pub chunk: Chunk,        // 字节码块
    pub name: *mut ObjString, // 函数名
}
```

---

## 2. 指令集架构

### 2.1 指令格式

**定长指令: 1-3 bytes**

```
┌─────────┬─────────┬─────────┐
│ Opcode  │ Op1     │ Op2     │
│ 1 byte  │ 1 byte  │ 1 byte  │
└─────────┴─────────┴─────────┘

- 无操作数指令: 1 byte
- 单操作数指令: 2 bytes
- 双操作数指令: 3 bytes
```

### 2.2 Opcode 定义

```rust
#[repr(u8)]
pub enum OpCode {
    // ===== 常量加载 (0x00-0x0F) =====
    LoadConst0 = 0x00,    // 加载常量池第 0 项
    LoadConst1,           // 1
    ...
    LoadConst15,          // 15 (常用常量内联优化)
    LoadConst,            // 0x10 + u8 索引
    LoadConstWide,        // 0x11 + u16 索引 (常量池 > 256)
    
    LoadNull = 0x18,      // null
    LoadTrue,             // true
    LoadFalse,            // false
    LoadZero,             // SMI 0 (优化)
    LoadOne,              // SMI 1 (优化)
    
    // ===== 栈操作 (0x20-0x2F) =====
    Pop = 0x20,           // 弹出栈顶
    Dup,                  // 复制栈顶
    DupTop2,              // 复制栈顶两个值
    Swap,                 // 交换栈顶两个
    SwapTop3,             // 循环交换栈顶三个: abc -> bca
    
    // ===== 局部变量 (0x30-0x3F) =====
    // 局部变量使用寄存器式访问，前 16 个内联优化
    LoadLocal0 = 0x30,    // 加载局部变量 0
    LoadLocal1,
    ...
    LoadLocal15,
    LoadLocal,            // 0x40 + u8 索引
    LoadLocalWide,        // 0x41 + u16 索引
    
    StoreLocal0 = 0x48,   // 存储到局部变量 0
    StoreLocal1,
    ...
    StoreLocal15,
    StoreLocal,
    StoreLocalWide,
    
    // ===== 全局变量 (0x58-0x5F) =====
    LoadGlobal = 0x58,    // u8 全局变量索引
    LoadGlobalWide,       // u16
    StoreGlobal,
    StoreGlobalWide,
    DefineGlobal,         // 定义新全局变量
    
    // ===== 算术运算 (0x60-0x6F) =====
    // 二元运算: 弹出两个操作数，压入结果
    Add = 0x60,           // +
    Sub,                  // -
    Mul,                  // *
    Div,                  // /
    Mod,                  // %
    Pow,                  // ** (幂运算)
    
    // 一元运算
    Neg = 0x68,           // 取负
    Inc,                  // ++ (前缀)
    Dec,                  // -- (前缀)
    
    // ===== 比较运算 (0x70-0x77) =====
    Equal = 0x70,         // ==
    NotEqual,             // !=
    Greater,              // >
    GreaterEqual,         // >=
    Less,                 // <
    LessEqual,            // <=
    Is,                   // 同一性比较
    
    // ===== 逻辑运算 (0x78-0x7B) =====
    Not = 0x78,           // !
    And,                  // 逻辑与 (短路)
    Or,                   // 逻辑或 (短路)
    
    // ===== 控制流 (0x80-0x8F) =====
    Jump = 0x80,          // i16 偏移 (有符号)
    JumpIfFalse,          // 条件跳转 (i16)
    JumpIfTrue,
    JumpBack,             // 负向跳转专用 (循环优化)
    
    // ===== 函数调用 (0x90-0x9F) =====
    Call = 0x90,          // u8 参数个数
    Call0, Call1, Call2,  // 0-3 参数优化
    Call3,
    TailCall,             // 尾调用优化
    Return,               // 返回
    ReturnValue,          // 带返回值
    
    // ===== 闭包 (0xA0-0xAF) =====
    Closure = 0xA0,       // u8 函数常量索引 + upvalue 表
    GetUpvalue,
    SetUpvalue,
    CloseUpvalue,         // 关闭 open upvalue
    
    // ===== 列表操作 (0xB0-0xBF) =====
    BuildList = 0xB0,     // u8 元素个数
    BuildList0,           // []
    BuildList1,           // [a]
    BuildList2,
    BuildList3,
    IndexGet,             // list[index]
    IndexSet,             // list[index] = value
    IndexDelete,          // del list[index]
    ListAppend,           // list.append(value)
    ListLen,              // len(list)
    
    // ===== 对象/属性 (0xC0-0xCF) =====
    GetField = 0xC0,      // u8 字段索引 (编译期确定)
    GetFieldWide,         // u16
    SetField,             // obj.field = value
    SetFieldWide,
    MethodCall,           // obj.method() 优化
    
    // ===== 其他 (0xF0-0xFF) =====
    Print = 0xF0,         // 调试用打印
    Assert,               // 断言
    Breakpoint,           // 调试断点
    Invalid = 0xFF,       // 非法指令
}
```

### 2.3 指令编码示例

```rust
/// 字节码块
pub struct Chunk {
    pub code: Vec<u8>,         // 指令字节
    pub constants: Vec<Value>, // 常量池
    pub lines: Vec<usize>,     // 行号信息 (用于调试)
}

impl Chunk {
    /// 写入单字节指令
    pub fn write_op(&mut self, op: OpCode, line: usize) {
        self.code.push(op as u8);
        self.lines.push(line);
    }
    
    /// 写入带 u8 操作数的指令
    pub fn write_op_u8(&mut self, op: OpCode, operand: u8, line: usize) {
        self.code.push(op as u8);
        self.code.push(operand);
        self.lines.push(line);
        self.lines.push(line);
    }
    
    /// 写入带 i16 操作数的指令 (跳转用)
    pub fn write_jump(&mut self, op: OpCode, offset: i16, line: usize) {
        self.code.push(op as u8);
        self.code.extend_from_slice(&offset.to_le_bytes());
        self.lines.push(line);
        self.lines.push(line);
        self.lines.push(line);
    }
    
    /// 添加常量，返回索引
    pub fn add_constant(&mut self, value: Value) -> usize {
        let idx = self.constants.len();
        self.constants.push(value);
        idx
    }
}
```

---

## 3. 虚拟机架构

### 3.1 栈帧结构

```
┌─────────────────────────────┐ ← Stack Top
│          操作数栈            │
│    (函数执行期间动态增长)      │
├─────────────────────────────┤
│      局部变量 (寄存器区)       │
│    (编译期确定大小)            │
├─────────────────────────────┤
│      返回地址 (IP 保存)        │
├─────────────────────────────┤
│      函数对象引用             │
├─────────────────────────────┤
│      调用者的栈基址 (FP)       │ ← Frame Pointer
└─────────────────────────────┘
```

### 3.2 VM 状态

```rust
pub struct VM {
    // 执行状态
    ip: *const u8,              // 指令指针
    stack: Vec<Value>,          // 值栈
    fp: usize,                  // 当前栈帧基址
    
    // 全局状态
    globals: HashMap<String, Value>,
    
    // 堆/GC
    heap: Heap,
    
    // 开放 upvalue 链 (用于闭包)
    open_upvalues: *mut ObjUpvalue,
}

pub struct ObjUpvalue {
    pub location: *mut Value,    // 指向栈上的值
    pub closed: Value,           // 关闭后的值
    pub next: *mut ObjUpvalue,   // 链表
}
```

### 3.3 主执行循环 (草图)

```rust
impl VM {
    pub fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        use OpCode::*;
        
        loop {
            let instruction = unsafe { *self.ip };
            self.ip = self.ip.add(1);
            
            match unsafe { std::mem::transmute::<u8, OpCode>(instruction) } {
                LoadConst0 => self.push(chunk.constants[0]),
                LoadConst1 => self.push(chunk.constants[1]),
                // ... LoadConst15
                
                LoadConst => {
                    let idx = unsafe { *self.ip } as usize;
                    self.ip = self.ip.add(1);
                    self.push(chunk.constants[idx]);
                }
                
                LoadNull => self.push(Value::NULL),
                LoadTrue => self.push(Value::TRUE),
                LoadFalse => self.push(Value::FALSE),
                
                Pop => { self.pop(); }
                Dup => {
                    let v = self.peek(0);
                    self.push(v);
                }
                
                Add => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(self.add_values(a, b)?);
                }
                
                JumpIfFalse => {
                    let offset = unsafe { 
                        i16::from_le_bytes([*self.ip, *self.ip.add(1)]) 
                    } as isize;
                    self.ip = self.ip.add(2);
                    
                    if self.peek(0).is_falsey() {
                        self.ip = self.ip.offset(offset);
                    }
                }
                
                Call => {
                    let arg_count = unsafe { *self.ip };
                    self.ip = self.ip.add(1);
                    self.call_value(arg_count)?;
                }
                
                Return => return InterpretResult::Ok,
                
                Invalid => return InterpretResult::RuntimeError("Invalid opcode"),
                _ => return InterpretResult::RuntimeError("Unknown opcode"),
            }
        }
    }
    
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
    
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }
    
    fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance]
    }
}
```

---

## 4. AST → Bytecode 编译

### 4.1 编译器结构

```rust
pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,      // 局部变量表
    scope_depth: usize,      // 当前作用域深度
    function: *mut ObjFunction,
    function_type: FunctionType,
}

pub struct Local {
    pub name: String,
    pub depth: usize,
    pub is_captured: bool,   // 是否被闭包捕获
}

impl Compiler {
    /// 编译表达式
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr.as_ref() {
            ExprKind::LiteralInt(lit) => {
                // 尝试 SMI，否则装箱
                let value = if let Ok(n) = i32::try_from(lit.value) {
                    Value::smi(n)
                } else {
                    // 大整数装箱
                    let obj = self.alloc_int64(lit.value);
                    Value::object(obj)
                };
                let idx = self.chunk.add_constant(value);
                self.emit_constant(idx);
            }
            
            ExprKind::Binary(bin) => {
                self.compile_expr(&bin.left)?;
                self.compile_expr(&bin.right)?;
                
                let op = match bin.op {
                    KauboTokenKind::Plus => OpCode::Add,
                    KauboTokenKind::Minus => OpCode::Sub,
                    KauboTokenKind::Asterisk => OpCode::Mul,
                    KauboTokenKind::Slash => OpCode::Div,
                    // ... 其他运算符
                    _ => return Err(CompileError::InvalidOperator),
                };
                self.emit_op(op);
            }
            
            ExprKind::VarRef(var) => {
                if let Some(local_idx) = self.resolve_local(&var.name) {
                    self.emit_op_u8(OpCode::LoadLocal, local_idx as u8);
                } else {
                    // 全局变量
                    let name = self.chunk.add_constant(Value::string(&var.name));
                    self.emit_op_u8(OpCode::LoadGlobal, name as u8);
                }
            }
            
            ExprKind::Assign(assign) => {
                self.compile_expr(&assign.value)?;
                // ... 存储到变量
            }
            
            // ... 其他表达式类型
        }
        Ok(())
    }
    
    /// 编译语句
    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt.as_ref() {
            StmtKind::Expr(expr) => {
                self.compile_expr(&expr.expression)?;
                self.emit_op(OpCode::Pop); // 表达式结果丢弃
            }
            
            StmtKind::VarDecl(decl) => {
                self.compile_expr(&decl.initializer)?;
                // 添加到局部变量表或生成全局定义指令
            }
            
            StmtKind::If(if_stmt) => {
                self.compile_expr(&if_stmt.if_condition)?;
                
                let then_jump = self.emit_jump(OpCode::JumpIfFalse);
                self.compile_block(&if_stmt.then_body)?;
                
                let else_jump = self.emit_jump(OpCode::Jump);
                self.patch_jump(then_jump);
                
                if let Some(else_body) = &if_stmt.else_body {
                    self.compile_block(else_body)?;
                }
                self.patch_jump(else_jump);
            }
            
            StmtKind::While(while_stmt) => {
                let loop_start = self.chunk.code.len();
                
                self.compile_expr(&while_stmt.condition)?;
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
                
                self.compile_block(&while_stmt.body)?;
                self.emit_loop(loop_start);
                
                self.patch_jump(exit_jump);
            }
            
            // ... 其他语句类型
        }
        Ok(())
    }
}
```

---

## 5. 类型支持与扩展规划

### 5.1 当前前端支持的类型

| 类型 | 语法 | 字节码支持 | 备注 |
|------|------|-----------|------|
| **SMI 整数** | `42` | ✅ 立即数 | -10亿 ~ +10亿 范围 |
| **大整数** | (超出范围自动装箱) | ✅ Heap Int64 | SMI 超限时自动降级 |
| **浮点数** | `3.14` | ⚠️ 预留 | 前端暂未实现词法分析 |
| **字符串** | `"hello"` | ✅ Heap String | UTF-8 编码 |
| **布尔** | `true`/`false` | ✅ 立即数 | Special 值编码 |
| **Null** | `null` | ✅ 立即数 | Special 值编码 |
| **列表** | `[1, 2, 3]` | ✅ Heap List | 支持 BuildList/IndexGet |
| **函数** | `\|x\| { ... }` | ✅ Heap Function | 字节码块 + 元信息 |
| **闭包** | (自动装箱) | ⚠️ 预留 | 需要 upvalue 机制 |
| **对象/类** | `obj.field` | ⚠️ 预留 | 仅成员访问，无类定义 |

### 5.2 扩展接口设计

为支持未来类型扩展，保留以下机制：

```rust
// ObjType 预留标签 (3 bits 目前只用 0-3)
pub enum ObjType {
    // 当前使用
    Int64 = 0,        // 0b000
    String = 1,       // 0b001
    List = 2,         // 0b010
    Function = 3,     // 0b011
    
    // 预留扩展 (4-7)
    Closure = 4,      // 0b100 - 闭包支持
    Class = 5,        // 0b101 - 面向对象
    Instance = 6,     // 0b110 - 类实例
    Reserved = 7,     // 0b111 - 未来使用
}

// Opcode 预留空间
// 0xD0-0xEF: 对象/类相关指令 (预留)
// 0xFC-0xFE: 扩展指令前缀
// 0xFF: Invalid (保持为非法指令用于调试)
```

**不阻塞核心功能的扩展**:
- `IndexSet` 指令: 先实现为 `todo!()`，不影响列表读取
- `SetField` 指令: 同样预留，前期只做成员读取
- 闭包 upvalue: VM 结构预留字段，前期不使用

---

## 6. 渐进式实现策略

**原则**: 先跑通整体，再逐步完善。

### Phase 2.1: 核心 Value + 算术 (MVP) ✅ 已完成
**目标**: 能执行 `1 + 2 * 3`

```rust
// 实现范围
- Value: SMI + Float64 + Special (null/true/false) ✅
- 指令: LoadConst, LoadNull/True/False, Add/Sub/Mul/Div, Pop, Return ✅
- VM: 栈操作 + 主循环 ✅
- 编译器: 字面量 + 二元运算 ✅
```

**已实现功能**:
- NaN Boxing Value 表示 (SMI + Float + Special)
- 基础算术指令 (Add/Sub/Mul/Div/Neg)
- 比较指令 (Equal/Greater/Less)
- 跳转指令 (Jump/JumpIfFalse/JumpBack)
- AST → Bytecode 编译器 (字面量、二元运算、一元运算)
- End-to-End 测试通过: `return 1 + 2 * 3;` → `7`

### Phase 2.2: 变量与控制流 🚧 进行中
**目标**: 能执行 `var x = 5; if (x > 0) { return x; }`

```rust
// 新增实现
- 局部变量: LoadLocal/StoreLocal (前 16 个)
- 全局变量: 简单 HashMap 支持
- 比较: Equal/Greater/Less (已有)
- 跳转: Jump/JumpIfFalse (已有)
- 控制流: if/else, while 循环 (编译器待实现)
```

### Phase 2.3: 函数与列表
**目标**: 能执行 `var f = \|x\| { return x + 1; }; f(5);`

```rust
// 新增实现
- 函数: Call/Return, 栈帧管理
- 列表: BuildList, IndexGet
- 字符串: 基础操作
```

### Phase 2.4: 闭包与完善
**目标**: 完整支持当前 AST 的所有特性

```rust
// 新增实现
- 闭包: Closure/GetUpvalue/SetUpvalue
- IndexSet, SetField (前端语法需同步实现)
- 错误处理: 堆栈追踪
```

### Phase 2.5: 优化与扩展
**目标**: 性能优化 + Phase 3 新类型支持

```rust
// 可选优化
- 指令缓存/内联缓存
- GC 实现
- Float 字面量支持 (前端)
- 类/对象系统
```

---

## 7. 当前状态 (Phase 2.1 已完成)

### 已实现的文件

| 文件 | 功能 | 测试状态 |
|------|------|---------|
| `runtime/value.rs` | NaN boxing, SMI, Float, Special | ✅ 9 个测试通过 |
| `runtime/bytecode/mod.rs` | OpCode 枚举定义 | ✅ 3 个测试通过 |
| `runtime/bytecode/chunk.rs` | Chunk 结构, 写入方法, 反汇编 | ✅ 4 个测试通过 |
| `runtime/vm.rs` | 栈 + 主循环 + 核心指令 | ✅ 8 个测试通过 |
| `runtime/compiler.rs` | 字面量 + 二元运算 + return | ✅ 10 个测试通过 |

### 已实现的 OpCode (共 25 个)

| 类别 | 已实现 |
|------|--------|
| 常量加载 | LoadConst0-15, LoadConst, LoadNull, LoadTrue, LoadFalse, LoadZero, LoadOne |
| 栈操作 | Pop, Dup, Swap |
| 算术运算 | Add, Sub, Mul, Div, Neg |
| 比较运算 | Equal, Greater, Less |
| 控制流 | Jump, JumpIfFalse, JumpBack |
| 函数 | Return, ReturnValue |
| 调试 | Print |

### End-to-End 测试示例

```rust
// tests/runtime/compiler.rs
#[test]
fn test_run_complex() {
    // 1 + 2 * 3 = 7
    let result = run_code("return 1 + 2 * 3;").unwrap();
    assert_eq!(result.as_smi(), Some(7));
}

#[test]
fn test_run_division() {
    // 5 / 2 = 2.5
    let result = run_code("return 5 / 2;").unwrap();
    assert!(result.is_float());
    assert_eq!(result.as_float(), 2.5);
}

#[test]
fn test_run_comparison() {
    let result = run_code("return 2 > 1;").unwrap();
    assert!(result.is_true());
}
```

### 下一步任务 (Phase 2.2)

| 优先级 | 任务 | 说明 |
|--------|------|------|
| 🔴 高 | 局部变量 | LoadLocal/StoreLocal 指令实现 |
| 🔴 高 | 变量编译 | 编译器支持 VarRef/VarDecl |
| 🟡 中 | 全局变量 | HashMap 存储全局变量 |
| 🟡 中 | 控制流编译 | if/while/for 语句编译 |
| 🟢 低 | 逻辑运算 | And/Or 短路求值 |

---

*最后更新: 2026-02-08*
*设计方案: NaN Boxing + SMI + 定长指令 + UTF-8*
*实现策略: 渐进式，先 MVP 再完善*

---

*最后更新: 2026-02-08*
*设计方案: NaN Boxing + SMI + 定长指令 + UTF-8*
