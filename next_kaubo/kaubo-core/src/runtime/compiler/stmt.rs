//! 语句编译

use crate::compiler::parser::expr::{VarRef};
use crate::compiler::parser::stmt::{ForStmt, IfStmt, ModuleStmt, WhileStmt};
use crate::compiler::parser::{ExprKind, Stmt, StmtKind};
use crate::core::{
    object::ObjString,
    MethodTableEntry, OperatorTableEntry, OpCode, Value,
};
use kaubo_log::trace;

use super::{
    context::{Export, ModuleInfo, VarType},
    var, CompileError, Compiler, StructInfo,
};

/// 编译语句
pub fn compile_stmt(compiler: &mut Compiler, stmt: &Stmt) -> Result<(), CompileError> {
    match stmt.as_ref() {
        StmtKind::Expr(expr) => {
            crate::runtime::compiler::expr::compile_expr(compiler, &expr.expression)?;
            // 表达式语句的结果丢弃
            compiler.chunk.write_op(OpCode::Pop, 0);
        }

        StmtKind::VarDecl(decl) => {
            // 先声明变量（占位）
            let idx = var::add_local(compiler, &decl.name)?;

            // 编译初始化表达式
            crate::runtime::compiler::expr::compile_expr(compiler, &decl.initializer)?;

            // 标记为已初始化并存储
            var::mark_initialized(compiler);
            var::emit_store_local(compiler, idx);

            // 记录变量类型（通过类型推断）
            if let Some(var_type) = compiler.get_expr_type(&decl.initializer) {
                trace!(compiler.logger, "VarDecl: {} has type {:?}", decl.name, var_type);
                compiler.var_types.insert(decl.name.clone(), var_type);
            } else {
                trace!(compiler.logger, "VarDecl: {} type unknown", decl.name);
            }

            // 如果是 pub 且在当前模块中，记录导出
            if decl.is_public {
                if let Some(ref mut module) = compiler.current_module {
                    let shape_id = module.exports.len() as u16;
                    module.exports.push(Export {
                        name: decl.name.clone(),
                        is_public: true,
                        local_idx: idx,
                        shape_id,
                    });
                    module
                        .export_name_to_shape_id
                        .insert(decl.name.clone(), shape_id);
                }
            }
        }

        StmtKind::Return(ret) => {
            if let Some(value) = &ret.value {
                crate::runtime::compiler::expr::compile_expr(compiler, value)?;
                compiler.chunk.write_op(OpCode::ReturnValue, 0);
            } else {
                compiler.chunk.write_op(OpCode::LoadNull, 0);
                compiler.chunk.write_op(OpCode::Return, 0);
            }
        }

        StmtKind::Print(print) => {
            crate::runtime::compiler::expr::compile_expr(compiler, &print.expression)?;
            compiler.chunk.write_op(OpCode::Print, 0);
        }

        StmtKind::Empty(_) => {}

        StmtKind::Block(block) => {
            var::begin_scope(compiler);
            for stmt in &block.statements {
                compile_stmt(compiler, stmt)?;
            }
            var::end_scope(compiler);
        }

        StmtKind::If(if_stmt) => {
            compile_if(compiler, if_stmt)?;
        }

        StmtKind::While(while_stmt) => {
            compile_while(compiler, while_stmt)?;
        }

        StmtKind::For(for_stmt) => {
            compile_for(compiler, for_stmt)?;
        }

        StmtKind::Module(module_stmt) => {
            compile_module(compiler, module_stmt)?;
        }

        StmtKind::Import(import_stmt) => {
            compile_import(compiler, import_stmt)?;
        }

        StmtKind::Struct(_) => {
            // Struct 定义是编译期类型信息，不生成运行时代码
            // shape 信息已在 type checker 中生成
        }

        StmtKind::Impl(impl_stmt) => {
            compile_impl_block(compiler, impl_stmt)?;
        }
    }
    Ok(())
}

/// 编译 impl 块
/// 将每个方法编译为函数，并记录到 Chunk.method_table 供 VM 初始化时使用
fn compile_impl_block(
    compiler: &mut Compiler,
    impl_stmt: &crate::compiler::parser::stmt::ImplStmt,
) -> Result<(), CompileError> {

    // 获取 shape_id（必须在 struct_infos 中存在）
    let shape_id = compiler
        .struct_infos
        .get(&impl_stmt.struct_name)
        .map(|info| info.shape_id)
        .unwrap_or(0);

    // 确保 struct_info 存在
    let struct_info = compiler
        .struct_infos
        .entry(impl_stmt.struct_name.clone())
        .or_insert_with(|| StructInfo {
            shape_id: 0,
            field_names: Vec::new(),
            method_names: Vec::new(),
        });

    // 先记录所有方法名，确定 method_idx
    let start_idx = struct_info.method_names.len();
    for method in &impl_stmt.methods {
        struct_info.method_names.push(method.name.clone());
    }

    // 编译每个方法
    for (i, method) in impl_stmt.methods.iter().enumerate() {
        let method_idx = (start_idx + i) as u8;
        let function_name = format!("{}_{}", impl_stmt.struct_name, method.name);

        if let ExprKind::Lambda(lambda) = method.lambda.as_ref() {
            // 创建独立编译器编译方法体（方法不捕获 upvalues）
            let mut method_compiler = Compiler::new_with_logger(compiler.logger.clone());
            // 复制 struct_infos 用于类型推断
            method_compiler.struct_infos = compiler.struct_infos.clone();
            // 继承模块信息（用于访问 std 等模块）
            method_compiler.module_aliases = compiler.module_aliases.clone();
            method_compiler.imported_modules = compiler.imported_modules.clone();

            trace!(method_compiler.logger, "compile_impl_method: struct={}, method={}, params={:?}", 
                impl_stmt.struct_name, method.name, lambda.params);

            // 添加参数作为局部变量（self 已经在 lambda.params 中）
            for (param_name, param_type) in &lambda.params {
                let _idx = var::add_local(&mut method_compiler, param_name)?;
                var::mark_initialized(&mut method_compiler);
                // 记录参数类型（用于字段访问优化）
                if param_name == "self" {
                    // self 总是当前 struct 类型
                    trace!(method_compiler.logger, "  adding self type: {}", impl_stmt.struct_name);
                    method_compiler.var_types.insert(
                        param_name.clone(),
                        VarType::Struct(impl_stmt.struct_name.clone()),
                    );
                } else if let Some(crate::compiler::parser::type_expr::TypeExpr::Named(named_type)) = param_type {
                    method_compiler.var_types.insert(
                        param_name.clone(),
                        VarType::Struct(named_type.name.clone()),
                    );
                }
            }
            
            trace!(method_compiler.logger, "  var_types after setup: {:?}", method_compiler.var_types);

            // 编译方法体
            compile_stmt(&mut method_compiler, &lambda.body)?;
            method_compiler.chunk.write_op(OpCode::LoadNull, 0);
            method_compiler.chunk.write_op(OpCode::Return, 0);

            // 创建函数对象
            let arity = lambda.params.len() as u8;
            let function = Box::new(crate::core::object::ObjFunction::new(
                method_compiler.chunk.clone(),
                arity,
                Some(function_name),
            ));
            let function_ptr = Box::into_raw(function);
            let function_value = Value::function(function_ptr);

            // 添加到常量池，获取索引
            let const_idx = compiler.chunk.add_constant(function_value);

            // 添加到方法表（VM 初始化时会注册到 Shape）
            compiler.chunk.method_table.push(MethodTableEntry {
                shape_id,
                method_idx,
                const_idx,
            });
            
            // 如果是运算符方法，添加到运算符表
            if method.name.starts_with("operator ") {
                let op_name = &method.name[9..]; // 去掉 "operator " 前缀
                compiler.chunk.operator_table.push(OperatorTableEntry {
                    shape_id,
                    operator_name: op_name.to_string(),
                    const_idx,
                });
            }
        }
    }

    Ok(())
}

/// 编译 if/elif/else 语句
fn compile_if(compiler: &mut Compiler, if_stmt: &IfStmt) -> Result<(), CompileError> {
    // 编译 if 条件
    crate::runtime::compiler::expr::compile_expr(compiler, &if_stmt.if_condition)?;

    // 如果条件为假，跳过 then 分支
    let then_jump = compiler.chunk.write_jump(OpCode::JumpIfFalse, 0);

    // 编译 then 分支
    compile_stmt(compiler, &if_stmt.then_body)?;

    // then 分支执行完后，跳过所有 elif 和 else 分支
    let else_jump = compiler.chunk.write_jump(OpCode::Jump, 0);

    // 修补 then_jump，指向 elif 或 else 分支的开始位置
    compiler.chunk.patch_jump(then_jump);

    // 保存所有 elif 分支的跳转，最后统一修补到 else 分支之后
    let mut elif_jumps = Vec::new();

    // 编译 elif 分支
    for (elif_cond, elif_body) in if_stmt
        .elif_conditions
        .iter()
        .zip(if_stmt.elif_bodies.iter())
    {
        // 编译 elif 条件
        crate::runtime::compiler::expr::compile_expr(compiler, elif_cond)?;

        // 如果条件为假，跳过这个 elif 的 body
        let elif_jump = compiler.chunk.write_jump(OpCode::JumpIfFalse, 0);

        // 编译 elif body
        compile_stmt(compiler, elif_body)?;

        // elif body 执行完后，跳过剩余的分支
        let next_jump = compiler.chunk.write_jump(OpCode::Jump, 0);
        elif_jumps.push(next_jump);

        // 修补 elif_jump，指向下一个 elif 或 else 分支
        compiler.chunk.patch_jump(elif_jump);
    }

    // 编译 else 分支（如果存在）
    if let Some(else_body) = &if_stmt.else_body {
        compile_stmt(compiler, else_body)?;
    } else {
        // 如果没有 else 分支，加载 null
        compiler.chunk.write_op(OpCode::LoadNull, 0);
    }

    // 修补所有跳转到 else 分支之后的跳转
    if if_stmt.elif_conditions.is_empty() {
        // 只有 if-else，没有 elif
        compiler.chunk.patch_jump(else_jump);
    } else {
        // 有 elif 分支，修补 if 的 else_jump 和所有 elif 的 next_jump
        compiler.chunk.patch_jump(else_jump);
        for jump in elif_jumps {
            compiler.chunk.patch_jump(jump);
        }
    }

    Ok(())
}

/// 编译 while 循环
fn compile_while(compiler: &mut Compiler, while_stmt: &WhileStmt) -> Result<(), CompileError> {
    // 记录循环开始位置
    let loop_start = compiler.chunk.code.len();

    // 编译循环条件
    crate::runtime::compiler::expr::compile_expr(compiler, &while_stmt.condition)?;

    // 如果条件为假，跳出循环
    let exit_jump = compiler.chunk.write_jump(OpCode::JumpIfFalse, 0);

    // 编译循环体
    compile_stmt(compiler, &while_stmt.body)?;

    // 跳回循环开始位置
    compiler.chunk.write_loop(loop_start, 0);

    // 修补退出跳转
    compiler.chunk.patch_jump(exit_jump);

    Ok(())
}

/// 编译 for-in 循环（基于迭代器协议）
/// 语法: for var item in iterable { body }
fn compile_for(compiler: &mut Compiler, for_stmt: &ForStmt) -> Result<(), CompileError> {
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
    var::begin_scope(compiler);

    // 1. 获取迭代器: var $iter = iterable.iter();
    crate::runtime::compiler::expr::compile_expr(compiler, &for_stmt.iterable)?;
    compiler.chunk.write_op(OpCode::GetIter, 0);
    let iter_idx = var::add_local(compiler, "$iter")?;
    var::mark_initialized(compiler);
    var::emit_store_local(compiler, iter_idx);

    // 2. 声明迭代变量（只声明一次，循环内赋值）
    let var_idx = var::add_local(compiler, &var_name)?;
    var::mark_initialized(compiler);

    // 3. 循环开始
    let loop_start = compiler.chunk.code.len();

    // 4. 获取下一个值
    var::emit_load_local(compiler, iter_idx);
    compiler.chunk.write_op(OpCode::IterNext, 0);

    // 5. 复制值用于 null 检查
    compiler.chunk.write_op(OpCode::Dup, 0);

    // 6. 检查是否为 null（结束标记）
    compiler.chunk.write_op(OpCode::LoadNull, 0);
    compiler.chunk.write_op_u8(OpCode::Equal, 0xFF, 0); // Equal 现在需要 cache_idx
    let exit_jump = compiler.chunk.write_jump(OpCode::JumpIfFalse, 0);

    // 是 null，退出循环
    // 注意：JumpIfFalse 已经弹出 true，栈上是 [null]
    compiler.chunk.write_op(OpCode::Pop, 0); // 弹出 null 值
    let exit_patch = compiler.chunk.write_jump(OpCode::Jump, 0); // 跳到循环外

    // 7. 不是 null，赋值给迭代变量
    compiler.chunk.patch_jump(exit_jump);
    // 注意：JumpIfFalse 已经弹出 false，栈上只剩 next 值
    var::emit_store_local(compiler, var_idx); // 弹出 next 值，存入 item

    // 8. 编译循环体
    compile_stmt(compiler, &for_stmt.body)?;

    // 9. 跳回循环开始
    compiler.chunk.write_loop(loop_start, 0);

    // 10. 修补退出跳转
    compiler.chunk.patch_jump(exit_patch);

    // 11. 退出作用域（item 和 $iter 被清理）
    var::end_scope(compiler);

    Ok(())
}

/// 编译模块定义
/// module name { ... }
/// 模块在运行时是一个 ObjModule 对象，导出项按索引存储
fn compile_module(compiler: &mut Compiler, module_stmt: &ModuleStmt) -> Result<(), CompileError> {
    // 进入模块作用域
    var::begin_scope(compiler);

    // 创建模块信息
    let module_info = ModuleInfo {
        name: module_stmt.name.clone(),
        exports: Vec::new(),
        export_name_to_shape_id: std::collections::HashMap::new(),
    };

    // 设置当前模块
    let prev_module = compiler.current_module.take();
    compiler.current_module = Some(module_info);

    // 编译模块体
    compile_stmt(compiler, &module_stmt.body)?;

    // 收集导出信息并生成模块对象
    if let Some(info) = compiler.current_module.take() {
        let export_count = info.exports.len();

        // 对于每个导出项，加载其值（正序压栈，BuildModule 会 reverse）
        for export in info.exports.iter() {
            // 加载导出值（从局部变量）
            var::emit_load_local(compiler, export.local_idx);
        }

        // 生成 BuildModule 指令创建模块对象
        if export_count > 255 {
            return Err(CompileError::Unimplemented(
                "Module has too many exports (max 255)".to_string(),
            ));
        }
        compiler.chunk
            .write_op_u8(OpCode::BuildModule, export_count as u8, 0);

        // 定义全局变量：模块名
        let module_name_obj = Box::new(ObjString::new(module_stmt.name.clone()));
        let module_name_ptr = Box::into_raw(module_name_obj);
        let module_name_val = Value::string(module_name_ptr);
        let module_name_idx = compiler.chunk.add_constant(module_name_val);
        compiler.chunk
            .write_op_u8(OpCode::DefineGlobal, module_name_idx, 0);

        // 保存模块信息
        compiler.modules.push(info);
    }

    // 恢复之前的模块上下文
    compiler.current_module = prev_module;

    // 退出模块作用域
    var::end_scope(compiler);

    Ok(())
}

/// 编译导入语句
/// import module; 或 from module import item;
fn compile_import(
    compiler: &mut Compiler,
    import_stmt: &crate::compiler::parser::stmt::ImportStmt,
) -> Result<(), CompileError> {
    // 检查模块是否存在（同文件内模块或标准库模块）
    let module_name = &import_stmt.module_path;

    if !compiler.is_module_name(module_name) && !is_std_module(compiler, module_name) {
        return Err(CompileError::Unimplemented(format!(
            "Module '{module_name}' not found"
        )));
    }

    // 将模块名加入导入列表
    compiler.imported_modules.push(module_name.clone());

    // 处理 from...import 语句：为每个导入的项创建局部变量
    if !import_stmt.items.is_empty() {
        // from module import item1, item2, ...
        for item_name in &import_stmt.items {
            // 为导入的项创建局部变量
            var::add_local(compiler, item_name)?;

            // 加载模块并获取导出项
            // 1. 加载模块名（字符串常量）
            let module_str_obj = Box::new(ObjString::new(module_name.clone()));
            let module_str_ptr = Box::into_raw(module_str_obj);
            let module_name_val = Value::string(module_str_ptr);
            let module_name_constant = compiler.chunk.add_constant(module_name_val);
            var::emit_constant(compiler, module_name_constant);

            // 2. 获取模块对象
            compiler.chunk.write_op(OpCode::GetModule, 0);

            // 3. 添加导出项名称到常量池（用于 GetModuleExport）
            let item_str_obj = Box::new(ObjString::new(item_name.clone()));
            let item_str_ptr = Box::into_raw(item_str_obj);
            let item_name_val = Value::string(item_str_ptr);
            let item_name_constant = compiler.chunk.add_constant(item_name_val);

            if item_name_constant > 254 {
                return Err(CompileError::TooManyConstants);
            }

            // 4. 使用 GetModuleExport 从模块获取导出项
            // 操作数: u8 常量池索引
            compiler.chunk.write_op(OpCode::GetModuleExport, 0);
            compiler.chunk.code.push(item_name_constant);
            compiler.chunk.lines.push(0);

            // 5. 存储到局部变量
            let local_idx = (compiler.locals.len() - 1) as u8;
            var::emit_store_local(compiler, local_idx);

            // 6. 标记为已初始化
            var::mark_initialized(compiler);
        }
    } else if let Some(alias) = &import_stmt.alias {
        // import module as alias
        // 记录别名到模块名的映射（用于后续成员访问解析）
        compiler.module_aliases
            .insert(alias.clone(), module_name.clone());

        // 为别名创建局部变量
        var::add_local(compiler, alias)?;

        // 加载模块名
        let module_str_obj = Box::new(ObjString::new(module_name.clone()));
        let module_str_ptr = Box::into_raw(module_str_obj);
        let module_name_val = Value::string(module_str_ptr);
        let module_name_constant = compiler.chunk.add_constant(module_name_val);
        var::emit_constant(compiler, module_name_constant);

        // 获取模块对象
        compiler.chunk.write_op(OpCode::GetModule, 0);

        // 存储到别名变量
        let local_idx = (compiler.locals.len() - 1) as u8;
        var::emit_store_local(compiler, local_idx);
        var::mark_initialized(compiler);
    }
    // 否则是简单的 import module;，不做特殊处理（运行时会加载模块到全局）

    Ok(())
}

/// 检查是否是标准库模块
fn is_std_module(_compiler: &Compiler, name: &str) -> bool {
    name == "std"
}
