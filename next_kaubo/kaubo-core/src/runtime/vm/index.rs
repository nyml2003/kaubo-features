//! 索引操作 (index_get_base, index_set_base)

use crate::core::{
    ObjClosure, Operator, Value, VM,
};

/// 基础类型索引获取（用于 IndexGet）
/// 返回 Ok(Some(value)) - 成功获取值
/// 返回 Ok(None) - 基础类型不匹配，需要尝试 operator get
/// 返回 Err(e) - 基础类型处理出错（如索引越界）
pub fn index_get_base(
    _vm: &VM,
    obj_val: Value,
    index_val: Value,
) -> Result<Option<Value>, String> {
    // 整数索引：列表索引或自定义类型的 operator get
    if let Some(idx) = index_val.as_smi() {
        let i = idx as usize;

        // 列表索引（内置类型）
        if let Some(list_ptr) = obj_val.as_list() {
            let list = unsafe { &*list_ptr };
            if i >= list.len() {
                return Err(format!(
                    "Index out of bounds: {} (length {})",
                    i,
                    list.len()
                ));
            }
            return Ok(Some(list.get(i).unwrap_or(Value::NULL)));
        }

        // 对于自定义 struct，整数索引尝试 operator get（不直接访问字段）
        // 字段访问应该使用 .field_name 或 ["field_name"]
        if obj_val.is_struct() {
            return Ok(None);
        }

        // 整数索引但不匹配任何基础类型，尝试 operator get
        return Ok(None);
    }

    // 字符串键：JSON 对象或 struct 字段（struct 字符串键将在 release 版移除）
    if let Some(key_ptr) = index_val.as_string() {
        let key = unsafe { &(*key_ptr).chars };

        // JSON 对象索引
        if let Some(json_ptr) = obj_val.as_json() {
            
            let json = unsafe { &*json_ptr };
            return Ok(Some(json.get(key).unwrap_or(Value::NULL)));
        }

        // Struct 字段访问（过渡阶段保留，后续只支持 .field）
        if let Some(struct_ptr) = obj_val.as_struct() {
            let struct_obj = unsafe { &*struct_ptr };
            let shape = unsafe { &*struct_obj.shape };

            if let Some(field_idx) = shape.get_field_index(key) {
                return Ok(Some(
                    struct_obj
                        .get_field(field_idx as usize)
                        .unwrap_or(Value::NULL),
                ));
            }
            // 字段名不存在，尝试 operator get
            return Ok(None);
        }

        // 字符串键但不匹配 JSON 或 struct，尝试 operator get
        return Ok(None);
    }

    // 其他索引类型，尝试 operator get
    Ok(None)
}

/// 基础类型索引设置（用于 IndexSet）
/// 返回 Ok(true) - 成功设置值
/// 返回 Ok(false) - 基础类型不匹配，需要尝试 operator set
/// 返回 Err(e) - 基础类型处理出错（如索引越界）
pub fn index_set_base(
    _vm: &mut VM,
    obj_val: Value,
    key_val: Value,
    value: Value,
) -> Result<bool, String> {
    // 字符串键：JSON 对象
    if let Some(key_ptr) = key_val.as_string() {
        let key = unsafe { &(*key_ptr).chars };

        if let Some(json_ptr) = obj_val.as_json() {
            
            let json = unsafe { &mut *json_ptr };
            json.set(key.clone(), value);
            return Ok(true);
        }

        // 字符串键但不匹配 JSON，尝试 operator set
        return Ok(false);
    }

    // 整数键：列表
    if let Some(idx) = key_val.as_smi() {
        let i = idx as usize;

        if let Some(list_ptr) = obj_val.as_list() {
            let list = unsafe { &mut *list_ptr };
            if i >= list.len() {
                return Err(format!(
                    "Index out of bounds: {} (length {})",
                    i,
                    list.len()
                ));
            }
            list.elements[i] = value;
            return Ok(true);
        }

        // 整数键但不匹配列表，尝试 operator set
        return Ok(false);
    }

    // 其他键类型，尝试 operator set
    Ok(false)
}

/// 调用 operator set（三元运算符）
pub fn call_set_operator(
    vm: &mut VM,
    obj: Value,
    index: Value,
    value: Value,
) -> Result<(), String> {
    if let Some(closure) = vm.find_operator(obj, Operator::Set) {
        return call_operator_closure_set(vm, closure, obj, index, value);
    }

    Err(format!(
        "OperatorError: 类型 '{}' 不支持索引赋值",
        vm.get_type_name(obj)
    ))
}

/// 调用 operator set 闭包（三个参数）
fn call_operator_closure_set(
    vm: &mut VM,
    closure: *mut ObjClosure,
    obj: Value,
    index: Value,
    value: Value,
) -> Result<(), String> {
    use crate::core::OpCode::*;
    

    let closure_ref = unsafe { &*closure };
    let func = unsafe { &*closure_ref.function };

    if func.arity != 3 {
        return Err(format!(
            "operator set expects 3 arguments (self, index, value) but got {}",
            func.arity
        ));
    }

    // 创建局部变量表（参数: self, index, value）
    let mut locals: Vec<Value> = vec![obj, index, value];

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

            Add => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.add_values(a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Sub => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.sub_values(a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Mul => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.mul_values(a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Div => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.div_values(a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Mod => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.mod_values(a, b) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }
            Neg => {
                let v = vm.stack.pop().expect("Stack underflow");
                match vm.neg_value(v) {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return Err(e),
                }
            }

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
                match vm.compare_values(a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(ord == std::cmp::Ordering::Greater)),
                    Err(e) => return Err(e),
                }
            }
            Less => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.compare_values(a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(ord == std::cmp::Ordering::Less)),
                    Err(e) => return Err(e),
                }
            }
            GreaterEqual => {
                let _cache_idx = unsafe { *ip };
                ip = unsafe { ip.add(1) };
                let b = vm.stack.pop().expect("Stack underflow");
                let a = vm.stack.pop().expect("Stack underflow");
                match vm.compare_values(a, b) {
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
                match vm.compare_values(a, b) {
                    Ok(ord) => vm.stack.push(Value::bool_from(
                        ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal,
                    )),
                    Err(e) => return Err(e),
                }
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

                let mut fields = Vec::with_capacity(field_count as usize);
                for _ in 0..field_count {
                    fields.push(vm.stack.pop().expect("Stack underflow"));
                }

                let shape_ptr = vm.get_shape(shape_id);
                if shape_ptr.is_null() {
                    return Err(format!("Shape ID {shape_id} not found"));
                }

                let obj = crate::core::ObjStruct::new(shape_ptr, fields);
                let ptr = Box::into_raw(Box::new(obj));
                vm.stack.push(Value::struct_instance(ptr));
            }

            Return => {
                // operator set 不返回值
                return Ok(());
            }
            ReturnValue => {
                // operator set 忽略返回值
                vm.stack.pop();
                return Ok(());
            }

            _ => {
                return Err(format!("Unsupported opcode in operator set: {op:?}"));
            }
        }
    }
}
