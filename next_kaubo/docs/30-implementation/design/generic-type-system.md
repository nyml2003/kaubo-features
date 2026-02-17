# Kaubo 编译时泛型系统设计

> 状态：设计文档 v2.1 | 目标：实现完整的编译时泛型系统

---

## 1. 设计概览

### 1.1 核心能力

| 功能 | 示例 |
|------|------|
| 泛型匿名函数 | `\|[T] x: T\| -> T { return x; }` |
| 泛型 struct | `struct Box[T] { value: T }` |
| 泛型 impl | `impl[T] Box[T] { ... }` |
| 类型推导 | `identity(42)` 推导为 `\|int\| -> int` |
| 多类型参数 | `\|[T, U] x: T, y: U\| -> Tuple[T, U] { ... }` |
| 嵌套泛型 | `Box[List[T]]` |

### 1.2 语法统一原则

**统一使用 `[]` 表示泛型参数**，避免 `<>` 与小于运算符冲突：

```kaubo
// 类型定义
struct Box[T] { value: T }
impl[T] Box[T] { ... }

// 表达式
|[T] x: T| -> T { return x; }

// 类型标注
var b: Box[int] = Box[int] { value: 42 };
var list: List[List[string]] = [];
```

### 1.3 Lambda 语法规范

Kaubo 的 lambda **只支持完整语法形式**：

```kaubo
// ✅ 正确：完整形式
|[T] x: T| -> T { return x; };           // 泛型，带返回类型
|x: int| -> int { return x * 2; };       // 具体类型
|x| { return x + 1; };                   // 无类型标注
|| { return 42; };                       // 无参数

// ❌ 错误：不支持简写
var f = |x| x * 2;                       // 缺少 {} 和 return
var g = |x| { x; };                      // 缺少 return
```

---

## 2. 类型系统扩展

### 2.1 TypeExpr 扩展

```rust
// kaubo-core/src/compiler/parser/type_expr.rs

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(NamedType),                    // int, string, bool
    TypeParam(TypeParam),                // NEW: 类型参数 T
    List(Box<TypeExpr>),                 // List[T]
    Tuple(Vec<TypeExpr>),                // Tuple[T, U]
    Function(FunctionType),              // |T| -> U
    GenericInstance(GenericInstance),    // NEW: Box[int]
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<TypeBound>,         // 未来：约束如 T: Add
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeBound {
    Trait(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericInstance {
    pub base_name: String,              // "Box"
    pub type_args: Vec<TypeExpr>,       // [int]
}
```

### 2.2 类型替换算法

```rust
impl TypeExpr {
    /// 应用类型替换
    /// List[T] + {T → int} = List[int]
    pub fn substitute(&self, subst: &HashMap<String, TypeExpr>) -> TypeExpr {
        match self {
            TypeExpr::TypeParam(param) => {
                subst.get(&param.name)
                    .cloned()
                    .unwrap_or_else(|| self.clone())
            }
            TypeExpr::List(elem) => {
                TypeExpr::List(Box::new(elem.substitute(subst)))
            }
            TypeExpr::Tuple(elems) => {
                TypeExpr::Tuple(elems.iter()
                    .map(|e| e.substitute(subst))
                    .collect())
            }
            TypeExpr::Function(func) => {
                TypeExpr::Function(FunctionType {
                    params: func.params.iter()
                        .map(|p| p.substitute(subst))
                        .collect(),
                    return_type: func.return_type.as_ref()
                        .map(|r| Box::new(r.substitute(subst))),
                })
            }
            TypeExpr::GenericInstance(inst) => {
                TypeExpr::GenericInstance(GenericInstance {
                    base_name: inst.base_name.clone(),
                    type_args: inst.type_args.iter()
                        .map(|a| a.substitute(subst))
                        .collect(),
                })
            }
            _ => self.clone(),
        }
    }
    
    /// 收集所有类型参数
    pub fn collect_type_params(&self) -> HashSet<String> {
        let mut params = HashSet::new();
        self.collect_type_params_into(&mut params);
        params
    }
    
    fn collect_type_params_into(&self, params: &mut HashSet<String>) {
        match self {
            TypeExpr::TypeParam(p) => { params.insert(p.name.clone()); }
            TypeExpr::List(e) => e.collect_type_params_into(params),
            TypeExpr::Tuple(elems) => {
                for e in elems { e.collect_type_params_into(params); }
            }
            TypeExpr::Function(f) => {
                for p in &f.params { p.collect_type_params_into(params); }
                if let Some(r) = &f.return_type { r.collect_type_params_into(params); }
            }
            TypeExpr::GenericInstance(inst) => {
                for a in &inst.type_args { a.collect_type_params_into(params); }
            }
            _ => {}
        }
    }
}
```

---

## 3. AST 扩展

### 3.1 语句扩展

```rust
// kaubo-core/src/compiler/parser/stmt.rs

/// 泛型 struct 定义
#[derive(Debug, Clone, PartialEq)]
pub struct StructStmt {
    pub name: String,
    pub type_params: Vec<TypeParam>,    // NEW: [T]
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

/// 泛型 impl 定义
#[derive(Debug, Clone, PartialEq)]
pub struct ImplStmt {
    pub target_type: TypeExpr,          // Box[T]
    pub type_params: Vec<TypeParam>,    // NEW: impl[T] 中的 [T]
    pub methods: Vec<MethodDef>,
    pub span: Span,
}
```

### 3.2 表达式扩展

```rust
// kaubo-core/src/compiler/parser/expr.rs

/// 泛型 lambda
#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    pub type_params: Vec<TypeParam>,    // NEW: [T, U]
    pub params: Vec<LambdaParam>,       // (name, Option<TypeExpr>)
    pub return_type: Option<TypeExpr>,  // -> Type
    pub body: Stmt,                     // BlockStmt
    pub span: Span,
}

/// 泛型实例化表达式（未来扩展）
#[derive(Debug, Clone, PartialEq)]
pub struct GenericInstantiation {
    pub generic: Expr,
    pub type_args: Vec<TypeExpr>,
    pub span: Span,
}
```

---

## 4. 类型检查器扩展

### 4.1 结构扩展

```rust
// kaubo-core/src/compiler/parser/type_checker.rs

pub struct TypeChecker {
    // ... 现有字段 ...
    
    /// 泛型 struct 定义表
    generic_structs: HashMap<String, GenericDef>,
    
    /// 泛型 impl 方法表
    generic_impls: HashMap<String, Vec<GenericImplDef>>,
    
    /// 类型参数作用域栈
    type_param_stack: Vec<HashMap<String, TypeParam>>,
}

pub struct GenericDef {
    pub params: Vec<TypeParam>,
    pub kind: GenericKind,
}

pub enum GenericKind {
    Struct(Vec<(String, TypeExpr)>),     // 字段列表
    Impl(Vec<MethodDef>),                // 方法列表
}

pub struct GenericImplDef {
    pub target_type: TypeExpr,
    pub type_params: Vec<TypeParam>,
    pub methods: Vec<MethodDef>,
}
```

### 4.2 类型参数环境

```rust
impl TypeChecker {
    /// 进入泛型作用域
    fn enter_generic_scope(&mut self, params: &[TypeParam]) {
        let mut scope = HashMap::new();
        for param in params {
            scope.insert(param.name.clone(), param.clone());
        }
        self.type_param_stack.push(scope);
    }
    
    /// 离开泛型作用域
    fn exit_generic_scope(&mut self) {
        self.type_param_stack.pop();
    }
    
    /// 查找类型参数
    fn lookup_type_param(&self, name: &str) -> Option<&TypeParam> {
        for scope in self.type_param_stack.iter().rev() {
            if let Some(param) = scope.get(name) {
                return Some(param);
            }
        }
        None
    }
    
    /// 检查是否是类型参数
    fn is_type_param(&self, name: &str) -> bool {
        self.lookup_type_param(name).is_some()
    }
}
```

### 4.3 泛型 struct 检查

```rust
impl TypeChecker {
    fn check_struct_def(&mut self, stmt: &StructStmt) -> TypeCheckResult<Option<TypeExpr>> {
        // 检查类型参数合法性
        if !stmt.type_params.is_empty() {
            // 进入泛型作用域
            self.enter_generic_scope(&stmt.type_params);
            
            // 检查字段类型（可能引用类型参数）
            let mut field_types = Vec::new();
            for field in &stmt.fields {
                self.check_type_expr(&field.type_annotation)?;
                field_types.push((field.name.clone(), field.type_annotation.clone()));
            }
            
            // 离开泛型作用域
            self.exit_generic_scope();
            
            // 存储泛型定义
            let def = GenericDef {
                params: stmt.type_params.clone(),
                kind: GenericKind::Struct(field_types),
            };
            self.generic_structs.insert(stmt.name.clone(), def);
        } else {
            // 非泛型 struct，原有逻辑
            let fields: Vec<(String, TypeExpr)> = stmt.fields.iter()
                .map(|f| (f.name.clone(), f.type_annotation.clone()))
                .collect();
            self.struct_types.insert(stmt.name.clone(), fields);
            
            // 创建 shape
            let shape_id = self.next_shape_id;
            self.next_shape_id += 1;
            let shape = ObjShape::new_with_types(
                shape_id,
                stmt.name.clone(),
                stmt.fields.iter().map(|f| f.name.clone()).collect(),
                stmt.fields.iter().map(|f| f.type_annotation.to_string()).collect(),
            );
            self.shapes.push(shape);
        }
        
        Ok(None)
    }
    
    /// 检查类型表达式
    fn check_type_expr(&self, ty: &TypeExpr) -> TypeCheckResult<()> {
        match ty {
            TypeExpr::Named(named) => {
                if self.is_type_param(&named.name) {
                    return Ok(());
                }
                if !self.is_known_type(&named.name) {
                    return Err(TypeError::UndefinedVar { ... });
                }
            }
            TypeExpr::TypeParam(param) => {
                if !self.is_type_param(&param.name) {
                    return Err(TypeError::UndefinedVar { ... });
                }
            }
            // ... 其他类型检查
            _ => {}
        }
        Ok(())
    }
}
```

### 4.4 泛型实例化

```rust
impl TypeChecker {
    /// 实例化泛型 struct
    fn instantiate_generic_struct(
        &mut self,
        name: &str,
        type_args: &[TypeExpr],
    ) -> TypeCheckResult<(Vec<(String, TypeExpr)>, u16)> {
        let def = self.generic_structs.get(name)
            .ok_or(TypeError::UndefinedVar { ... })?;
        
        // 检查类型参数数量
        if def.params.len() != type_args.len() {
            return Err(TypeError::Mismatch { ... });
        }
        
        // 创建替换映射 {T → int}
        let mut subst = HashMap::new();
        for (param, arg) in def.params.iter().zip(type_args.iter()) {
            subst.insert(param.name.clone(), arg.clone());
        }
        
        // 应用替换
        let fields = match &def.kind {
            GenericKind::Struct(fields) => fields.iter()
                .map(|(name, ty)| (name.clone(), ty.substitute(&subst)))
                .collect::<Vec<_>>(),
            _ => panic!("Expected struct"),
        };
        
        // 生成 mangled name
        let mangled = format!("{}[{}]", name,
            type_args.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "));
        
        // 创建或获取 shape
        let shape_id = if let Some(id) = self.get_shape_id(&mangled) {
            id
        } else {
            let id = self.next_shape_id;
            self.next_shape_id += 1;
            let shape = ObjShape::new_with_types(
                id,
                mangled,
                fields.iter().map(|(n, _)| n.clone()).collect(),
                fields.iter().map(|(_, t)| t.to_string()).collect(),
            );
            self.shapes.push(shape);
            id
        };
        
        Ok((fields, shape_id))
    }
}
```

### 4.5 泛型 lambda 检查

```rust
impl TypeChecker {
    fn check_lambda(&mut self, lambda: &Lambda) -> TypeCheckResult<Option<TypeExpr>> {
        // 进入泛型作用域
        self.enter_generic_scope(&lambda.type_params);
        
        // 创建新的变量环境
        let old_env = self.env.clone();
        self.env = TypeEnv::child(&old_env);
        
        // 添加参数到环境
        let mut param_types = Vec::new();
        for (param_name, param_type) in &lambda.params {
            let ty = if let Some(t) = param_type {
                // 参数有类型标注，检查是否有效
                self.check_type_expr(t)?;
                t.clone()
            } else {
                // 无标注，视为 any
                TypeExpr::named("any")
            };
            self.env.define(param_name.clone(), ty.clone());
            param_types.push(ty);
        }
        
        // 压入期望返回类型
        self.return_type_stack.push(lambda.return_type.clone());
        
        // 检查函数体
        let body_type = self.check_statement(&lambda.body)?;
        
        // 弹出返回类型
        self.return_type_stack.pop();
        
        // 恢复环境
        self.env = old_env;
        
        // 离开泛型作用域
        self.exit_generic_scope();
        
        // 确定返回类型
        let return_type = lambda.return_type.clone()
            .or(body_type);
        
        Ok(Some(TypeExpr::Function(FunctionType {
            params: param_types,
            return_type: return_type.map(Box::new),
        })))
    }
}
```

---

## 5. 类型推导算法

### 5.1 约束求解

```rust
/// 类型约束
#[derive(Debug, Clone)]
pub struct Constraint {
    pub expected: TypeExpr,
    pub actual: TypeExpr,
    pub location: ErrorLocation,
}

/// 统一两个类型
pub fn unify(
    t1: &TypeExpr,
    t2: &TypeExpr,
) -> Result<HashMap<String, TypeExpr>, String> {
    match (t1, t2) {
        // 相同类型
        _ if t1 == t2 => Ok(HashMap::new()),
        
        // 类型变量绑定
        (TypeExpr::TypeParam(p), _) => bind(&p.name, t2),
        (_, TypeExpr::TypeParam(p)) => bind(&p.name, t1),
        
        // 递归统一
        (TypeExpr::List(e1), TypeExpr::List(e2)) => unify(e1, e2),
        
        (TypeExpr::Tuple(elems1), TypeExpr::Tuple(elems2)) => {
            if elems1.len() != elems2.len() {
                return Err("Tuple length mismatch".to_string());
            }
            let mut subst = HashMap::new();
            for (a, b) in elems1.iter().zip(elems2.iter()) {
                let s = unify(&substitute(a, &subst), &substitute(b, &subst))?;
                subst.extend(s);
            }
            Ok(subst)
        }
        
        (TypeExpr::Function(f1), TypeExpr::Function(f2)) => {
            if f1.params.len() != f2.params.len() {
                return Err("Function arity mismatch".to_string());
            }
            let mut subst = HashMap::new();
            for (p1, p2) in f1.params.iter().zip(f2.params.iter()) {
                let s = unify(&substitute(p1, &subst), &substitute(p2, &subst))?;
                subst.extend(s);
            }
            if let (Some(r1), Some(r2)) = (&f1.return_type, &f2.return_type) {
                let s = unify(&substitute(r1, &subst), &substitute(r2, &subst))?;
                subst.extend(s);
            }
            Ok(subst)
        }
        
        (TypeExpr::GenericInstance(i1), TypeExpr::GenericInstance(i2)) => {
            if i1.base_name != i2.base_name {
                return Err(format!("Type mismatch: {} vs {}", i1.base_name, i2.base_name));
            }
            if i1.type_args.len() != i2.type_args.len() {
                return Err("Type argument count mismatch".to_string());
            }
            let mut subst = HashMap::new();
            for (a1, a2) in i1.type_args.iter().zip(i2.type_args.iter()) {
                let s = unify(&substitute(a1, &subst), &substitute(a2, &subst))?;
                subst.extend(s);
            }
            Ok(subst)
        }
        
        _ => Err(format!("Cannot unify {} with {}", t1, t2)),
    }
}

fn bind(var: &str, ty: &TypeExpr) -> Result<HashMap<String, TypeExpr>, String> {
    // Occurs check: 防止 T = List[T]
    if occurs_check(var, ty) {
        return Err(format!("Infinite type: {} occurs in {}", var, ty));
    }
    let mut subst = HashMap::new();
    subst.insert(var.to_string(), ty.clone());
    Ok(subst)
}

fn occurs_check(var: &str, ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::TypeParam(p) => p.name == var,
        TypeExpr::List(e) => occurs_check(var, e),
        TypeExpr::Tuple(elems) => elems.iter().any(|e| occurs_check(var, e)),
        TypeExpr::Function(f) => {
            f.params.iter().any(|p| occurs_check(var, p))
                || f.return_type.as_ref().map_or(false, |r| occurs_check(var, r))
        }
        TypeExpr::GenericInstance(inst) => {
            inst.type_args.iter().any(|a| occurs_check(var, a))
        }
        _ => false,
    }
}

fn substitute(ty: &TypeExpr, subst: &HashMap<String, TypeExpr>) -> TypeExpr {
    ty.substitute(subst)
}
```

### 5.2 调用点类型推导

```rust
impl TypeChecker {
    /// 从函数调用推导类型参数
    fn infer_type_args(
        &mut self,
        func_type: &FunctionType,
        arg_types: &[Option<TypeExpr>],
        expected_return: Option<&TypeExpr>,
    ) -> TypeCheckResult<HashMap<String, TypeExpr>> {
        let mut constraints = Vec::new();
        
        // 从参数收集约束
        for (param, arg) in func_type.params.iter().zip(arg_types.iter()) {
            if let Some(arg_type) = arg {
                constraints.push(Constraint {
                    expected: param.clone(),
                    actual: arg_type.clone(),
                    location: ErrorLocation::Unknown,
                });
            }
        }
        
        // 从期望返回类型收集约束
        if let Some(expected) = expected_return {
            if let Some(ret) = &func_type.return_type {
                constraints.push(Constraint {
                    expected: (**ret).clone(),
                    actual: expected.clone(),
                    location: ErrorLocation::Unknown,
                });
            }
        }
        
        // 求解所有约束
        let mut final_subst = HashMap::new();
        for constraint in constraints {
            let subst = unify(
                &constraint.expected.substitute(&final_subst),
                &constraint.actual.substitute(&final_subst),
            ).map_err(|e| TypeError::CannotInfer {
                message: e,
                location: constraint.location,
            })?;
            final_subst.extend(subst);
        }
        
        Ok(final_subst)
    }
}
```

---

## 6. 单态化（Monomorphization）

### 6.1 单态化管理器

```rust
pub struct Monomorphizer {
    /// 已实例化的函数：(name, type_args) -> mangled_name
    instantiated_functions: HashMap<(String, Vec<TypeExpr>), String>,
    /// 待实例化队列
    pending: Vec<InstantiationTask>,
    /// 已实例化的 struct shape
    instantiated_structs: HashMap<String, u16>, // mangled_name -> shape_id
}

struct InstantiationTask {
    generic_name: String,
    type_args: Vec<TypeExpr>,
    mangled_name: String,
    kind: InstantiationKind,
}

enum InstantiationKind {
    Function(Lambda),    // 泛型 lambda
    Struct,              // 泛型 struct
}
```

### 6.2 名称修饰

```rust
impl Monomorphizer {
    /// 生成 mangled 名称
    /// identity[int] -> identity$int
    /// Box[List[string]] -> Box$List$string
    pub fn mangle_name(name: &str, type_args: &[TypeExpr]) -> String {
        let args_str = type_args.iter()
            .map(|t| t.to_string().replace(|c: char| !c.is_alphanumeric(), "_"))
            .collect::<Vec<_>>()
            .join("_");
        format!("{}${}", name, args_str)
    }
    
    /// 获取或创建实例化
    pub fn get_or_instantiate_function(
        &mut self,
        var_name: &str,
        lambda: &Lambda,
        type_args: &[TypeExpr],
    ) -> String {
        let key = (var_name.to_string(), type_args.to_vec());
        
        if let Some(mangled) = self.instantiated_functions.get(&key) {
            return mangled.clone();
        }
        
        let mangled = Self::mangle_name(var_name, type_args);
        
        self.pending.push(InstantiationTask {
            generic_name: var_name.to_string(),
            type_args: type_args.to_vec(),
            mangled_name: mangled.clone(),
            kind: InstantiationKind::Function(lambda.clone()),
        });
        
        self.instantiated_functions.insert(key, mangled.clone());
        mangled
    }
}
```

### 6.3 Lambda 实例化

```rust
impl Monomorphizer {
    /// 实例化泛型 lambda
    fn instantiate_lambda(
        &self,
        lambda: &Lambda,
        type_args: &[TypeExpr],
    ) -> Lambda {
        // 创建替换映射
        let mut subst = HashMap::new();
        for (param, arg) in lambda.type_params.iter().zip(type_args.iter()) {
            subst.insert(param.name.clone(), arg.clone());
        }
        
        // 替换参数类型
        let concrete_params = lambda.params.iter()
            .map(|(name, ty)| {
                (name.clone(), ty.as_ref().map(|t| t.substitute(&subst)))
            })
            .collect();
        
        // 替换返回类型
        let concrete_return = lambda.return_type.as_ref()
            .map(|t| t.substitute(&subst));
        
        // 替换函数体中的类型标注（深度遍历）
        let concrete_body = self.substitute_stmt(&lambda.body, &subst);
        
        Lambda {
            type_params: vec![],  // 实例化后无类型参数
            params: concrete_params,
            return_type: concrete_return,
            body: concrete_body,
            span: lambda.span,
        }
    }
    
    /// 替换语句中的类型
    fn substitute_stmt(&self, stmt: &Stmt, subst: &HashMap<String, TypeExpr>) -> Stmt {
        // 深度遍历 AST，替换所有 TypeExpr
        // ... 实现省略
        stmt.clone()
    }
}
```

---

## 7. Parser 扩展

### 7.1 泛型 lambda 解析

```rust
impl Parser {
    /// 解析 lambda
    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        self.expect(KauboTokenKind::Pipe)?;
        
        // 检查是否有类型参数 [T]
        let type_params = if self.check(KauboTokenKind::LeftBracket) {
            self.parse_lambda_type_params()?
        } else {
            vec![]
        };
        
        // 解析参数列表
        let params = self.parse_lambda_params()?;
        
        self.expect(KauboTokenKind::Pipe)?;
        
        // 解析可选返回类型
        let return_type = if self.match_token(KauboTokenKind::Arrow) {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        
        // 解析函数体（必须是代码块）
        let body = self.parse_block_statement()?;
        
        Ok(Expr::new(ExprKind::Lambda(Lambda {
            type_params,
            params,
            return_type,
            body,
            span: self.current_span(),
        })))
    }
    
    /// 解析 lambda 类型参数 [T, U]
    fn parse_lambda_type_params(&mut self) -> ParseResult<Vec<TypeParam>> {
        self.expect(KauboTokenKind::LeftBracket)?;
        
        let mut params = vec![];
        
        // 解析第一个类型参数
        if !self.check(KauboTokenKind::RightBracket) {
            params.push(self.parse_type_param()?);
            
            // 解析后续类型参数
            while self.match_token(KauboTokenKind::Comma) {
                params.push(self.parse_type_param()?);
            }
        }
        
        self.expect(KauboTokenKind::RightBracket)?;
        Ok(params)
    }
    
    /// 解析单个类型参数
    fn parse_type_param(&mut self) -> ParseResult<TypeParam> {
        let name = self.expect_identifier()?;
        // 未来：解析约束 [T: Add + Clone]
        Ok(TypeParam {
            name,
            bounds: vec![],
        })
    }
}
```

### 7.2 泛型 struct/impl 解析

```rust
impl Parser {
    /// 解析 struct（支持 [T]）
    fn parse_struct_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'struct'
        let name = self.expect_identifier()?;
        
        // 解析可选类型参数 [T]
        let type_params = if self.check(KauboTokenKind::LeftBracket) {
            self.parse_type_params_bracket()?
        } else {
            vec![]
        };
        
        // ... 剩余解析逻辑
    }
    
    /// 解析 impl（支持 [T]）
    fn parse_impl_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'impl'
        
        // 解析可选 impl 类型参数 [T]
        let impl_type_params = if self.check(KauboTokenKind::LeftBracket) {
            self.parse_type_params_bracket()?
        } else {
            vec![]
        };
        
        // 解析目标类型
        let target_type = self.parse_type_expression()?;
        
        // ... 剩余解析逻辑
    }
    
    /// 解析 [] 类型参数
    fn parse_type_params_bracket(&mut self) -> ParseResult<Vec<TypeParam>> {
        self.expect(KauboTokenKind::LeftBracket)?;
        
        let mut params = vec![];
        if !self.check(KauboTokenKind::RightBracket) {
            params.push(self.parse_type_param()?);
            while self.match_token(KauboTokenKind::Comma) {
                params.push(self.parse_type_param()?);
            }
        }
        
        self.expect(KauboTokenKind::RightBracket)?;
        Ok(params)
    }
}
```

### 7.3 类型表达式解析

```rust
impl Parser {
    /// 解析类型表达式（支持 Box[T]）
    fn parse_type_expression(&mut self) -> ParseResult<TypeExpr> {
        let token = self.current_token()?;
        
        if token.kind != KauboTokenKind::Identifier {
            return Err(...);
        }
        
        let type_name = token.text.clone().unwrap_or_default();
        self.consume();
        
        // 检查是否是泛型实例化 Box[T]
        if self.check(KauboTokenKind::LeftBracket) {
            self.parse_generic_instance(type_name)
        } else {
            Ok(TypeExpr::named(type_name))
        }
    }
    
    /// 解析泛型实例化
    fn parse_generic_instance(&mut self, base_name: String) -> ParseResult<TypeExpr> {
        self.expect(KauboTokenKind::LeftBracket)?;
        
        let mut type_args = vec![];
        type_args.push(self.parse_type_expression()?);
        
        while self.match_token(KauboTokenKind::Comma) {
            type_args.push(self.parse_type_expression()?);
        }
        
        self.expect(KauboTokenKind::RightBracket)?;
        
        Ok(TypeExpr::GenericInstance(GenericInstance {
            base_name,
            type_args,
        }))
    }
}
```

---

## 8. 完整语法示例

### 8.1 泛型 Lambda

```kaubo
// 单类型参数
var identity = |[T] x: T| -> T { return x; };

// 多类型参数
var pair = |[T, U] first: T, second: U| -> Tuple[T, U] {
    return Tuple[T, U] { first, second };
};

// 无返回类型标注
var print_any = |[T] x: T| { 
    print(x); 
    return; 
};

// 调用
var x = identity(42);              // 推导为 identity$int
var p = pair(1, "hello");          // 推导为 pair$int$string
```

### 8.2 泛型 Struct

```kaubo
struct Box[T] {
    value: T,
}

struct Pair[T, U] {
    first: T,
    second: U,
}

// 使用
var b = Box[int] { value: 42 };
var p = Pair[string, int] { first: "age", second: 25 };
```

### 8.3 泛型 Impl

```kaubo
impl[T] Box[T] {
    new: |value: T| -> Box[T] {
        return Box[T] { value: value };
    },
    
    get: |self| -> T {
        return self.value;
    },
    
    map: |[U] self, f: |T| -> U| -> Box[U] {
        return Box[U] { value: f(self.value) };
    },
}

// 使用
var b = Box::new(42);
var val = b.get();
var doubled = b.map(|[U] x| -> int { return x * 2; });
```

### 8.4 内置泛型方法

```kaubo
impl[T] List[T] {
    map: |[U] self, f: |T| -> U| -> List[U] {
        var result = List[U] {};
        for item in self {
            result.push(f(item));
        }
        return result;
    },
    
    filter: |self, pred: |T| -> bool| -> List[T] {
        var result = List[T] {};
        for item in self {
            if pred(item) {
                result.push(item);
            }
        }
        return result;
    },
    
    reduce: |[U] self, op: |U, T| -> U, init: U| -> U {
        var acc = init;
        for item in self {
            acc = op(acc, item);
        }
        return acc;
    },
}

// 使用
var nums: List[int] = [1, 2, 3];
var doubled = nums.map(|[U] x| -> int { return x * 2; });
var strings = nums.map(|[U] x| -> string { return x.to_string(); });
var evens = nums.filter(|x| -> bool { return x % 2 == 0; });
var sum = nums.reduce(|[U] a, b| -> int { return a + b; }, 0);
```

---

## 9. 实现路线图

### Phase 1：基础类型扩展（2周）

| 周次 | 任务 | 文件 |
|------|------|------|
| 1 | 扩展 TypeExpr，实现类型替换 | `type_expr.rs` |
| 2 | Parser 扩展（统一 [] 语法） | `parser.rs` |

### Phase 2：类型检查（2周）

| 周次 | 任务 | 文件 |
|------|------|------|
| 3 | 扩展 TypeChecker，泛型定义检查 | `type_checker.rs` |
| 4 | 泛型实例化检查 | `type_checker.rs` |

### Phase 3：类型推导（2周）

| 周次 | 任务 | 文件 |
|------|------|------|
| 5 | 实现 unify 算法 | `constraint_solver.rs` |
| 6 | Lambda 和调用点类型推导 | `type_checker.rs` |

### Phase 4：单态化（2周）

| 周次 | 任务 | 文件 |
|------|------|------|
| 7 | 实现 Monomorphizer | `monomorphizer.rs` |
| 8 | 编译器集成 | `compiler/` |

### Phase 5：内置方法（2周）

| 周次 | 任务 | 文件 |
|------|------|------|
| 9 | List[T] 泛型方法类型定义 | `builtin_generics.rs` |
| 10 | 链式调用类型推导 | `type_checker.rs` |

### Phase 6：完成（2周）

| 周次 | 任务 |
|------|------|
| 11 | 性能优化（实例化缓存） |
| 12 | 文档和示例 |

---

## 10. 关键测试用例

```rust
#[test]
fn test_generic_lambda() {
    let code = r#"
        var identity = |[T] x: T| -> T { return x; };
        var x = identity(42);
    "#;
    assert!(parse_and_check(code).is_ok());
}

#[test]
fn test_generic_struct() {
    let code = r#"
        struct Box[T] { value: T; }
        var b = Box[int] { value: 42 };
    "#;
    assert!(parse_and_check(code).is_ok());
}

#[test]
fn test_generic_list_map() {
    let code = r#"
        var nums: List[int] = [1, 2, 3];
        var strings = nums.map(|[U] x| -> string { return x.to_string(); });
    "#;
    assert!(parse_and_check(code).is_ok());
}

#[test]
fn test_generic_chain() {
    let code = r#"
        var nums: List[int] = [1, 2, 3, 4, 5];
        var result = nums.filter(|x| -> bool { return x > 2; })
                         .map(|[U] x| -> int { return x * 2; });
    "#;
    assert!(parse_and_check(code).is_ok());
}

#[test]
fn test_generic_impl() {
    let code = r#"
        struct Box[T] { value: T; }
        impl[T] Box[T] {
            get: |self| -> T { return self.value; },
        }
        var b = Box[int] { value: 42 };
        var x = b.get();
    "#;
    assert!(parse_and_check(code).is_ok());
}

#[test]
fn test_nested_generic() {
    let code = r#"
        struct Box[T] { value: T; }
        var b: Box[List[int]] = Box[List[int]] { value: [1, 2, 3] };
    "#;
    assert!(parse_and_check(code).is_ok());
}

#[test]
fn test_type_mismatch() {
    let code = r#"
        struct Box[T] { value: T; }
        var b: Box[int] = Box[int] { value: "hello" };
    "#;
    assert!(parse_and_check(code).is_err());
}
```

---

## 11. 设计决策总结

| 决策 | 选择 | 理由 |
|------|------|------|
| 泛型语法 | 统一 `[]` | 语法一致，无歧义，实现简单 |
| Lambda 类型参数 | `[T]` 在 `||` 内 | 紧凑，作用域清晰 |
| 类型定义类型参数 | `[T]` 在名称后 | 与使用一致：Box[T] |
| Lambda 语法 | 完整形式 | `\|[T] x: T\| -> T { return x; }` |
| 单态化策略 | 编译时展开 | 零运行时开销 |
| 类型约束 | Phase 2 | `[T: Add]` 未来扩展 |
| 默认类型参数 | 不支持 | 保持简单，避免过度设计 |

---

---

## 12. 详细实施计划

### 12.1 实施概览

| 阶段 | 周期 | 目标 | 产出 |
|------|------|------|------|
| Phase 0 | 1周 | 准备和基础重构 | 可扩展的 TypeExpr |
| Phase 1 | 2周 | Parser 支持泛型语法 | 可解析泛型代码 |
| Phase 2 | 2周 | 类型检查基础 | 可检查泛型定义 |
| Phase 3 | 2周 | 类型推导 | 自动推导类型参数 |
| Phase 4 | 2周 | 单态化和代码生成 | 可执行泛型代码 |
| Phase 5 | 2周 | 内置泛型方法 | List[T] 方法泛型化 |
| Phase 6 | 1周 | 优化和文档 | 生产可用 |

**总计：约 12 周**

---

### 12.2 Phase 0：基础重构（1周）

**目标**：为泛型系统打好基础，不破坏现有功能

#### Week 0 任务清单

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 重构 TypeExpr | `type_expr.rs` | 添加 `TypeParam` 和 `GenericInstance`，确保向后兼容 |
| P0 | 类型替换基础 | `type_expr.rs` | 实现 `substitute()` 方法，支持嵌套类型替换 |
| P1 | 添加类型参数收集 | `type_expr.rs` | 实现 `collect_type_params()` 用于约束检查 |
| P1 | 单元测试 | `type_expr.rs` (tests) | 测试替换算法：List[T] → List[int] |

**关键代码变更**：
```rust
// type_expr.rs - 扩展枚举
pub enum TypeExpr {
    // ... 现有类型 ...
    TypeParam(TypeParam),           // NEW
    GenericInstance(GenericInstance), // NEW
}
```

**验收标准**：
- [ ] 所有现有测试通过
- [ ] 新增类型表达式单元测试通过
- [ ] 类型替换算法测试覆盖 100%

---

### 12.3 Phase 1：Parser 扩展（2周）

**目标**：Parser 能正确解析所有泛型语法

#### Week 1：基础语法解析

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 解析 [T] 类型参数 | `parser.rs` | 新增 `parse_type_params_bracket()` |
| P0 | 泛型 struct 解析 | `parser.rs` | 修改 `parse_struct_statement()` 支持 `[T]` |
| P0 | 泛型 impl 解析 | `parser.rs` | 修改 `parse_impl_statement()` 支持 `[T]` |
| P1 | 泛型 lambda 解析 | `parser.rs` | 修改 `parse_lambda()` 支持 `|[T] x: T|` |

#### Week 2：类型表达式和实例化

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 泛型类型表达式 | `parser.rs` | `parse_type_expression()` 支持 `Box[T]` |
| P0 | 嵌套泛型解析 | `parser.rs` | 支持 `Box[List[T]]` |
| P1 | 泛型 struct 字面量 | `parser.rs` | 支持 `Box[int] { value: 42 }` |
| P1 | 错误处理 | `parser.rs` | 友好的泛型语法错误提示 |
| P2 | Parser 测试 | `parser.rs` (tests) | 覆盖所有泛型语法场景 |

**关键代码变更**：
```rust
// parser.rs
fn parse_lambda(&mut self) -> ParseResult<Expr> {
    self.expect(KauboTokenKind::Pipe)?;
    
    // 检查类型参数 [T]
    let type_params = if self.check(KauboTokenKind::LeftBracket) {
        self.parse_lambda_type_params()?
    } else { vec![] };
    
    // ... 原有逻辑 ...
}
```

**验收标准**：
- [ ] 所有 Phase 0 测试通过
- [ ] 新增 20+ Parser 测试用例
- [ ] 可解析文档中所有示例代码

---

### 12.4 Phase 2：类型检查基础（2周）

**目标**：TypeChecker 能检查泛型定义和实例化

#### Week 3：泛型定义检查

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 扩展 TypeChecker 结构 | `type_checker.rs` | 添加 `generic_structs`, `type_param_stack` |
| P0 | 类型参数作用域 | `type_checker.rs` | 实现 `enter/exit_generic_scope()` |
| P0 | 泛型 struct 检查 | `type_checker.rs` | `check_struct_def()` 支持泛型 |
| P1 | 类型表达式检查 | `type_checker.rs` | `check_type_expr()` 验证类型参数 |

#### Week 4：实例化和 impl 检查

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 泛型实例化 | `type_checker.rs` | `instantiate_generic_struct()` |
| P0 | 实例缓存 | `type_checker.rs` | 避免重复创建相同实例 |
| P1 | 泛型 impl 检查 | `type_checker.rs` | `check_impl_def()` 支持泛型 |
| P1 | 泛型 lambda 检查 | `type_checker.rs` | `check_lambda()` 支持 `[T]` |
| P2 | 类型错误报告 | `type_checker.rs` | 友好的泛型类型错误 |

**关键代码变更**：
```rust
// type_checker.rs
pub struct TypeChecker {
    // ... 现有字段 ...
    generic_structs: HashMap<String, GenericDef>,
    type_param_stack: Vec<HashMap<String, TypeParam>>,
}

impl TypeChecker {
    fn instantiate_generic_struct(&mut self, name: &str, type_args: &[TypeExpr]) 
        -> TypeCheckResult<(Vec<(String, TypeExpr)>, u16)> {
        // 创建替换映射 {T → int}
        // 应用替换
        // 创建/获取 shape
    }
}
```

**验收标准**：
- [ ] 可检查泛型 struct 定义
- [ ] 可检查泛型实例化
- [ ] 类型不匹配错误正确报告
- [ ] 所有 Phase 1 测试通过

---

### 12.5 Phase 3：类型推导（2周）

**目标**：从调用点自动推导类型参数

#### Week 5：约束求解基础

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 约束结构 | `constraint_solver.rs` (new) | 定义 `Constraint` 结构 |
| P0 | unify 算法 | `constraint_solver.rs` | 实现类型统一核心算法 |
| P0 | Occurs Check | `constraint_solver.rs` | 防止无限类型 T = List[T] |
| P1 | 替换组合 | `constraint_solver.rs` | 多约束组合求解 |
| P1 | 单元测试 | `constraint_solver.rs` (tests) | 测试 unify 各种场景 |

#### Week 6：调用点推导

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 约束收集 | `type_checker.rs` | 从调用参数收集约束 |
| P0 | 类型参数推导 | `type_checker.rs` | `infer_type_args()` 实现 |
| P1 | 上下文敏感推导 | `type_checker.rs` | 利用期望返回类型推导 |
| P1 | 泛型方法推导 | `type_checker.rs` | `check_member_access()` 泛型方法 |
| P2 | 推导错误报告 | `type_checker.rs` | 无法推导时友好提示 |

**关键代码变更**：
```rust
// constraint_solver.rs
pub fn unify(t1: &TypeExpr, t2: &TypeExpr) 
    -> Result<HashMap<String, TypeExpr>, String> {
    match (t1, t2) {
        (TypeExpr::TypeParam(p), _) => bind(&p.name, t2),
        (TypeExpr::List(e1), TypeExpr::List(e2)) => unify(e1, e2),
        // ... 其他类型 ...
    }
}
```

**验收标准**：
- [ ] `identity(42)` 推导为 `identity[int]`
- [ ] `pair(1, "a")` 推导为 `pair[int, string]`
- [ ] 嵌套泛型推导正确
- [ ] 推导失败时错误信息清晰

---

### 12.6 Phase 4：单态化和代码生成（2周）

**目标**：泛型代码能编译执行

#### Week 7：单态化管理器

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | Monomorphizer 结构 | `monomorphizer.rs` (new) | 管理实例化过程 |
| P0 | 名称修饰 | `monomorphizer.rs` | `mangle_name()` Box[T] → Box$T |
| P0 | 实例化缓存 | `monomorphizer.rs` | 避免重复实例化 |
| P1 | Lambda 实例化 | `monomorphizer.rs` | 替换类型参数生成具体 lambda |
| P1 | AST 类型替换 | `monomorphizer.rs` | 深度遍历替换类型标注 |

#### Week 8：编译器集成

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 编译器接口 | `compiler/mod.rs` | 集成 Monomorphizer |
| P0 | 延迟实例化 | `compiler/mod.rs` | 收集实例化任务 |
| P0 | 具体代码生成 | `compiler/` | 编译实例化后的代码 |
| P1 | 泛型调用编译 | `compiler/expr.rs` | 解析为具体函数引用 |
| P2 | 端到端测试 | `tests/` | 完整流程测试 |

**关键代码变更**：
```rust
// monomorphizer.rs
pub struct Monomorphizer {
    instantiated_functions: HashMap<(String, Vec<TypeExpr>), String>,
    pending: Vec<InstantiationTask>,
}

impl Monomorphizer {
    pub fn instantiate_lambda(&self, lambda: &Lambda, type_args: &[TypeExpr]) -> Lambda {
        // 替换类型参数
        // 返回具体 lambda
    }
}
```

**验收标准**：
- [ ] 泛型代码可编译执行
- [ ] 单测：identity(42) 返回 42
- [ ] 单测：Box[int] 正确使用
- [ ] 性能：实例化缓存有效

---

### 12.7 Phase 5：内置泛型方法（2周）

**目标**：List[T] 等方法支持泛型类型推导

#### Week 9：内置方法类型定义

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 泛型方法类型表 | `builtin_generics.rs` (new) | 定义 List[T].map 等类型 |
| P0 | map 方法类型 | `builtin_generics.rs` | `|[U] self, f: |T| -> U| -> List[U]` |
| P0 | filter 方法类型 | `builtin_generics.rs` | `|self, pred: |T| -> bool| -> List[T]` |
| P0 | reduce 方法类型 | `builtin_generics.rs` | `|[U] self, op: |U, T| -> U, U| -> U` |
| P1 | 方法类型实例化 | `builtin_generics.rs` | 根据 T 实例化方法类型 |

#### Week 10：链式调用推导

| 优先级 | 任务 | 文件 | 详细说明 |
|--------|------|------|----------|
| P0 | 成员访问推导 | `type_checker.rs` | `check_member_access()` 泛型方法 |
| P0 | 链式调用类型传递 | `type_checker.rs` | `list.filter().map()` 类型传递 |
| P1 | 复杂推导场景 | `type_checker.rs` | `nums.map(|x| -> U { ... })` |
| P2 | 内置方法测试 | `tests/` | 覆盖所有内置泛型方法 |

**关键代码变更**：
```rust
// builtin_generics.rs
impl BuiltinGenericMethods {
    pub fn list_method_type(method: &str, elem_type: &TypeExpr) -> Option<TypeExpr> {
        match method {
            "map" => Some(TypeExpr::Function(FunctionType {
                params: vec![
                    TypeExpr::list(elem_type.clone()),
                    TypeExpr::Function(FunctionType {
                        params: vec![elem_type.clone()],
                        return_type: Some(Box::new(TypeExpr::TypeParam(TypeParam { name: "U", .. }))),
                    }),
                ],
                return_type: Some(Box::new(TypeExpr::list(TypeExpr::TypeParam(...)))),
            })),
            // ...
        }
    }
}
```

**验收标准**：
- [ ] `nums.map(|x| -> int { return x * 2; })` 类型正确
- [ ] `nums.filter(|x| -> bool { return x > 0; })` 类型正确
- [ ] 链式调用 `list.filter().map()` 类型传递正确
- [ ] 所有现有 List 方法测试通过

---

### 12.8 Phase 6：优化和文档（1周）

**目标**：生产可用，文档完善

#### Week 11-12 任务

| 优先级 | 任务 | 说明 |
|--------|------|------|
| P0 | 实例化缓存优化 | 避免重复编译相同实例 |
| P0 | 编译时间优化 | 惰性实例化，按需编译 |
| P1 | 错误信息优化 | 泛型类型错误友好提示 |
| P1 | 用户文档 | 泛型语法指南，最佳实践 |
| P2 | 性能测试 | 对比泛型前后性能 |
| P2 | 示例代码 | 提供 5+ 完整示例 |

**验收标准**：
- [ ] 编译时间增加 < 20%
- [ ] 运行时性能无退化
- [ ] 文档完整，示例可运行
- [ ] 所有测试通过

---

### 12.9 文件修改清单

#### 修改现有文件

| 文件 | 修改内容 | 预估行数 |
|------|----------|----------|
| `type_expr.rs` | 扩展 TypeExpr，添加类型替换 | +200 |
| `stmt.rs` | 扩展 StructStmt, ImplStmt | +50 |
| `expr.rs` | 扩展 Lambda | +30 |
| `parser.rs` | 泛型语法解析 | +400 |
| `type_checker.rs` | 泛型类型检查核心 | +600 |
| `compiler/mod.rs` | 集成单态化 | +200 |

#### 新增文件

| 文件 | 用途 | 预估行数 |
|------|------|----------|
| `constraint_solver.rs` | 类型约束求解 | +300 |
| `monomorphizer.rs` | 单态化管理 | +400 |
| `builtin_generics.rs` | 内置泛型方法类型 | +200 |

**总计：约 2400 行新增代码**

---

### 12.10 依赖关系图

```
Phase 0 (基础重构)
    ↓
Phase 1 (Parser)
    ↓
Phase 2 (类型检查)
    ↓
Phase 3 (类型推导) ←→ Phase 4 (单态化)
    ↓
Phase 5 (内置方法)
    ↓
Phase 6 (优化文档)
```

**关键路径**：Phase 0 → Phase 1 → Phase 2 → Phase 4 → Phase 5 → Phase 6

**可并行**：Phase 3 和 Phase 4 部分工作可并行

---

### 12.11 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| Parser 歧义 | 中 | 高 | 充分测试，使用 [T] 语法避免冲突 |
| 类型推导失败 | 低 | 中 | 良好的错误信息，显式标注回退 |
| 代码膨胀 | 中 | 中 | 实例化缓存，限制嵌套深度 |
| 编译时间增加 | 中 | 低 | 惰性实例化，增量编译 |
| 与现有代码冲突 | 低 | 高 | 渐进式实现，保持向后兼容 |

---

### 12.12 混合编译策略：单态化 + 类型擦除

为避免代码过度膨胀，采用**混合策略**：

| 类型类别 | 策略 | 说明 |
|----------|------|------|
| **基本类型** (int, float, bool) | 单态化 | 零开销，代码量小 |
| **堆类型** (struct, List, String) | 类型擦除 | 避免代码膨胀，统一处理 |

#### 基本类型单态化

```kaubo
// 为每个基本类型生成独立函数
|[T] identity(x: T) -> T { return x; }

// 编译后
identity$int      // int 版本
identity$float    // float 版本
identity$bool     // bool 版本
```

#### 堆类型类型擦除

```kaubo
// struct 和集合类型走类型擦除
struct Box[T] { value: T; }
impl[T] Box[T] {
    get: |self| -> T { return self.value; }
}

// 编译后：统一使用 any/void* 表示
Box$get: |self: Box$any| -> any {
    return self.value;  // 运行时类型检查
}
```

#### 实现要点

```rust
// monomorphizer.rs
impl Monomorphizer {
    /// 判断类型是否是基本类型
    fn is_primitive_type(&self, ty: &TypeExpr) -> bool {
        match ty {
            TypeExpr::Named(n) => matches!(n.name.as_str(),
                "int" | "float" | "bool" | "string"  // string 是否基本类型？
            ),
            _ => false,
        }
    }
    
    /// 决定编译策略
    fn select_strategy(&self, type_args: &[TypeExpr]) -> CompilationStrategy {
        if type_args.iter().all(|t| self.is_primitive_type(t)) {
            CompilationStrategy::Monomorphize  // 单态化
        } else {
            CompilationStrategy::TypeErasure   // 类型擦除
        }
    }
}

enum CompilationStrategy {
    Monomorphize,   // 生成独立函数
    TypeErasure,    // 使用统一表示
}
```

#### 类型擦除实现

```rust
// 运行时类型信息
pub struct TypeInfo {
    type_id: u64,
    size: usize,
    align: usize,
    // 方法表指针等
}

// 擦除后的值表示
pub struct ErasedValue {
    data: *mut c_void,      // 实际数据指针
    type_info: *const TypeInfo,  // 类型信息
}

// 编译泛型函数为擦除版本
fn compile_erased_lambda(lambda: &Lambda) -> Function {
    // 参数和返回类型统一为 ErasedValue
    // 内部根据 type_info 进行类型检查和转换
}
```

#### 实例化缓存仍然必要

即使采用混合策略，**实例化缓存仍然重要**：

| 场景 | 缓存策略 | 说明 |
|------|----------|------|
| **基本类型单态化** | ✅ 需要缓存 | int/float/bool 种类有限，缓存收益高 |
| **堆类型类型擦除** | ❌ 不需要缓存 | 统一走擦除路径，无实例化 |

```rust
// 基本类型缓存示例
identity(42)      // 第1次：编译 identity$int，缓存
identity(100)     // 第2次：命中缓存，直接复用
identity(x)       // 第3次：命中缓存

// 堆类型统一处理
Box[int] { ... }  // 走 Box$any 擦除路径，不缓存
Box[string] { ... }  // 同样走 Box$any，无额外代码生成
```

**缓存收益**：
- 基本类型种类有限（通常 < 10 种）
- 相同类型组合重复出现频繁
- 编译时间减少 30-50%

#### 优缺点

| 方案 | 优点 | 缺点 |
|------|------|------|
| **纯单态化** | 零运行时开销 | 堆类型导致代码极度膨胀 |
| **纯类型擦除** | 代码体积小 | 基本类型也有运行时开销 |
| **混合策略** | 兼顾性能和体积 | 实现复杂，需要判断类型类别 |

**建议**：默认混合策略，配合基本类型实例化缓存

---

### 12.13 最小可运行版本（MVP）

如果资源有限，可优先实现 MVP（约 6 周）：

| 阶段 | 范围 | 产出 |
|------|------|------|
| MVP 0 | Phase 0 + Phase 1 | 可解析泛型语法 |
| MVP 1 | Phase 2 (简化) | 可检查显式泛型 |
| MVP 2 | Phase 4 (简化，仅基本类型) | 可执行显式泛型 |

**MVP 语法范围**：
```kaubo
// 支持（基本类型单态化）
|[T] identity(x: T) -> T { return x; }  // T 只能是 int/float/bool
var x = identity(42);  // 生成 identity$int

// 不支持（后续迭代）
struct Box[T] { ... }   // 堆类型类型擦除
list.map(|x| ...)       // 内置泛型方法
```

---

*文档版本：2.3 | 最后更新：2026-02-16*
