# Kaubo 类型系统审计与修复方案

> 2026-06-12 · v0.1.0 → v0.2.0

---

## 一、现状：一根主线，两套并行系统

```
编译管线:
  Source → Lexer → Parser → Codegen → Chunk → VM

TypeChecker (968 行)  ← 从未被调用
Codegen (get_expr_type) ← 自己的类型推断，和 TypeChecker 矛盾
```

**根因：TypeChecker 是死代码。** 所有类型检查逻辑已实现（968 行，9 个测试），但没有一个 Pass 调用它。

---

## 二、10 个 Gap 审计

| # | 领域 | 分类 | 影响 |
|---|------|------|------|
| 1 | **TypeChecker 未集成** | MISSING | 所有类型检查不执行 |
| 2 | **两套并行类型系统** | INCONSISTENT | TypeExpr (checker) vs VarType (codegen) 不一致 |
| 3 | **`as` 类型转换** | SILENT | 无效 cast 不报错, 运行时返回 NULL |
| 4 | **除法语义** | INCONSISTENT | TypeChecker: `int/int→int`, VM: `int/int→float` |
| 5 | **`List[T]` vs 运行时** | INCONSISTENT | 类型标注不存, codegen 硬编码 `List<int>` |
| 6 | **变量类型标注** | MISSING | `: Type` 解析了但 codegen 不读 |
| 7 | **return 类型检查** | MISSING | 逻辑有, 未接线 |
| 8 | **隐式转换** | MISSING | 只有 `int→float`, 其余全是 TODO |
| 9 | **TypeError** | MISSING | 5 个错误类型已定义, 从不产出 |
| 10 | **测试覆盖** | MISSING | 9 个隔离单元测试, 0 个管线集成测试 |

### 详细

#### Gap 1: TypeChecker 未集成

- 文件: `src/pipeline/parser/type_checker.rs` (968 行), 9 tests
- `ParserPass` / `CodeGenPass` / `CompilePass` / `VmExecutionPass` 都不调用它
- 缺少 `check_module()` 方法（只有 `check_statement()` / `check_expression()`）

#### Gap 2: 两套并行类型系统

| 维度 | TypeChecker (TypeExpr) | Codegen (VarType) |
|------|----------------------|-------------------|
| 定义 | `type_expr.rs` | `codegen/context.rs` |
| 变体 | Named, List\<T\>, Tuple, Function | Int, Float, String, Bool, Struct, List, Json |
| 函数类型 | 完整 param/return 类型 | 无功能类型 |
| list 推理 | 统一元素类型 | **硬编码 `List<int>`** |
| 调用返回类型 | 查函数类型 | 只处理 struct 方法 |

#### Gap 3: `as` 类型转换

- 编译: `cast_to_*` opcodes 直接生成，不验证
- 类型检查: `_ => Ok(None)` 跳过 As 表达式的检查 (line 553)
- 运行时: 无效 cast → NULL (不报错)

#### Gap 4: 除法语义不一致

- VM: `div_values` 永远返回 `float` (line 117: "避免整数除法的困惑")
- TypeChecker: `int / int → int` (line 631)
- 如果接线: `var x: int = 5/2;` → 类型检查通过, 但 x 实际是 float

#### Gap 5: `List[T]` vs 运行时

- `ObjList` 存 `Vec<Value>`, 无类型信息
- TypeChecker 可以推断 `List<int>` 但运行时无法辨别
- Codegen `get_expr_type` 硬编码: `LiteralList → List(Int)`

#### Gap 6: 变量类型标注

- Parser 解析 `: int` → 存到 `VarDeclStmt.type_annotation`
- Codegen 编译 `VarDecl` → 只读 `decl.initializer`, 不读 `decl.type_annotation`
- 结果: `var x: int = 3.14;` → 不报错

#### Gap 7: return 类型检查

- `check_return()` (line 510-531) 完整实现
- `check_lambda()` 用 `return_type_stack` 验证返回类型
- 但无 Pass 调用 → 从未执行

#### Gap 8: 隐式转换

- `is_compatible()` 只有 `int → float` (line 953)
- ← `null → any` (TODO)
- ← `List` 协变 (TODO)
- ← `Function` 子类型 (TODO)
- ← 箭头 `int → float → any` (TODO)

#### Gap 9: TypeError

- `TypeError { Mismatch, ReturnTypeMismatch, UndefinedVar, UnsupportedOp, CannotInfer }`
- 5 个变体, 都带 `{ expected, found, location }`
- 只在 9 个单元测试中产出, 编译管线从不产出

#### Gap 10: 测试

- TypeChecker: 9 个隔离测试
- Codegen: ~30 个端到端测试 (无类型检查)
- 缺少: TypeChecker 挂入管线的集成测试

---

## 三、修复方案

### Phase A: 接线（激活现有代码）

```
1. 加 TypeCheckPass, 实现 Pass trait
   ─ data_format: AstIn → TypedAstOut
   ─ run(): check_module(ast) → Ok(TypedModule) or Err(TypeError)

2. 加 check_module() 方法
   ─ 遍历 module.statements, 调 check_statement()
   ─ 返回 Vec<TypeError> 或第一个错误

3. 插进编译管线
   ─ CompilePass: ParserPass → TypeCheckPass → CodeGenPass
```

**量: ~50 行**

### Phase B: 修复语义不一致

```
1. check_binary: Slash 永远返回 float (和 VM 一致)
   ─ 从 `else { Ok(Some(int)) }` → `Ok(Some(float))`

2. check_expression: 加 As 分支
   ─ 验证 from_type → to_type 的合法性
   ─ int→float, float→int, any→bool 允许，其他返回 TypeError

3. compile_stmt: 读 type_annotation
   ─ 如果 decl.type_annotation 存在 → 对比 inferred type → 不匹配报 CompileError

4. is_compatible: 加 null → any
```

**量: ~30 行**

### Phase C: 编译报错映射

```
5. TypeError → CompileError 映射
   ─ Mismatch → CompileError::TypeMismatch
   ─ UndefinedVar → CompileError::UndefinedVariable
   ─ 新增 CompileError 变体
```

**量: ~15 行**

---

## 四、不做（延后）

| 项 | 原因 |
|----|------|
| 泛型 `struct Box[T]` | 复杂度高, 先保证基础类型检查 |
| SSA / Phi / 流类型分析 | 依赖 HIR lowering |
| List 运行时类型擦除 → 保留类型 | 需要重构 ObjList |
| 函数重载 | 不需要 |
| 子类型协变/逆变 | 先保证基础, 后续加 |

---

*2026-06-12*
