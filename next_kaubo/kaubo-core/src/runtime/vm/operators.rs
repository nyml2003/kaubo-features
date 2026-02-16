//! 运算符实现 (add_values, sub_values 等)

use crate::core::{
    InlineCacheEntry, ObjClosure, ObjShape, ObjString, Operator, Value, VM,
};

/// 加法（仅基础类型）
pub fn add_values(_vm: &VM, a: Value, b: Value) -> Result<Value, String> {
    // 字符串拼接
    if let (Some(ap), Some(bp)) = (a.as_string(), b.as_string()) {
        let a_str = unsafe { &(*ap).chars };
        let b_str = unsafe { &(*bp).chars };
        let concatenated = format!("{a_str}{b_str}");
        let string_obj = Box::new(ObjString::new(concatenated));
        return Ok(Value::string(Box::into_raw(string_obj)));
    }

    // 优先尝试整数加法
    if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
        // 检查溢出
        if let Some(sum) = ai.checked_add(bi) {
            if (-(1 << 30)..(1 << 30)).contains(&sum) {
                return Ok(Value::smi(sum));
            }
        }
    }

    // 检查是否都是数值类型
    let a_is_num = a.is_int() || a.is_float();
    let b_is_num = b.is_int() || b.is_float();

    if a_is_num && b_is_num {
        // 回退到浮点数
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        return Ok(Value::float(af + bf));
    }

    // 不是基础类型，返回错误以便尝试运算符重载
    Err("Non-primitive types need operator overloading".to_string())
}

/// 减法（仅基础类型）
pub fn sub_values(_vm: &VM, a: Value, b: Value) -> Result<Value, String> {
    if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
        if let Some(diff) = ai.checked_sub(bi) {
            if (-(1 << 30)..(1 << 30)).contains(&diff) {
                return Ok(Value::smi(diff));
            }
        }
    }

    let a_is_num = a.is_int() || a.is_float();
    let b_is_num = b.is_int() || b.is_float();

    if a_is_num && b_is_num {
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        return Ok(Value::float(af - bf));
    }

    Err("Non-primitive types need operator overloading".to_string())
}

/// 乘法（仅基础类型）
pub fn mul_values(_vm: &VM, a: Value, b: Value) -> Result<Value, String> {
    if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
        if let Some(prod) = ai.checked_mul(bi) {
            if (-(1 << 30)..(1 << 30)).contains(&prod) {
                return Ok(Value::smi(prod));
            }
        }
    }

    let a_is_num = a.is_int() || a.is_float();
    let b_is_num = b.is_int() || b.is_float();

    if a_is_num && b_is_num {
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        return Ok(Value::float(af * bf));
    }

    Err("Non-primitive types need operator overloading".to_string())
}

/// 除法（仅基础类型）
pub fn div_values(_vm: &VM, a: Value, b: Value) -> Result<Value, String> {
    let a_is_num = a.is_int() || a.is_float();
    let b_is_num = b.is_int() || b.is_float();

    if a_is_num && b_is_num {
        // 除法总是返回浮点数（避免整数除法的困惑）
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        if bf == 0.0 {
            return Err("Division by zero".to_string());
        }

        return Ok(Value::float(af / bf));
    }

    Err("Non-primitive types need operator overloading".to_string())
}

/// 取模/求余（仅基础类型）
pub fn mod_values(_vm: &VM, a: Value, b: Value) -> Result<Value, String> {
    let a_is_num = a.is_int() || a.is_float();
    let b_is_num = b.is_int() || b.is_float();

    if !a_is_num || !b_is_num {
        return Err("Non-primitive types need operator mod".to_string());
    }

    if a_is_num && b_is_num {
        // 优先尝试整数取模
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            if bi == 0 {
                return Err("Modulo by zero".to_string());
            }
            return Ok(Value::smi(ai % bi));
        }

        // 回退到浮点数
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        if bf == 0.0 {
            return Err("Modulo by zero".to_string());
        }

        return Ok(Value::float(af % bf));
    }

    Err("Non-primitive types need operator overloading".to_string())
}

/// 取负（仅基础类型）
pub fn neg_value(_vm: &VM, v: Value) -> Result<Value, String> {
    // 检查是否为数值类型
    let is_numeric = v.is_int() || v.is_float();

    if !is_numeric {
        return Err("Non-primitive type needs operator neg".to_string());
    }

    if let Some(i) = v.as_smi() {
        if i != i32::MIN {
            // 避免溢出
            return Ok(Value::smi(-i));
        }
    }

    let f = if v.is_float() {
        v.as_float()
    } else {
        v.as_smi().map(|n| n as f64).unwrap_or(0.0)
    };
    Ok(Value::float(-f))
}

/// 比较（仅基础数值类型）
pub fn compare_values(_vm: &VM, a: Value, b: Value) -> Result<std::cmp::Ordering, String> {
    // 检查是否都是数值类型
    let a_is_num = a.is_int() || a.is_float();
    let b_is_num = b.is_int() || b.is_float();

    if !a_is_num || !b_is_num {
        return Err("Non-primitive types need operator lt".to_string());
    }

    let af = if a.is_float() {
        a.as_float()
    } else {
        a.as_smi().map(|n| n as f64).unwrap_or(0.0)
    };

    let bf = if b.is_float() {
        b.as_float()
    } else {
        b.as_smi().map(|n| n as f64).unwrap_or(0.0)
    };

    Ok(af.partial_cmp(&bf).unwrap_or(std::cmp::Ordering::Equal))
}

// ==================== 运算符重载 ====================

/// 获取值的 Shape ID
pub fn get_shape_id(_vm: &VM, value: Value) -> u16 {
    // 基础类型使用预定义 Shape ID
    if value.is_int() {
        0 // Int
    } else if value.is_float() {
        1 // Float
    } else if value.is_string() {
        2 // String
    } else if value.is_list() {
        3 // List
    } else if value.is_json() {
        4 // Json
    } else if value.is_closure() {
        5 // Function/Closure
    } else if value.is_module() {
        6 // Module
    } else if value.is_struct() {
        // Struct 需要动态获取 Shape ID
        if let Some(ptr) = value.as_struct() {
            unsafe { (*ptr).shape_id() }
        } else {
            0
        }
    } else {
        0
    }
}

/// 获取值的类型名称（用于错误信息）
pub fn get_type_name(value: Value) -> &'static str {
    if value.is_int() {
        "int"
    } else if value.is_float() {
        "float"
    } else if value.is_string() {
        "string"
    } else if value.is_list() {
        "List"
    } else if value.is_json() {
        "json"
    } else if value.is_closure() {
        "function"
    } else if value.is_module() {
        "module"
    } else if value.is_struct() {
        "struct"
    } else if value.is_coroutine() {
        "coroutine"
    } else if value.is_null() {
        "null"
    } else if value.is_bool() {
        "bool"
    } else {
        "unknown"
    }
}

/// 查找运算符（Level 3：元表查找）
pub fn find_operator(vm: &VM, value: Value, op: Operator) -> Option<*mut ObjClosure> {
    let shape_id = get_shape_id(vm, value);

    // 获取 Shape
    if let Some(shape_ptr) = vm.shapes.get(&shape_id) {
        let shape = unsafe { &**shape_ptr };
        return shape.get_operator(op);
    }

    None
}

/// 调用二元运算符（带反向运算符回退）
pub fn call_binary_operator(
    vm: &mut VM,
    op: Operator,
    a: Value,
    b: Value,
) -> Result<Value, String> {
    // 1. 尝试左操作数的运算符
    if let Some(closure) = find_operator(vm, a, op) {
        return call_operator_closure(vm, closure, &[a, b]);
    }

    // 2. 尝试反向运算符
    if let Some(reverse_op) = op.reverse() {
        if let Some(closure) = find_operator(vm, b, reverse_op) {
            return call_operator_closure(vm, closure, &[b, a]);
        }
    }

    // 3. 报错
    Err(format!(
        "OperatorError: 类型 '{}' 不支持运算符 '{}'",
        get_type_name(a),
        op.symbol()
    ))
}

/// 调用 operator call（可调用对象，变长参数）
pub fn call_callable_operator(
    vm: &mut VM,
    op: Operator,
    args: &[Value],
) -> Result<Value, String> {
    if args.is_empty() {
        return Err("call operator requires at least self argument".to_string());
    }

    let self_val = args[0];

    // 查找 operator call
    if let Some(closure) = find_operator(vm, self_val, op) {
        return call_operator_closure_varargs(vm, closure, args);
    }

    Err(format!(
        "OperatorError: 类型 '{}' 不可调用",
        get_type_name(self_val)
    ))
}

/// 调用运算符闭包（变长参数版本）
pub fn call_operator_closure_varargs(
    vm: &mut VM,
    closure: *mut ObjClosure,
    args: &[Value],
) -> Result<Value, String> {
    use crate::core::ObjFunction;
    use crate::core::OpCode::*;

    let closure_ref = unsafe { &*closure };
    let func = unsafe { &*closure_ref.function };

    if func.arity != args.len() as u8 {
        return Err(format!(
            "Expected {} arguments but got {}",
            func.arity,
            args.len()
        ));
    }

    // 创建局部变量表（参数）
    let mut locals: Vec<Value> = args.to_vec();

    // 创建指令指针
    let mut ip = func.chunk.code.as_ptr();
    let code_end = unsafe { func.chunk.code.as_ptr().add(func.chunk.code.len()) };

    // 执行运算符闭包的字节码
    loop {
        if ip >= code_end {
            return Err("Unexpected end of operator bytecode".to_string());
        }

        let instruction = unsafe { *ip };
        ip = unsafe { ip.add(1) };
        let op = unsafe { std::mem::transmute::<u8, crate::core::OpCode>(instruction) };

        match op {
            LoadConst => {
                let idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                if let Some(val) = func.chunk.constants.get(idx as usize) {
                    vm.stack.push(*val);
                }
            }
            LoadConst0 => {
                if let Some(val) = func.chunk.constants.first() {
                    vm.stack.push(*val);
                }
            }
            LoadConst1 => {
                if let Some(val) = func.chunk.constants.get(1) {
                    vm.stack.push(*val);
                }
            }
            LoadConst2 => {
                if let Some(val) = func.chunk.constants.get(2) {
                    vm.stack.push(*val);
                }
            }
            LoadConst3 => {
                if let Some(val) = func.chunk.constants.get(3) {
                    vm.stack.push(*val);
                }
            }
            LoadNull => vm.stack.push(Value::NULL),
            LoadTrue => vm.stack.push(Value::TRUE),
            LoadFalse => vm.stack.push(Value::FALSE),

            LoadLocal0 => vm.stack.push(locals[0]),
            LoadLocal1 => vm.stack.push(locals[1]),
            LoadLocal2 => vm.stack.push(locals[2]),
            LoadLocal3 => vm.stack.push(locals[3]),
            LoadLocal => {
                let idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                vm.stack.push(locals[idx as usize]);
            }
            StoreLocal => {
                let idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let val = vm.stack.pop().expect("Stack underflow");
                if (idx as usize) < locals.len() {
                    locals[idx as usize] = val;
                } else {
                    locals.resize(idx as usize + 1, Value::NULL);
                    locals[idx as usize] = val;
                }
            }
            StoreLocal0 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.is_empty() {
                    locals.push(val);
                } else {
                    locals[0] = val;
                }
            }
            StoreLocal1 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.len() < 2 {
                    locals.resize(2, Value::NULL);
                }
                locals[1] = val;
            }
            StoreLocal2 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.len() < 3 {
                    locals.resize(3, Value::NULL);
                }
                locals[2] = val;
            }
            StoreLocal3 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.len() < 4 {
                    locals.resize(4, Value::NULL);
                }
                locals[3] = val;
            }

            // 算术运算
            Add => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match add_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Sub => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match sub_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Mul => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match mul_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Div => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match div_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Mod => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match mod_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Neg => {
                let v = vm.stack.pop().expect("Stack underflow");
                match neg_value(vm, v) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }

            // 比较运算
            Equal => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                vm.stack.push(Value::bool_from(a == b));
            }
            NotEqual => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                vm.stack.push(Value::bool_from(a != b));
            }
            Greater => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match compare_values(vm, a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(ord == std::cmp::Ordering::Greater)),
                    Err(e) => return Err(e),
                }
            }
            Less => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match compare_values(vm, a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(ord == std::cmp::Ordering::Less)),
                    Err(e) => return Err(e),
                }
            }
            GreaterEqual => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match compare_values(vm, a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(
                        ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal,
                    )),
                    Err(e) => return Err(e),
                }
            }
            LessEqual => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match compare_values(vm, a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(
                        ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal,
                    )),
                    Err(e) => return Err(e),
                }
            }

            // 栈操作
            Pop => {
                vm.stack.pop();
            }
            Dup => {
                let v = vm.stack.last().copied().unwrap();
                vm.stack.push(v);
            }

            // 跳转
            Jump => {
                let offset = crate::runtime::vm::execution::read_i16_at_ptr(ip);
                ip = unsafe { ip.add(2) };
                ip = unsafe { ip.offset(offset as isize) };
            }
            JumpIfFalse => {
                let offset = crate::runtime::vm::execution::read_i16_at_ptr(ip);
                ip = unsafe { ip.add(2) };
                let val = vm.stack.pop().expect("Stack underflow");
                if !val.is_truthy() {
                    ip = unsafe { ip.offset(offset as isize) };
                }
            }

            // 结构体
            BuildStruct => {
                let shape_id = crate::runtime::vm::execution::read_u16_at_ptr(ip);
                ip = unsafe { ip.add(2) };
                let field_count = unsafe { *ip };
                ip = unsafe { ip.add(1) };

                let mut fields = Vec::with_capacity(field_count as usize);
                for _ in 0..field_count {
                    fields.push(vm.stack.pop().expect("Stack underflow"));
                }

                let shape_ptr = crate::runtime::vm::shape::get_shape(vm, shape_id);
                if shape_ptr.is_null() {
                    return Err(format!("Shape ID {shape_id} not found"));
                }

                let obj = crate::core::ObjStruct::new(shape_ptr, fields);
                let ptr = Box::into_raw(Box::new(obj));
                vm.stack.push(Value::struct_instance(ptr));
            }

            GetField => {
                let field_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let obj_val = vm.stack.pop().expect("Stack underflow");
                if let Some(ptr) = obj_val.as_struct() {
                    let obj = unsafe { &*ptr };
                    if (field_idx as usize) < obj.fields.len() {
                        vm.stack.push(obj.fields[field_idx as usize]);
                    } else {
                        return Err("Field index out of bounds".to_string());
                    }
                } else {
                    return Err(format!(
                        "Expected struct instance, got {}",
                        get_type_name(obj_val)
                    ));
                }
            }

            IndexGet => {
                let index_val = vm.stack.pop().expect("Stack underflow");
                let obj_val = vm.stack.pop().expect("Stack underflow");

                // 整数索引：List 或 Struct 字段
                if let Some(idx) = index_val.as_smi() {
                    let i = idx as usize;

                    if let Some(list_ptr) = obj_val.as_list() {
                        let list = unsafe { &*list_ptr };
                        if i >= list.len() {
                            return Err(format!(
                                "Index out of bounds: {} (length {})",
                                i,
                                list.len()
                            ));
                        }
                        vm.stack.push(list.get(i).unwrap_or(Value::NULL));
                    } else if let Some(struct_ptr) = obj_val.as_struct() {
                        let struct_obj = unsafe { &*struct_ptr };
                        if i < struct_obj.field_count() {
                            vm.stack.push(struct_obj.fields[i]);
                        } else {
                            return Err(format!("Field index out of bounds: {i}"));
                        }
                    } else {
                        return Err("Expected list or struct for integer index".to_string());
                    }
                } else {
                    return Err("Expected integer index".to_string());
                }
            }

            ReturnValue => {
                return Ok(vm.stack.pop().expect("Stack underflow"));
            }
            Return => {
                return Ok(Value::NULL);
            }

            _ => {
                return Err(format!("Unsupported opcode in operator: {op:?}"));
            }
        }
    }
}

/// 调用一元运算符（Neg, Not 等）
#[allow(dead_code)]
pub fn call_unary_operator(vm: &mut VM, op: Operator, value: Value) -> Result<Value, String> {
    if let Some(closure) = find_operator(vm, value, op) {
        return call_operator_closure(vm, closure, &[value]);
    }

    Err(format!(
        "OperatorError: 类型 '{}' 不支持运算符 '{}'",
        get_type_name(value),
        op.symbol()
    ))
}

/// 调用运算符闭包（辅助方法）
pub fn call_operator_closure(
    vm: &mut VM,
    closure: *mut ObjClosure,
    args: &[Value],
) -> Result<Value, String> {
    use crate::core::ObjFunction;
    use crate::core::OpCode::*;

    // 获取闭包信息
    let closure_ref = unsafe { &*closure };
    let func = unsafe { &*closure_ref.function };

    if func.arity != args.len() as u8 {
        return Err(format!(
            "Expected {} arguments but got {}",
            func.arity,
            args.len()
        ));
    }

    // 创建局部变量表（参数）
    let mut locals: Vec<Value> = args.to_vec();

    // 执行运算符闭包的字节码

    // 创建指令指针
    let mut ip = func.chunk.code.as_ptr();
    let code_end = unsafe { func.chunk.code.as_ptr().add(func.chunk.code.len()) };

    // 执行运算符闭包的字节码
    loop {
        if ip >= code_end {
            return Err("Unexpected end of operator bytecode".to_string());
        }

        let instruction = unsafe { *ip };
        ip = unsafe { ip.add(1) };
        let op = unsafe { std::mem::transmute::<u8, crate::core::OpCode>(instruction) };

        match op {
            LoadConst => {
                let idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                if let Some(val) = func.chunk.constants.get(idx as usize) {
                    vm.stack.push(*val);
                }
            }
            LoadConst0 => {
                if let Some(val) = func.chunk.constants.first() {
                    vm.stack.push(*val);
                }
            }
            LoadConst1 => {
                if let Some(val) = func.chunk.constants.get(1) {
                    vm.stack.push(*val);
                }
            }
            LoadConst2 => {
                if let Some(val) = func.chunk.constants.get(2) {
                    vm.stack.push(*val);
                }
            }
            LoadConst3 => {
                if let Some(val) = func.chunk.constants.get(3) {
                    vm.stack.push(*val);
                }
            }
            LoadNull => vm.stack.push(Value::NULL),
            LoadTrue => vm.stack.push(Value::TRUE),
            LoadFalse => vm.stack.push(Value::FALSE),

            LoadLocal0 => vm.stack.push(locals[0]),
            LoadLocal1 => vm.stack.push(locals[1]),
            LoadLocal2 => vm.stack.push(locals[2]),
            LoadLocal3 => vm.stack.push(locals[3]),
            LoadLocal => {
                let idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                vm.stack.push(locals[idx as usize]);
            }
            StoreLocal => {
                let idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let val = vm.stack.pop().expect("Stack underflow");
                if (idx as usize) < locals.len() {
                    locals[idx as usize] = val;
                } else {
                    locals.resize(idx as usize + 1, Value::NULL);
                    locals[idx as usize] = val;
                }
            }
            StoreLocal0 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.is_empty() {
                    locals.push(val);
                } else {
                    locals[0] = val;
                }
            }
            StoreLocal1 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.len() < 2 {
                    locals.resize(2, Value::NULL);
                }
                locals[1] = val;
            }
            StoreLocal2 => {
                let val = vm.stack.pop().expect("Stack underflow");
                if locals.len() < 3 {
                    locals.resize(3, Value::NULL);
                }
                locals[2] = val;
            }

            Pop => {
                vm.stack.pop();
            }
            Dup => {
                let v = vm.stack.last().copied().unwrap();
                vm.stack.push(v);
            }

            Jump => {
                let offset = crate::runtime::vm::execution::read_i16_at_ptr(ip);
                ip = unsafe { ip.add(2) };
                ip = unsafe { ip.offset(offset as isize) };
            }
            JumpIfFalse => {
                let offset = crate::runtime::vm::execution::read_i16_at_ptr(ip);
                ip = unsafe { ip.add(2) };
                let val = vm.stack.pop().expect("Stack underflow");
                if !val.is_truthy() {
                    ip = unsafe { ip.offset(offset as isize) };
                }
            }

            BuildStruct => {
                let shape_id = crate::runtime::vm::execution::read_u16_at_ptr(ip);
                ip = unsafe { ip.add(2) };
                let field_count = unsafe { *ip };
                ip = unsafe { ip.add(1) };

                // 编译器按 shape 字段顺序的逆序入栈，所以弹出后直接是正确顺序
                let mut fields = Vec::with_capacity(field_count as usize);
                for _ in 0..field_count {
                    fields.push(vm.stack.pop().expect("Stack underflow"));
                }
                // 不需要 reverse，编译器已经处理好顺序

                let shape_ptr = crate::runtime::vm::shape::get_shape(vm, shape_id);
                if shape_ptr.is_null() {
                    return Err(format!("Shape ID {shape_id} not found"));
                }

                let obj = crate::core::ObjStruct::new(shape_ptr, fields);
                let ptr = Box::into_raw(Box::new(obj));
                vm.stack.push(Value::struct_instance(ptr));
            }

            GetField => {
                let field_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let obj_val = vm.stack.pop().expect("Stack underflow");
                if let Some(ptr) = obj_val.as_struct() {
                    let obj = unsafe { &*ptr };
                    if (field_idx as usize) < obj.fields.len() {
                        vm.stack.push(obj.fields[field_idx as usize]);
                    } else {
                        return Err("Field index out of bounds".to_string());
                    }
                } else {
                    return Err(format!(
                        "Expected struct instance, got {}",
                        get_type_name(obj_val)
                    ));
                }
            }

            IndexGet => {
                let index_val = vm.stack.pop().expect("Stack underflow");
                let obj_val = vm.stack.pop().expect("Stack underflow");

                // 整数索引：List 或 Struct 字段（过渡阶段保留 struct 整数索引）
                if let Some(idx) = index_val.as_smi() {
                    let i = idx as usize;

                    // List 索引
                    if let Some(list_ptr) = obj_val.as_list() {
                        let list = unsafe { &*list_ptr };
                        if i >= list.len() {
                            return Err(format!(
                                "Index out of bounds: {} (length {})",
                                i,
                                list.len()
                            ));
                        }
                        vm.stack.push(list.get(i).unwrap_or(Value::NULL));
                    }
                    // Struct 字段索引（过渡阶段，后续只支持 .field）
                    else if let Some(struct_ptr) = obj_val.as_struct() {
                        let struct_obj = unsafe { &*struct_ptr };
                        if i < struct_obj.field_count() {
                            vm.stack.push(struct_obj.fields[i]);
                        } else {
                            return Err(format!("Field index out of bounds: {i}"));
                        }
                    } else {
                        return Err("Expected list or struct for integer index".to_string());
                    }
                } else {
                    return Err("Expected integer index".to_string());
                }
            }

            Add => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match add_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Sub => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match sub_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Mul => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match mul_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Div => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match div_values(vm, a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Neg => {
                let v = vm.stack.pop().expect("Stack underflow");
                match neg_value(vm, v) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            CastToString => {
                let v = vm.stack.pop().expect("Stack underflow");
                // 在 operator str 中，假设输入已经是基础类型
                // 直接转换为字符串
                let s = v.to_string();
                let string_obj = Box::new(ObjString::new(s));
                vm.stack.push(Value::string(Box::into_raw(string_obj)));
            }
            Less => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match compare_values(vm, a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(ord == std::cmp::Ordering::Less)),
                    Err(e) => return Err(e),
                }
            }

            ReturnValue => {
                return Ok(vm.stack.pop().expect("Stack underflow"));
            }
            Return => {
                return Ok(Value::NULL);
            }

            _ => {
                return Err(format!("Unsupported opcode in operator: {op:?}"));
            }
        }
    }
}

/// 分配内联缓存槽（Level 2 优化）
#[allow(dead_code)]
pub fn allocate_inline_cache(vm: &mut VM) -> u8 {
    let index = vm.inline_caches.len();
    vm.inline_caches.push(InlineCacheEntry::empty());
    index as u8
}

/// 获取内联缓存条目（如果匹配）
pub fn inline_cache_get(
    vm: &VM,
    cache_idx: u8,
    left: Value,
    right: Value,
) -> Option<*mut ObjClosure> {
    let cache = vm.inline_caches.get(cache_idx as usize)?;
    let left_shape = get_shape_id(vm, left);
    let right_shape = get_shape_id(vm, right);

    if cache.matches(left_shape, right_shape) {
        Some(cache.closure)
    } else {
        None
    }
}

/// 更新内联缓存
pub fn inline_cache_update(
    vm: &mut VM,
    cache_idx: u8,
    left: Value,
    right: Value,
    closure: *mut ObjClosure,
) {
    // 先计算 shape_id，避免借用冲突
    let left_shape = get_shape_id(vm, left);
    let right_shape = get_shape_id(vm, right);
    if let Some(cache) = vm.inline_caches.get_mut(cache_idx as usize) {
        cache.update(left_shape, right_shape, closure);
    }
}

/// 调用二元运算符并缓存结果（Level 2）
pub fn call_binary_operator_cached(
    vm: &mut VM,
    op: Operator,
    a: Value,
    b: Value,
    cache_idx: u8,
) -> Result<Value, String> {
    // 1. 尝试左操作数的运算符
    if let Some(closure) = find_operator(vm, a, op) {
        inline_cache_update(vm, cache_idx, a, b, closure);
        return call_operator_closure(vm, closure, &[a, b]);
    }

    // 2. 尝试反向运算符
    if let Some(reverse_op) = op.reverse() {
        if let Some(closure) = find_operator(vm, b, reverse_op) {
            inline_cache_update(vm, cache_idx, a, b, closure);
            return call_operator_closure(vm, closure, &[b, a]);
        }
    }

    // 3. 报错
    Err(format!(
        "OperatorError: 类型 '{}' 不支持运算符 '{}'",
        get_type_name(a),
        op.symbol()
    ))
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ObjString, VM};

    #[test]
    fn test_add_integers() {
        let vm = VM::new();
        let a = Value::smi(5);
        let b = Value::smi(3);

        let result = add_values(&vm, a, b).unwrap();
        assert_eq!(result.as_smi(), Some(8));
    }

    #[test]
    fn test_add_floats() {
        let vm = VM::new();
        let a = Value::float(2.5);
        let b = Value::float(1.5);

        let result = add_values(&vm, a, b).unwrap();
        assert!(result.is_float());
        assert_eq!(result.as_float(), 4.0);
    }

    #[test]
    fn test_add_mixed() {
        let vm = VM::new();
        let a = Value::smi(5);
        let b = Value::float(2.5);

        let result = add_values(&vm, a, b).unwrap();
        assert!(result.is_float());
        assert_eq!(result.as_float(), 7.5);
    }

    #[test]
    fn test_string_concatenation() {
        let vm = VM::new();
        let a_str = Box::into_raw(Box::new(ObjString::new("Hello".to_string())));
        let b_str = Box::into_raw(Box::new(ObjString::new("World".to_string())));
        let a = Value::string(a_str);
        let b = Value::string(b_str);

        let result = add_values(&vm, a, b).unwrap();
        assert!(result.is_string());
        unsafe {
            assert_eq!((*result.as_string().unwrap()).chars, "HelloWorld");
        }
    }

    #[test]
    fn test_subtraction() {
        let vm = VM::new();
        let a = Value::smi(10);
        let b = Value::smi(3);

        let result = sub_values(&vm, a, b).unwrap();
        assert_eq!(result.as_smi(), Some(7));
    }

    #[test]
    fn test_multiplication() {
        let vm = VM::new();
        let a = Value::smi(6);
        let b = Value::smi(7);

        let result = mul_values(&vm, a, b).unwrap();
        assert_eq!(result.as_smi(), Some(42));
    }

    #[test]
    fn test_division() {
        let vm = VM::new();
        let a = Value::smi(10);
        let b = Value::smi(4);

        let result = div_values(&vm, a, b).unwrap();
        assert!(result.is_float());
        assert_eq!(result.as_float(), 2.5);
    }

    #[test]
    fn test_division_by_zero() {
        let vm = VM::new();
        let a = Value::smi(10);
        let b = Value::smi(0);

        let result = div_values(&vm, a, b);
        assert!(result.is_err());
    }

    #[test]
    fn test_modulo() {
        let vm = VM::new();
        let a = Value::smi(17);
        let b = Value::smi(5);

        let result = mod_values(&vm, a, b).unwrap();
        assert_eq!(result.as_smi(), Some(2));
    }

    #[test]
    fn test_negation() {
        let vm = VM::new();
        let v = Value::smi(42);

        let result = neg_value(&vm, v).unwrap();
        assert_eq!(result.as_smi(), Some(-42));
    }

    #[test]
    fn test_comparison() {
        let vm = VM::new();
        let a = Value::smi(10);
        let b = Value::smi(5);

        let result = compare_values(&vm, a, b).unwrap();
        assert_eq!(result, std::cmp::Ordering::Greater);

        let result = compare_values(&vm, b, a).unwrap();
        assert_eq!(result, std::cmp::Ordering::Less);

        let result = compare_values(&vm, a, a).unwrap();
        assert_eq!(result, std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_get_type_name() {
        assert_eq!(get_type_name(Value::smi(1)), "int");
        assert_eq!(get_type_name(Value::float(1.0)), "float");
        assert_eq!(get_type_name(Value::NULL), "null");
        assert_eq!(get_type_name(Value::TRUE), "bool");
    }
}
