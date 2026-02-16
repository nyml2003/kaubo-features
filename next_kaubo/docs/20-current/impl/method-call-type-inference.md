# 方法调用类型推断增强设计文档

> 状态：待评审 | 目标：支持 `list.filter().map()` 和 `struct.method()` 的类型推断

---

## 1. 问题描述

### 1.1 当前现象

```kaubo
var result = [1, 2, 3, 4, 5]
    .filter(|x| { return x > 2; })   // ❌ 类型丢失
    .map(|x| { return x * 10; });    // ❌ 在 None 上调用
// 错误：Cannot infer type for variable 'result'
```

### 1.2 根因分析

`TypeChecker::check_member_access` 仅处理两种情况：

1. `std.xxx` 模块函数访问
2. `struct.field` 字段访问

**缺失**：内置类型方法调用和 struct 方法调用的返回类型推断

```rust
// kaubo-core/src/compiler/parser/type_checker.rs:729
fn check_member_access(&mut self, member_access: &MemberAccess) -> ... {
    // 1. 处理 std.xxx
    if obj_var.name == "std" { ... }
    
    // 2. 处理 struct 字段
    if let Some(fields) = self.struct_types.get(&named_type.name) { ... }
    
    // 3. ❌ 其他情况返回 None，导致类型丢失
    Ok(None)
}
```

---

## 2. 设计目标

### 2.1 必须支持的场景

| 场景 | 示例 | 期望返回类型 |
|-----|------|-------------|
| 内置类型链式调用 | `[1,2,3].filter(...).map(...)` | `List<int>` |
| 内置类型终结方法 | `list.len()` | `int` |
| struct 方法调用 | `point.distance(other)` | `float` |
| 复杂链式 | `list.filter(...).map(...).reduce(...)` | 推导出的具体类型 |

### 2.2 非目标（当前阶段）

- 泛型方法类型参数推断（如 `map` 返回 `List<T>` 中 `T` 的精确推导）
- 方法重载解析
- 协变/逆变类型检查

---

## 3. 最终方案

### 3.1 架构概览

```
TypeChecker
├── struct_types: HashMap<String, Vec<(field_name, TypeExpr)>>
├── struct_methods: HashMap<String, HashMap<String, TypeExpr>>  // 新增
└── builtin_method_types: BuiltinMethodTypeTable               // 新增
```

### 3.2 数据结构

#### 3.2.1 内置方法返回类型表

```rust
// 内置类型方法返回类型定义
pub struct BuiltinMethodTypeTable;

impl BuiltinMethodTypeTable {
    /// 获取 List 方法的返回类型
    pub fn list_method_return(method_name: &str) -> Option<TypeExpr> {
        match method_name {
            // 返回 List（支持链式）
            "filter" | "map" => Some(TypeExpr::list(TypeExpr::named("any"))),
            // 返回 void
            "push" | "clear" | "foreach" => Some(TypeExpr::named("void")),
            // 返回 int
            "len" => Some(TypeExpr::named("int")),
            // 返回 bool
            "is_empty" | "any" | "all" => Some(TypeExpr::named("bool")),
            // 返回 any
            "remove" | "find" | "reduce" => Some(TypeExpr::named("any")),
            _ => None,
        }
    }
    
    pub fn string_method_return(method_name: &str) -> Option<TypeExpr> {
        match method_name {
            "len" => Some(TypeExpr::named("int")),
            "is_empty" => Some(TypeExpr::named("bool")),
            _ => None,
        }
    }
    
    pub fn json_method_return(method_name: &str) -> Option<TypeExpr> {
        match method_name {
            "len" => Some(TypeExpr::named("int")),
            "is_empty" => Some(TypeExpr::named("bool")),
            _ => None,
        }
    }
}
```

#### 3.2.2 Struct 方法表存储

```rust
pub struct TypeChecker {
    // ... 现有字段 ...
    
    /// struct 方法返回类型表
    /// Key: struct_name -> method_name -> return_type
    struct_method_types: HashMap<String, HashMap<String, TypeExpr>>,
}
```

### 3.3 关键实现

#### 3.3.1 增强 `check_impl_def`

```rust
fn check_impl_def(&mut self, impl_stmt: &ImplStmt) -> TypeCheckResult<Option<TypeExpr>> {
    // 验证 struct 存在 ...
    
    let mut method_types = HashMap::new();
    
    for method in &impl_stmt.methods {
        // 检查方法 lambda 表达式
        let method_type = self.check_expression(&method.lambda)?;
        
        // 提取返回类型
        if let Some(TypeExpr::Function(func_type)) = method_type {
            let return_ty = func_type.return_type
                .map(|t| *t)
                .unwrap_or_else(|| TypeExpr::named("void"));
            method_types.insert(method.name.clone(), return_ty);
        }
    }
    
    // 存储方法返回类型表
    self.struct_method_types.insert(
        impl_stmt.struct_name.clone(), 
        method_types
    );
    
    Ok(None)
}
```

#### 3.3.2 增强 `check_member_access`

```rust
fn check_member_access(
    &mut self, 
    member_access: &MemberAccess
) -> TypeCheckResult<Option<TypeExpr>> {
    // 1. 保留：std.xxx 处理
    if let ExprKind::VarRef(obj_var) = member_access.object.as_ref() {
        if obj_var.name == "std" {
            return Ok(self.get_stdlib_function_type(&member_access.member));
        }
    }
    
    // 获取对象类型
    let obj_type = self.check_expression(&member_access.object)?;
    
    match obj_type {
        // 2. 内置类型方法
        Some(TypeExpr::List(_)) => {
            if let Some(ret_ty) = BuiltinMethodTypeTable::list_method_return(&member_access.member) {
                return Ok(Some(ret_ty));
            }
        }
        Some(TypeExpr::Named(named)) if named.name == "string" => {
            if let Some(ret_ty) = BuiltinMethodTypeTable::string_method_return(&member_access.member) {
                return Ok(Some(ret_ty));
            }
        }
        Some(TypeExpr::Named(named)) if named.name == "json" => {
            if let Some(ret_ty) = BuiltinMethodTypeTable::json_method_return(&member_access.member) {
                return Ok(Some(ret_ty));
            }
        }
        
        // 3. struct 字段访问（现有）
        Some(TypeExpr::Named(named)) => {
            if let Some(fields) = self.struct_types.get(&named.name) {
                if let Some((_, field_ty)) = fields.iter().find(|(n, _)| n == &member_access.member) {
                    return Ok(Some(field_ty.clone()));
                }
            }
            
            // 4. 新增：struct 方法访问
            if let Some(methods) = self.struct_method_types.get(&named.name) {
                if let Some(ret_ty) = methods.get(&member_access.member) {
                    return Ok(Some(ret_ty.clone()));
                }
            }
        }
        _ => {}
    }
    
    // 无法推断，返回 None（非严格模式下允许）
    Ok(None)
}
```

#### 3.3.3 链式调用支持

```rust
fn check_function_call(&mut self, call: &FunctionCall) -> TypeCheckResult<Option<TypeExpr>> {
    // 获取函数表达式的类型（对于方法调用，就是方法的返回类型）
    let func_type = self.check_expression(&call.function_expr)?;
    
    if let Some(TypeExpr::Function(func)) = func_type {
        // 返回函数的返回类型
        Ok(func.return_type.map(|t| *t))
    } else {
        Ok(None)
    }
}
```

---

## 4. 类型推导规则

### 4.1 内置类型方法

| 类型 | 方法 | 返回类型 | 链式支持 |
|-----|------|---------|---------|
| List | `filter`, `map` | `List<any>` | ✅ |
| List | `push`, `clear` | `void` | ❌ |
| List | `len` | `int` | ❌ |
| List | `is_empty`, `any`, `all` | `bool` | ❌ |
| List | `reduce` | `any` | ❌ |
| List | `find`, `remove` | `any` | ❌ |
| String | `len` | `int` | ❌ |
| String | `is_empty` | `bool` | ❌ |
| Json | `len` | `int` | ❌ |
| Json | `is_empty` | `bool` | ❌ |

### 4.2 链式调用示例

```kaubo
// 场景 1：纯链式（全部返回 List）
var a = [1,2,3,4,5]
    .filter(|x| { return x > 2; })  // List<any>
    .map(|x| { return x * 10; });    // List<any>
// a: List<any> ✅

// 场景 2：以终结方法结尾
var b = [1,2,3]
    .filter(|x| { return x > 1; })  // List<any>
    .len();                          // int
// b: int ✅

// 场景 3：reduce 返回 any
var c = [1,2,3].reduce(|acc, x| { return acc + x; }, 0);
// c: any（如需精确类型，需显式标注）
```

---

## 5. 实现步骤

| 序号 | 任务 | 文件 | 备注 |
|------|------|------|------|
| 1 | 添加内置方法返回类型表 | `kaubo-core/src/compiler/parser/type_checker.rs` | 新增 `BuiltinMethodTypeTable` |
| 2 | 添加 struct 方法表 | `kaubo-core/src/compiler/parser/type_checker.rs` | `struct_method_types` 字段 |
| 3 | 增强 `check_impl_def` | `kaubo-core/src/compiler/parser/type_checker.rs` | 填充方法表 |
| 4 | 增强 `check_member_access` | `kaubo-core/src/compiler/parser/type_checker.rs` | 内置类型 + struct 方法 |
| 5 | 测试链式调用 | `kaubo-core/src/compiler/parser/type_checker.rs` | 添加测试用例 |
| 6 | 验证示例 | `examples/calc/main.kaubo` | 确保无类型错误 |

---

## 6. 边界情况

### 6.1 未知方法

```kaubo
var a = [1,2,3].unknown_method();  // 返回 None，依赖运行时检查
```

**处理**：返回 `None`，非严格模式下通过，运行时报错。

### 6.2 泛型精度

```kaubo
// 当前：返回 List<any>
// 理想：返回 List<int>（需泛型推导增强）
var a = [1,2,3].filter(|x| { return x > 1; });
```

**处理**：Phase 1 返回 `List<any>`，后续迭代可增强元素类型推导。

### 6.3 方法名与字段名冲突

```kaubo
struct Point {
    x: float,
    distance: float  // 字段名
}

impl Point {
    distance: |self, other: Point| -> float { ... }  // 方法名
}

var p = Point { x: 0.0, distance: 0.0 };
print(p.distance);      // 访问字段？还是方法？
```

**处理**：按现有语义，字段优先。如需调用方法，使用显式语法（待定义）。

---

## 7. 兼容性

| 方面 | 影响 | 说明 |
|-----|------|------|
| 语法 | 无变化 | 仅增强类型推导 |
| 运行时 | 无变化 | 不涉及字节码修改 |
| 性能 | 编译期轻微增加 | 多几次 HashMap 查找 |
| 错误信息 | 改进 | 类型错误更精确 |

---

## 8. 验证示例

```kaubo
// test_method_type_inference.kaubo

// 内置类型链式调用
var list_result = [1, 2, 3, 4, 5]
    .filter(|x| { return x > 2; })
    .map(|x| { return x * 10; });
print(list_result);  // [30, 40, 50]

// reduce 计算总和（显式标注类型）
var sum: int = [1, 2, 3, 4, 5].reduce(|acc, x| { return acc + x; }, 0);
print(sum);  // 15

// struct 方法调用
struct Point {
    x: float,
    y: float
}

impl Point {
    distance: |self, other: Point| -> float {
        var dx = self.x - other.x;
        var dy = self.y - other.y;
        return sqrt(dx * dx + dy * dy);
    }
}

var p1 = Point { x: 0.0, y: 0.0 };
var p2 = Point { x: 3.0, y: 4.0 };
var dist = p1.distance(p2);  // 推导为 float
print(dist);  // 5.0
```

---

## 9. 决策记录

| 日期 | 决策 | 理由 |
|------|------|------|
| 2026-02-16 | 分两阶段实现 | 先解决 "无法推断" 问题，再增强精度 |
| 2026-02-16 | List 元素类型暂时用 `any` | 泛型推导复杂度较高，后续迭代 |
| 2026-02-16 | 字段优先于方法 | 与现有语义一致，避免破坏性变更 |

---

*最后更新：2026-02-16*  
*状态：待评审*  
*作者：Kimi*
