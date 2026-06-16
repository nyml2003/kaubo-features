# Kaubo v2 — 完整设计文档

> 状态: 讨论稿
> 目标: 编译器简洁高效，C ABI 友好，强类型全推断函数式脚本语言

---

## 零、设计原则

以下原则贯穿全部设计决策，优先级从高到低：

| # | 原则 | 含义 |
|---|------|------|
| 1 | **零隐式** | 无隐式 import、无隐式类型转换、无隐式捕获。所有行为在源码中可见 |
| 2 | **零 panic** | 所有错误通过 `Result<T, E>` 传递，不调用 `panic!()`/`unreachable!()`/`unwrap()`。WASM 兼容 |
| 3 | **一切皆表达式** | if/blocks/赋值都返回值。`;` 为分隔符无额外语义。block 最后一个表达式即返回值 |
| 4 | **VM 零内置** | VM 只负责执行。所有功能（print/type_of/assert/math）以模块形式接入，用户显式 import |
| 5 | **全推断无标注** | 类型标注完全可选。HM Algorithm W 实现 let-多态泛型。仅 C FFI 边界需要标注 |
| 6 | **DAG 依赖图** | 禁止递归和前向引用。调用图天生无环，tree-shaking 和调用栈分析极简化 |
| 7 | **编译期消解** | 值类型方法 (`42.as_float()`)、方法分发等在编译期直接得到最终结果，运行时零开销 |

---

## 目录

1. [设计约束](#一设计约束)
2. [语法](#二语法)
3. [类型系统 (HM)](#三类型系统-hm-algorithm-w)
    - [位类型方法 (编译期转写)](#位类型方法-编译期改写)
    - [类型擦除与泛型代码生成](#类型擦除与泛型代码生成)
    - [错误处理](#35-错误处理)
4. [CPS 变换](#四cps-变换)
5. [值表示 (分层方案)](#五值表示分层方案)
    - [内存管理](#内存管理)
6. [寄存器 VM](#六寄存器-vm)
    - [资源限制](#资源限制)
7. [async/await 运行时](#七asyncawait-运行时)
    - [并发设计预留](#并发设计预留)
8. [模块与包系统](#八模块与包系统)
    - [二进制格式规范](#二进制格式规范)
    - [标准库规范](#标准库规范)
9. [C 互操作](#九c-互操作)
10. [编译器架构](#十编译器架构)
11. [实施路线图](#十一实施路线图)
12. [与当前 v0.1.7 对比](#十二与当前-v017-对比)
13. [附录](#附录)

---

## 一、设计约束

| 决策 | 结论 | 理由 |
|------|------|------|
| 函数 | 只支持匿名 lambda，**不支持递归** | 语法统一，依赖图 DAG，调用栈可静态分析 |
| 绑定 | `const`(不可变) + `var`(可变) | C/JS/TS 直觉 |
| 类型 | HM Algorithm W 全推断 | 泛型自动获得，零标注 |
| 泛型 | 类型擦除 | VM 不用单态化 |
| 类型名 | `Int64` / `Float64` / `String` / `Bool` / `Null` | 首字母大写，语义精确 |
| 类型转换 | **禁止**隐式/显式类型转换 | 值类型方法 `42.as_float()` 编译期转写为寄存器指令 |
| 范式 | 一切皆表达式 | if/blocks/赋值都返回值，`;` 为分隔符无额外语义 |
| 赋值 | `var x = expr` / `x = new_val` 都返回 `null` | 禁止链式赋值，杜绝 `if (x=5)` footgun |
| 返回 | block 最后一个表达式即返回值，`return` 可选 | 表达式导向，无需强制写 return |
| 闭包 | 隐式捕获外层变量 | 暂先隐式，后续版本可能支持显式捕获列表 |
| 控制流 | CPS 变换为 block jump | break/continue/return 统一为 jump |
| VM | 寄存器式，32-bit 定长指令 | CPS block 天然寄存器语义 |
| 值表示 | 分层：类型特化寄存器 + BoxedValue 桥接 | 单态热路径零开销，多态自动装箱 |
| NaN-boxing | **删除** | 静态全推断不需要运行时 tag |
| 异步 | async/await (与 CPS 同构) | 替代协程 yield/resume |
| 模块 | JAR 式 .kaubop + ESM 式导入 | 自包含分发 + tree-shaking |
| C ABI | `.kauboi` 接口文件 + `kaubo.h` 嵌入 | C 实现作为特殊模块导入，类似 Python numpy |
| 错误处理 | 全 `Result<T, Error>` 传递，**零 panic** | WASM/Web 友好，不依赖 unwinding |

> **v2.1 收尾**: ADT + 模式匹配暂不收尾 v2.0 MVP，先做完整基础类型系统。

**删除清单：** NaN-boxing、operator 重载、elif、pass、pub、module 关键字、协程 yield/resume、递归 (`rec`)、`as` 类型转换、JSON 字面量、文件 I/O、`clone()`、`env()`/`now()`、kaubo-config crate、inline caching。**v2.0 暂不收尾:** ADT/match (留到 v2.1)、变长参数 (留到 v2.1)。

**铁律：零 panic。** 所有编译器/VM 错误通过 `Result<T, CompiledError>` / `Result<T, RuntimeError>` 传递，不调用 `panic!()`、`unreachable!()`、`unwrap()`。WASM 目标不支持栈展开，panic 等于致命终止。

---

## 二、语法

```js
// ── 绑定 ──
const pi = 3.14159;          // 不可变前缀
var counter = 0;              // 可变前缀
counter = counter + 1;        // 赋值表达式，返回 null

// 类型标注 (可选，仅需要时)
const flag: Bool = true;

// ── `;` 为分隔符，无额外语义 ──
// 函数体最后一个表达式即返回值，return 可选
const add = |a, b| { a + b };           // 最后表达式 → 返回 a+b
const log = |x| { print(x); null; };   // 显式返回 null

// ── 基本类型 ──
// Int64  Float64  String  Bool  Null

// ── 值类型方法 (编译期转写为寄存器指令，非真实方法调用) ──
42.as_float()                 // Int64 → Float64 (编译为 itof 指令)
3.14.as_int()                // Float64 → Int64 (编译为 ftoi 指令)
42.to_string()               // Int64 → String (编译为 itos 指令)
3.14.to_string()             // Float64 → String (编译为 ftos 指令)

// ── 函数 (唯一形式: 匿名 lambda) ──
const add = |a, b| { a + b };
const greet = |name| -> String { "Hello, " + name };

// ── 调用规则: 依赖图 DAG ──
const inc = |x| { x + 1 };
const inc_twice = |x| { inc(inc(x)) };  // ✅ 可调已定义的函数
// const self = |x| { self(x) };     // ❌ 编译错误: 不支持递归
// const fwd = |x| { g(x) };         // ❌ 编译错误: g 未定义

// ── 表达式控制流 ──
const abs = |x| { if x < 0 { -x } else { x } };

const result = {
    var x = compute();               // 声明 → null
    x = x + 10;                      // 赋值 → null
    x                                // block 最后一个表达式是返回值
};

// ── 结构体 ──
struct Point { x: Float64, y: Float64 }

impl Point {
    dist: |self, other| -> Float64 {
        const dx = self.x - other.x;
        const dy = self.y - other.y;
        return sqrt(dx * dx + dy * dy);
    },
}

const p1 = Point { x: 100.0, y: 200.0 };
const p2 = Point { x: 200.0, y: 100.0 };
print(p1.dist(p2));

// ── 循环 ──
var i = 0;
while i < 10 {
    print(i);
    i = i + 1;
}

const xs = [1, 2, 3];
for const x in xs { print(x) }

// ── 函数式组合 ──
const doubled =
    [1, 2, 3, 4, 5]
    |> map(|x| x * 2)
    |> filter(|x| x > 5)
    |> fold(0, add);

const process = trim >> to_upper >> replace("-", "_");

// ── 闭包 (隐式捕获外层变量) ──
const make_counter = |init| {
    var count = init;
    return || {
        count = count + 1;      // count 隐式捕获
        return count;
    };
};

// ── async/await ──
const fetch_user = async |id| {
    const resp = await http_get("/users/" + id.to_string());
    if resp.status != 200 {
        return { error: "fetch failed" };
    }
    return { body: resp.body };
};

// ── 模块 ──
import "std/prelude";       // 显式 import，无隐式
export const greet = |name| { "Hello, " + name };
export struct User { name: String, age: Int64 }

import { sin, cos } from "math";
import "math" as m;

// ── 注释 ──
// 单行注释
/* 块注释 */
```

### 字符串转义

```
\n  换行    \r  回车
\t  制表    \\  反斜杠
\"  双引号  \'  单引号
```

### 关键词 (23 个)

```
// 语法关键字 (19)
const   var     if      else    for     in      while
break   continue return  struct  impl    export  import
from    as      async   await   self
```

布尔字面量: `true` `false`
逻辑运算符: `not` `and` `or`

### 运算符优先级

| 优先级 | 运算符 |
|--------|--------|
| 最低 | `\|>` (pipe) |
| | `>>` (compose) |
| | `=` (赋值) |
| | `or` |
| | `and` |
| | `==` `!=` `>` `<` `>=` `<=` |
| | `+` `-` |
| | `*` `/` `%` |
| | `.` (成员访问) |
| 最高 | `not` (一元) |

---

## 三、类型系统 (HM Algorithm W)

### 核心数据结构

```rust
struct TypeVar(usize); // 唯一 ID，由全局计数器生成

enum Type {
    Var(TypeVar),                       // 未解类型变量
    Con(Name),                          // 构造器: Int64, Float64, String, Bool
    Arrow(Box<Type>, Box<Type>),        // 函数类型
    Record(StructId),                   // 具名结构体
    List(Box<Type>),                    // 列表
}

struct Scheme {                         // 多态类型方案
    bound: Vec<TypeVar>,                // ∀ 量化的变量
    body: Type,                         // 类型体
}

type Subst = HashMap<TypeVar, Type>;
type TypeEnv = HashMap<Name, Scheme>;
```

### 算法

```rust
fn infer(env: &TypeEnv, expr: &Expr) -> Result<(Subst, Type), TypeError>

fn unify(t1: &Type, t2: &Type) -> Result<Subst, TypeError>
    // Case: Var + anything -> generalize Var
    // Case: Con(name) + Con(name) -> Ok
    // Case: Arrow(a,b) + Arrow(c,d) -> unify(a,c) + unify(b,d)
    // Case: List(a) + List(b) -> unify(a,b)
    // Case: Record(id) + Record(id) -> Ok

fn generalize(env: &TypeEnv, ty: &Type) -> Scheme
    // 找出 ty 中不在 env 内的所有自由类型变量
    // 包装为 forall vars. ty

fn instantiate(scheme: &Scheme) -> Type
    // 为 bound 变量生成 fresh TypeVar
    // 替换 body 中的 bound 变量
```

### 关键规则

```
// Literal
  infer(env, IntLit(42)) -> (∅, Con("Int64"))

// VarRef (let-多态)
  infer(env, VarRef("f")) -> instantiate(env.lookup("f"))

// Lambda
  infer(env, Lambda(params, body)):
    fresh_vars = [new TypeVar() for _ in params]
    env' = env.extend(params -> fresh_vars)
    (s, ret_ty) = infer(env', body)
    return (s, Arrow(s(fresh_vars[0]), ...Arrow(s(fresh_vars[n]), ret_ty)))

// Const binding (let-多态入口)
  check_stmt(env, ConstDecl(name, rhs)):
    (s, ty) = infer(env, rhs)
    scheme = generalize(env.apply_subst(s), ty)
    env.insert(name, scheme)

// Function call
  infer(env, Call(func, args)):
    (s1, func_ty) = infer(env, func)
    (s2, arg_tys) = infer_all(env, args)
    ret_var = new TypeVar()
    s3 = unify(s2(func_ty), Arrow(arg_tys, ret_var))
    return (s3 ∘ s2 ∘ s1, ret_var)

// 调用约束 (编译期检查)
//   1. 禁止自递归: const f = |x| { ... f(x) }  → 编译错误
//   2. 禁止前向引用: const f = |x| { g(x) }; const g = ... → 错误
//   3. 仅允许调用已在作用域内(const 声明顺序在前)的函数
//   4. async 函数体中可使用 await；同步函数体中出现 await → CompileError::AwaitInSyncFunction
//   依赖图是 DAG，使 tree-shaking 和调用栈分析极简化

// let-多态限制:
//   仅 const 绑定支持 generalize (let-多态)，var 绑定为单态
//   const id = |x| { x };  →  ∀a. a → a   (泛型)
//   var id = |x| { x };    →  α → α (单态，不 generalize)
//   List<T> 底层固定为 Vec<BoxedValue>，即使 List<Int64> 元素仍装箱；
//   编译器在局部作用域内追踪元素类型可消除冗余 unbox

### 位类型方法 (编译期改写)

`Int64.as_float()`、`Float64.to_string()` 等**不是真实方法调用**。HM 推断时将这些调用识别为位类型内置操作，codegen 直接在寄存器间生成转换指令：

| Kaubo 写法 | HM 类型 | Codegen 指令 |
|-----------|---------|-------------|
| `42.as_float()` | `Int64 → Float64` | `itof r_dst, r_src` |
| `3.14.as_int()` | `Float64 → Int64` | `ftoi r_dst, r_src` |
| `x.to_string()` | `Int64 → String` | `itos r_dst, r_src` |
| `y.to_string()` | `Float64 → String` | `ftos r_dst, r_src` |
| `s.as_int()` | `String → Int64` | `stoi r_dst, r_src` |

HM 对这些方法调用不起泛型多态路径——在推断阶段直接识别 receiver 的 `Con` 类型和调用名，匹配到特化规则，生成对应的转换指令。零运行时开销。

### C 模块互操作 (.kauboi)

C 语言实现的高性能模块通过 `.kauboi` 接口文件导入——类似 Python 的 numpy（`.pyi` + `.so`）。Kaubo 侧不需要 `extern`、`dlopen` 等语法。

```
// mystats.kauboi (接口文件 —— 相当于 .pyi / .d.ts / .h)
export const fast_sort: |list: List(Int64)| -> List(Int64);
export const matrix_mul: |a: List(List(Float64)), b: List(List(Float64))| -> List(List(Float64));
export const PI: Float64;
```

```
mystats.kaubop              // 包结构
├── package.json
├── mystats.kauboi           // 导出类型声明
└── mystats.so               // C/Rust/Zig 编译产物
```

```
// Kaubo 侧 —— 和 import 普通模块完全一样
import { fast_sort, matrix_mul } from "mystats";
const result = fast_sort([3, 1, 2]);

import "mystats" as ms;
print(ms.PI);
```

**VM 侧：** 加载 `mystats.so` → `dlopen` + `dlsym("fast_sort")` → 用 `.kauboi` 里的类型签名做参数 marshalling → 调用。**语言语法零感知，全走模块系统。**

### 类型擦除与泛型代码生成

HM 推断后所有类型标注被擦除。代码生成根据调用点选择单态路径或装箱路径。

**单态化判定算法：**
```
for each Call(func, args):
    if func 的 HM 类型方案中有自由类型变量 (∀ quantified):
        if 所有实参的推断类型在调用点完全确定:
            → 单态化 (复制一份 bytecode，特化为具体类型)
        else:
            → 装箱 (所有参数 box 为 BoxedValue)
    else:
        → 类型特化路径 (已知类型，直接选 add_int/fadd 等)
```

```
// 单态调用 —— 零装箱
const int_id = |x: int| { x };
const r = int_id(42);          // HM 推倒 id 此处特化为 Int64 → Int64，直接 i64

// 高阶多态 —— 装箱
const twice = |f, x| { f(f(x)) };   // HM: ∀a b. (a→a) → a → a
const r = twice(|x| x + 1, 5);      // HM 可推倒全部实参 → 可能内联展开
const r = twice(some_poly_fn, val); // 无法内联 → 参数装箱为 BoxedValue
```

**List<T> 底层存储：**
列表在 VM 中统一存储为 `Vec<BoxedValue>`。单态上下文下 `List<Int64>` 的元素仍触 boxing（插入集合时），但编译器可以在局部作用域跟踪元素类型并消除冗余 unbox。

**类型断言：** Unbox 指令执行类型校验——BoxedValue 的 tag 必须匹配目标寄存器类型。不匹配时返回 `RuntimeError::TypeAssertion`（不是 panic）。

```rust
OP_UNBOX { dst, src } => {
    let (dst_type, boxed) = (reg_map[dst], &ptrs[src]);
    match (dst_type, boxed) {
        (Int, BoxedValue::Int(n)) => ints[dst] = n,
        (Float, BoxedValue::Float(f)) => floats[dst] = f,
        (Ptr, BoxedValue::String(s)) => { ptrs[dst] = ptrs[s]; retain(dst); }
        (Ptr, BoxedValue::Struct(s)) if s.def.id == expected_id => { ptrs[dst] = s; retain(dst); }
        _ => return Err(RuntimeError::TypeAssertion {
            expected: dst_type, got: boxed.tag()
        }),
    }
}
```

---
### 3.5 错误处理

**铁律：零 panic。** 全部错误通过 `Result<T, E>` 返回，WASM 兼容。

**编译期错误 (`Result<(), Vec<CompileError>>`)：**

```rust
enum CompileError {
    ParseError { pos: SourcePos, msg: String },
    TypeError { pos: SourcePos, expected: Type, got: Type },
    UnboundVariable { pos: SourcePos, name: String },
    ForwardReference { pos: SourcePos, name: String },
    SelfRecursion { pos: SourcePos, name: String },
    TooManyRegisters { func: String, count: usize },
    CycleDetected { names: Vec<String> },
}
```

HM 推断采用**错误恢复**模式——遇到类型错误不立即退出，而是为出错表达式注入一个 `Type::Var(fresh)` 继续检查剩余代码。最终收集所有错误一次性返回。

**运行时错误 (`Result<Reg, RuntimeError>`)：**

```rust
enum RuntimeError {
    StackOverflow,
    DivisionByZero,
    IndexOutOfBounds { index: i64, len: usize },
    NullAccess,
    TypeAssertion { expected: TypeTag, got: TypeTag },
    IoError { reason: String },
    ImportFailed { path: String, reason: String },
    Bug { msg: String },  // 编译器保证不应出现的情况
}

type TypeTag = u8;  // 0=Int64, 1=Float64, 2=String, 3=Bool, 4=List, ...
```

所有 VM 操作返回 `Result<(), RuntimeError>`：
```rust
fn div_int(ints: &mut [i64], dst: Reg, s1: Reg, s2: Reg) -> Result<(), RuntimeError> {
    if ints[s2] == 0 {
        return Err(RuntimeError::DivisionByZero);
    }
    ints[dst] = ints[s1] / ints[s2];
    Ok(())
}
```

**用户侧 (v2.0)：** 哨兵 `null`。调用失败时返回 `null`：
```js
const result = some_op(x);
if result == null { print("failed"); }
```

**v2.1 引入 `Result<T, E>` + ADT match：**
```js
match safe_div(a, b) {
    Ok val -> print(val),
    Err msg -> print("error: " + msg),
}
```

**C FFI 错误 marshalling：**
```c
#define KB_TAG_ERROR 0xFF
KbValue kb_error(const char* msg);
int kb_is_error(KbValue v);
const char* kb_error_message(KbValue v);
```

---

## 四、CPS 变换

### 中间表示

```rust
struct HirModule {
    functions: Vec<HirFunction>,
    constants: Vec<Constant>,
    structs: Vec<StructDef>,   // field layout
}

struct HirFunction {
    name: String,
    sig: Scheme,               // 类型方案 (保留用于多态代码生成)
    blocks: Vec<Block>,
    entry: BlockId,
    reg_count: usize,
}

struct Block {
    id: BlockId,
    params: Vec<Reg>,          // 块参数 (约束: HM 推导的寄存器)
    instrs: Vec<Instr>,
    term: Terminator,
}

enum Instr {
    BinOp(Reg, BinOp, Reg, Reg),
    UnOp(Reg, UnOp, Reg),
    LoadConst(Reg, usize),         // const pool index
    Move(Reg, Reg),
    NewStruct(Reg, StructId, Vec<Reg>),
    GetField(Reg, Reg, u16),
    SetField(Reg, Reg, u16, Reg),
    NewList(Reg, Vec<Reg>),
    IndexGet(Reg, Reg, Reg),
    IndexSet(Reg, Reg, Reg, Reg),
    Box(Reg, Reg),                        // 单态值 -> BoxedValue
    Unbox(Reg, Reg),                      // BoxedValue -> 单态值 (带类型断言)
    Nop,
}

enum BinOp {
    Add, Sub, Mul, Div, Mod,
    FAdd, FSub, FMul, FDiv,
    SAdd,                              // string concat
    Eq, Ne, Lt, Le, Gt, Ge,
    FEq, FNe, FLt, FLe, FGt, FGe,
}

enum UnOp { Neg, FNeg, Not }

enum Terminator {
    Jump(BlockId, Vec<Reg>),
    Branch(Reg, BlockId, Vec<Reg>, BlockId, Vec<Reg>),
    Return(Reg),
    Call(Reg, Vec<Reg>, BlockId),       // func, args, continuation block
    TailCall(Reg, Vec<Reg>),
    Suspend,                             // async yield (隐式 continuation 在 frame)
}
```

### 变换示例

```
源:
const f = |n| {
    var i = 0;
    while i < n {
        if i == 5 { break }
        if i == 3 { continue }
        print(i);
        i = i + 1;
    }
    return i;
};

经过 CPS:

block_0 [entry]:              // params: [n]
    i = 0
    jump(block_loop, i)

block_1 [loop_header]:        // params: [i]
    t0 = lt i, n
    branch(t0, block_body, block_exit, i)

block_2 [body]:
    t1 = eq i, 5
    branch(t1, block_exit, block_cf, i)    // break -> jump exit

block_3 [cf]:
    t2 = eq i, 3
    branch(t2, block_loop, block_body2, i)  // continue -> jump loop

block_4 [body2]:
    print(i)
    i2 = add i, 1
    jump(block_loop, i2)

block_5 [exit]:               // params: [i]
    return i
```

**break/continue/return 全部消失，只剩 jump 和 branch。**

---

## 五、值表示 (分层方案)

### 设计原则

HM 推断让编译期知道每个值的精确类型。90% 的代码是单态的——值走类型特化寄存器路径。仅泛型函数和集合操作走 BoxedValue。

```
类型特化寄存器 (热路径，零开销)
    int_regs:   Vec<i64>
    float_regs: Vec<f64>
    ptr_regs:   Vec<GcPtr>    // 堆对象 (string/list/struct/closure/ADT)
        │
        │ 多态边界: Box/Unbox 指令
        ▼
BoxedValue (冷路径，泛型桥接)
    enum BoxedValue {
        Int(i64),
        Float(f64),
        String(Gc<Str>),
        List(Gc<Vec<BoxedValue>>),
        Struct(Gc<StructObj>),
        Closure(Gc<ObjClosure>),
        Null,
    }
```

### 执行路径

```
// 单态函数
const add = |a, b| { a + b };        // HM 推断: Int64 -> Int64 -> Int64

// 代码生成:
//   add_int r2, r0, r1               <-- 直接 int_regs 运算

// 执行: i64 加法，零 tag 检查


// 泛型函数
const id = |x| { x };               // HM 推断: ∀a. a -> a

// 代码生成:
//   mov r0, r1                       <-- 任何类型走 BoxedValue

// 执行: BoxedValue 按位复制

// 列表操作
const f = |x| { [x, x + 1] };       // HM 推断: Int64 -> List(Int64)

// 代码生成:
//   box r1, r0                       <-- r1 = BoxedValue::Int(r0)
//   add_int r3, r0, 1
//   box r4, r3
//   newlist r2, [r1, r4]             <-- r2 = Gc<Vec<BoxedValue>>

// 执行: 装箱两次，分配列表
```

### 寄存器分配

```rust
struct RegFile {
    ints: Vec<i64>,
    floats: Vec<f64>,
    ptrs: Vec<GcPtr>,
}

struct RegAlloc {
    int_map: HashMap<VirtReg, usize>,    // 虚拟 reg -> int_regs 索引
    float_map: HashMap<VirtReg, usize>,
    ptr_map: HashMap<VirtReg, usize>,
}

// 代码生成时已知类型，直接选目标数组
fn emit_binop(dst: VirtReg, op: BinOp, src1: VirtReg, src2: VirtReg) {
    match op_type(dst) {
        RegType::Int   => { /* 生成 add_int 指令，操作 int_regs */ }
        RegType::Float => { /* 生成 fadd 指令，操作 float_regs */ }
        RegType::Ptr   => { /* Boxed 操作 */ }
    }
}
```

### 内存管理

```rust
// 引用计数 GC，只在堆对象上
// 单态寄存器 (int_regs, float_regs) 直接栈分配，零开销

struct Gc<T> { ptr: *mut T, rc: *mut u32 }

// Box/Unbox 指令自动处理 retain/release
// 编译器确保 ptr 正确增删引用计数
```

**循环引用处理：** RC 的天然缺陷。v2.0 策略：

- DAG 依赖图 + 不支持递归 → 函数级引用环不存在
- 自引用 struct（如 `struct Node { val: Int64, next: Node }`）产生数据级引用环 → **v2.0 不禁止，不检测，用户需手动置 `null` 断环**
- v2.1 补充 `weak T` 类型：`struct Node { val: Int64, next: weak Node }` 不增加 RC

**栈寄存器根扫描：** 调用帧退出时遍历当前帧的 `ptr_regs[ptr_base..ptr_base+ptr_count]`，对每个 `GcPtr` 执行 `release()`。release 到 0 时级联释放子对象。

**闭包 upvalue 生命周期：** upvalue 在捕获时 `retain()`。闭包对象 (`ObjClosure`) 释放时对其 upvalue 列表逐项 `release()`。闭包传递到外层作用域时 upvalue 保持存活。

**帧退出批量释放：**
```rust
fn pop_frame(vm: &mut VM, frame: &CallFrame) -> Result<(), RuntimeError> {
    for i in frame.ptr_base..frame.ptr_base + frame.ptr_count {
        vm.release(vm.ptrs[i])?;
    }
    Ok(())
}
```

### 对比当前 NaN-boxing 方案

| | NaN-boxing (v0.1.7) | 分层方案 (v2) |
|---|---|---|
| **单态 i64 运算** | 编码+解码位运算 (~5 条指令) | 直接 i64 运算 (1 条指令) |
| **整数范围** | 31-bit SMI | 64-bit |
| **unsafe 代码** | ~200 处 | ~10 处 |
| **Rust 类型安全** | u64 传送全流程 | 编译期保证 |
| **泛型函数** | NaN-boxed pointer fiddling | BoxedValue enum (safe) |
| **BoxedValue 大小** | 8 字节 | 16 字节 (tag + payload) |

---

## 六、寄存器 VM

### 指令编码

```
32-bit 固定宽度，4 类:

┌─ 6 ─┬─ 8 ─┬─ 9 ─┬─ 9 ─┐
│ op  │ dst │ src1 │ src2│   三寄存: add_int r5, r1, r2
└─────┴─────┴─────┴─────┘

┌─ 6 ─┬─ 8 ─┬─ 18 ──────┐
│ op  │ dst │  imm18     │   立即数: loadk r5, 42
└─────┴─────┴────────────┘

┌─ 6 ─┬─ 26 ─────────────┐
│ op  │   block_id/idx    │   跳转/大索引: jump block_5
└─────┴───────────────────┘

┌─ 6 ─┬─ 8 ─┬─ 8 ─┬─ 10 ─┐
│ op  │ reg │ reg  │ flags │   变体: branch reg, tb, fb
└─────┴─────┴─────┴───────┘
```

### 指令集 (44 条)

```
整数运算    add_int  sub_int  mul_int  div_int  mod_int  neg_int
浮点运算    fadd     fsub     fmul     fdiv     fneg
比较        eq_int   ne_int   lt_int   le_int   gt_int   ge_int
            feq      flt      fgt
字符串      sadd

转换指令    itof     ftoi     itos     ftos     stoi     // 位类型转换 (编译期改写)

数据移动    mov      loadk    load_const
堆分配      newstruct   newlist
字段访问    getfield    setfield
索引        indexget    indexset
逻辑        not

装箱/拆箱   box       unbox

控制流      jump      branch
调用        call      tailcall     ret
异步        await     suspend
```

总计: 44 条指令

### VM 结构

```rust
struct VM {
    ints: Vec<i64>,             // 整数寄存器
    floats: Vec<f64>,           // 浮点寄存器
    ptrs: Vec<GcPtr>,           // 堆对象寄存器
    reg_map: Vec<RegLoc>,       // 虚拟 reg -> 物理 reg 映射

    frames: Vec<CallFrame>,
    blocks: Vec<BlockPtr>,      // 块起始 IP 地址查找表
    consts: Vec<Constant>,      // 编译期常量池

    module_loader: ModuleLoader,
    async_scheduler: AsyncScheduler,
    ffi: FfiContext,
}

enum RegLoc { Int(usize), Float(usize), Ptr(usize) }

struct CallFrame {
    reg_base: usize,            // 虚拟 reg 偏移
    int_base: usize,            // int_regs 偏移
    float_base: usize,
    ptr_base: usize,
    ret_block: BlockId,
    ip: *const u32,
}
```

### 主循环 (核心)

```rust
fn execute(vm: &mut VM, entry: BlockId) -> Result<Reg, RuntimeError> {
    let mut ip = vm.blocks[entry];

    loop {
        let inst = unsafe { *ip };
        ip = ip.add(1);
        let op = (inst >> 26) as u8;

        match op {
            // ── 整数 ──
            OP_ADD_INT => {
                let (dst, s1, s2) = decode_rrr(inst);
                vm.ints[dst] = vm.ints[s1].wrapping_add(vm.ints[s2]);
            }

            // ── 比较 ──
            OP_EQ_INT => {
                let (dst, s1, s2) = decode_rrr(inst);
                vm.ints[dst] = (vm.ints[s1] == vm.ints[s2]) as i64;
            }

            // ── 移动 ──
            OP_MOV => {
                let (dst, src) = decode_rr(inst);
                match (vm.reg_map[dst], vm.reg_map[src]) {
                    (IntLoc(d), IntLoc(s)) => vm.ints[d] = vm.ints[s],
                    (FloatLoc(d), FloatLoc(s)) => vm.floats[d] = vm.floats[s],
                    (PtrLoc(d), PtrLoc(s)) => {
                        vm.ptrs[d] = vm.ptrs[s];
                        vm.retain(d);
                    },
                    _ => return Err(RuntimeError::Bug {
                        msg: format!("MOV type mismatch: {:?} -> {:?}", vm.reg_map[src], vm.reg_map[dst])
                    }),
                }
            }

            // ── Box/Unbox ──
            OP_BOX => {
                let (dst, src) = decode_rr(inst);
                vm.ptrs[dst] = box_value(vm, dst_reg, src_reg);
            }
            OP_UNBOX => {
                let (dst, src) = decode_rr(inst);
                let boxed = vm.ptrs[src].as_boxed();
                unbox_into(vm, boxed, dst);
            }

            // ── 控制流 ──
            OP_JUMP => {
                let block_id = (inst & 0x3FFFFFF) as usize;
                ip = vm.blocks[block_id];
            }
            OP_BRANCH => {
                let cond = ((inst >> 18) & 0xFF) as usize;
                let tb = ((inst >> 8) & 0x3FF) as usize;
                let fb = (inst & 0x3FF) as usize;
                ip = if vm.ints[cond] != 0 { vm.blocks[tb] } else { vm.blocks[fb] };
            }

            // ── 调用 ──
            OP_CALL => { push_frame(&mut vm.frames, ip, ret_block); ip = entry; }
            OP_TAILCALL => { /* 复用当前帧，jump 到新函数 entry */ }
            OP_RET => {
                if let Some(frame) = vm.frames.pop() {
                    ip = frame.ip;
                } else { return Ok(dst); }
            }

            // ── Async ──
            OP_AWAIT => {
                vm.ints[dst] = vm.async_scheduler.await_val(reg)?;
            }
            OP_SUSPEND => {
                vm.async_scheduler.register(current_frame);
                return Ok(Unit);
            }
        }
    }
}
```

**零栈操作。零控制流 opcode (jump/branch 只是 IP 赋值)。零 tag 检查 (单态路径)。** 零 panic，全部 Result 返回。

### 资源限制

寄存器上限 **编译期校验**，调用深度 **运行期校验**。

```rust
const MAX_REGISTERS: usize = 65536;    // u16 索引上限 (编译期)
const MAX_CALL_DEPTH: usize = 1024;    // 调用栈深度 (运行期)
const MAX_BLOCKS: usize = 65536;       // u16 block ID

fn allocate_reg(alloc: &mut RegAlloc, ty: RegType) -> Result<VirtReg, CompileError> {
    // 编译期检查
    if alloc.next_reg >= alloc.max_regs {
        return Err(CompileError::TooManyRegisters {
            func: alloc.current_func.clone(), count: alloc.next_reg
        });
    }
    // ...
}

fn push_frame(vm: &mut VM) -> Result<(), RuntimeError> {
    // 运行期检查
    if vm.frames.len() >= MAX_CALL_DEPTH {
        return Err(RuntimeError::StackOverflow);
    }
    vm.frames.push(frame);
    Ok(())
}
```

寄存器数组 (`ints`, `floats`, `ptrs`) 按 `Vec` 扩容，但每个函数帧的虚拟寄存器数在代码生成时已确定（线性扫描分配器计算）。帧内寄存器数超过上限时**编译期报错**，不静默。

---

## 七、async/await 运行时

CPS block 与 async/await 同构:

```rust
// async 函数 -> CPS blocks
// await -> Suspend terminator

// 源:
const fetch_key = async |key| {
    const val = await cache.get(key);
    return val;
};

// CPS:
// block_entry:
//   ... box args ...
//   call(cache.get, [key], block_after_await)
//   suspend()
//
// block_after_await:      // params: [val]
//   return val

// VM 执行:
// Suspend -> 保存帧到 AsyncScheduler -> 从 run loop 返回
// I/O done -> AsyncScheduler 把帧压回 VM -> 从 continuation block 恢复
```

```rust
struct AsyncScheduler {
    pending: VecDeque<TaskId>,
    tasks: HashMap<TaskId, SuspendedFrame>,
    io: IoPoller,                       // epoll/kqueue/IOCP
}

struct SuspendedFrame {
    frame: CallFrame,
    on_resolve: BlockId,
}
```

### 并发设计预留

v2.0 **不做**多线程，仅单线程事件循环。架构预留：

- `AsyncScheduler` 通过 `IoPoller` trait 抽象 I/O，可替换为多线程 work-stealing 调度器（不改变上层 API）
- `RegFile` 当前 `!Send`；多线程需每线程独立 `RegFile` + 共享堆
- GC：当前非原子 RC；多线程需 Atomic RC 或切换 Tracing GC
- `.kauboi` C 扩展可内部多线程（C 侧自己管理），Kaubo VM 不感知

```
// 预留: IoPoller trait 解耦 I/O 模型
trait IoPoller {
    fn register(&mut self, fd: i32, interest: Interest) -> TaskId;
    fn poll(&mut self, timeout: Duration) -> Vec<(TaskId, Ready)>;
}
```

---

## 八、模块与包系统

### 模块 = 文件

```
hello.kaubo          -> 模块名 "hello"
utils/math.kaubo     -> 模块名 "math" (在 "utils" 目录下)
```

### 包格式

```
math.kaubop  (ZIP archive)
├── package.json
│   {
│     "name": "math",
│     "version": "1.0.0",
│     "kaubo": ">=0.2.0",
│     "exports": { "./*": "./*.kaubor" },
│     "dependencies": {
│       "collections": "^1.0.0"
│     }
│   }
├── index.kaubor        # 主入口 (编译后字节码 + CPS blocks)
├── advanced.kaubor
├── types.kaubor.meta   # 导出类型签名 (供 HM 推断跨模块使用)
└── index.kaubor.map    # source map (可选)
```

### 编译流程

```
import { sin, cos } from "math";

1. Resolve: 查找 math -> 优先 *.kaubop，fallback 源码
2. Load: 解压 -> 加载 index.kaubor -> 读取 .meta 获取导出类型
3. Link: 只抽取 sin, cos 的 chunk (tree-shaking)
4. 未引用的函数/类型/变量不被打包
```

### Tree-shaking 算法

```
mark_reachable(root_module):
    q = [root_module]
    while q:
        m = q.pop()
        for import in m.imports:
            for sym in import.symbols:
                mark(sym)
                if is_function(sym):
                    walk_blocks(sym):  // CPS blocks: 追踪 call -> 标记 callee
                        for block in function.blocks:
                            for instr in block.instrs:
                                if is_call(instr):
                                    mark(instr.target)
                            if is_call(block.term):
                                mark(block.term.target)

emit(marked_symbols):
    // 只输出 marked 的 chunk 到输出文件
```

### 动态导入

```js
const math = import("math");   // -> Promise<Module>，基于 async/await
```

### 二进制格式规范

**`.kaubor` 字节码文件：**

```
Header (16 bytes):
  magic:    [u8; 4]   = [0x6B, 0x62, 0x6F, 0x72]  // "kbor"
  version:  u16        // 格式版本 (从 1 开始，不兼容递增)
  flags:    u16        // bit 0: debug, bit 1: compressed
  checksum: u32        // CRC32 of body
  body_len: u32        // 字节码体长度

Body (变长):
  const_pool:    u16 count + [Constant; count]
  structs:       u16 count + [StructDef; count]
  functions:     u16 count + [FunctionDef; count]
  blocks:        u16 count + [Block; count]
  exports:       u16 count + [(name: PascalString, type_sig: PascalString); count]
```

**版本兼容策略：** 主版本号不匹配 → 拒绝加载。`.kaubop` 中 `package.json` 通过 `"kaubo": ">=0.2.0 <0.3.0"` 约束 VM 版本。

**`.meta` 导出类型文件（纯文本）：**
```
greet: |String| -> String
PI: Float64
User: {name: String, age: Int64}
```

Tree-shaking 依赖 `.meta` 中的导出签名在编译期做死代码消除。

### 标准库规范

**VM 零内置。** 所有功能以模块形式接入，对用户 import 方式完全相同。

```
std/
├── prelude.kaubop    // print, type_of, assert, 位类型方法 (需显式 import)
├── math.kaubop       // sqrt, sin, cos, floor, ceil, PI, E
├── list.kaubor       // range, map, filter, fold, find, any, all
├── string.kaubor     // trim, to_upper, split, join, replace, substring, contains
├── io.kaubor         // print (Rust NativeFn) - 过渡期保留 print 语句
└── testing.kaubor    // assert (Rust NativeFn)
```

每个 `.kaubop` 可以是：
- **Rust 实现** — 编译器以 `NativeFn` 方式链接进二进制，加载时注册到模块
- **Kaubo 源码** — 编译为 `.kaubor` 字节码，正常加载
- **C 实现** — `.kauboi` 接口文件 + `.so`，运行时 dlopen

用户不感知实现语言。`import { sin } from "math"` 可能是 Rust、Kaubo 或 C——全看编译/打包时的元数据。

**prelude 模块：** 每个需 `print`/`type_of`/`assert` 的文件必须显式 import：
```js
import "std/prelude";          // 无隐式 import，用户自行导入
```
prelude 包含：
- `print(val)` — 输出到 stdout/WASM 回调
- `type_of(val) → String` — 运行时类型自省
- `assert(cond: Bool)` / `assert(cond: Bool, msg: String)` — 断言
- 位类型转换方法的 HM 特化规则

**print 过渡方案：** v2.0 期保留 `print expr;` 语句语法，编译器内部转写为对 `std::print(expr)` 的调用。v2.x 稳定后移除语句形式。

---

## 九、C 互操作

分为两个层面:
- **模块级**: `.kauboi` + `.so`，Kaubo 语法零感知 (详见第 3 节 C 模块互操作)
- **嵌入级**: `kaubo.h`，C 程序嵌入 Kaubo VM

### `.kauboi` 接口文件 (模块级 C 互操作)

类似 Python numpy 模型。用 C/Rust/Zig 写高性能模块 → 编译为 `.so` → 用 `.kauboi` 声明导出类型 → Kaubo 侧 `import` 使用。

```
mystats.kaubop
├── package.json
├── mystats.kauboi        // 导出类型声明 (语法同 Kaubo export，但只声明签名)
└── mystats.so            // C 编译产物
```

`.kauboi` 文件格式:
```
// 纯类型声明，无函数体。类似 .pyi / .d.ts / C header
export const fast_sort: |list: List(Int64)| -> List(Int64);
export const matrix_mul: |a: List(List(Float64)), b: List(List(Float64))| -> List(List(Float64));
export const PI: Float64;
```

导入方式和普通模块完全一致:
```js
import { fast_sort } from "mystats";
const result = fast_sort([3, 1, 2]);
```

### `kaubo.h` (嵌入级 C API)

C 程序将 Kaubo VM 作为嵌入脚本引擎使用。

```c
#include <stdint.h>

// ── Kaubo 值 ──
typedef struct {
    uint64_t tag;
    uint64_t payload;
} KbValue;

// 类型 tag
#define KB_TAG_I64    0
#define KB_TAG_F64    1
#define KB_TAG_STRING 2
#define KB_TAG_BOOL   3
#define KB_TAG_NULL   4

// 构造值
KbValue kb_i64(int64_t n);
KbValue kb_f64(double f);
KbValue kb_string(const char* s);
KbValue kb_bool(int b);
KbValue kb_null(void);

// 析取值
int      kb_is_i64(KbValue v);
int      kb_is_f64(KbValue v);
int      kb_is_string(KbValue v);
int64_t  kb_to_i64(KbValue v);
double   kb_to_f64(KbValue v);
const char* kb_to_cstring(KbValue v);

// VM
typedef uint64_t KbVm;
KbVm     kb_vm_new(void);
void     kb_vm_free(KbVm vm);

// 执行 Kaubo 代码
KbValue  kb_eval(KbVm vm, const char* source);
KbValue  kb_call(KbVm vm, const char* fn_name, int argc, ...);

// 注册 C 函数
typedef KbValue (*KbNativeFn)(KbValue* args, int argc);
void     kb_register(KbVm vm, const char* name, KbNativeFn fn);
```

### 嵌入示例

```c
#include "kaubo.h"

KbValue my_add(KbValue* args, int argc) {
    return kb_i64(kb_to_i64(args[0]) + kb_to_i64(args[1]));
}

int main() {
    KbVm vm = kb_vm_new();

    // 注册 C 函数
    kb_register(vm, "c_add", my_add);

    // 从 Kaubo 调用 C
    kb_eval(vm, "const result = c_add(1, 2);");
    KbValue r = kb_eval(vm, "result");
    printf("1 + 2 = %ld\n", kb_to_i64(r));

    // C 调用 Kaubo
    kb_eval(vm, "const add = |a, b| { a + b };");
    KbValue r2 = kb_call(vm, "add", kb_i64(3), kb_i64(4));
    printf("3 + 4 = %ld\n", kb_to_i64(r2));

    kb_vm_free(vm);
}
```

---

## 十、编译器架构

```
Source (.kaubo)
    │
    ▼
┌──────────────────────────────────────────┐
│ kaubo-syntax  (lexer + parser)          │
│ 递归下降 + Pratt 表达式解析               │
│ -> AST (Module)                         │
│ ~800 行                                  │
└────────────┬─────────────────────────────┘
             │ AST
             ▼
┌──────────────────────────────────────────┐
│ kaubo-infer  (HM 类型推断)               │
│ Algorithm W + record 核实                 │
│ -> TypedAST (每个节点带推断类型)           │
│ ~800 行 (v2.0 不收尾 ADT/match，大幅精简)  │
└────────────┬─────────────────────────────┘
             │ TypedAST
             ▼
┌──────────────────────────────────────────┐
│ CPS Transform  (控制流消解)              │
│ while/for/if/break/continue              │
│ -> 全部变成 block jump                    │
│ ~400 行                                  │
└────────────┬─────────────────────────────┘
             │ CPS blocks (带类型)
             ▼
┌──────────────────────────────────────────┐
│ kaubo-ir  (优化 + 代码生成)             │
│                                          │
│ optimization.rs:                         │
│   常量折叠  死块消除  内联  块合并          │
│   Box elimination                       │
│                                          │
│ codegen.rs:                             │
│   线性寄存器分配  类型特化指令选择          │
│   32-bit 定长编码                         │
│ -> Chunk (字节码)                        │
│ ~1500 行                                 │
└────────────┬─────────────────────────────┘
             │ Chunk
             ▼
┌──────────────────────────────────────────┐
│ kaubo-vm  (执行引擎)                     │
│   execute.rs    主循环 (块调度器)         │
│   regfile.rs    分层寄存器               │
│   gc.rs         引用计数                 │
│   module.rs     模块加载 + .kauboi C 互操作│
│   async.rs      async/await 调度         │
│   stdlib.rs     标准库                   │
│ ~4000 行                                 │
└────────────┬─────────────────────────────┘
             │
    ┌────────┴────────┐
    ▼                 ▼
kaubo-cli         kaubo-wasm
(~100 行)         (~400 行)
```

### 5 个 crate，单向依赖链

```
kaubo-syntax
    │
kaubo-infer
    │
kaubo-ir
    │
kaubo-vm
    │
    ├── kaubo-cli
    └── kaubo-wasm
```

---

## 十一、实施路线图 (8 周)

```
Phase 0  [2天]   清理
  · 删除 kaubo-config、kaubo-pipeline、lexer v2 死代码
  · 拆分 execution.rs 为多文件 (为后续重写做准备)
  · 统一 HIR 路径 (删除 AST->bytecode 直接路径)

Phase 1  [1周]   新语法
  · 新 Token 集 (17 关键字 + 运算符 + 定界符)
  · 递归下降 parser -> AST (表达式导向)
  · AST 定义: Module { exports, stmts } + 统一 Expr
  · 语义检查: 禁止自递归和未定义引用

Phase 2  [1周]   HM 类型推断  <-- 核心难点
  · TypeVar, Type, Scheme, Subst 数据结构
  · unify() + infer() + generalize() + instantiate()
  · Lambda / const / var 规则的 HM 扩展
  · Record 结构体字段类型检查
  · List 泛型列表类型
  · v2.0 不收尾 ADT/match (留到 v2.1)

Phase 3  [1周]   CPS Transform
  · AST -> CPS blocks 降级
  · 控制流消解: if/while/for -> block + jump/branch
  · break/continue -> jump 到对应 block
  · return -> jump(exit_block, val)
  · Lambda -> 单独 HirFunction + entry block

Phase 4  [1周]   优化 + 代码生成
  · 块级: 常辆折叠、死块消除、基本内联、块合并
  · Box elimination: 单态上下文中消除 box/unbox 对
  · 线性扫描寄存器分配
  · 类型特化指令选择 (add_int vs add_float vs boxed 路径)
  · 32-bit 定长编码输出

Phase 5  [1.5周] 寄存器 VM
  · RegFile (int_regs, float_regs, ptr_regs) + GcPtr
  · 块调度器主循环
  · 指令解码 (位操作) + dispatch (44 arm match)
  · 调用栈 (CallFrame) + 闭包 (Upvalue)
  · 引用计数 GC
  · 标准库端口 (print, math, string, list 核心操作)

Phase 6  [1周]   async/await + C 模块互操作
  · AsyncScheduler (事件循环 + 帧挂起/恢复)
  · await/suspend 指令实现
  · .kauboi 接口文件解析 + .so 加载 + 类型 marshalling
  · kaubo.h 嵌入 API

Phase 7  [1周]   模块系统
  · .kaubop ZIP 格式读写
  · 静态 import 解析 + tree-shaking
  · 跨模块类型签名 (.meta 文件)
  · 动态 import() 基于 async/await

Phase 8  [1周]   WASM + VSCode + 文档
  · kaubo-wasm 适配新语法
  · VSCode 语法高亮更新
  · 示例 + 语言文档
  · 测试套件
```

---

## 十二、与当前 v0.1.7 对比

| | v0.1.7 | v2 |
|---|---|---|
| **crate 数** | 9 | 5 |
| **代码量 (Rust)** | ~32,000 行 | 估计 ~14,000 行 |
| **unsafe 块** | ~300 | 估计 ~15 |
| **opcode 数** | 97 | 44 |
| **指令编码** | 变长 1-4 字节 | 固定 4 字节 |
| **执行模型** | 纯栈式 | 寄存器式 |
| **控制流 opcode** | 6 (含隐式 break/continue) | 0 (CPS 消解) |
| **值表示** | NaN-boxed u64 | 分层: int_regs + float_regs + ptr_regs + BoxedValue |
| **单态算术** | ~15 指令 (位运算 + tag check) | 1 条指令 |
| **整数范围** | 31-bit SMI | 64-bit |
| **类型系统** | 简单递归 walk，无泛型 | HM 全推断 + let-多态 |
| **泛型** | 不支持 | let-多态 (类型擦除) |
| **类型名** | int/float/string/bool | Int64/Float64/String/Bool |
| **类型转换** | `as` 关键字 | 值类型方法 (编译期转写) |
| **JSON 字面量** | `json { "k": v }` | 删除 (struct 字面量取代) |
| **文件 I/O** | read_file/write_file/... | 删除 (.kauboi C 覆盖) |
| **异步** | 协程 yield/resume (裸指针) | async/await (CPS 同构) |
| **C FFI** | 零 | .kauboi (模块级) + kaubo.h (嵌入级) |
| **模块** | 基础 import | .kaubop + tree-shaking + 动态 import |
| **inline caching** | 有 (operators) | 无 (类型特化替代) |
| **操作符重载** | 有 (impl operator) | 删除 |
| **ADT/模式匹配** | 无 | 无 (v2.1 收尾) |
| **递归** | 有 (`rec`) | 不支持 (依赖图 DAG) |
| **编译路径** | AST->bytecode + HIR 半成品 | 唯一: AST->TypedAST->CPS->HIR->Codegen |
| **编译器架构** | 裸指针 *mut Compiler | 安全 Rust，索引/arena |
| **VM 循环** | 1778 行单文件 match | 分模块，~300 行主循环 |

---

## 附录

### A. 关键词全集 (23 个)

```
// 语法关键字 (19)
const     不可变绑定
var       可变绑定
if else   条件表达式
for in    遍历循环
while     条件循环
break     退出循环
continue  跳过迭代
return    函数返回
struct    结构体定义
impl      实现块
export    模块导出
import    模块导入
from      命名导入
as        重命名导入
async     异步函数标记
await     等待异步值
self      方法接收者

// 布尔字面量 (2)
true      真
false     假

// 逻辑运算符 (3)
not       逻辑非
and       逻辑与
or        逻辑或
```

### B. 指令集全集

```
// 整数 (6)
add_int  sub_int  mul_int  div_int  mod_int  neg_int

// 浮点 (5)
fadd  fsub  fmul  fdiv  fneg

// 比较 (8)
eq_int  ne_int  lt_int  le_int  gt_int  ge_int
feq  flt

// 字符串 (1)
sadd

// 位类型转换 (5)
itof  ftoi  itos  ftos  stoi

// 逻辑 (1)
not

// 数据移动 (3)
mov  loadk  load_const

// 堆分配 (2)
newstruct  newlist

// 字段/索引 (4)
getfield  setfield  indexget  indexset

// 装箱桥接 (2)
box  unbox

// 控制流 (2)
jump  branch

// 调用 (3)
call  tailcall  ret

// 异步 (2)
await  suspend

总计: 44 条 (v2.0 MVP)
```

### C. 编译器流水线 (速查图)

```
Source (.kaubo)
    │
    ▼
kaubo-syntax ──> AST
    │
    ▼
kaubo-infer ──> TypedAST
    │
    ▼
CPS Transform ──> CPS blocks
    │
    ▼
kaubo-ir (opt + codegen) ──> Chunk (32-bit bytecode)
    │
    ▼
kaubo-vm (寄存器 VM)
    │
    ├── kaubo-cli
    └── kaubo-wasm
```
