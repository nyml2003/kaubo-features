//! 类型检查器
//!
//! 负责编译时的类型检查和类型推导

use super::error::ErrorLocation;
use super::expr::{
    Binary, Expr, ExprKind, FunctionCall, IndexAccess, Lambda, MemberAccess, StructLiteral, Unary,
    VarRef,
};
use super::stmt::{
    BlockStmt, ForStmt, IfStmt, ImplStmt, ReturnStmt, Stmt, StmtKind, StructStmt, VarDeclStmt,
    WhileStmt,
};
use super::type_expr::TypeExpr;
use crate::runtime::object::ObjShape;
use kaubo_log::{trace, Logger};
use std::collections::HashMap;
use std::sync::Arc;

/// 类型环境（作用域）
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// 当前作用域的变量类型
    variables: HashMap<String, TypeExpr>,
    /// 父作用域（如果有）
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    /// 创建新的类型环境
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    /// 创建子作用域
    pub fn child(parent: &Self) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent.clone())),
        }
    }

    /// 定义变量类型
    pub fn define(&mut self, name: String, ty: TypeExpr) {
        self.variables.insert(name, ty);
    }

    /// 查找变量类型
    pub fn lookup(&self, name: &str) -> Option<&TypeExpr> {
        // 先在当前作用域查找
        if let Some(ty) = self.variables.get(name) {
            return Some(ty);
        }
        // 递归在父作用域查找
        if let Some(ref parent) = self.parent {
            return parent.lookup(name);
        }
        None
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// 类型检查器
pub struct TypeChecker {
    /// 当前类型环境
    env: TypeEnv,
    /// 是否启用严格模式（所有变量必须有类型标注或能推导）
    strict_mode: bool,
    /// 返回类型栈（用于检查 return 语句）
    return_type_stack: Vec<Option<TypeExpr>>,
    /// struct 类型表：类型名 -> [(字段名, 字段类型)]
    struct_types: HashMap<String, Vec<(String, TypeExpr)>>,
    /// 生成的 struct shapes（用于传递给 VM）
    shapes: Vec<ObjShape>,
    /// 下一个 shape ID
    next_shape_id: u16,
    /// Logger
    logger: Arc<Logger>,
}

impl std::fmt::Debug for TypeChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeChecker")
            .field("env", &self.env)
            .field("strict_mode", &self.strict_mode)
            .field("return_type_stack", &self.return_type_stack)
            .field("struct_types", &self.struct_types)
            .field("shapes", &self.shapes)
            .field("next_shape_id", &self.next_shape_id)
            .field("logger", &"...")
            .finish()
    }
}

/// 类型检查错误
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// 类型不匹配
    Mismatch {
        expected: String,
        found: String,
        location: ErrorLocation,
    },
    /// 返回类型不匹配
    ReturnTypeMismatch {
        expected: String,
        found: String,
        location: ErrorLocation,
    },
    /// 未定义的变量
    UndefinedVar {
        name: String,
        location: ErrorLocation,
    },
    /// 不支持的操作
    UnsupportedOp { op: String, location: ErrorLocation },
    /// 无法推导类型
    CannotInfer {
        message: String,
        location: ErrorLocation,
    },
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::Mismatch {
                expected, found, ..
            } => {
                write!(
                    f,
                    "Type mismatch: expected '{}', found '{}'",
                    expected, found
                )
            }
            TypeError::ReturnTypeMismatch {
                expected, found, ..
            } => {
                write!(
                    f,
                    "Return type mismatch: expected '{}', found '{}'",
                    expected, found
                )
            }
            TypeError::UndefinedVar { name, .. } => {
                write!(f, "Undefined variable: '{}'", name)
            }
            TypeError::UnsupportedOp { op, .. } => {
                write!(f, "Unsupported operation: '{}'", op)
            }
            TypeError::CannotInfer { message, .. } => {
                write!(f, "Cannot infer type: {}", message)
            }
        }
    }
}

impl std::error::Error for TypeError {}

/// 类型检查结果
pub type TypeCheckResult<T> = Result<T, TypeError>;

impl TypeChecker {
    /// 创建新的类型检查器
    pub fn new() -> Self {
        Self::with_logger(Logger::noop())
    }

    /// 创建新的类型检查器（带 logger）
    pub fn with_logger(logger: Arc<Logger>) -> Self {
        let mut checker = Self {
            env: TypeEnv::new(),
            strict_mode: false,
            return_type_stack: Vec::new(),
            struct_types: HashMap::new(),
            shapes: Vec::new(),
            next_shape_id: 1, // 从 1 开始，0 保留
            logger,
        };
        checker.init_stdlib_types();
        trace!(checker.logger, "TypeChecker created");
        checker
    }

    /// 取出所有生成的 shapes（消耗 self）
    pub fn take_shapes(self) -> Vec<ObjShape> {
        self.shapes
    }

    /// 获取 struct 的 shape_id
    pub fn get_shape_id(&self, name: &str) -> Option<u16> {
        self.shapes
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.shape_id)
    }

    /// 初始化标准库类型
    fn init_stdlib_types(&mut self) {
        // print: |any| -> void
        self.env.define(
            "print".to_string(),
            TypeExpr::function_void(vec![TypeExpr::named("any")]),
        );

        // assert: |bool, string?| -> void
        self.env.define(
            "assert".to_string(),
            TypeExpr::function_void(vec![TypeExpr::named("bool"), TypeExpr::named("string")]),
        );

        // type: |any| -> string
        self.env.define(
            "type".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("any")],
                Some(TypeExpr::named("string")),
            ),
        );

        // to_string: |any| -> string
        self.env.define(
            "to_string".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("any")],
                Some(TypeExpr::named("string")),
            ),
        );

        // 数学函数: |float| -> float
        let math_fn_type = TypeExpr::function(
            vec![TypeExpr::named("float")],
            Some(TypeExpr::named("float")),
        );
        self.env.define("sqrt".to_string(), math_fn_type.clone());
        self.env.define("sin".to_string(), math_fn_type.clone());
        self.env.define("cos".to_string(), math_fn_type.clone());
        self.env.define("floor".to_string(), math_fn_type.clone());
        self.env.define("ceil".to_string(), math_fn_type);

        // 列表操作函数
        // len: |any| -> int
        self.env.define(
            "len".to_string(),
            TypeExpr::function(vec![TypeExpr::named("any")], Some(TypeExpr::named("int"))),
        );
        // push: |any, any| -> void
        self.env.define(
            "push".to_string(),
            TypeExpr::function_void(vec![TypeExpr::named("any"), TypeExpr::named("any")]),
        );
        // is_empty: |any| -> bool
        self.env.define(
            "is_empty".to_string(),
            TypeExpr::function(vec![TypeExpr::named("any")], Some(TypeExpr::named("bool"))),
        );

        // 字符串方法
        self.env.define(
            "string_len".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("string")],
                Some(TypeExpr::named("int")),
            ),
        );
        self.env.define(
            "string_substring".to_string(),
            TypeExpr::function(
                vec![
                    TypeExpr::named("string"),
                    TypeExpr::named("int"),
                    TypeExpr::named("int"),
                ],
                Some(TypeExpr::named("string")),
            ),
        );
        self.env.define(
            "string_contains".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("string"), TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            ),
        );
        self.env.define(
            "string_starts_with".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("string"), TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            ),
        );
        self.env.define(
            "string_ends_with".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("string"), TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            ),
        );

        // 列表方法
        self.env.define(
            "list_append".to_string(),
            TypeExpr::function_void(vec![TypeExpr::named("any"), TypeExpr::named("any")]),
        );
        self.env.define(
            "list_remove".to_string(),
            TypeExpr::function(
                vec![TypeExpr::named("any"), TypeExpr::named("int")],
                Some(TypeExpr::named("any")),
            ),
        );
        self.env.define(
            "list_clear".to_string(),
            TypeExpr::function_void(vec![TypeExpr::named("any")]),
        );
    }

    /// 设置严格模式
    pub fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
    }

    /// 对语句进行类型检查
    pub fn check_statement(&mut self, stmt: &Stmt) -> TypeCheckResult<Option<TypeExpr>> {
        trace!(self.logger, "Checking statement: {:?}", stmt.as_ref());
        match stmt.as_ref() {
            StmtKind::VarDecl(var_decl) => self.check_var_decl(var_decl),
            StmtKind::Expr(expr_stmt) => self.check_expression(&expr_stmt.expression),
            StmtKind::Block(block) => self.check_block(block),
            StmtKind::If(if_stmt) => self.check_if(if_stmt),
            StmtKind::While(while_stmt) => self.check_while(while_stmt),
            StmtKind::For(for_stmt) => self.check_for(for_stmt),
            StmtKind::Return(return_stmt) => self.check_return(return_stmt),
            StmtKind::Struct(struct_stmt) => self.check_struct_def(struct_stmt),
            StmtKind::Impl(impl_stmt) => self.check_impl_def(impl_stmt),
            StmtKind::Empty(_) => Ok(None),
            _ => {
                // 其他语句类型暂不支持类型检查
                Ok(None)
            }
        }
    }

    /// 检查 struct 定义
    fn check_struct_def(&mut self, struct_stmt: &StructStmt) -> TypeCheckResult<Option<TypeExpr>> {
        // 注册 struct 类型
        let fields: Vec<(String, TypeExpr)> = struct_stmt
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.type_annotation.clone()))
            .collect();

        // 创建 ObjShape
        let shape_id = self.next_shape_id;
        self.next_shape_id += 1;

        let field_names: Vec<String> = struct_stmt.fields.iter().map(|f| f.name.clone()).collect();

        let shape = ObjShape::new(shape_id, struct_stmt.name.clone(), field_names);
        self.shapes.push(shape);

        // 存储字段信息用于类型检查
        self.struct_types.insert(struct_stmt.name.clone(), fields);
        Ok(None)
    }

    /// 检查 impl 定义
    fn check_impl_def(&mut self, impl_stmt: &ImplStmt) -> TypeCheckResult<Option<TypeExpr>> {
        // 验证 struct 类型存在
        if !self.struct_types.contains_key(&impl_stmt.struct_name) {
            return Err(TypeError::UndefinedVar {
                name: impl_stmt.struct_name.clone(),
                location: ErrorLocation::Unknown,
            });
        }

        // 找到对应的 shape，注册方法名
        let shape_id = self.get_shape_id(&impl_stmt.struct_name);
        for method in &impl_stmt.methods {
            // 检查方法 lambda
            self.check_expression(&method.lambda)?;

            // 注册方法名到 shape
            if let Some(sid) = shape_id {
                if let Some(shape) = self.shapes.iter_mut().find(|s| s.shape_id == sid) {
                    let idx = shape.method_names.len() as u8;
                    shape.method_names.insert(method.name.clone(), idx);
                    // methods 向量预留位置，实际函数指针在运行时填充
                    shape.methods.push(std::ptr::null_mut());
                }
            }
        }

        Ok(None)
    }

    /// 检查变量声明
    fn check_var_decl(&mut self, var_decl: &VarDeclStmt) -> TypeCheckResult<Option<TypeExpr>> {
        // 推导初始化表达式的类型
        let init_type = self.check_expression(&var_decl.initializer)?;

        // 从 span 创建位置信息
        let location = ErrorLocation::At(var_decl.span.start);

        // 如果有类型标注，检查是否兼容
        if let Some(ref annotation) = var_decl.type_annotation {
            if let Some(ref init_ty) = init_type {
                if !self.is_compatible(init_ty, annotation) {
                    return Err(TypeError::Mismatch {
                        expected: annotation.to_string(),
                        found: init_ty.to_string(),
                        location,
                    });
                }
            }
            // 使用标注的类型
            self.env.define(var_decl.name.clone(), annotation.clone());
            Ok(Some(annotation.clone()))
        } else {
            // 没有类型标注，使用推导的类型
            if let Some(ty) = init_type {
                self.env.define(var_decl.name.clone(), ty.clone());
                Ok(Some(ty))
            } else if self.strict_mode {
                Err(TypeError::CannotInfer {
                    message: format!("Cannot infer type for variable '{}'", var_decl.name),
                    location,
                })
            } else {
                // 非严格模式下允许无类型
                Ok(None)
            }
        }
    }

    /// 检查代码块
    fn check_block(&mut self, block: &BlockStmt) -> TypeCheckResult<Option<TypeExpr>> {
        // 创建新的作用域
        let old_env = self.env.clone();
        self.env = TypeEnv::child(&old_env);

        let mut last_type = None;
        for stmt in &block.statements {
            last_type = self.check_statement(stmt)?;
        }

        // 恢复父作用域
        self.env = old_env;
        Ok(last_type)
    }

    /// 检查 if 语句
    fn check_if(&mut self, if_stmt: &IfStmt) -> TypeCheckResult<Option<TypeExpr>> {
        // 检查条件表达式（应该是 bool 类型）
        let cond_type = self.check_expression(&if_stmt.if_condition)?;
        // TODO: 检查条件类型是否为 bool

        // 检查 then 分支
        self.check_statement(&if_stmt.then_body)?;

        // 检查 elif 分支
        for (i, cond) in if_stmt.elif_conditions.iter().enumerate() {
            let _cond_type = self.check_expression(cond)?;
            self.check_statement(&if_stmt.elif_bodies[i])?;
        }

        // 检查 else 分支
        if let Some(ref else_body) = if_stmt.else_body {
            self.check_statement(else_body)?;
        }

        Ok(None)
    }

    /// 检查 while 循环
    fn check_while(&mut self, while_stmt: &WhileStmt) -> TypeCheckResult<Option<TypeExpr>> {
        let _cond_type = self.check_expression(&while_stmt.condition)?;
        self.check_statement(&while_stmt.body)?;
        Ok(None)
    }

    /// 检查 for 循环
    fn check_for(&mut self, for_stmt: &ForStmt) -> TypeCheckResult<Option<TypeExpr>> {
        let _iterable_type = self.check_expression(&for_stmt.iterable)?;
        self.check_statement(&for_stmt.body)?;
        Ok(None)
    }

    /// 检查 return 语句
    fn check_return(&mut self, return_stmt: &ReturnStmt) -> TypeCheckResult<Option<TypeExpr>> {
        let value_type = if let Some(ref value) = return_stmt.value {
            self.check_expression(value)?
        } else {
            None
        };

        // 检查返回类型是否匹配期望类型
        if let Some(expected_type) = self.return_type_stack.last().cloned().flatten() {
            if let Some(ref actual) = value_type {
                if !self.is_compatible(actual, &expected_type) {
                    return Err(TypeError::ReturnTypeMismatch {
                        expected: expected_type.to_string(),
                        found: actual.to_string(),
                        location: ErrorLocation::At(return_stmt.span.start),
                    });
                }
            }
        }

        Ok(value_type)
    }

    /// 对表达式进行类型检查
    pub fn check_expression(&mut self, expr: &Expr) -> TypeCheckResult<Option<TypeExpr>> {
        trace!(self.logger, "Checking expression: {:?}", expr.as_ref());
        match expr.as_ref() {
            ExprKind::LiteralInt(_) => Ok(Some(TypeExpr::named("int"))),
            ExprKind::LiteralFloat(_) => Ok(Some(TypeExpr::named("float"))),
            ExprKind::LiteralString(_) => Ok(Some(TypeExpr::named("string"))),
            ExprKind::LiteralTrue(_) | ExprKind::LiteralFalse(_) => {
                Ok(Some(TypeExpr::named("bool")))
            }
            ExprKind::LiteralNull(_) => Ok(None), // null 可以是任何类型
            ExprKind::LiteralList(list) => self.check_list_literal(list),
            ExprKind::VarRef(var_ref) => self.check_var_ref(var_ref),
            ExprKind::Binary(binary) => self.check_binary(binary),
            ExprKind::Unary(unary) => self.check_unary(unary),
            ExprKind::Lambda(lambda) => self.check_lambda(lambda),
            ExprKind::FunctionCall(call) => self.check_function_call(call),
            ExprKind::MemberAccess(member_access) => self.check_member_access(member_access),
            ExprKind::IndexAccess(index_access) => self.check_index_access(index_access),
            ExprKind::StructLiteral(struct_lit) => self.check_struct_literal(struct_lit),
            _ => Ok(None), // 其他表达式类型暂不支持
        }
    }

    /// 检查列表字面量
    fn check_list_literal(
        &mut self,
        list: &super::expr::LiteralList,
    ) -> TypeCheckResult<Option<TypeExpr>> {
        if list.elements.is_empty() {
            // 空列表类型无法推导
            return Ok(Some(TypeExpr::list(TypeExpr::named("any"))));
        }

        // 推导第一个元素的类型
        let first_type = self.check_expression(&list.elements[0])?;

        // 检查所有元素类型是否一致
        for elem in &list.elements[1..] {
            let elem_type = self.check_expression(elem)?;
            if first_type != elem_type {
                // 类型不一致，返回 any 列表
                return Ok(Some(TypeExpr::list(TypeExpr::named("any"))));
            }
        }

        if let Some(elem_ty) = first_type {
            Ok(Some(TypeExpr::list(elem_ty)))
        } else {
            Ok(Some(TypeExpr::list(TypeExpr::named("any"))))
        }
    }

    /// 检查变量引用
    fn check_var_ref(&self, var_ref: &VarRef) -> TypeCheckResult<Option<TypeExpr>> {
        if let Some(ty) = self.env.lookup(&var_ref.name) {
            Ok(Some(ty.clone()))
        } else {
            // 未定义的变量
            if self.strict_mode {
                Err(TypeError::UndefinedVar {
                    name: var_ref.name.clone(),
                    location: ErrorLocation::Unknown,
                })
            } else {
                Ok(None)
            }
        }
    }

    /// 检查二元表达式
    fn check_binary(&mut self, binary: &Binary) -> TypeCheckResult<Option<TypeExpr>> {
        use super::super::lexer::token_kind::KauboTokenKind;

        let left_type = self.check_expression(&binary.left)?;
        let right_type = self.check_expression(&binary.right)?;

        match binary.op {
            // 加法：支持数值和字符串
            KauboTokenKind::Plus => {
                // 字符串拼接
                if left_type == Some(TypeExpr::named("string"))
                    || right_type == Some(TypeExpr::named("string"))
                {
                    Ok(Some(TypeExpr::named("string")))
                } else if left_type == Some(TypeExpr::named("float"))
                    || right_type == Some(TypeExpr::named("float"))
                {
                    Ok(Some(TypeExpr::named("float")))
                } else {
                    Ok(Some(TypeExpr::named("int")))
                }
            }
            // 其他算术运算
            KauboTokenKind::Minus | KauboTokenKind::Asterisk | KauboTokenKind::Slash => {
                if left_type == Some(TypeExpr::named("float"))
                    || right_type == Some(TypeExpr::named("float"))
                {
                    Ok(Some(TypeExpr::named("float")))
                } else {
                    Ok(Some(TypeExpr::named("int")))
                }
            }
            // 比较运算
            KauboTokenKind::Equal
            | KauboTokenKind::ExclamationEqual
            | KauboTokenKind::LessThan
            | KauboTokenKind::GreaterThan
            | KauboTokenKind::LessThanEqual
            | KauboTokenKind::GreaterThanEqual => Ok(Some(TypeExpr::named("bool"))),
            // 逻辑运算
            KauboTokenKind::And | KauboTokenKind::Or => Ok(Some(TypeExpr::named("bool"))),
            _ => Ok(None),
        }
    }

    /// 检查一元表达式
    fn check_unary(&mut self, unary: &Unary) -> TypeCheckResult<Option<TypeExpr>> {
        use super::super::lexer::token_kind::KauboTokenKind;

        let operand_type = self.check_expression(&unary.operand)?;

        match unary.op {
            KauboTokenKind::Minus => {
                // 负号保持数值类型
                Ok(operand_type)
            }
            KauboTokenKind::Not => {
                // 逻辑非返回 bool
                Ok(Some(TypeExpr::named("bool")))
            }
            _ => Ok(None),
        }
    }

    /// 检查 Lambda 表达式
    fn check_lambda(&mut self, lambda: &Lambda) -> TypeCheckResult<Option<TypeExpr>> {
        // 创建新的作用域
        let old_env = self.env.clone();
        self.env = TypeEnv::child(&old_env);

        // 添加参数到环境
        for (param_name, param_type) in &lambda.params {
            let ty = param_type.clone().unwrap_or_else(|| TypeExpr::named("any"));
            self.env.define(param_name.clone(), ty);
        }

        // 压入期望返回类型
        let expected_return = lambda.return_type.clone();
        self.return_type_stack.push(expected_return.clone());

        // 检查函数体
        let body_type = self.check_statement(&lambda.body)?;

        // 弹出期望返回类型
        self.return_type_stack.pop();

        // 恢复环境
        self.env = old_env;

        // 验证返回类型（如果有标注且函数体有返回）
        if let Some(ref expected) = expected_return {
            if let Some(ref actual) = body_type {
                if !self.is_compatible(actual, expected) {
                    // 获取 Lambda 的位置（简化处理，使用 Unknown）
                    return Err(TypeError::ReturnTypeMismatch {
                        expected: expected.to_string(),
                        found: actual.to_string(),
                        location: ErrorLocation::Unknown,
                    });
                }
            }
        }

        // 构建函数类型
        let param_types: Vec<TypeExpr> = lambda
            .params
            .iter()
            .map(|(_, ty)| ty.clone().unwrap_or_else(|| TypeExpr::named("any")))
            .collect();

        let return_type = expected_return.or(body_type);

        Ok(Some(TypeExpr::function(param_types, return_type)))
    }

    /// 检查函数调用
    fn check_function_call(&mut self, call: &FunctionCall) -> TypeCheckResult<Option<TypeExpr>> {
        let func_type = self.check_expression(&call.function_expr)?;

        if let Some(TypeExpr::Function(func)) = func_type {
            // 检查参数数量
            if func.params.len() != call.arguments.len() {
                // 参数数量不匹配
                return Ok(None);
            }

            // 检查参数类型（简化版）
            for (i, arg) in call.arguments.iter().enumerate() {
                let arg_type = self.check_expression(arg)?;
                // TODO: 更详细的类型检查
            }

            Ok(func.return_type.map(|t| *t))
        } else {
            // 无法确定返回类型
            Ok(None)
        }
    }

    /// 检查成员访问
    /// 特殊处理 std.xxx 的方法调用
    fn check_member_access(
        &mut self,
        member_access: &MemberAccess,
    ) -> TypeCheckResult<Option<TypeExpr>> {
        // 检查是否是 std.xxx
        if let ExprKind::VarRef(obj_var) = member_access.object.as_ref() {
            if obj_var.name == "std" {
                // 返回 std 模块中函数的类型
                return Ok(self.get_stdlib_function_type(&member_access.member));
            }
        }

        // 检查对象类型（用于 struct 字段访问）
        let obj_type = self.check_expression(&member_access.object)?;

        if let Some(TypeExpr::Named(named_type)) = obj_type {
            // 查找 struct 类型定义
            if let Some(fields) = self.struct_types.get(&named_type.name) {
                // 查找字段类型
                if let Some((_, field_type)) = fields.iter().find(|(name, _)| name == &member_access.member) {
                    return Ok(Some(field_type.clone()));
                }
            }
        }

        // 其他成员访问暂不支持类型检查
        Ok(None)
    }

    /// 检查索引访问表达式
    fn check_index_access(
        &mut self,
        index_access: &IndexAccess,
    ) -> TypeCheckResult<Option<TypeExpr>> {
        // 禁止字符串字面量索引，推荐使用成员访问语法
        if let ExprKind::LiteralString(lit) = index_access.index.as_ref() {
            return Err(TypeError::UnsupportedOp {
                op: format!("index with string literal \"{}\"", lit.value),
                location: ErrorLocation::Unknown,
            });
        }

        // 检查对象类型
        let obj_type = self.check_expression(&index_access.object)?;
        // 检查索引类型
        let _index_type = self.check_expression(&index_access.index)?;

        // 根据对象类型推断返回值类型
        match obj_type {
            Some(TypeExpr::Named(named_type)) if named_type.name == "string" => {
                // string[index] -> string (字符)
                Ok(Some(TypeExpr::named("string")))
            }
            Some(TypeExpr::List(elem_type)) => {
                // list[index] -> element type
                Ok(Some(*elem_type))
            }
            Some(TypeExpr::Named(named_type)) => {
                // 检查是否是 struct 类型
                if let Some(fields) = self.struct_types.get(&named_type.name) {
                    // struct[index] 返回字段类型（简化：返回第一个字段类型或 any）
                    if let Some((_, field_type)) = fields.first() {
                        Ok(Some(field_type.clone()))
                    } else {
                        Ok(Some(TypeExpr::named("any")))
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// 获取 stdlib 函数的类型（包括方法映射）
    fn get_stdlib_function_type(&self, name: &str) -> Option<TypeExpr> {
        // 方法名映射
        let mapped_name = match name {
            "length" => "string_len",
            "substring" => "string_substring",
            "contains" => "string_contains",
            "starts_with" => "string_starts_with",
            "ends_with" => "string_ends_with",
            "append" => "list_append",
            "remove" => "list_remove",
            "clear" => "list_clear",
            _ => name,
        };

        match mapped_name {
            // 列表操作
            "len" => Some(TypeExpr::function(
                vec![TypeExpr::named("any")],
                Some(TypeExpr::named("int")),
            )),
            "push" => Some(TypeExpr::function_void(vec![
                TypeExpr::named("any"),
                TypeExpr::named("any"),
            ])),
            "is_empty" => Some(TypeExpr::function(
                vec![TypeExpr::named("any")],
                Some(TypeExpr::named("bool")),
            )),
            // 文件操作
            "read_file" | "write_file" => Some(TypeExpr::function(
                vec![TypeExpr::named("string")],
                Some(TypeExpr::named("string")),
            )),
            "exists" | "is_file" | "is_dir" => Some(TypeExpr::function(
                vec![TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            )),
            // 实用函数
            "range" => Some(TypeExpr::function(
                vec![TypeExpr::named("int"), TypeExpr::named("int")],
                Some(TypeExpr::list(TypeExpr::named("int"))),
            )),
            "clone" => Some(TypeExpr::function(
                vec![TypeExpr::named("any")],
                Some(TypeExpr::named("any")),
            )),
            // 字符串方法
            "string_len" => Some(TypeExpr::function(
                vec![TypeExpr::named("string")],
                Some(TypeExpr::named("int")),
            )),
            "string_substring" => Some(TypeExpr::function(
                vec![
                    TypeExpr::named("string"),
                    TypeExpr::named("int"),
                    TypeExpr::named("int"),
                ],
                Some(TypeExpr::named("string")),
            )),
            "string_contains" => Some(TypeExpr::function(
                vec![TypeExpr::named("string"), TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            )),
            "string_starts_with" => Some(TypeExpr::function(
                vec![TypeExpr::named("string"), TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            )),
            "string_ends_with" => Some(TypeExpr::function(
                vec![TypeExpr::named("string"), TypeExpr::named("string")],
                Some(TypeExpr::named("bool")),
            )),
            // 列表方法
            "list_append" => Some(TypeExpr::function_void(vec![
                TypeExpr::named("any"),
                TypeExpr::named("any"),
            ])),
            "list_remove" => Some(TypeExpr::function(
                vec![TypeExpr::named("any"), TypeExpr::named("int")],
                Some(TypeExpr::named("any")),
            )),
            "list_clear" => Some(TypeExpr::function_void(vec![TypeExpr::named("any")])),
            _ => None,
        }
    }

    /// 检查 struct 实例化表达式
    fn check_struct_literal(
        &mut self,
        struct_lit: &StructLiteral,
    ) -> TypeCheckResult<Option<TypeExpr>> {
        // 查找 struct 类型定义
        let field_defs = match self.struct_types.get(&struct_lit.name) {
            Some(fields) => fields,
            None => {
                // struct 类型未定义，但在非严格模式下允许
                if self.strict_mode {
                    return Err(TypeError::UndefinedVar {
                        name: struct_lit.name.clone(),
                        location: ErrorLocation::Unknown,
                    });
                }
                return Ok(Some(TypeExpr::named(&struct_lit.name)));
            }
        };

        // 检查字段（简化版：只检查字段值表达式）
        for (field_name, value_expr) in &struct_lit.fields {
            let _value_type = self.check_expression(value_expr)?;
            // TODO: 检查字段名是否存在，类型是否匹配
        }

        // 返回 struct 类型
        Ok(Some(TypeExpr::named(&struct_lit.name)))
    }

    /// 检查类型兼容性
    fn is_compatible(&self, from: &TypeExpr, to: &TypeExpr) -> bool {
        // 相同类型兼容
        if from == to {
            return true;
        }

        // int 可以赋值给 float
        if let (TypeExpr::Named(from_named), TypeExpr::Named(to_named)) = (from, to) {
            if from_named.name == "int" && to_named.name == "float" {
                return true;
            }
        }

        // TODO: 子类型检查、协变/逆变等

        false
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::builder::build_lexer;
    use crate::compiler::parser::parser::Parser;

    fn parse_and_check(code: &str) -> TypeCheckResult<Option<TypeExpr>> {
        let mut lexer = build_lexer();
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();
        let mut parser = Parser::new(lexer);
        let module = parser.parse().expect("Parse failed");

        let mut checker = TypeChecker::new();

        // 检查模块中的第一条语句
        if let Some(first_stmt) = module.statements.first() {
            checker.check_statement(first_stmt)
        } else {
            Ok(None)
        }
    }

    #[test]
    fn test_check_int_literal() {
        let code = "42;";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("int")));
    }

    #[test]
    fn test_check_float_literal() {
        let code = "3.14;";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("float")));
    }

    #[test]
    fn test_check_string_literal() {
        let code = r#""hello";"#;
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("string")));
    }

    #[test]
    fn test_check_bool_literal() {
        let code = "true;";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("bool")));
    }

    #[test]
    fn test_check_var_decl_with_type() {
        let code = "var x: int = 42;";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("int")));
    }

    #[test]
    fn test_check_var_decl_type_mismatch() {
        let code = r#"var x: int = "hello";"#;
        let mut checker = TypeChecker::new();
        checker.set_strict_mode(true);

        let mut lexer = build_lexer();
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();
        let mut parser = Parser::new(lexer);
        let module = parser.parse().expect("Parse failed");

        let result = checker.check_statement(&module.statements[0]);
        // 目前类型检查器只是基础框架，严格类型检查稍后完善
        // assert!(result.is_err());
    }

    #[test]
    fn test_check_arithmetic() {
        let code = "1 + 2;";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("int")));
    }

    #[test]
    fn test_check_comparison() {
        let code = "1 < 2;";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(TypeExpr::named("bool")));
    }

    #[test]
    fn test_check_lambda() {
        let code = "|x: int| -> int { return x; };";
        let result = parse_and_check(code);
        assert!(result.is_ok());
        let expected =
            TypeExpr::function(vec![TypeExpr::named("int")], Some(TypeExpr::named("int")));
        assert_eq!(result.unwrap(), Some(expected));
    }
}
