//! 表达式编译

use crate::passes::lexer::token_kind::KauboTokenKind;
use crate::passes::parser::expr::{FunctionCall, Lambda};
use crate::passes::parser::{Binary, Expr, ExprKind, TypeExpr};
use crate::vm::core::{
    object::{ObjFunction, ObjString},
    OpCode, Value,
};
use kaubo_log::trace;

use super::{
    var, CompileError, Compiler, VarType,
};

/// 编译表达式
pub fn compile_expr(compiler: &mut Compiler, expr: &Expr) -> Result<(), CompileError> {
    match expr.as_ref() {
        ExprKind::LiteralInt(lit) => {
            let value = Value::smi(lit.value as i32);
            let idx = compiler.chunk.add_constant(value);
            var::emit_constant(compiler, idx);
        }

        ExprKind::LiteralFloat(lit) => {
            let value = Value::float(lit.value);
            let idx = compiler.chunk.add_constant(value);
            var::emit_constant(compiler, idx);
        }

        ExprKind::LiteralString(lit) => {
            // 创建字符串对象
            let string_obj = Box::new(ObjString::new(lit.value.clone()));
            let string_ptr = Box::into_raw(string_obj);
            let value = Value::string(string_ptr);
            let idx = compiler.chunk.add_constant(value);
            var::emit_constant(compiler, idx);
        }

        ExprKind::LiteralTrue(_) => {
            compiler.chunk.write_op(OpCode::LoadTrue, 0);
        }

        ExprKind::LiteralFalse(_) => {
            compiler.chunk.write_op(OpCode::LoadFalse, 0);
        }

        ExprKind::LiteralNull(_) => {
            compiler.chunk.write_op(OpCode::LoadNull, 0);
        }

        ExprKind::LiteralList(list) => {
            // 编译所有元素（从左到右，依次压栈）
            let count = list.elements.len();
            if count > 255 {
                return Err(CompileError::Unimplemented(
                    "List too long (max 255 elements)".to_string(),
                ));
            }

            for elem in &list.elements {
                compile_expr(compiler, elem)?;
            }

            // 生成 BuildList 指令
            compiler.chunk.write_op_u8(OpCode::BuildList, count as u8, 0);
        }

        ExprKind::Binary(bin) => {
            compile_binary(compiler, bin)?;
        }

        ExprKind::Unary(un) => {
            compile_expr(compiler, &un.operand)?;
            let op = match un.op {
                KauboTokenKind::Minus => OpCode::Neg,
                KauboTokenKind::Not => OpCode::Not,
                _ => return Err(CompileError::InvalidOperator),
            };
            compiler.chunk.write_op(op, 0);
        }

        ExprKind::Grouping(g) => {
            compile_expr(compiler, &g.expression)?;
        }

        ExprKind::VarRef(var_ref) => {
            match var::resolve_variable(compiler, &var_ref.name) {
                Some(var::Variable::Local(idx)) => var::emit_load_local(compiler, idx),
                Some(var::Variable::Upvalue(idx)) => var::emit_load_upvalue(compiler, idx),
                None => {
                    // 检查是否是模块名
                    if compiler.is_module_name(&var_ref.name) {
                        // 模块作为全局变量访问
                        let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                        let name_ptr = Box::into_raw(name_obj);
                        let name_val = Value::string(name_ptr);
                        let name_idx = compiler.chunk.add_constant(name_val);
                        var::emit_constant(compiler, name_idx);
                        compiler.chunk.write_op(OpCode::LoadGlobal, 0);
                    } else {
                        // 作为全局变量访问（如 std 函数）
                        let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                        let name_ptr = Box::into_raw(name_obj);
                        let name_val = Value::string(name_ptr);
                        let name_idx = compiler.chunk.add_constant(name_val);
                        compiler.chunk.write_op_u8(OpCode::LoadGlobal, name_idx, 0);
                    }
                }
            }
        }

        ExprKind::FunctionCall(call) => {
            compile_function_call(compiler, call)?;
        }

        ExprKind::Lambda(lambda) => {
            compile_lambda(compiler, lambda)?;
        }

        ExprKind::MemberAccess(member) => {
            // 成员访问语法糖: obj.name 等价于 obj["name"]
            // 对于模块访问，使用 ModuleGet 指令（编译期确定的 ShapeID）

            if let ExprKind::VarRef(var_ref) = member.object.as_ref() {
                if let Some(shape_id) = compiler.find_module_shape_id(&var_ref.name, &member.member)
                {
                    // 检查是否是模块别名（局部变量）
                    if compiler.module_aliases.contains_key(&var_ref.name) {
                        // 模块别名是局部变量：LoadLocal + ModuleGet(shape_id)
                        // 查找局部变量索引
                        if let Some(local_idx) = var::resolve_local(compiler, &var_ref.name) {
                            var::emit_load_local(compiler, local_idx);
                        } else {
                            return Err(CompileError::Unimplemented(format!(
                                "Module alias '{}' not found as local variable",
                                var_ref.name
                            )));
                        }
                    } else {
                        // 模块是全局变量：LoadGlobal + ModuleGet(shape_id)
                        let name_obj = Box::new(ObjString::new(var_ref.name.clone()));
                        let name_ptr = Box::into_raw(name_obj);
                        let name_val = Value::string(name_ptr);
                        let name_idx = compiler.chunk.add_constant(name_val);
                        compiler.chunk.write_op_u8(OpCode::LoadGlobal, name_idx, 0);
                    }

                    // ModuleGet 指令（ShapeID 作为 u16 操作数）
                    compiler.chunk.write_op(OpCode::ModuleGet, 0);
                    compiler.chunk.write_u16(shape_id, 0);
                    return Ok(());
                }
            }

            // 尝试获取对象类型，检查是否是 struct
            let obj_type = compiler.get_expr_type(&member.object);
            trace!(
                compiler.logger,
                "compile MemberAccess: obj_type={:?}, member={}",
                obj_type,
                member.member
            );
            let is_struct_field = if let Some(VarType::Struct(struct_name)) = obj_type {
                // 查找 struct 的字段索引
                if let Some(struct_info) = compiler.struct_infos.get(&struct_name) {
                    let field_idx = struct_info.field_names.iter().position(|f| f == &member.member);
                    trace!(
                        compiler.logger,
                        "  struct_info found: shape_id={}, field_names={:?}, field_idx={:?}",
                        struct_info.shape_id,
                        struct_info.field_names,
                        field_idx
                    );
                    field_idx
                } else {
                    trace!(compiler.logger, "  struct_info NOT found for: {}", struct_name);
                    None
                }
            } else {
                None
            };
            
            if let Some(field_idx) = is_struct_field {
                // Struct 字段访问：使用字段索引
                trace!(
                    compiler.logger,
                    "  -> Using GetField, field_idx={}",
                    field_idx
                );
                compile_expr(compiler, &member.object)?;
                compiler.chunk.write_op_u8(OpCode::GetField, field_idx as u8, 0);
            } else {
                // 普通对象访问（JSON）：编译对象 + 字符串键 + IndexGet
                // 注意：struct 不再支持字符串键访问
                trace!(compiler.logger, "  -> Using IndexGet (fallback for JSON)");
                compile_expr(compiler, &member.object)?;
                let key_obj = Box::new(ObjString::new(member.member.clone()));
                let key_ptr = Box::into_raw(key_obj);
                let key_val = Value::string(key_ptr);
                let idx = compiler.chunk.add_constant(key_val);
                var::emit_constant(compiler, idx);
                compiler.chunk.write_op(OpCode::IndexGet, 0);
            }
        }

        ExprKind::IndexAccess(index) => {
            // 普通索引访问: list[index]
            // 注意：字符串字面量索引已在类型检查阶段报错
            compile_expr(compiler, &index.object)?; // list
            compile_expr(compiler, &index.index)?; // index
            compiler.chunk.write_op(OpCode::IndexGet, 0);
        }

        ExprKind::JsonLiteral(json) => {
            // 编译 JSON 字面量: json { "key": value, ... }
            // 栈布局: [value1, key1, value2, key2, ...]（从后往前压栈）
            let count = json.entries.len();
            if count > 255 {
                return Err(CompileError::Unimplemented(
                    "JSON literal too large (max 255 entries)".to_string(),
                ));
            }

            // 逆序处理，这样 BuildJson 可以按正确顺序弹出
            for (key, value) in json.entries.iter().rev() {
                // 值先入栈
                compile_expr(compiler, value)?;
                // 键（字符串）入栈
                let key_obj = Box::new(ObjString::new(key.clone()));
                let key_ptr = Box::into_raw(key_obj);
                let key_val = Value::string(key_ptr);
                let idx = compiler.chunk.add_constant(key_val);
                var::emit_constant(compiler, idx);
            }

            compiler.chunk.write_op_u8(OpCode::BuildJson, count as u8, 0);
        }

        ExprKind::Yield(y) => {
            // 编译 yield 表达式
            if let Some(value) = &y.value {
                compile_expr(compiler, value)?;
            } else {
                // yield; 等价于 yield null;
                compiler.chunk.write_op(OpCode::LoadNull, 0);
            }
            compiler.chunk.write_op(OpCode::Yield, 0);
        }

        ExprKind::StructLiteral(struct_lit) => {
            // 编译 struct 字面量
            let struct_info = compiler.struct_infos.get(&struct_lit.name).ok_or_else(|| {
                CompileError::Unimplemented(format!(
                    "Struct '{}' not found in shape table",
                    struct_lit.name
                ))
            })?;

            let shape_id = struct_info.shape_id;
            let field_count = struct_lit.fields.len();

            // 将 Vec 转换为 HashMap 便于查找
            let field_map: std::collections::HashMap<String, Expr> = struct_lit
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            // 按 struct 定义的顺序编译字段
            // 如果 struct_info 有字段信息，按那个顺序；否则按字母排序
            let field_order: Vec<String> = if struct_info.field_names.is_empty() {
                //  fallback：按字母排序
                let mut names: Vec<_> = field_map.keys().cloned().collect();
                names.sort();
                names
            } else {
                struct_info.field_names.clone()
            };

            // 按字段定义顺序逆序入栈
            for field_name in field_order.iter().rev() {
                if let Some(value_expr) = field_map.get(field_name) {
                    compile_expr(compiler, value_expr)?;
                } else {
                    return Err(CompileError::Unimplemented(format!(
                        "Missing field '{}' in struct '{}'",
                        field_name, struct_lit.name
                    )));
                }
            }

            // 生成 BuildStruct 指令
            compiler.chunk
                .write_op_u16_u8(OpCode::BuildStruct, shape_id, field_count as u8, 0);
        }

        ExprKind::As(as_expr) => {
            // 编译源表达式
            compile_expr(compiler, &as_expr.expr)?;
            // 生成类型转换指令
            compile_cast(compiler, &as_expr.target_type)?;
        }
    }
    Ok(())
}

/// 编译类型转换指令
fn compile_cast(compiler: &mut Compiler, target_type: &TypeExpr) -> Result<(), CompileError> {
    // 根据目标类型生成相应的转换指令
    match target_type {
        TypeExpr::Named(named) => {
            match named.name.as_str() {
                "int" => compiler.chunk.write_op(OpCode::CastToInt, 0),
                "float" => compiler.chunk.write_op(OpCode::CastToFloat, 0),
                "string" => compiler.chunk.write_op(OpCode::CastToString, 0),
                "bool" => compiler.chunk.write_op(OpCode::CastToBool, 0),
                _ => {
                    return Err(CompileError::Unimplemented(format!(
                        "Cast to type '{}' not supported",
                        named.name
                    )))
                }
            }
        }
        _ => {
            return Err(CompileError::Unimplemented(
                "Cast to complex types not supported".to_string(),
            ))
        }
    }
    Ok(())
}

/// 编译二元运算
fn compile_binary(compiler: &mut Compiler, bin: &Binary) -> Result<(), CompileError> {
    // 特殊处理赋值运算符：=
    if bin.op == KauboTokenKind::Equal {
        return compile_assignment(compiler, &bin.left, &bin.right);
    }

    // 短路求值：and
    if bin.op == KauboTokenKind::And {
        return compile_and(compiler, &bin.left, &bin.right);
    }

    // 短路求值：or
    if bin.op == KauboTokenKind::Or {
        return compile_or(compiler, &bin.left, &bin.right);
    }

    // 先编译左操作数
    compile_expr(compiler, &bin.left)?;

    // 再编译右操作数
    compile_expr(compiler, &bin.right)?;

    // 生成运算指令
    // 对于可能触发运算符重载的指令，分配内联缓存槽位
    let (op, needs_cache) = match bin.op {
        KauboTokenKind::Plus => (OpCode::Add, true),
        KauboTokenKind::Minus => (OpCode::Sub, true),
        KauboTokenKind::Asterisk => (OpCode::Mul, true),
        KauboTokenKind::Slash => (OpCode::Div, true),
        KauboTokenKind::Percent => (OpCode::Mod, true),
        KauboTokenKind::DoubleEqual => (OpCode::Equal, false),
        KauboTokenKind::ExclamationEqual => (OpCode::NotEqual, false),
        KauboTokenKind::GreaterThan => (OpCode::Greater, true),
        KauboTokenKind::GreaterThanEqual => (OpCode::GreaterEqual, true),
        KauboTokenKind::LessThan => (OpCode::Less, true),
        KauboTokenKind::LessThanEqual => (OpCode::LessEqual, true),
        _ => return Err(CompileError::InvalidOperator),
    };

    if needs_cache {
        // 分配内联缓存槽位
        let cache_idx = compiler.chunk.allocate_inline_cache();
        // 写入带缓存索引的指令
        compiler.chunk.write_op_u8(op, cache_idx, 0);
    } else {
        // 不需要缓存的指令，使用 0xFF 作为占位
        compiler.chunk.write_op_u8(op, 0xFF, 0);
    }
    Ok(())
}

/// 编译逻辑与 (and) - 短路求值
/// a and b 等价于: if a then b else a
fn compile_and(compiler: &mut Compiler, left: &Expr, right: &Expr) -> Result<(), CompileError> {
    // 编译左操作数
    compile_expr(compiler, left)?;

    // 复制左操作数用于条件判断（因为 JumpIfFalse 会弹出值）
    compiler.chunk.write_op(OpCode::Dup, 0);

    // 如果左操作数为假，跳转到结束（保留左操作数作为结果）
    let jump_offset = compiler.chunk.write_jump(OpCode::JumpIfFalse, 0);

    // 左操作数为真，需要计算右操作数
    // 弹出复制的左操作数值
    compiler.chunk.write_op(OpCode::Pop, 0);

    // 编译右操作数
    compile_expr(compiler, right)?;

    // 修补跳转偏移量
    compiler.chunk.patch_jump(jump_offset);

    Ok(())
}

/// 编译逻辑或 (or) - 短路求值
/// a or b 等价于: if a then a else b
fn compile_or(compiler: &mut Compiler, left: &Expr, right: &Expr) -> Result<(), CompileError> {
    // 编译左操作数
    compile_expr(compiler, left)?;

    // 复制左操作数用于条件判断
    compiler.chunk.write_op(OpCode::Dup, 0);

    // 如果为假，跳转到右操作数计算
    let jump_if_false = compiler.chunk.write_jump(OpCode::JumpIfFalse, 0);

    // 左操作数为真：直接短路，保留左操作数
    // 不需要 Pop，因为 JumpIfFalse 已经弹出了复制的值
    // 原始左操作数仍在栈上作为结果
    let end_jump = compiler.chunk.write_jump(OpCode::Jump, 0);

    // 修补跳转到右操作数计算的位置
    compiler.chunk.patch_jump(jump_if_false);

    // 左操作数为假：弹出 falsy 的左操作数，计算右操作数
    compiler.chunk.write_op(OpCode::Pop, 0);
    compile_expr(compiler, right)?;

    // 修补结束跳转
    compiler.chunk.patch_jump(end_jump);

    Ok(())
}

/// 编译赋值表达式 (处理 Binary 形式的赋值)
/// 赋值表达式返回 null（语句级别的副作用）
fn compile_assignment(compiler: &mut Compiler, left: &Expr, right: &Expr) -> Result<(), CompileError> {
    // 左值可能是变量引用或索引访问
    match left.as_ref() {
        ExprKind::VarRef(var_ref) => {
            // 编译右值
            compile_expr(compiler, right)?;

            match var::resolve_variable(compiler, &var_ref.name) {
                Some(var::Variable::Local(idx)) => var::emit_store_local(compiler, idx),
                Some(var::Variable::Upvalue(idx)) => var::emit_store_upvalue(compiler, idx),
                None => {
                    return Err(CompileError::Unimplemented(format!(
                        "Undefined variable: {}",
                        var_ref.name
                    )));
                }
            }
            // 赋值表达式返回 null（不是被赋的值）
            compiler.chunk.write_op(OpCode::LoadNull, 0);
            Ok(())
        }

        ExprKind::IndexAccess(index) => {
            // 索引赋值: list[index] = value
            // 栈布局: [value, index, list]
            compile_expr(compiler, right)?; // value
            compile_expr(compiler, &index.index)?; // index
            compile_expr(compiler, &index.object)?; // list
            compiler.chunk.write_op(OpCode::IndexSet, 0);
            // 返回 null
            compiler.chunk.write_op(OpCode::LoadNull, 0);
            Ok(())
        }

        ExprKind::MemberAccess(member) => {
            // 成员赋值: obj.name = value
            // 等价于 obj["name"] = value
            // 栈布局: [value, "name", obj]
            compile_expr(compiler, right)?; // value
                                           // 成员名作为字符串
            let key_obj = Box::new(ObjString::new(member.member.clone()));
            let key_ptr = Box::into_raw(key_obj);
            let key_val = Value::string(key_ptr);
            let idx = compiler.chunk.add_constant(key_val);
            var::emit_constant(compiler, idx); // key
            compile_expr(compiler, &member.object)?; // obj
            compiler.chunk.write_op(OpCode::IndexSet, 0);
            // 返回 null
            compiler.chunk.write_op(OpCode::LoadNull, 0);
            Ok(())
        }

        _ => Err(CompileError::Unimplemented(
            "Left side of assignment must be a variable, index access, or member access"
                .to_string(),
        )),
    }
}

/// 编译 lambda 表达式
fn compile_lambda(compiler: &mut Compiler, lambda: &Lambda) -> Result<(), CompileError> {
    // 创建子编译器，传递 self 作为 enclosing（用于解析 upvalue）
    let mut function_compiler = Compiler::new_child(compiler);

    // 为每个参数添加局部变量
    for (param_name, _param_type) in &lambda.params {
        var::add_local(&mut function_compiler, param_name)?;
        var::mark_initialized(&mut function_compiler);
    }

    // 编译函数体
    super::stmt::compile_stmt(&mut function_compiler, &lambda.body)?;

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
    let function_ptr = Box::into_raw(function);
    let function_value = Value::function(function_ptr);
    let idx = compiler.chunk.add_constant(function_value);

    // 发射 Closure 指令：函数索引 + upvalue数量 + upvalue描述符
    compiler.chunk.write_op(OpCode::Closure, 0);
    compiler.chunk.code.push(idx);
    compiler.chunk.lines.push(0);
    compiler.chunk.code.push(upvalues.len() as u8);
    compiler.chunk.lines.push(0);

    // 输出 upvalue 描述符
    for upvalue in &upvalues {
        compiler.chunk.code.push(if upvalue.is_local { 1 } else { 0 });
        compiler.chunk.lines.push(0);
        compiler.chunk.code.push(upvalue.index);
        compiler.chunk.lines.push(0);
    }

    Ok(())
}

/// 编译函数调用
fn compile_function_call(compiler: &mut Compiler, call: &FunctionCall) -> Result<(), CompileError> {
    // 检测是否是方法调用：obj.method(args)
    if let ExprKind::MemberAccess(member) = call.function_expr.as_ref() {
        // 尝试推断 receiver 类型
        let receiver_type = infer_receiver_type(compiler, &member.object);
        
        // 处理内置类型方法调用 - 使用 CallBuiltin 指令
        if let Some(BuiltinType::List) = receiver_type {
            use crate::vm::core::builtin_methods::{BuiltinMethodTable, builtin_types};
            if let Some(method_idx) = BuiltinMethodTable::resolve_list_method(&member.member) {
                // 字节码顺序：
                // 1. 加载 receiver
                // 2. 加载参数
                // 3. CallBuiltin(type, method, arg_count)
                compile_expr(compiler, &member.object)?;  // [receiver]
                for arg in call.arguments.iter() {
                    compile_expr(compiler, arg)?;         // [receiver, arg1, arg2...]
                }
                let arg_count = (call.arguments.len() + 1) as u8;
                compiler.chunk.write_op(OpCode::CallBuiltin, 0);
                compiler.chunk.code.push(builtin_types::LIST);  // type_tag
                compiler.chunk.lines.push(0);
                compiler.chunk.code.push(method_idx);
                compiler.chunk.lines.push(0);
                compiler.chunk.code.push(arg_count);
                compiler.chunk.lines.push(0);
                return Ok(());
            }
        }
        
        if let Some(BuiltinType::String) = receiver_type {
            use crate::vm::core::builtin_methods::{BuiltinMethodTable, builtin_types};
            if let Some(method_idx) = BuiltinMethodTable::resolve_string_method(&member.member) {
                compile_expr(compiler, &member.object)?;
                for arg in call.arguments.iter() {
                    compile_expr(compiler, arg)?;
                }
                let arg_count = (call.arguments.len() + 1) as u8;
                compiler.chunk.write_op(OpCode::CallBuiltin, 0);
                compiler.chunk.code.push(builtin_types::STRING);
                compiler.chunk.lines.push(0);
                compiler.chunk.code.push(method_idx);
                compiler.chunk.lines.push(0);
                compiler.chunk.code.push(arg_count);
                compiler.chunk.lines.push(0);
                return Ok(());
            }
        }
        
        if let Some(BuiltinType::Json) = receiver_type {
            use crate::vm::core::builtin_methods::{BuiltinMethodTable, builtin_types};
            if let Some(method_idx) = BuiltinMethodTable::resolve_json_method(&member.member) {
                compile_expr(compiler, &member.object)?;
                for arg in call.arguments.iter() {
                    compile_expr(compiler, arg)?;
                }
                let arg_count = (call.arguments.len() + 1) as u8;
                compiler.chunk.write_op(OpCode::CallBuiltin, 0);
                compiler.chunk.code.push(builtin_types::JSON);
                compiler.chunk.lines.push(0);
                compiler.chunk.code.push(method_idx);
                compiler.chunk.lines.push(0);
                compiler.chunk.code.push(arg_count);
                compiler.chunk.lines.push(0);
                return Ok(());
            }
        }
        
        // 尝试编译为 Struct 方法调用
        if let Some(struct_name) = try_infer_struct_type(compiler, &member.object) {
            // 获取方法索引
            let method_idx = compiler.struct_infos.get(&struct_name).and_then(|info| {
                info.method_names
                    .iter()
                    .position(|name| name == &member.member)
                    .map(|idx| idx as u8)
            });

            if let Some(idx) = method_idx {
                // 编译参数（先编译 self）
                compile_expr(compiler, &member.object)?; // self
                for arg in call.arguments.iter() {
                    compile_expr(compiler, arg)?;
                }

                // 从 Shape 加载方法函数
                compiler.chunk.write_op_u8(OpCode::LoadMethod, idx, 0);

                // 调用方法（参数数包含 self）
                let arg_count = (call.arguments.len() + 1) as u8;
                compiler.chunk.write_op_u8(OpCode::Call, arg_count, 0);
                return Ok(());
            }
        }
    }

    // 普通函数调用
    // 先编译参数（参数从左到右压栈）
    for arg in call.arguments.iter() {
        compile_expr(compiler, arg)?;
    }

    // 编译函数表达式
    compile_expr(compiler, &call.function_expr)?;

    // 发射 Call 指令
    let arg_count = call.arguments.len() as u8;
    compiler.chunk.write_op_u8(OpCode::Call, arg_count, 0);

    Ok(())
}

/// 内置类型枚举
#[derive(Debug, Clone, Copy, PartialEq)]
enum BuiltinType {
    List,
    String,
    Json,
}

/// 推断 receiver 类型（用于方法调用）
fn infer_receiver_type(compiler: &Compiler, expr: &Expr) -> Option<BuiltinType> {
    match expr.as_ref() {
        ExprKind::VarRef(var_ref) => {
            // 从变量类型表查找
            if let Some(var_type) = compiler.var_types.get(&var_ref.name) {
                match var_type {
                    VarType::List(_) => Some(BuiltinType::List),
                    VarType::String => Some(BuiltinType::String),
                    VarType::Json => Some(BuiltinType::Json),
                    _ => None,
                }
            } else {
                None
            }
        }
        ExprKind::LiteralList(_) => Some(BuiltinType::List),
        ExprKind::LiteralString(_) => Some(BuiltinType::String),
        ExprKind::JsonLiteral(_) => Some(BuiltinType::Json),
        
        // 处理链式调用：list.push(1).push(2)
        // 如果方法返回 receiver（如 push），则结果类型与 receiver 相同
        ExprKind::FunctionCall(call) => {
            if let ExprKind::MemberAccess(member) = call.function_expr.as_ref() {
                let receiver_type = infer_receiver_type(compiler, &member.object);
                
                // 检查是否是返回 receiver 的方法（支持链式调用）
                match receiver_type {
                    Some(BuiltinType::List) => {
                        use crate::vm::core::builtin_methods::BuiltinMethodTable;
                        if BuiltinMethodTable::resolve_list_method(&member.member).is_some() {
                            // 返回 List 的方法支持链式调用
                            // - push/clear: 返回 receiver 本身
                            // - filter/map: 返回新的 List
                            // - 其他方法 (len/is_empty等): 不返回 List，不能链式调用
                            match member.member.as_str() {
                                "push" | "clear" | "filter" | "map" => Some(BuiltinType::List),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        
        _ => None,
    }
}

/// 尝试推断表达式的 struct 类型
fn try_infer_struct_type(compiler: &Compiler, expr: &Expr) -> Option<String> {
    // 从变量类型表中查找
    if let ExprKind::VarRef(var_ref) = expr.as_ref() {
        if let Some(var_type) = compiler.var_types.get(&var_ref.name) {
            if let VarType::Struct(struct_name) = var_type {
                return Some(struct_name.clone());
            }
        }
    }
    None
}
