//! AST → Bytecode 编译器

use crate::compiler::parser::expr::{FunctionCall, Lambda, VarRef};
use crate::compiler::parser::stmt::{ForStmt, IfStmt, ModuleStmt, WhileStmt};
use crate::compiler::parser::{Binary, Expr, ExprKind, Module, Stmt, StmtKind};
use crate::runtime::{
    Value,
    bytecode::{OpCode, chunk::Chunk},
    object::{ObjFunction, ObjList, ObjString},
};
use std::collections::HashMap;

/// 编译错误
#[derive(Debug, Clone)]
pub enum CompileError {
    InvalidOperator,
    TooManyConstants,
    TooManyLocals,
    VariableAlreadyExists(String),
    UninitializedVariable(String),
    Unimplemented(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::InvalidOperator => write!(f, "Invalid operator"),
            CompileError::TooManyConstants => write!(f, "Too many constants in one chunk"),
            CompileError::TooManyLocals => write!(f, "Too many local variables"),
            CompileError::VariableAlreadyExists(name) => {
                write!(f, "Variable '{}' already exists", name)
            }
            CompileError::UninitializedVariable(name) => {
                write!(f, "Variable '{}' is not initialized", name)
            }
            CompileError::Unimplemented(s) => write!(f, "Unimplemented: {}", s),
        }
    }
}

/// 局部变量信息
#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: usize,
    is_initialized: bool,
    is_captured: bool, // 是否被内层闭包捕获
}

/// Upvalue 描述（编译时）
#[derive(Debug, Clone)]
struct Upvalue {
    name: String,
    index: u8,        // upvalue 索引
    is_local: bool,   // true=捕获外层局部变量, false=继承外层的 upvalue
}

/// 导出项信息
#[derive(Debug, Clone)]
struct Export {
    name: String,
    is_public: bool,  // pub 修饰
    local_idx: u8,    // 对应的局部变量索引
    shape_id: u16,    // ShapeID（编译期确定的静态索引）
}

/// 模块信息（编译时）
#[derive(Debug, Clone)]
struct ModuleInfo {
    name: String,
    exports: Vec<Export>,
    export_name_to_shape_id: HashMap<String, u16>, // 名称到 ShapeID 的映射
}

/// 变量类型（解析结果）
#[derive(Debug, Clone, Copy)]
enum Variable {
    Local(u8),    // 局部变量索引
    Upvalue(u8),  // upvalue 索引
}

/// AST 编译器
pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,       // 局部变量表
    upvalues: Vec<Upvalue>,   // upvalue 描述表
    scope_depth: usize,       // 当前作用域深度
    max_locals: usize,        // 最大局部变量数量（用于栈预分配）
    enclosing: *mut Compiler, // 指向父编译器（用于解析 upvalue）
    current_module: Option<ModuleInfo>, // 当前正在编译的模块
    modules: Vec<ModuleInfo>, // 文件内所有模块
    imported_modules: Vec<String>, // 当前导入的模块名列表
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            max_locals: 0,
            enclosing: std::ptr::null_mut(),
            current_module: None,
            modules: Vec::new(),
            imported_modules: Vec::new(),
        }
    }

    /// 创建子编译器（用于编译嵌套函数）
    fn new_child(enclosing: *mut Compiler) -> Self {
        Self {
            chunk: Chunk::new(),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            max_locals: 0,
            enclosing,
            current_module: None,
            modules: Vec::new(),
            imported_modules: Vec::new(),
        }
    }

    /// 编译整个模块（视为匿名函数体）
    /// 返回 (Chunk, 最大局部变量数量)
    pub fn compile(&mut self, module: &Module) -> Result<(Chunk, usize), CompileError> {
        for stmt in &module.statements {
            self.compile_stmt(stmt)?;
        }

        // 默认返回 null
        self.chunk.write_op(OpCode::LoadNull, 0);
        self.chunk.write_op(OpCode::Return, 0);

        Ok((self.chunk.clone(), self.max_locals))
    }

    /// 编译语句
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt.as_ref() {
            StmtKind::Expr(expr) => {
                self.compile_expr(&expr.expression)?;
                // 表达式语句的结果丢弃
                self.chunk.write_op(OpCode::Pop, 0);
            }

            StmtKind::VarDecl(decl) => {
                // 先声明变量（占位）
                let idx = self.add_local(&decl.name)?;

                // 编译初始化表达式
                self.compile_expr(&decl.initializer)?;

                // 标记为已初始化并存储
                self.mark_initialized();
                self.emit_store_local(idx);
                
                // 如果是 pub 且在当前模块中，记录导出
                if decl.is_public {
                    if let Some(ref mut module) = self.current_module {
                        let shape_id = module.exports.len() as u16;
                        module.exports.push(Export {
                            name: decl.name.clone(),
                            is_public: true,
                            local_idx: idx,
                            shape_id,
                        });
                        module.export_name_to_shape_id.insert(decl.name.clone(), shape_id);
                    }
                }
            }

            StmtKind::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.compile_expr(value)?;
                    self.chunk.write_op(OpCode::ReturnValue, 0);
                } else {
                    self.chunk.write_op(OpCode::LoadNull, 0);
                    self.chunk.write_op(OpCode::Return, 0);
                }
            }

            StmtKind::Print(print) => {
                self.compile_expr(&print.expression)?;
                self.chunk.write_op(OpCode::Print, 0);
            }

            StmtKind::Empty(_) => {}

            StmtKind::Block(block) => {
                self.begin_scope();
                for stmt in &block.statements {
                    self.compile_stmt(stmt)?;
                }
                self.end_scope();
            }

            StmtKind::If(if_stmt) => {
                self.compile_if(if_stmt)?;
            }

            StmtKind::While(while_stmt) => {
                self.compile_while(while_stmt)?;
            }

            StmtKind::For(for_stmt) => {
                self.compile_for(for_stmt)?;
            }

            StmtKind::Module(module_stmt) => {
                self.compile_module(module_stmt)?;
            }

            StmtKind::Import(import_stmt) => {
                self.compile_import(import_stmt)?;
            }
        }
        Ok(())
    }

    /// 编译表达式
    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr.as_ref() {
            ExprKind::LiteralInt(lit) => {
                let value = Value::smi(lit.value as i32);
                let idx = self.chunk.add_constant(value);
                self.emit_constant(idx);
            }

            ExprKind::LiteralString(lit) => {
                // 创建字符串对象
                let string_obj = Box::new(ObjString::new(lit.value.clone()));
                let string_ptr = Box::into_raw(string_obj) as *mut ObjString;
                let value = Value::string(string_ptr);
                let idx = self.chunk.add_constant(value);
                self.emit_constant(idx);
            }

            ExprKind::LiteralTrue(_) => {
                self.chunk.write_op(OpCode::LoadTrue, 0);
            }

            ExprKind::LiteralFalse(_) => {
                self.chunk.write_op(OpCode::LoadFalse, 0);
            }

            ExprKind::LiteralNull(_) => {
                self.chunk.write_op(OpCode::LoadNull, 0);
            }

            ExprKind::LiteralList(list) => {
                // 编译所有元素（从左到右，依次压栈）
                let count = list.elements.len();
                if count > 255 {
                    return Err(CompileError::Unimplemented(
                        "List too long (max 255 elements)".to_string()
                    ));
                }
                
                for elem in &list.elements {
                    self.compile_expr(elem)?;
                }
                
                // 生成 BuildList 指令
                self.chunk.write_op_u8(OpCode::BuildList, count as u8, 0);
            }

            ExprKind::Binary(bin) => {
                self.compile_binary(bin)?;
            }

            ExprKind::Unary(un) => {
                self.compile_expr(&un.operand)?;
                let op = match un.op {
                    crate::compiler::lexer::token_kind::KauboTokenKind::Minus => OpCode::Neg,
                    crate::compiler::lexer::token_kind::KauboTokenKind::Not => OpCode::Not,
                    _ => return Err(CompileError::InvalidOperator),
                };
                self.chunk.write_op(op, 0);
            }

            ExprKind::Grouping(g) => {
                self.compile_expr(&g.expression)?;
            }

            ExprKind::VarRef(var_ref) => {
                match self.resolve_variable(&var_ref.name) {
                    Some(Variable::Local(idx)) => self.emit_load_local(idx),
                    Some(Variable::Upvalue(idx)) => self.emit_load_upvalue(idx),
                    None => {
                        // 检查是否是模块名（全局变量）
                        if self.is_module_name(&var_ref.name) {
                            // 模块作为全局变量访问
                            let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                            let name_ptr = Box::into_raw(name_obj) as *mut ObjString;
                            let name_val = Value::string(name_ptr);
                            let name_idx = self.chunk.add_constant(name_val);
                            self.emit_constant(name_idx);
                            self.chunk.write_op(OpCode::LoadGlobal, 0);
                        } else {
                            return Err(CompileError::Unimplemented(format!(
                                "Undefined variable: {}",
                                var_ref.name
                            )));
                        }
                    }
                }
            }

            ExprKind::FunctionCall(call) => {
                self.compile_function_call(call)?;
            }

            ExprKind::Lambda(lambda) => {
                self.compile_lambda(lambda)?;
            }

            ExprKind::MemberAccess(member) => {
                // 成员访问语法糖: obj.name 等价于 obj["name"]
                // 对于模块访问，使用 ModuleGet 指令（编译期确定的 ShapeID）
                
                if let ExprKind::VarRef(var_ref) = member.object.as_ref() {
                    if let Some(shape_id) = self.find_module_shape_id(&var_ref.name, &member.member) {
                        // 模块访问：LoadGlobal + ModuleGet(shape_id)
                        // 加载模块（全局变量）
                        let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                        let name_ptr = Box::into_raw(name_obj) as *mut ObjString;
                        let name_val = Value::string(name_ptr);
                        let name_idx = self.chunk.add_constant(name_val);
                        self.chunk.write_op_u8(OpCode::LoadGlobal, name_idx, 0);
                        
                        // ModuleGet 指令（ShapeID 作为 u16 操作数）
                        self.chunk.write_op(OpCode::ModuleGet, 0);
                        self.chunk.write_u16(shape_id, 0);
                        return Ok(());
                    }
                }
                
                // 普通对象访问（JSON）：编译对象 + 字符串键 + IndexGet
                self.compile_expr(&member.object)?;
                let key_obj = Box::new(ObjString::new(member.member.clone()));
                let key_ptr = Box::into_raw(key_obj) as *mut ObjString;
                let key_val = Value::string(key_ptr);
                let idx = self.chunk.add_constant(key_val);
                self.emit_constant(idx);
                self.chunk.write_op(OpCode::IndexGet, 0);
            }

            ExprKind::IndexAccess(index) => {
                // 编译索引访问: list[index]
                self.compile_expr(&index.object)?;  // list
                self.compile_expr(&index.index)?;   // index
                self.chunk.write_op(OpCode::IndexGet, 0);
            }

            ExprKind::JsonLiteral(json) => {
                // 编译 JSON 字面量: json { "key": value, ... }
                // 栈布局: [value1, key1, value2, key2, ...]（从后往前压栈）
                let count = json.entries.len();
                if count > 255 {
                    return Err(CompileError::Unimplemented(
                        "JSON literal too large (max 255 entries)".to_string()
                    ));
                }
                
                // 逆序处理，这样 BuildJson 可以按正确顺序弹出
                for (key, value) in json.entries.iter().rev() {
                    // 值先入栈
                    self.compile_expr(value)?;
                    // 键（字符串）入栈
                    let key_obj = Box::new(ObjString::new(key.clone()));
                    let key_ptr = Box::into_raw(key_obj) as *mut ObjString;
                    let key_val = Value::string(key_ptr);
                    let idx = self.chunk.add_constant(key_val);
                    self.emit_constant(idx);
                }
                
                self.chunk.write_op_u8(OpCode::BuildJson, count as u8, 0);
            }

            ExprKind::Yield(y) => {
                // 编译 yield 表达式
                if let Some(value) = &y.value {
                    self.compile_expr(value)?;
                } else {
                    // yield; 等价于 yield null;
                    self.chunk.write_op(OpCode::LoadNull, 0);
                }
                self.chunk.write_op(OpCode::Yield, 0);
            }
        }
        Ok(())
    }

    /// 编译二元运算
    fn compile_binary(&mut self, bin: &Binary) -> Result<(), CompileError> {
        // 特殊处理赋值运算符：=
        if bin.op == crate::compiler::lexer::token_kind::KauboTokenKind::Equal {
            return self.compile_assignment(&bin.left, &bin.right);
        }

        // 先编译左操作数
        self.compile_expr(&bin.left)?;

        // 再编译右操作数
        self.compile_expr(&bin.right)?;

        // 生成运算指令
        let op = match bin.op {
            crate::compiler::lexer::token_kind::KauboTokenKind::Plus => OpCode::Add,
            crate::compiler::lexer::token_kind::KauboTokenKind::Minus => OpCode::Sub,
            crate::compiler::lexer::token_kind::KauboTokenKind::Asterisk => OpCode::Mul,
            crate::compiler::lexer::token_kind::KauboTokenKind::Slash => OpCode::Div,
            crate::compiler::lexer::token_kind::KauboTokenKind::DoubleEqual => OpCode::Equal,
            crate::compiler::lexer::token_kind::KauboTokenKind::ExclamationEqual => {
                OpCode::NotEqual
            }
            crate::compiler::lexer::token_kind::KauboTokenKind::GreaterThan => OpCode::Greater,
            crate::compiler::lexer::token_kind::KauboTokenKind::GreaterThanEqual => {
                OpCode::GreaterEqual
            }
            crate::compiler::lexer::token_kind::KauboTokenKind::LessThan => OpCode::Less,
            crate::compiler::lexer::token_kind::KauboTokenKind::LessThanEqual => OpCode::LessEqual,
            _ => return Err(CompileError::InvalidOperator),
        };

        self.chunk.write_op(op, 0);
        Ok(())
    }

    /// 编译赋值表达式 (处理 Binary 形式的赋值)
    /// 赋值表达式返回 null（语句级别的副作用）
    fn compile_assignment(&mut self, left: &Expr, right: &Expr) -> Result<(), CompileError> {
        // 左值可能是变量引用或索引访问
        match left.as_ref() {
            ExprKind::VarRef(var_ref) => {
                // 编译右值
                self.compile_expr(right)?;
                
                match self.resolve_variable(&var_ref.name) {
                    Some(Variable::Local(idx)) => self.emit_store_local(idx),
                    Some(Variable::Upvalue(idx)) => self.emit_store_upvalue(idx),
                    None => {
                        return Err(CompileError::Unimplemented(format!(
                            "Undefined variable: {}",
                            var_ref.name
                        )));
                    }
                }
                // 赋值表达式返回 null（不是被赋的值）
                self.chunk.write_op(OpCode::LoadNull, 0);
                Ok(())
            }
            
            ExprKind::IndexAccess(index) => {
                // 索引赋值: list[index] = value
                // 栈布局: [value, index, list]
                self.compile_expr(right)?;     // value
                self.compile_expr(&index.index)?; // index
                self.compile_expr(&index.object)?; // list
                self.chunk.write_op(OpCode::IndexSet, 0);
                // 返回 null
                self.chunk.write_op(OpCode::LoadNull, 0);
                Ok(())
            }
            
            ExprKind::MemberAccess(member) => {
                // 成员赋值: obj.name = value
                // 等价于 obj["name"] = value
                // 栈布局: [value, "name", obj]
                self.compile_expr(right)?;     // value
                // 成员名作为字符串
                let key_obj = Box::new(ObjString::new(member.member.clone()));
                let key_ptr = Box::into_raw(key_obj) as *mut ObjString;
                let key_val = Value::string(key_ptr);
                let idx = self.chunk.add_constant(key_val);
                self.emit_constant(idx);       // key
                self.compile_expr(&member.object)?; // obj
                self.chunk.write_op(OpCode::IndexSet, 0);
                // 返回 null
                self.chunk.write_op(OpCode::LoadNull, 0);
                Ok(())
            }
            
            _ => Err(CompileError::Unimplemented(
                "Left side of assignment must be a variable, index access, or member access".to_string(),
            )),
        }
    }

    /// 发射常量加载指令 (优化：使用 LoadConst0-15)
    fn emit_constant(&mut self, idx: u8) {
        let op = match idx {
            0 => OpCode::LoadConst0,
            1 => OpCode::LoadConst1,
            2 => OpCode::LoadConst2,
            3 => OpCode::LoadConst3,
            4 => OpCode::LoadConst4,
            5 => OpCode::LoadConst5,
            6 => OpCode::LoadConst6,
            7 => OpCode::LoadConst7,
            8 => OpCode::LoadConst8,
            9 => OpCode::LoadConst9,
            10 => OpCode::LoadConst10,
            11 => OpCode::LoadConst11,
            12 => OpCode::LoadConst12,
            13 => OpCode::LoadConst13,
            14 => OpCode::LoadConst14,
            15 => OpCode::LoadConst15,
            _ => {
                self.chunk.write_op_u8(OpCode::LoadConst, idx, 0);
                return;
            }
        };
        self.chunk.write_op(op, 0);
    }

    // ==================== 局部变量管理 ====================

    /// 进入新作用域
    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// 退出作用域，返回弹出的变量数量
    fn end_scope(&mut self) -> usize {
        self.scope_depth -= 1;

        let mut popped = 0;
        while let Some(local) = self.locals.last() {
            if local.depth <= self.scope_depth {
                break;
            }
            self.locals.pop();
            popped += 1;
        }
        popped
    }

    /// 添加局部变量，返回其在栈中的索引
    fn add_local(&mut self, name: &str) -> Result<u8, CompileError> {
        // 检查局部变量数量上限
        if self.locals.len() >= 256 {
            return Err(CompileError::TooManyLocals);
        }

        // 检查同作用域内是否已有同名变量
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if local.name == name {
                return Err(CompileError::VariableAlreadyExists(name.to_string()));
            }
        }

        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scope_depth,
            is_initialized: false,
            is_captured: false,
        });

        // 更新最大局部变量数
        self.max_locals = self.max_locals.max(self.locals.len());

        Ok((self.locals.len() - 1) as u8)
    }

    /// 标记最后一个变量为已初始化
    fn mark_initialized(&mut self) {
        if let Some(local) = self.locals.last_mut() {
            local.is_initialized = true;
        }
    }

    /// 解析变量名，返回其在局部变量表中的索引
    fn resolve_local(&self, name: &str) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                if !local.is_initialized {
                    // 不能在初始化完成前使用
                    return None;
                }
                return Some(i as u8);
            }
        }
        None
    }

    /// 标记局部变量被捕获
    fn mark_captured(&mut self, index: usize) {
        if let Some(local) = self.locals.get_mut(index) {
            local.is_captured = true;
        }
    }

    /// 添加 upvalue 描述，返回其索引
    fn add_upvalue(&mut self, name: &str, index: u8, is_local: bool) -> u8 {
        // 检查是否已存在相同的 upvalue
        for (i, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i as u8;
            }
        }
        
        let upvalue = Upvalue {
            name: name.to_string(),
            index,
            is_local,
        };
        self.upvalues.push(upvalue);
        (self.upvalues.len() - 1) as u8
    }

    /// 递归解析 upvalue
    /// 返回 Some(idx) 如果是 upvalue，None 如果未找到
    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        // 没有外层编译器，无法解析
        if self.enclosing.is_null() {
            return None;
        }

        unsafe {
            // 1. 在外层编译器查找局部变量
            if let Some(local_idx) = (*self.enclosing).resolve_local(name) {
                // 找到了！标记为被捕获
                (*self.enclosing).mark_captured(local_idx as usize);
                // 添加 upvalue 描述（指向外层局部变量）
                return Some(self.add_upvalue(name, local_idx, true));
            }

            // 2. 递归在外层查找 upvalue
            if let Some(upvalue_idx) = (*self.enclosing).resolve_upvalue(name) {
                // 继承外层的 upvalue
                return Some(self.add_upvalue(name, upvalue_idx, false));
            }
        }

        None
    }

    /// 统一变量解析：Local 或 Upvalue
    fn resolve_variable(&mut self, name: &str) -> Option<Variable> {
        // 1. 先查找局部变量
        if let Some(idx) = self.resolve_local(name) {
            return Some(Variable::Local(idx));
        }
        
        // 2. 查找 upvalue（需要可变引用，因为可能添加 upvalue 描述）
        if let Some(idx) = self.resolve_upvalue(name) {
            return Some(Variable::Upvalue(idx));
        }
        
        None
    }

    /// 查找模块导出项的 ShapeID
    /// 返回 Some(shape_id) 如果找到模块和导出项
    fn find_module_shape_id(&self, module_name: &str, export_name: &str) -> Option<u16> {
        // 在已编译的模块中查找
        for module in &self.modules {
            if module.name == module_name {
                return module.export_name_to_shape_id.get(export_name).copied();
            }
        }
        // 在当前正在编译的模块中查找
        if let Some(ref current) = self.current_module {
            if current.name == module_name {
                return current.export_name_to_shape_id.get(export_name).copied();
            }
        }
        // 在标准库模块中查找
        self.find_std_module_shape_id(module_name, export_name)
    }
    
    /// 查找标准库模块的 ShapeID
    fn find_std_module_shape_id(&self, module_name: &str, export_name: &str) -> Option<u16> {
        match module_name {
            "std" => match export_name {
                // 核心函数 (0-3)
                "print" => Some(0),
                "assert" => Some(1),
                "type" => Some(2),
                "to_string" => Some(3),
                // 数学函数 (4-8)
                "sqrt" => Some(4),
                "sin" => Some(5),
                "cos" => Some(6),
                "floor" => Some(7),
                "ceil" => Some(8),
                // 数学常量 (9-10)
                "PI" => Some(9),
                "E" => Some(10),
                _ => None,
            },
            _ => None,
        }
    }

    /// 检查名称是否是已定义的模块名（包括导入的）
    fn is_module_name(&self, name: &str) -> bool {
        // 检查是否是已导入的模块
        if self.imported_modules.iter().any(|m| m == name) {
            return true;
        }
        
        // 检查标准库模块（硬编码，启动时自动加载）
        if name == "std" {
            return true;
        }
        
        // 在已编译的模块中查找
        for module in &self.modules {
            if module.name == name {
                return true;
            }
        }
        // 在当前正在编译的模块中查找
        if let Some(ref current) = self.current_module {
            if current.name == name {
                return true;
            }
        }
        false
    }

    /// 发射局部变量加载指令
    fn emit_load_local(&mut self, idx: u8) {
        match idx {
            0 => self.chunk.write_op(OpCode::LoadLocal0, 0),
            1 => self.chunk.write_op(OpCode::LoadLocal1, 0),
            2 => self.chunk.write_op(OpCode::LoadLocal2, 0),
            3 => self.chunk.write_op(OpCode::LoadLocal3, 0),
            4 => self.chunk.write_op(OpCode::LoadLocal4, 0),
            5 => self.chunk.write_op(OpCode::LoadLocal5, 0),
            6 => self.chunk.write_op(OpCode::LoadLocal6, 0),
            7 => self.chunk.write_op(OpCode::LoadLocal7, 0),
            _ => self.chunk.write_op_u8(OpCode::LoadLocal, idx, 0),
        }
    }

    /// 发射局部变量存储指令
    fn emit_store_local(&mut self, idx: u8) {
        match idx {
            0 => self.chunk.write_op(OpCode::StoreLocal0, 0),
            1 => self.chunk.write_op(OpCode::StoreLocal1, 0),
            2 => self.chunk.write_op(OpCode::StoreLocal2, 0),
            3 => self.chunk.write_op(OpCode::StoreLocal3, 0),
            4 => self.chunk.write_op(OpCode::StoreLocal4, 0),
            5 => self.chunk.write_op(OpCode::StoreLocal5, 0),
            6 => self.chunk.write_op(OpCode::StoreLocal6, 0),
            7 => self.chunk.write_op(OpCode::StoreLocal7, 0),
            _ => self.chunk.write_op_u8(OpCode::StoreLocal, idx, 0),
        }
    }

    /// 发射 upvalue 加载指令
    fn emit_load_upvalue(&mut self, idx: u8) {
        self.chunk.write_op_u8(OpCode::GetUpvalue, idx, 0);
    }

    /// 发射 upvalue 存储指令
    fn emit_store_upvalue(&mut self, idx: u8) {
        self.chunk.write_op_u8(OpCode::SetUpvalue, idx, 0);
    }

    // ==================== 控制流编译 ====================

    /// 编译 if/elif/else 语句
    fn compile_if(&mut self, if_stmt: &IfStmt) -> Result<(), CompileError> {
        // 编译 if 条件
        self.compile_expr(&if_stmt.if_condition)?;

        // 如果条件为假，跳过 then 分支
        let then_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 编译 then 分支
        self.compile_stmt(&if_stmt.then_body)?;

        // then 分支执行完后，跳过所有 elif 和 else 分支
        let else_jump = self.chunk.write_jump(OpCode::Jump, 0);

        // 修补 then_jump，指向 elif 或 else 分支的开始位置
        self.chunk.patch_jump(then_jump);

        // 保存所有 elif 分支的跳转，最后统一修补到 else 分支之后
        let mut elif_jumps = Vec::new();

        // 编译 elif 分支
        for (elif_cond, elif_body) in if_stmt
            .elif_conditions
            .iter()
            .zip(if_stmt.elif_bodies.iter())
        {
            // 编译 elif 条件
            self.compile_expr(elif_cond)?;

            // 如果条件为假，跳过这个 elif 的 body
            let elif_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

            // 编译 elif body
            self.compile_stmt(elif_body)?;

            // elif body 执行完后，跳过剩余的分支
            let next_jump = self.chunk.write_jump(OpCode::Jump, 0);
            elif_jumps.push(next_jump);

            // 修补 elif_jump，指向下一个 elif 或 else 分支
            self.chunk.patch_jump(elif_jump);
        }

        // 编译 else 分支（如果存在）
        if let Some(else_body) = &if_stmt.else_body {
            self.compile_stmt(else_body)?;
        } else {
            // 如果没有 else 分支，加载 null
            self.chunk.write_op(OpCode::LoadNull, 0);
        }

        // 修补所有跳转到 else 分支之后的跳转
        if if_stmt.elif_conditions.is_empty() {
            // 只有 if-else，没有 elif
            self.chunk.patch_jump(else_jump);
        } else {
            // 有 elif 分支，修补 if 的 else_jump 和所有 elif 的 next_jump
            self.chunk.patch_jump(else_jump);
            for jump in elif_jumps {
                self.chunk.patch_jump(jump);
            }
        }

        Ok(())
    }

    /// 编译 while 循环
    fn compile_while(&mut self, while_stmt: &WhileStmt) -> Result<(), CompileError> {
        // 记录循环开始位置
        let loop_start = self.chunk.code.len();

        // 编译循环条件
        self.compile_expr(&while_stmt.condition)?;

        // 如果条件为假，跳出循环
        let exit_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 编译循环体
        self.compile_stmt(&while_stmt.body)?;

        // 跳回循环开始位置
        self.chunk.write_loop(loop_start, 0);

        // 修补退出跳转
        self.chunk.patch_jump(exit_jump);

        Ok(())
    }

    /// 编译 for-in 循环（基于迭代器协议）
    /// 语法: for var item in iterable { body }
    fn compile_for(&mut self, for_stmt: &ForStmt) -> Result<(), CompileError> {
        // 解析迭代变量声明
        let var_name = match for_stmt.iterator.as_ref() {
            ExprKind::VarRef(VarRef { name }) => name.clone(),
            _ => {
                return Err(CompileError::Unimplemented(
                    "For loop iterator must be a variable".to_string(),
                ));
            }
        };

        // 进入 for 循环作用域
        self.begin_scope();

        // 1. 获取迭代器: var $iter = iterable.iter();
        self.compile_expr(&for_stmt.iterable)?;
        self.chunk.write_op(OpCode::GetIter, 0);
        let iter_idx = self.add_local("$iter")?;
        self.mark_initialized();
        self.emit_store_local(iter_idx);

        // 2. 声明迭代变量（只声明一次，循环内赋值）
        let var_idx = self.add_local(&var_name)?;
        self.mark_initialized();

        // 3. 循环开始
        let loop_start = self.chunk.code.len();

        // 4. 获取下一个值
        self.emit_load_local(iter_idx);
        self.chunk.write_op(OpCode::IterNext, 0);
        
        // 5. 复制值用于 null 检查
        self.chunk.write_op(OpCode::Dup, 0);
        
        // 6. 检查是否为 null（结束标记）
        self.chunk.write_op(OpCode::LoadNull, 0);
        self.chunk.write_op(OpCode::Equal, 0);
        let exit_jump = self.chunk.write_jump(OpCode::JumpIfFalse, 0);
        
        // 是 null，退出循环
        // 注意：JumpIfFalse 已经弹出 true，栈上是 [null]
        self.chunk.write_op(OpCode::Pop, 0); // 弹出 null 值
        let exit_patch = self.chunk.write_jump(OpCode::Jump, 0); // 跳到循环外

        // 7. 不是 null，赋值给迭代变量
        self.chunk.patch_jump(exit_jump);
        // 注意：JumpIfFalse 已经弹出 false，栈上只剩 next 值
        self.emit_store_local(var_idx);  // 弹出 next 值，存入 item

        // 8. 编译循环体
        self.compile_stmt(&for_stmt.body)?;

        // 9. 跳回循环开始
        self.chunk.write_loop(loop_start, 0);

        // 10. 修补退出跳转
        self.chunk.patch_jump(exit_patch);

        // 11. 退出作用域（item 和 $iter 被清理）
        self.end_scope();
        
        Ok(())
    }

    // ==================== 模块编译 ====================

    /// 编译模块定义
    /// module name { ... }
    /// 模块在运行时是一个 ObjModule 对象，导出项按索引存储
    fn compile_module(&mut self, module_stmt: &ModuleStmt) -> Result<(), CompileError> {
        // 进入模块作用域
        self.begin_scope();
        
        // 创建模块信息
        let module_info = ModuleInfo {
            name: module_stmt.name.clone(),
            exports: Vec::new(),
            export_name_to_shape_id: HashMap::new(),
        };
        
        // 设置当前模块
        let prev_module = self.current_module.take();
        self.current_module = Some(module_info);
        
        // 编译模块体
        self.compile_stmt(&module_stmt.body)?;
        
        // 收集导出信息并生成模块对象
        if let Some(info) = self.current_module.take() {
            let export_count = info.exports.len();
            
            // 对于每个导出项，加载其值（正序压栈，BuildModule 会 reverse）
            for export in info.exports.iter() {
                // 加载导出值（从局部变量）
                self.emit_load_local(export.local_idx);
            }
            
            // 生成 BuildModule 指令创建模块对象
            if export_count > 255 {
                return Err(CompileError::Unimplemented(
                    "Module has too many exports (max 255)".to_string()
                ));
            }
            self.chunk.write_op_u8(OpCode::BuildModule, export_count as u8, 0);
            
            // 定义全局变量：模块名
            let module_name_obj = Box::new(ObjString::new(module_stmt.name.clone()));
            let module_name_ptr = Box::into_raw(module_name_obj) as *mut ObjString;
            let module_name_val = Value::string(module_name_ptr);
            let module_name_idx = self.chunk.add_constant(module_name_val);
            self.chunk.write_op_u8(OpCode::DefineGlobal, module_name_idx, 0);
            
            // 保存模块信息
            self.modules.push(info);
        }
        
        // 恢复之前的模块上下文
        self.current_module = prev_module;
        
        // 退出模块作用域
        self.end_scope();
        
        Ok(())
    }
    
    /// 编译导入语句
    /// import module; 或 from module import item;
    fn compile_import(&mut self, import_stmt: &crate::compiler::parser::stmt::ImportStmt) -> Result<(), CompileError> {
        // 检查模块是否存在（同文件内模块或标准库模块）
        let module_name = &import_stmt.module_path;
        
        if !self.is_module_name(module_name) && !self.is_std_module(module_name) {
            return Err(CompileError::Unimplemented(format!(
                "Module '{}' not found",
                module_name
            )));
        }
        
        // 将模块名加入导入列表
        self.imported_modules.push(module_name.clone());
        
        Ok(())
    }
    
    /// 检查是否是标准库模块
    fn is_std_module(&self, name: &str) -> bool {
        name == "std"
    }

    // ==================== 函数编译 ====================

    /// 编译 lambda 表达式
    fn compile_lambda(&mut self, lambda: &Lambda) -> Result<(), CompileError> {
        // 创建子编译器，传递 self 作为 enclosing（用于解析 upvalue）
        let mut function_compiler = Compiler::new_child(self);

        // 为每个参数添加局部变量
        for param in &lambda.params {
            function_compiler.add_local(param)?;
            function_compiler.mark_initialized();
        }

        // 编译函数体
        function_compiler.compile_stmt(&lambda.body)?;

        // 函数体末尾添加返回 null（如果没有显式返回）
        function_compiler.chunk.write_op(OpCode::LoadNull, 0);
        function_compiler.chunk.write_op(OpCode::Return, 0);

        // 获取 upvalue 信息
        let upvalues = std::mem::take(&mut function_compiler.upvalues);

        // 创建函数对象
        let function = Box::new(ObjFunction::new(
            function_compiler.chunk,
            lambda.params.len() as u8,
            None, // 暂时不支持函数名
        ));

        // 将函数对象作为常量添加到当前 chunk
        let function_ptr = Box::into_raw(function) as *mut ObjFunction;
        let function_value = Value::function(function_ptr);
        let idx = self.chunk.add_constant(function_value);

        // 发射 Closure 指令：函数索引 + upvalue数量 + upvalue描述符
        self.chunk.write_op(OpCode::Closure, 0);
        self.chunk.code.push(idx as u8);
        self.chunk.lines.push(0);
        self.chunk.code.push(upvalues.len() as u8);
        self.chunk.lines.push(0);
        
        // 输出 upvalue 描述符
        for upvalue in &upvalues {
            self.chunk.code.push(if upvalue.is_local { 1 } else { 0 });
            self.chunk.lines.push(0);
            self.chunk.code.push(upvalue.index);
            self.chunk.lines.push(0);
        }

        Ok(())
    }

    /// 编译函数调用
    fn compile_function_call(&mut self, call: &FunctionCall) -> Result<(), CompileError> {
        // 检查是否是内置协程函数
        if let ExprKind::VarRef(var_ref) = call.function_expr.as_ref() {
            match var_ref.name.as_str() {
                "create_coroutine" => {
                    // 编译参数（应该是一个闭包）
                    if call.arguments.len() != 1 {
                        return Err(CompileError::Unimplemented(
                            "create_coroutine expects exactly 1 argument".to_string()
                        ));
                    }
                    self.compile_expr(&call.arguments[0])?;
                    // 发射 CreateCoroutine 指令
                    self.chunk.write_op(OpCode::CreateCoroutine, 0);
                    return Ok(());
                }
                "resume" => {
                    // resume(co, value...)
                    if call.arguments.is_empty() {
                        return Err(CompileError::Unimplemented(
                            "resume expects at least 1 argument".to_string()
                        ));
                    }
                    // 编译参数（第一个是协程，后面是传入值）
                    for arg in &call.arguments {
                        self.compile_expr(arg)?;
                    }
                    // 发射 Resume 指令
                    let arg_count = (call.arguments.len() - 1) as u8;
                    self.chunk.write_op_u8(OpCode::Resume, arg_count, 0);
                    return Ok(());
                }
                "coroutine_status" => {
                    // coroutine_status(co)
                    if call.arguments.len() != 1 {
                        return Err(CompileError::Unimplemented(
                            "coroutine_status expects exactly 1 argument".to_string()
                        ));
                    }
                    self.compile_expr(&call.arguments[0])?;
                    // 发射 CoroutineStatus 指令
                    self.chunk.write_op(OpCode::CoroutineStatus, 0);
                    return Ok(());
                }
                _ => {}
            }
        }
        
        // 普通函数调用
        // 先编译参数（参数从左到右压栈）
        for arg in call.arguments.iter() {
            self.compile_expr(arg)?;
        }

        // 编译函数表达式
        self.compile_expr(&call.function_expr)?;

        // 发射 Call 指令
        let arg_count = call.arguments.len() as u8;
        self.chunk.write_op_u8(OpCode::Call, arg_count, 0);

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// 编译模块的便捷函数
/// 返回 (Chunk, 局部变量数量)
pub fn compile(module: &Module) -> Result<(Chunk, usize), CompileError> {
    let mut compiler = Compiler::new();
    compiler.compile(module)
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::builder::build_lexer;
    use crate::compiler::parser::parser::Parser;
    use crate::runtime::{InterpretResult, VM};

    fn compile_code(code: &str) -> Result<(Chunk, usize), CompileError> {
        let mut lexer = build_lexer();
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();

        let mut parser = Parser::new(lexer);
        let ast = parser
            .parse()
            .map_err(|e| CompileError::Unimplemented(format!("Parse error: {:?}", e)))?;

        compile(&ast)
    }

    fn run_code(code: &str) -> Result<Value, String> {
        let (chunk, local_count) =
            compile_code(code).map_err(|e| format!("Compile error: {:?}", e))?;
        let mut vm = VM::new();
        let result = vm.interpret_with_locals(&chunk, local_count);
        match result {
            InterpretResult::Ok => {
                // 返回值在栈顶
                vm.stack_top().ok_or("Empty stack".to_string())
            }
            InterpretResult::RuntimeError(msg) => Err(msg),
            InterpretResult::CompileError(msg) => Err(msg),
        }
    }

    #[test]
    fn test_compile_literal() {
        let (chunk, _) = compile_code("42;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_binary() {
        let (chunk, _) = compile_code("1 + 2;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_complex() {
        let (chunk, _) = compile_code("1 + 2 * 3;").unwrap();
        assert!(chunk.code.len() > 0);
    }

    // ===== End-to-End 测试 =====
    // 设计理念：代码文件 = 匿名函数体
    // - 表达式语句结果被 Pop（无副作用）
    // - 使用 return 显式返回值

    #[test]
    fn test_run_literal() {
        // 表达式语句结果丢弃，返回 null
        let result = run_code("42;").unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_run_addition() {
        // 使用 return 获取结果
        let result = run_code("return 1 + 2;").unwrap();
        assert_eq!(result.as_smi(), Some(3));
    }

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

    #[test]
    fn test_run_true() {
        let result = run_code("return true;").unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_run_false() {
        let result = run_code("return false;").unwrap();
        assert!(result.is_false());
    }

    #[test]
    fn test_run_null() {
        let result = run_code("return null;").unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_run_print() {
        // print 语句返回 null
        let result = run_code("print 42;").unwrap();
        assert!(result.is_null());
    }

    // ===== 变量测试 =====

    #[test]
    fn test_run_variable_declaration() {
        // var x = 5; return x;
        let result = run_code("var x = 5; return x;").unwrap();
        assert_eq!(result.as_smi(), Some(5));
    }

    #[test]
    fn test_run_variable_expression() {
        // var x = 5; var y = x + 3; return y;
        let result = run_code("var x = 5; var y = x + 3; return y;").unwrap();
        assert_eq!(result.as_smi(), Some(8));
    }

    #[test]
    fn test_run_variable_assignment() {
        // var x = 5; x = 10; return x;
        // 注意：赋值表达式返回 null，所以要用分号分隔语句
        let result = run_code("var x = 5; x = 10; return x;").unwrap();
        assert_eq!(result.as_smi(), Some(10));
    }

    #[test]
    fn test_assignment_returns_null() {
        // 赋值表达式本身返回 null
        let result = run_code("var x = 5; return x = 10;").unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_run_multiple_variables() {
        // var a = 1; var b = 2; var c = 3; return a + b + c;
        let result = run_code("var a = 1; var b = 2; var c = 3; return a + b + c;").unwrap();
        assert_eq!(result.as_smi(), Some(6));
    }

    // ===== Lambda 测试 =====

    #[test]
    fn test_compile_lambda() {
        // 测试基本的 lambda 编译
        let (chunk, _) = compile_code("|x| { return x + 1; };").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_lambda_no_params() {
        // 测试无参数 lambda
        let (chunk, _) = compile_code("| | { return 42; };").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_lambda_multi_params() {
        // 测试多参数 lambda
        let (chunk, _) = compile_code("|a, b| { return a + b; };").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_compile_function_call() {
        // 测试函数调用
        let (chunk, _) = compile_code("var f = |x| { return x + 1; }; f(5);").unwrap();
        assert!(chunk.code.len() > 0);
    }

    #[test]
    fn test_run_lambda() {
        // 测试基本的 lambda 调用
        let result = run_code("var f = |x| { return x + 1; }; return f(5);").unwrap();
        assert_eq!(result.as_smi(), Some(6));
    }

    #[test]
    fn test_run_lambda_no_params() {
        // 测试无参数 lambda
        let result = run_code("var f = | | { return 42; }; return f();").unwrap();
        assert_eq!(result.as_smi(), Some(42));
    }

    #[test]
    fn test_run_lambda_multi_params() {
        // 测试多参数 lambda
        let result = run_code("var add = |a, b| { return a + b; }; return add(3, 4);").unwrap();
        assert_eq!(result.as_smi(), Some(7));
    }

    #[test]
    fn test_closure_capture() {
        // 测试基础闭包捕获
        let result = run_code("var x = 5; var f = || { return x; }; return f();").unwrap();
        assert_eq!(result.as_smi(), Some(5));
    }

    #[test]
    fn test_closure_capture_modify() {
        // 测试闭包捕获并修改外部变量
        // 第一次调用: y=10, y=11, 返回 11
        // 第二次调用: y=11, y=12, 返回 12
        // r1 + r2 = 11 + 12 = 23
        let result = run_code("
            var y = 10;
            var g = || { y = y + 1; return y; };
            var r1 = g();
            var r2 = g();
            return r1 + r2;
        ").unwrap();
        assert_eq!(result.as_smi(), Some(23));
    }

    #[test]
    fn test_closure_multi_capture() {
        // 测试多变量捕获
        let result = run_code("
            var a = 1;
            var b = 2;
            var h = || { return a + b; };
            return h();
        ").unwrap();
        assert_eq!(result.as_smi(), Some(3));
    }

    // TODO: 嵌套闭包测试
    // #[test]
    // fn test_nested_closure() {
    //     let result = run_code("
    //         var outer = 100;
    //         var f1 = || {
    //             var inner = 10;
    //             var f2 = || { return outer + inner; };
    //             return f2();
    //         };
    //         return f1();
    //     ").unwrap();
    //     assert_eq!(result.as_smi(), Some(110));
    // }
}
