# Kaubo VM 分析报告

> **注意**：本文档的部分问题已在 v2.x 修复。已修复项标注 ✅。

## 一、当前问题汇总

### 编译期类型漏洞（P0）

| # | 问题 | 位置 | 状态 |
|---|------|------|------|
| 1 | 比较运算符 (`==` `!=` `<` `>` `<=` `>=`) 不做类型 unify——`1 == 2.0` 编译期不报错 | infer.rs | ✅ 已修复 |
| 2 | `and`/`or` 同理不做 unify，且 CPS lowering 未实现 | infer.rs | 待修 |
| 3 | CPS Build 靠 `ValueHint::is_float()` 猜类型选 opcode，猜错读错寄存器组 | cps_build.rs | ✅ 已修复（类型导向分派） |

### VM 运行时 bug（P0）

| # | 问题 | 位置 | 状态 |
|---|------|------|------|
| 4 | 0x3A 双重用途——`GetVariantTag` 和 `Box` 共享 opcode | execute.rs | ✅ 已修复 |
| 5 | execute 入口只 resize ints 忘 resize floats | execute.rs | ✅ 已修复（统一寄存器组） |
| 6 | bind_params / block_ip 无边界检查 | execute.rs | 待修 |

### 值表示缺陷 —— ints[] + floats[] 双数组（P1）✅ 已修复

**旧方案**：`RegFile { ints: Vec<i64>, floats: Vec<f64> }`。每次 `write_int` 只写 ints，
`write_float` 写 floats 同时覆盖 ints 为 bitcast，`write_bool` 写 ints=0/1 并写 floats=0.0/1.0。
三个函数互相覆盖，语义混乱。

**新方案**：统一寄存器组 `regs: Vec<u64>`（JVM 风格）。操作码决定值的解释方式。详见第三节设计决策。

### 类型信息断层（P1）

| # | 问题 |
|---|------|
| 7 | infer 算出了操作数的精确类型（`t1 = Int64`、`t2 = Float64`），但 CPS Build 拿不到——中间只有 `ValueHint` 猜测 |
| 8 | `to_string()` / `to_float()` 硬编码在 infer 和 cps_build 两处 |
| 9 | stdlib 函数索引硬编码（`sqrt=3 sin=4 cos=5 floor=6 ceil=7`） |

### GC 问题

| # | 问题 | 说明 |
|---|------|------|
| 10 | RC 无循环检测 | closure upvalues 是 copied value，循环引用直接泄漏 |
| 11 | dummy slot 泄漏 | `VM::new()` 分配 dummy String 占位 slot 0，永不释放 |
| 12 | SetField 对 Variant 无 GC retain/release | 内存泄漏 |

### 编码问题

| # | 问题 | 说明 |
|---|------|------|
| 13 | 32-bit 定长编码已出现约束 | NewVariant tag 只 8bit，Branch fb 只 8bit |
| 14 | Branch/Jump 目标位宽不一致 | Call 17bit vs Branch 8bit |
| 15 | TailCall 实现不完整 | 不传参，不设当前函数 |
| 16 | Suspend 丢 ret_block | 恢复后不知道回哪 |

### 调用约定

| # | 问题 | 说明 |
|---|------|------|
| 17 | 每次 Call clone 全部寄存器到 CallFrame | 512+256 个值，即使只用 3 个 |
| 18 | Native 调用的结果固定写 reg 0 | 调用方无法选择结果寄存器 |

---

## 二、业界对照：类型在指令里 vs 类型在值里

所有语言 VM 的值表示分三派：

**派别 1：类型在指令里（JVM / WASM / .NET CLR）**
- JVM: `iadd` vs `fadd` vs `dadd`——操作码区分类型，局部变量表是统一的一排 slot
- WASM: `i32.add` vs `f64.add`——同样操作码区分，栈上值无 tag
- 存储是裸位，指令决定解释方式

**派别 2：类型在值里（Lua / JS / Python / Ruby）**
- LuaJIT: NaN-boxing，单 u64 承载所有类型
- Python: 所有值都是 PyObject*，ob_type 字段区分
- V8: Smi（低 bit=1）vs HeapObject（低 bit=0）
- 需要是因为动态类型——`+` 一条指令要处理多种类型

**派别 3：编译期全消去（Go / Rust / Swift）**
- 编译到原生代码，类型信息完全消去
- 栈上就是几个字节，编译器在生成代码时已确定类型

**Kaubo 属于派别 1**。`AddInt` 和 `FAdd` 已经是不同的 opcode。
CPython 需要 tag 是因为 `+` 要运行时检查类型；Kaubo 的 `+` 在编译期已消解为 `AddInt` 或 `FAdd`。
**kaubo 不需要运行时标记。**

---

## 三、设计决策

### 3.1 统一寄存器组（取代 ints/floats）

```
之前: RegFile { ints: Vec<i64>, floats: Vec<f64> }
之后: regs: Vec<u64>
```

- `AddInt`：读 `regs[b] as i64 + regs[c] as i64`，写 `regs[a]`
- `FAdd`：读 `f64::from_bits(regs[b]) + f64::from_bits(regs[c])`，写 `.to_bits()`
- `LoadConst(String)`：写堆句柄到 `regs[a]`

**理由**：JVM 的局部变量表就是统一 slot，不区分 int/float 存储区。
操作码决定怎么解释位模式。消除 `write_int`/`write_float`/`write_bool` 同步问题。

参考：LuaJIT（NaN-boxing 单 u64 组）、V8 Ignition（统一寄存器组）。

### 3.2 运算符 → @builtins（类型导向的 CPS Build）

```
现在: build_binary(left, op, right)
      → ValueHint::is_float() 猜类型 → bin_op_to_cps(op, is_float)

改后: build_binary(left, op, right, lhs_type, rhs_type)
      → bin_op_to_cps(op, lhs_type, rhs_type)  ← 按 Type 精确匹配
```

infer 已经算出了操作数的精确类型，但不传给 CPS Build。
改为传递 Type，按类型匹配 `@addInt` / `@fadd` / `@sadd`。
消除问题 #1 #2 #3 #7。

### 3.3 Print 退化为 PrintStr

```
现在: Print(reg) → 运行时猜 reg 是 int/float/string
改后: PrintStr(reg) → 只读堆句柄取 String

type → string 转换在 CPS 层完成（IToS / FToS / Move）
CPS Build 根据类型选正确的转换指令
```

**理由**：不需要运行时 tag。类型→字符串的转换是编译期已知的。

### 3.4 值语义

- `null`：HeapObj::Null（共享单例），运行时句柄非 0，与 Int(0) 可区分
- `bool`：true=1, false=0，由操作码保证只在 Bool 上下文中使用
- `0` `0.0` `false`：操作码区分，不做运行时 tag

---

## 四、实施计划

### 第一阶段 ✅ 已完成

| 改动 | 位置 | 效果 |
|------|------|------|
| 比较运算加 unify | infer.rs | `1 == 2.0` 编译报错 |
| build_binary 传 Type | cps_build.rs | 不再靠 `is_float: bool` 猜 |
| @builtins 分派表 | cps_build.rs | `(op, lhs_type, rhs_type)` 精确匹配指令 |
| ints/floats → regs: Vec<u64> | execute.rs | 消解同步 bug |
| 0x3A 冲突 + CallFrame 统一 | execute.rs | opcode 冲突修复 |

### 第二阶段（待做）

- opcode 枚举化
- 变长编码
- 循环引用 GC

### 第三阶段（远期）

- @builtins 前置到 parser + interface 承载运算符（部分已通过 Phase 4a interface/operator 实现）
- 效应系统 Suspend/Resume
