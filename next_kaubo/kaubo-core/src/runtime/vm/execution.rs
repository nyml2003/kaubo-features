//! run() 主执行循环、interpret 方法

use crate::core::{
    CallFrame, Chunk, CoroutineState, InterpretResult, ObjClosure, ObjFunction, ObjIterator,
    ObjJson, ObjList, ObjModule, ObjShape, ObjString, ObjStruct, ObjCoroutine,
    Operator, Value, VM,
};
use crate::core::OperatorTableEntry;
use crate::runtime::OpCode::*;

/// 从 Chunk 的 operator_table 注册运算符到 Shape
pub fn register_operators_from_chunk(vm: &mut VM, chunk: &Chunk) {
    for entry in &chunk.operator_table {
        let OperatorTableEntry {
            shape_id,
            operator_name,
            const_idx,
        } = entry;

        // 获取函数值
        if let Some(function_value) = chunk.constants.get(*const_idx as usize) {
            if let Some(function_ptr) = function_value.as_function() {
                // 创建闭包（运算符方法没有 upvalues）
                let closure = Box::into_raw(Box::new(ObjClosure::new(function_ptr)));

                // 获取或创建 Shape
                let shape_ptr = vm.shapes.entry(*shape_id).or_insert_with(|| {
                    // 如果 Shape 不存在，创建一个空的（这不应该发生，但做安全处理）
                    let shape = Box::into_raw(Box::new(ObjShape::new(
                        *shape_id,
                        format!("<anon_{shape_id}>"),
                        Vec::new(),
                    )));
                    shape
                });

                // 注册运算符
                unsafe {
                    use crate::core::Operator;
                    if let Some(op) = Operator::from_method_name(operator_name) {
                        (*(*shape_ptr as *mut ObjShape)).register_operator(op, closure);
                    }
                }
            }
        }
    }
}

/// 执行字节码的主循环
pub fn run(vm: &mut VM) -> InterpretResult {
    use crate::runtime::vm::{
        call, index, operators, shape, stack,
    };

    loop {
        // 调试: 打印当前栈状态和指令
        #[cfg(feature = "trace_execution")]
        trace_instruction(vm);

        // 读取操作码
        let instruction = unsafe { *current_ip(vm) };
        vm.advance_ip(1);
        let op = unsafe { std::mem::transmute::<u8, crate::runtime::OpCode>(instruction) };

        // VM 执行追踪 (logger removed - now in core)
        // trace!(logger, "execute: {:?}, stack: {:?}", op, self.stack);
        match op {
            // ===== 常量加载 =====
            LoadConst0 => push_const(vm, 0),
            LoadConst1 => push_const(vm, 1),
            LoadConst2 => push_const(vm, 2),
            LoadConst3 => push_const(vm, 3),
            LoadConst4 => push_const(vm, 4),
            LoadConst5 => push_const(vm, 5),
            LoadConst6 => push_const(vm, 6),
            LoadConst7 => push_const(vm, 7),
            LoadConst8 => push_const(vm, 8),
            LoadConst9 => push_const(vm, 9),
            LoadConst10 => push_const(vm, 10),
            LoadConst11 => push_const(vm, 11),
            LoadConst12 => push_const(vm, 12),
            LoadConst13 => push_const(vm, 13),
            LoadConst14 => push_const(vm, 14),
            LoadConst15 => push_const(vm, 15),

            LoadConst => {
                let idx = read_byte(vm);
                push_const(vm, idx as usize);
            }

            // ===== 特殊值 =====
            LoadNull => vm.stack.push(Value::NULL),
            LoadTrue => vm.stack.push(Value::TRUE),
            LoadFalse => vm.stack.push(Value::FALSE),
            LoadZero => vm.stack.push(Value::smi(0)),
            LoadOne => vm.stack.push(Value::smi(1)),

            // ===== 栈操作 =====
            Pop => {
                vm.stack.pop();
            }

            Dup => {
                let v = stack::peek(vm, 0);
                vm.stack.push(v);
            }

            Swap => {
                let len = vm.stack.len();
                if len >= 2 {
                    vm.stack.swap(len - 1, len - 2);
                }
            }

            // ===== 算术运算 =====
            Add => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);

                // 先尝试基础类型（Level 1）
                let result = operators::add_values(vm, a, b);
                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(_) => {
                        // 基础类型失败，尝试内联缓存（Level 2）
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                // 缓存命中，直接调用
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                // 缓存未命中，查找并更新缓存
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Add,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            // 无缓存，直接调用（Level 3）
                            match operators::call_binary_operator(vm, Operator::Add, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            Sub => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::sub_values(vm, a, b);
                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(_) => {
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Sub,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Sub, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            Mul => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::mul_values(vm, a, b);
                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(_) => {
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Mul,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Mul, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            Div => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::div_values(vm, a, b);
                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(_) => {
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Div,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Div, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            Mod => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::mod_values(vm, a, b);
                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(_) => {
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Mod,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Mod, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            Neg => {
                let v = vm.stack.pop().expect("Stack underflow");
                let result = operators::neg_value(vm, v);
                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(_) => {
                        // 基础类型失败，尝试运算符重载
                        match operators::call_unary_operator(vm, Operator::Neg, v) {
                            Ok(v) => vm.stack.push(v),
                            Err(e) => return InterpretResult::RuntimeError(e),
                        }
                    }
                }
            }

            Not => {
                let v = vm.stack.pop().expect("Stack underflow");
                // 逻辑取非：真值变为 false，假值变为 true
                if v.is_truthy() {
                    vm.stack.push(Value::FALSE);
                } else {
                    vm.stack.push(Value::TRUE);
                }
            }

            // ===== 比较运算 =====
            Equal => {
                let _cache_idx = read_byte(vm); // 读取占位符
                let (a, b) = stack::pop_two(vm);
                vm.stack.push(Value::bool_from(a == b));
            }

            NotEqual => {
                let _cache_idx = read_byte(vm); // 读取占位符
                let (a, b) = stack::pop_two(vm);
                vm.stack.push(Value::bool_from(a != b));
            }

            Greater => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::compare_values(vm, a, b);
                match result {
                    Ok(std::cmp::Ordering::Greater) => vm.stack.push(Value::TRUE),
                    Ok(_) => vm.stack.push(Value::FALSE),
                    Err(_) => {
                        // 基础类型失败，尝试运算符重载
                        // a > b 等价于 b < a，交换参数调用 operator lt
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, b, a) {
                                match operators::call_operator_closure(vm, cached, &[b, a]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Lt,
                                    b,
                                    a,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Lt, b, a) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            Less => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::compare_values(vm, a, b);
                match result {
                    Ok(std::cmp::Ordering::Less) => vm.stack.push(Value::TRUE),
                    Ok(_) => vm.stack.push(Value::FALSE),
                    Err(_) => {
                        // 基础类型失败，尝试运算符重载
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Lt,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Lt, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            LessEqual => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::compare_values(vm, a, b);
                match result {
                    Ok(std::cmp::Ordering::Less) | Ok(std::cmp::Ordering::Equal) => {
                        vm.stack.push(Value::TRUE)
                    }
                    Ok(_) => vm.stack.push(Value::FALSE),
                    Err(_) => {
                        // 基础类型失败，尝试运算符重载
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, a, b) {
                                match operators::call_operator_closure(vm, cached, &[a, b]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Le,
                                    a,
                                    b,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Le, a, b) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            GreaterEqual => {
                let cache_idx = read_byte(vm);
                let (a, b) = stack::pop_two(vm);
                let result = operators::compare_values(vm, a, b);
                match result {
                    Ok(std::cmp::Ordering::Greater) | Ok(std::cmp::Ordering::Equal) => {
                        vm.stack.push(Value::TRUE)
                    }
                    Ok(_) => vm.stack.push(Value::FALSE),
                    Err(_) => {
                        // 基础类型失败，尝试运算符重载
                        // a >= b 等价于 b <= a，交换参数调用 operator le
                        if cache_idx != 0xFF {
                            if let Some(cached) = operators::inline_cache_get(vm, cache_idx, b, a) {
                                match operators::call_operator_closure(vm, cached, &[b, a]) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            } else {
                                match operators::call_binary_operator_cached(
                                    vm,
                                    Operator::Le,
                                    b,
                                    a,
                                    cache_idx,
                                ) {
                                    Ok(v) => vm.stack.push(v),
                                    Err(e) => return InterpretResult::RuntimeError(e),
                                }
                            }
                        } else {
                            match operators::call_binary_operator(vm, Operator::Le, b, a) {
                                Ok(v) => vm.stack.push(v),
                                Err(e) => return InterpretResult::RuntimeError(e),
                            }
                        }
                    }
                }
            }

            // ===== 局部变量 =====
            LoadLocal0 => {
                let value = get_local(vm, 0);
                vm.stack.push(value);
            }
            LoadLocal1 => {
                let value = get_local(vm, 1);
                vm.stack.push(value);
            }
            LoadLocal2 => {
                let value = get_local(vm, 2);
                vm.stack.push(value);
            }
            LoadLocal3 => {
                let value = get_local(vm, 3);
                vm.stack.push(value);
            }
            LoadLocal4 => {
                let value = get_local(vm, 4);
                vm.stack.push(value);
            }
            LoadLocal5 => {
                let value = get_local(vm, 5);
                vm.stack.push(value);
            }
            LoadLocal6 => {
                let value = get_local(vm, 6);
                vm.stack.push(value);
            }
            LoadLocal7 => {
                let value = get_local(vm, 7);
                vm.stack.push(value);
            }
            LoadLocal => {
                let idx = read_byte(vm) as usize;
                let value = get_local(vm, idx);
                vm.stack.push(value);
            }

            StoreLocal0 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 0, value);
            }
            StoreLocal1 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 1, value);
            }
            StoreLocal2 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 2, value);
            }
            StoreLocal3 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 3, value);
            }
            StoreLocal4 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 4, value);
            }
            StoreLocal5 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 5, value);
            }
            StoreLocal6 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 6, value);
            }
            StoreLocal7 => {
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, 7, value);
            }
            StoreLocal => {
                let idx = read_byte(vm) as usize;
                let value = vm.stack.pop().expect("Stack underflow");
                set_local(vm, idx, value);
            }

            // ===== 全局变量 =====
            LoadGlobal => {
                let idx = read_byte(vm) as usize;
                let name = get_constant_string(vm, idx);
                if let Some(value) = vm.globals.get(&name) {
                    vm.stack.push(*value);
                } else {
                    return InterpretResult::RuntimeError(format!(
                        "Undefined global variable: {name}"
                    ));
                }
            }

            StoreGlobal => {
                let idx = read_byte(vm) as usize;
                let name = get_constant_string(vm, idx);
                let value = vm.stack.pop().expect("Stack underflow");
                vm.globals.insert(name, value);
            }

            DefineGlobal => {
                let idx = read_byte(vm) as usize;
                let name = get_constant_string(vm, idx);
                let value = vm.stack.pop().expect("Stack underflow");
                vm.globals.insert(name, value);
            }

            // ===== 控制流 =====
            Jump => {
                let offset = read_i16(vm);
                jump_ip(vm, offset as isize);
            }

            JumpIfFalse => {
                let offset = read_i16(vm);
                let condition = vm.stack.pop().expect("Stack underflow"); // 弹出条件
                if !condition.is_truthy() {
                    jump_ip(vm, offset as isize);
                }
            }

            JumpBack => {
                let offset = read_i16(vm);
                jump_ip(vm, offset as isize);
            }

            // ===== 函数 =====
            Call => {
                let arg_count = read_byte(vm);

                // 栈布局：[arg0, arg1, ..., argN, closure]
                // 先弹出闭包对象（栈顶）
                let callee = vm.stack.pop().expect("Stack underflow");
                if let Some(closure_ptr) = callee.as_closure() {
                    let closure = unsafe { &*closure_ptr };
                    let func = unsafe { &*closure.function };

                    if func.arity != arg_count {
                        return InterpretResult::RuntimeError(format!(
                            "Expected {} arguments but got {}",
                            func.arity, arg_count
                        ));
                    }

                    // 收集参数（从栈顶）
                    let mut locals = Vec::with_capacity(arg_count as usize);
                    for _ in 0..arg_count {
                        locals.push(vm.stack.pop().expect("Stack underflow"));
                    }
                    locals.reverse();

                    // 创建新的调用帧
                    let stack_base = vm.stack.len();
                    let new_frame = CallFrame {
                        closure: closure_ptr,
                        ip: func.chunk.code.as_ptr(),
                        locals,
                        stack_base,
                    };
                    vm.frames.push(new_frame);
                } else if let Some(func_ptr) = callee.as_function() {
                    // 向后兼容：直接调用函数（无闭包）
                    let func = unsafe { &*func_ptr };
                    if func.arity != arg_count {
                        return InterpretResult::RuntimeError(format!(
                            "Expected {} arguments but got {}",
                            func.arity, arg_count
                        ));
                    }

                    let mut locals = Vec::with_capacity(arg_count as usize);
                    for _ in 0..arg_count {
                        locals.push(vm.stack.pop().expect("Stack underflow"));
                    }
                    locals.reverse();

                    // 包装为闭包
                    let closure = Box::into_raw(Box::new(ObjClosure::new(func_ptr)));
                    let stack_base = vm.stack.len();
                    let new_frame = CallFrame {
                        closure,
                        ip: func.chunk.code.as_ptr(),
                        locals,
                        stack_base,
                    };
                    vm.frames.push(new_frame);
                } else if let Some(native_ptr) = callee.as_native() {
                    // 调用原生函数
                    let native = unsafe { &*native_ptr };

                    // 变参函数：arity=255 表示可变参数
                    // 否则参数数量必须等于 arity（或支持变参的函数内部处理）
                    if native.arity != 255 && arg_count != native.arity {
                        return InterpretResult::RuntimeError(format!(
                            "Expected {} arguments but got {}",
                            native.arity, arg_count
                        ));
                    }

                    // 收集参数（从栈顶）
                    let mut args = Vec::with_capacity(arg_count as usize);
                    for _ in 0..arg_count {
                        args.push(vm.stack.pop().expect("Stack underflow"));
                    }
                    args.reverse();

                    // 调用原生函数
                    match native.call(&args) {
                        Ok(result) => vm.stack.push(result),
                        Err(msg) => return InterpretResult::RuntimeError(msg),
                    }
                } else if let Some(native_vm_ptr) = callee.as_native_vm() {
                    // 调用 VM-aware 原生函数
                    let native_vm = unsafe { &*native_vm_ptr };

                    // 参数校验
                    if native_vm.arity != 255 && arg_count != native_vm.arity {
                        return InterpretResult::RuntimeError(format!(
                            "Expected {} arguments but got {}",
                            native_vm.arity, arg_count
                        ));
                    }

                    // 收集参数（从栈顶）
                    let mut args = Vec::with_capacity(arg_count as usize);
                    for _ in 0..arg_count {
                        args.push(vm.stack.pop().expect("Stack underflow"));
                    }
                    args.reverse();

                    // 调用 VM-aware 原生函数，传入 self (VM)
                    match (native_vm.function)(vm as *mut _ as *mut (), &args) {
                        Ok(result) => vm.stack.push(result),
                        Err(msg) => return InterpretResult::RuntimeError(msg),
                    }
                } else {
                    // 尝试 operator call（可调用对象）
                    // 收集参数
                    let mut args = Vec::with_capacity(arg_count as usize);
                    for _ in 0..arg_count {
                        args.push(vm.stack.pop().expect("Stack underflow"));
                    }
                    args.reverse();

                    // callee 作为 self，args 作为参数
                    let mut all_args = vec![callee];
                    all_args.extend(args);

                    match operators::call_callable_operator(vm, Operator::Call, &all_args) {
                        Ok(result) => vm.stack.push(result),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }
            }

            Closure => {
                // 从常量池加载函数对象
                let const_idx = read_byte(vm);
                let constant = current_chunk(vm).constants[const_idx as usize];
                let upvalue_count = read_byte(vm);

                if let Some(func_ptr) = constant.as_function() {
                    // 创建闭包对象
                    let mut closure = Box::new(ObjClosure::new(func_ptr));

                    // 捕获 upvalues
                    for _ in 0..upvalue_count {
                        let is_local = read_byte(vm) != 0;
                        let index = read_byte(vm);

                        if is_local {
                            // 捕获当前帧的局部变量
                            let location = call::current_local_ptr(vm, index as usize);
                            let upvalue = call::capture_upvalue(vm, location);
                            closure.add_upvalue(upvalue);
                        } else {
                            // 继承当前闭包的 upvalue
                            let current_closure = current_closure(vm);
                            let upvalue = unsafe {
                                (*current_closure).get_upvalue(index as usize).unwrap()
                            };
                            closure.add_upvalue(upvalue);
                        }
                    }

                    vm.stack.push(Value::closure(Box::into_raw(closure)));
                } else {
                    return InterpretResult::RuntimeError(
                        "Closure constant must be a function".to_string(),
                    );
                }
            }

            GetUpvalue => {
                let idx = read_byte(vm) as usize;
                let closure = current_closure(vm);
                let upvalue = unsafe { (*closure).get_upvalue(idx).unwrap() };
                let value = unsafe { (*upvalue).get() };
                vm.stack.push(value);
            }

            SetUpvalue => {
                let idx = read_byte(vm) as usize;
                let value = stack::peek(vm, 0);
                let closure = current_closure(vm);
                let upvalue = unsafe { (*closure).get_upvalue(idx).unwrap() };
                unsafe {
                    (*upvalue).set(value);
                }
            }

            CloseUpvalues => {
                let slot = read_byte(vm) as usize;
                call::close_upvalues(vm, slot);
            }

            Return => {
                // 1. 关闭当前帧的 upvalues
                call::close_upvalues(vm, 0);

                // 2. 弹出当前函数的调用帧
                vm.frames
                    .pop()
                    .expect("Runtime error: No call frame to pop");

                // 3. 压入 NULL 作为无返回值函数的返回值
                vm.stack.push(Value::NULL);

                // 4. 只有当调用帧为空（主函数返回）时，才终止VM执行；否则继续执行上层帧
                if vm.frames.is_empty() {
                    return InterpretResult::Ok;
                }
                // 非空则继续循环，执行上层帧的下一条指令
            }

            // ===== 修复后的 RETURN_VALUE 指令 =====
            ReturnValue => {
                // 1. 关闭当前帧的 upvalues
                call::close_upvalues(vm, 0);

                // 2. 弹出当前函数的调用帧
                vm.frames
                    .pop()
                    .expect("Runtime error: No call frame to pop");

                // 3. 保存栈顶的返回值（函数执行结果）
                let return_value = vm.stack.pop().expect("Stack underflow");

                // 4. 将返回值压回栈顶，供上层帧使用
                vm.stack.push(return_value);

                // 5. 仅主函数返回时终止，否则继续执行上层帧
                if vm.frames.is_empty() {
                    return InterpretResult::Ok;
                }
                // 非空则继续循环
            }

            // ===== 协程 =====
            CreateCoroutine => {
                // 从栈顶弹出闭包，创建协程对象
                let closure_val = vm.stack.pop().expect("Stack underflow");
                if let Some(closure_ptr) = closure_val.as_closure() {
                    let coroutine = Box::new(ObjCoroutine::new(closure_ptr));
                    vm.stack.push(Value::coroutine(Box::into_raw(coroutine)));
                } else {
                    return InterpretResult::RuntimeError(
                        "Coroutine must be created from a closure".to_string(),
                    );
                }
            }

            Resume => {
                handle_resume(vm);
            }

            Yield => {
                // 从栈顶弹出要返回的值
                let value = vm.stack.pop().expect("Stack underflow");

                // 保存返回值到当前栈
                vm.stack.push(value);

                // 返回特殊错误表示 yield（简化实现）
                return InterpretResult::RuntimeError("yield".to_string());
            }

            CoroutineStatus => {
                // 从栈顶弹出协程对象
                let coro_val = vm.stack.pop().expect("Stack underflow");
                if let Some(coro_ptr) = coro_val.as_coroutine() {
                    let coro = unsafe { &*coro_ptr };
                    let status = match coro.state {
                        CoroutineState::Suspended => 0i64,
                        CoroutineState::Running => 1i64,
                        CoroutineState::Dead => 2i64,
                    };
                    vm.stack.push(Value::smi(status as i32));
                } else {
                    return InterpretResult::RuntimeError("Expected a coroutine".to_string());
                }
            }

            // ===== 列表 =====
            BuildList => {
                let count = read_byte(vm) as usize;
                // 从栈顶弹出 count 个元素，创建列表
                let mut elements = Vec::with_capacity(count);
                for _ in 0..count {
                    elements.push(vm.stack.pop().expect("Stack underflow"));
                }
                elements.reverse(); // 栈顶是最后一个元素

                let list = Box::new(ObjList::from_vec(elements));
                let list_ptr = Box::into_raw(list);
                vm.stack.push(Value::list(list_ptr));
            }

            BuildJson => {
                let count = read_byte(vm) as usize;
                // 从栈顶弹出键值对（值先入栈，然后是键），创建 JSON 对象
                let mut entries = std::collections::HashMap::with_capacity(count);
                for _ in 0..count {
                    let key_val = vm.stack.pop().expect("Stack underflow");
                    let value = vm.stack.pop().expect("Stack underflow");

                    // 键必须是字符串
                    if let Some(key_ptr) = key_val.as_string() {
                        let key_str = unsafe { &(*key_ptr).chars };
                        entries.insert(key_str.clone(), value);
                    } else {
                        return InterpretResult::RuntimeError(
                            "JSON key must be a string".to_string(),
                        );
                    }
                }

                let json = Box::new(ObjJson::from_hashmap(entries));
                let json_ptr = Box::into_raw(json);
                vm.stack.push(Value::json(json_ptr));
            }

            BuildModule => {
                let count = read_byte(vm) as usize;
                // 从栈顶弹出导出值（逆序），创建模块对象
                // 按逆序弹出，这样先定义的导出项索引小
                let mut exports = Vec::with_capacity(count);
                for _ in 0..count {
                    exports.push(vm.stack.pop().expect("Stack underflow"));
                }
                exports.reverse();

                // 创建模块对象（暂时不设置名称和 name_to_index，后续由编译器提供）
                let module = Box::new(ObjModule::new(
                    String::new(),
                    exports,
                    std::collections::HashMap::new(),
                ));
                let module_ptr = Box::into_raw(module);
                vm.stack.push(Value::module(module_ptr));
            }

            ModuleGet => {
                // 栈顶: [module]
                let module_val = vm.stack.pop().expect("Stack underflow");
                let shape_id = read_u16(vm);

                if let Some(module_ptr) = module_val.as_module() {
                    let module = unsafe { &*module_ptr };
                    if let Some(value) = module.get_by_shape_id(shape_id) {
                        vm.stack.push(value);
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Module field with ShapeID {shape_id} not found"
                        ));
                    }
                } else {
                    return InterpretResult::RuntimeError(
                        "ModuleGet requires a module".to_string(),
                    );
                }
            }

            GetModuleExport => {
                // 栈顶: [module] -> value
                // 操作数: u8 常量池索引（导出项名称字符串）
                let module_val = vm.stack.pop().expect("Stack underflow");
                let name_idx = read_byte(vm) as usize;

                // 从常量池获取名称字符串
                let name =
                    if let Some(constant) = current_chunk(vm).constants.get(name_idx) {
                        if let Some(ptr) = constant.as_string() {
                            unsafe { (&*ptr).chars.clone() }
                        } else {
                            return InterpretResult::RuntimeError(
                                "GetModuleExport name must be a string".to_string(),
                            );
                        }
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Invalid constant index for GetModuleExport: {name_idx}"
                        ));
                    };

                if let Some(module_ptr) = module_val.as_module() {
                    let module = unsafe { &*module_ptr };
                    if let Some(value) = module.get(&name) {
                        vm.stack.push(value);
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Export '{name}' not found in module"
                        ));
                    }
                } else {
                    return InterpretResult::RuntimeError(
                        "GetModuleExport requires a module".to_string(),
                    );
                }
            }

            GetModule => {
                // 栈顶: [module_name] -> module
                let name_val = vm.stack.pop().expect("Stack underflow");

                let name = if let Some(ptr) = name_val.as_string() {
                    unsafe { (&*ptr).chars.clone() }
                } else {
                    return InterpretResult::RuntimeError(
                        "GetModule requires a string name".to_string(),
                    );
                };

                if let Some(value) = vm.globals.get(&name) {
                    vm.stack.push(*value);
                } else {
                    return InterpretResult::RuntimeError(format!("Module '{name}' not found"));
                }
            }

            // ===== Struct 相关指令 =====
            BuildStruct => {
                // 操作数: u16 shape_id + u8 field_count
                let shape_id = read_u16(vm);
                let field_count = read_byte(vm) as usize;

                // 从栈顶弹出字段值
                // 编译器按 shape 字段顺序的逆序入栈，所以弹出后直接是正确顺序
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    fields.push(vm.stack.pop().expect("Stack underflow"));
                }
                // 不需要 reverse，编译器已经处理好顺序

                // 创建 struct 实例
                let shape_ptr = shape::get_shape(vm, shape_id);
                if shape_ptr.is_null() {
                    return InterpretResult::RuntimeError(format!(
                        "Shape ID {shape_id} not found"
                    ));
                }

                let struct_obj = Box::new(ObjStruct::new(shape_ptr, fields));
                vm.stack.push(Value::struct_instance(Box::into_raw(struct_obj)));
            }

            GetField => {
                // 操作数: u8 字段索引
                let field_idx = read_byte(vm) as usize;

                let struct_val = vm.stack.pop().expect("Stack underflow");
                if let Some(struct_ptr) = struct_val.as_struct() {
                    let struct_obj = unsafe { &*struct_ptr };
                    if let Some(value) = struct_obj.get_field(field_idx) {
                        vm.stack.push(value);
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Field index {field_idx} out of bounds"
                        ));
                    }
                } else {
                    return InterpretResult::RuntimeError(
                        "GetField requires a struct instance".to_string(),
                    );
                }
            }

            SetField => {
                // 操作数: u8 字段索引
                let field_idx = read_byte(vm) as usize;

                // 栈布局: [value, struct]
                let struct_val = vm.stack.pop().expect("Stack underflow");
                let value = vm.stack.pop().expect("Stack underflow");

                if let Some(struct_ptr) = struct_val.as_struct() {
                    let struct_obj = unsafe { &mut *struct_ptr };
                    struct_obj.set_field(field_idx, value);
                } else {
                    return InterpretResult::RuntimeError(
                        "SetField requires a struct instance".to_string(),
                    );
                }
            }

            LoadMethod => {
                // 操作数: u8 方法索引
                let method_idx = read_byte(vm);

                // 栈顶是 receiver
                let receiver = stack::peek(vm, 0);
                if let Some(struct_ptr) = receiver.as_struct() {
                    let shape = unsafe { (*struct_ptr).shape };
                    if let Some(method) = unsafe { (*shape).get_method(method_idx) } {
                        // 压入函数对象（不是闭包）
                        vm.stack.push(Value::function(method));
                    } else {
                        return InterpretResult::RuntimeError(format!(
                            "Method index {method_idx} not found in shape"
                        ));
                    }
                } else {
                    return InterpretResult::RuntimeError(
                        "LoadMethod requires a struct instance".to_string(),
                    );
                }
            }

            IndexGet => {
                // 栈顶: [index, object]
                let index_val = vm.stack.pop().expect("Stack underflow");
                let obj_val = vm.stack.pop().expect("Stack underflow");

                // 首先尝试基础类型索引
                let base_result = index::index_get_base(vm, obj_val, index_val);

                match base_result {
                    Ok(Some(value)) => {
                        vm.stack.push(value);
                    }
                    Ok(None) => {
                        // 基础类型不匹配，尝试 operator get
                        match operators::call_binary_operator(vm, Operator::Get, obj_val, index_val) {
                            Ok(v) => vm.stack.push(v),
                            Err(e) => return InterpretResult::RuntimeError(e),
                        }
                    }
                    Err(e) => {
                        // 基础类型处理出错（如索引越界）
                        return InterpretResult::RuntimeError(e);
                    }
                }
            }

            IndexSet => {
                // 栈布局: [value, key, object] (object 在栈顶)
                let obj_val = vm.stack.pop().expect("Stack underflow");
                let key_val = vm.stack.pop().expect("Stack underflow");
                let value = vm.stack.pop().expect("Stack underflow");

                // 首先尝试基础类型索引设置
                let base_result = index::index_set_base(vm, obj_val, key_val, value);

                match base_result {
                    Ok(true) => {
                        // 基础类型设置成功
                    }
                    Ok(false) => {
                        // 基础类型不匹配，尝试 operator set
                        match index::call_set_operator(vm, obj_val, key_val, value) {
                            Ok(_) => {}
                            Err(e) => return InterpretResult::RuntimeError(e),
                        }
                    }
                    Err(e) => {
                        // 基础类型处理出错（如索引越界）
                        return InterpretResult::RuntimeError(e);
                    }
                }
            }

            GetIter => {
                // 获取迭代器：支持列表、协程和 JSON 对象
                let val = vm.stack.pop().expect("Stack underflow");

                if let Some(list_ptr) = val.as_list() {
                    // 列表 -> 列表迭代器
                    let iter = Box::new(ObjIterator::from_list(list_ptr));
                    let iter_ptr = Box::into_raw(iter);
                    vm.stack.push(Value::iterator(iter_ptr));
                } else if let Some(coro_ptr) = val.as_coroutine() {
                    // 协程 -> 协程迭代器
                    let iter = Box::new(ObjIterator::from_coroutine(coro_ptr));
                    let iter_ptr = Box::into_raw(iter);
                    vm.stack.push(Value::iterator(iter_ptr));
                } else if let Some(json_ptr) = val.as_json() {
                    // JSON 对象 -> JSON 迭代器（遍历键）
                    let iter = Box::new(unsafe { ObjIterator::from_json(json_ptr) });
                    let iter_ptr = Box::into_raw(iter);
                    vm.stack.push(Value::iterator(iter_ptr));
                } else {
                    return InterpretResult::RuntimeError(
                        "Can only iterate over lists, coroutines, or json objects".to_string(),
                    );
                }
            }

            IterNext => {
                handle_iter_next(vm);
            }

            // ===== 类型转换 =====
            CastToInt => {
                let v = vm.stack.pop().expect("Stack underflow");
                let result = if let Some(n) = v.as_int() {
                    Value::smi(n)
                } else if v.is_float() {
                    Value::smi(v.as_float() as i32)
                } else if let Some(s) = v.as_string() {
                    let s_ref = unsafe { &(*s).chars };
                    s_ref.parse::<i32>().map(Value::smi).unwrap_or(Value::NULL)
                } else {
                    Value::NULL
                };
                vm.stack.push(result);
            }

            CastToFloat => {
                let v = vm.stack.pop().expect("Stack underflow");
                let result = if let Some(n) = v.as_int() {
                    Value::float(n as f64)
                } else if v.is_float() {
                    v
                } else if let Some(s) = v.as_string() {
                    let s_ref = unsafe { &(*s).chars };
                    s_ref
                        .parse::<f64>()
                        .map(Value::float)
                        .unwrap_or(Value::NULL)
                } else {
                    Value::NULL
                };
                vm.stack.push(result);
            }

            CastToString => {
                let v = vm.stack.pop().expect("Stack underflow");

                // 基础类型：直接转换
                let result = if v.is_int()
                    || v.is_float()
                    || v.is_bool()
                    || v.is_string()
                    || v.is_null()
                {
                    let s = v.to_string();
                    let string_obj = Box::new(ObjString::new(s));
                    Ok(Value::string(Box::into_raw(string_obj)))
                } else {
                    // 自定义类型：尝试 operator str
                    operators::call_unary_operator(vm, Operator::Str, v)
                };

                match result {
                    Ok(v) => vm.stack.push(v),
                    Err(e) => return InterpretResult::RuntimeError(e),
                }
            }

            CastToBool => {
                let v = vm.stack.pop().expect("Stack underflow");
                vm.stack.push(Value::bool_from(v.is_truthy()));
            }

            // ===== 调试 =====
            Print => {
                let v = vm.stack.pop().expect("Stack underflow");
                println!("{v}");
            }

            Invalid => {
                return InterpretResult::RuntimeError("Invalid opcode".to_string());
            }

            _ => {
                return InterpretResult::RuntimeError(format!("Unimplemented opcode: {op:?}"));
            }
        }
    }
}

// Resume 指令处理
fn handle_resume(vm: &mut VM) {
    use crate::runtime::vm::{call, operators};

    // 操作数：传入值个数
    let arg_count = read_byte(vm);

    // 从栈顶弹出协程对象
    let coro_val = vm.stack.pop().expect("Stack underflow");
    if let Some(coro_ptr) = coro_val.as_coroutine() {
        let coro = unsafe { &mut *coro_ptr };

        // 检查协程状态
        if coro.state == CoroutineState::Dead {
            panic!("Cannot resume dead coroutine"); // 简化处理
        }

        // 收集传入的参数
        let mut args = Vec::with_capacity(arg_count as usize);
        for _ in 0..arg_count {
            args.push(vm.stack.pop().expect("Stack underflow"));
        }
        args.reverse();

        // 如果协程是第一次运行，需要初始化调用帧
        if coro.state == CoroutineState::Suspended && coro.frames.is_empty() {
            let closure = coro.entry_closure;
            let func = unsafe { &*(*closure).function };

            if func.arity != arg_count {
                panic!(
                    "Expected {} arguments but got {}",
                    func.arity, arg_count
                );
            }

            // 创建初始调用帧
            coro.frames.push(CallFrame {
                closure,
                ip: func.chunk.code.as_ptr(),
                locals: args,
                stack_base: 0,
            });
        }

        // 切换到协程上下文执行（简化版：直接在当前 VM 中运行）
        coro.state = CoroutineState::Running;

        // 保存当前 VM 状态
        let saved_stack = std::mem::take(&mut vm.stack);
        let saved_frames = std::mem::take(&mut vm.frames);
        let saved_upvalues = std::mem::take(&mut vm.open_upvalues);

        // 加载协程状态
        vm.stack = std::mem::take(&mut coro.stack);
        vm.frames = std::mem::take(&mut coro.frames);
        vm.open_upvalues = std::mem::take(&mut coro.open_upvalues);

        // 执行协程
        let result = run(vm);

        // 保存协程状态
        coro.stack = std::mem::take(&mut vm.stack);
        coro.frames = std::mem::take(&mut vm.frames);
        coro.open_upvalues = std::mem::take(&mut vm.open_upvalues);

        // 根据执行结果处理
        match result {
            InterpretResult::Ok => {
                coro.state = CoroutineState::Dead;
                // 协程正常结束，获取返回值
                let return_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                // 恢复主 VM 状态
                vm.stack = saved_stack;
                vm.frames = saved_frames;
                vm.open_upvalues = saved_upvalues;
                // 将返回值压入主栈
                vm.stack.push(return_val);
            }
            InterpretResult::RuntimeError(msg) => {
                if msg == "yield" {
                    // 协程通过 yield 挂起
                    coro.state = CoroutineState::Suspended;
                    // 获取 yield 值（在协程栈顶）
                    let yield_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                    // 恢复主 VM 状态
                    vm.stack = saved_stack;
                    vm.frames = saved_frames;
                    vm.open_upvalues = saved_upvalues;
                    // 将 yield 值压入主栈
                    vm.stack.push(yield_val);
                } else {
                    coro.state = CoroutineState::Dead;
                    // 恢复主 VM 状态再返回错误
                    vm.stack = saved_stack;
                    vm.frames = saved_frames;
                    vm.open_upvalues = saved_upvalues;
                    panic!("Coroutine runtime error: {}", msg);
                }
            }
            InterpretResult::CompileError(msg) => {
                coro.state = CoroutineState::Dead;
                // 恢复主 VM 状态再返回错误
                vm.stack = saved_stack;
                vm.frames = saved_frames;
                vm.open_upvalues = saved_upvalues;
                panic!("Coroutine compile error: {}", msg);
            }
        }
    } else {
        panic!("Can only resume coroutines");
    }
}

// IterNext 指令处理
fn handle_iter_next(vm: &mut VM) {
    use crate::runtime::vm::{call, operators};

    // 获取迭代器下一个值
    let iter_val = vm.stack.pop().expect("Stack underflow");

    if let Some(iter_ptr) = iter_val.as_iterator() {
        let iter = unsafe { &mut *iter_ptr };

        // 检查是否是协程迭代器
        if let Some(coro_ptr) = iter.as_coroutine() {
            // 协程迭代器：resume 协程获取下一个值
            let coro = unsafe { &mut *coro_ptr };

            if coro.state == CoroutineState::Dead {
                vm.stack.push(Value::NULL);
            } else {
                // 如果是第一次运行，初始化调用帧
                if coro.state == CoroutineState::Suspended && coro.frames.is_empty() {
                    let closure = coro.entry_closure;
                    let func = unsafe { &*(*closure).function };
                    // 创建初始调用帧（无参数）
                    coro.frames.push(CallFrame {
                        closure,
                        ip: func.chunk.code.as_ptr(),
                        locals: Vec::new(),
                        stack_base: 0,
                    });
                }

                // 执行 resume
                coro.state = CoroutineState::Running;

                // 保存当前 VM 状态
                let saved_stack = std::mem::take(&mut vm.stack);
                let saved_frames = std::mem::take(&mut vm.frames);
                let saved_upvalues = std::mem::take(&mut vm.open_upvalues);

                // 加载协程状态
                vm.stack = std::mem::take(&mut coro.stack);
                vm.frames = std::mem::take(&mut coro.frames);
                vm.open_upvalues = std::mem::take(&mut coro.open_upvalues);

                // 执行协程（无参数 resume）
                let result = run(vm);

                // 保存协程状态
                coro.stack = std::mem::take(&mut vm.stack);
                coro.frames = std::mem::take(&mut vm.frames);
                coro.open_upvalues = std::mem::take(&mut coro.open_upvalues);

                // 恢复主 VM 状态
                vm.stack = saved_stack;
                vm.frames = saved_frames;
                vm.open_upvalues = saved_upvalues;

                // 处理结果
                match result {
                    InterpretResult::Ok => {
                        coro.state = CoroutineState::Dead;
                        let return_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                        vm.stack.push(return_val);
                    }
                    InterpretResult::RuntimeError(msg) => {
                        if msg == "yield" {
                            coro.state = CoroutineState::Suspended;
                            let yield_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                            vm.stack.push(yield_val);
                        } else {
                            coro.state = CoroutineState::Dead;
                            panic!("Coroutine runtime error: {}", msg);
                        }
                    }
                    InterpretResult::CompileError(msg) => {
                        coro.state = CoroutineState::Dead;
                        panic!("Coroutine compile error: {}", msg);
                    }
                }
            }
        } else {
            // 普通迭代器
            match iter.next() {
                Some(value) => vm.stack.push(value),
                None => vm.stack.push(Value::NULL),
            }
        }
    } else {
        panic!("Expected iterator");
    }
}

// ==================== 辅助函数 ====================

/// 获取当前帧的指令指针
#[inline]
pub fn current_ip(vm: &VM) -> *const u8 {
    vm.frames.last().unwrap().ip
}

/// 获取当前帧的可变指令指针
#[inline]
pub fn current_ip_mut(vm: &mut VM) -> &mut *const u8 {
    &mut vm.frames.last_mut().unwrap().ip
}

/// 获取当前帧的可变 locals
#[inline]
pub fn current_locals_mut(vm: &mut VM) -> &mut Vec<Value> {
    &mut vm.frames.last_mut().unwrap().locals
}

/// 获取当前帧的 locals
#[inline]
pub fn current_locals(vm: &VM) -> &Vec<Value> {
    &vm.frames.last().unwrap().locals
}

/// 获取局部变量（自动扩展）
#[inline]
pub fn get_local(vm: &VM, idx: usize) -> Value {
    let locals = current_locals(vm);
    if idx < locals.len() {
        locals[idx]
    } else {
        Value::NULL
    }
}

/// 设置局部变量（自动扩展）
#[inline]
pub fn set_local(vm: &mut VM, idx: usize, value: Value) {
    let locals = current_locals_mut(vm);
    if idx >= locals.len() {
        locals.resize(idx + 1, Value::NULL);
    }
    locals[idx] = value;
}

/// 获取当前帧的 chunk
#[inline]
pub fn current_chunk(vm: &VM) -> &Chunk {
    vm.frames.last().unwrap().chunk()
}

/// 获取常量池中的字符串
#[inline]
pub fn get_constant_string(vm: &VM, idx: usize) -> String {
    let constant = current_chunk(vm).constants[idx];
    if let Some(s) = constant.as_string() {
        unsafe { (*s).chars.clone() }
    } else {
        String::new()
    }
}

/// 获取当前闭包
#[inline]
pub fn current_closure(vm: &VM) -> *mut ObjClosure {
    vm.frames.last().unwrap().closure
}

/// 前进指令指针
#[inline]
pub fn advance_ip(vm: &mut VM, offset: usize) {
    *current_ip_mut(vm) = unsafe { current_ip(vm).add(offset) };
}

/// 跳转指令指针
#[inline]
pub fn jump_ip(vm: &mut VM, offset: isize) {
    *current_ip_mut(vm) = unsafe { current_ip(vm).offset(offset) };
}

/// 读取下一个字节
#[inline]
pub fn read_byte(vm: &mut VM) -> u8 {
    let byte = unsafe { *current_ip(vm) };
    advance_ip(vm, 1);
    byte
}

/// 读取 i16
#[inline]
pub fn read_i16(vm: &mut VM) -> i16 {
    let b1 = read_byte(vm);
    let b2 = read_byte(vm);
    i16::from_le_bytes([b1, b2])
}

/// 读取 u16
#[inline]
pub fn read_u16(vm: &mut VM) -> u16 {
    let b1 = read_byte(vm);
    let b2 = read_byte(vm);
    u16::from_le_bytes([b1, b2])
}

/// 从给定指针读取 u16（小端序）
#[inline]
pub fn read_u16_at_ptr(ip: *const u8) -> u16 {
    let b1 = unsafe { *ip };
    let b2 = unsafe { *ip.add(1) };
    u16::from_le_bytes([b1, b2])
}

/// 从给定指针读取 i16（小端序）
#[inline]
pub fn read_i16_at_ptr(ip: *const u8) -> i16 {
    let b1 = unsafe { *ip };
    let b2 = unsafe { *ip.add(1) };
    i16::from_le_bytes([b1, b2])
}

/// 从常量池加载并压栈
#[inline]
pub fn push_const(vm: &mut VM, idx: usize) {
    let value = current_chunk(vm).constants[idx];
    vm.stack.push(value);
}

/// 追踪当前指令执行
#[cfg(feature = "trace_execution")]
pub fn trace_instruction(vm: &VM) {
    // 反汇编当前指令
    let frame = vm.frames.last().unwrap();
    let chunk = frame.chunk();
    let code = &chunk.code;

    // 计算偏移量：使用指针差值
    let offset = unsafe { frame.ip.offset_from(code.as_ptr()) };
    let offset = if offset >= 0 && (offset as usize) < code.len() {
        offset as usize
    } else {
        // IP 越界，记录警告
        kaubo_log::warn!(
            &vm.logger,
            "VM IP out of bounds: offset={}, code_len={}",
            offset,
            code.len()
        );
        0
    };

    let instruction = unsafe { *frame.ip };
    let op = unsafe { std::mem::transmute::<u8, crate::runtime::OpCode>(instruction) };

    // 使用 logger 记录栈状态和指令
    kaubo_log::trace!(
        &vm.logger,
        "{:04} {:?} | stack: {:?}",
        offset,
        op,
        vm.stack
    );
}
