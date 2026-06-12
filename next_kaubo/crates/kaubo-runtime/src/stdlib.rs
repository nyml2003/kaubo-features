//! 标准库实现
//!
//! std 模块是 Rust 原生实现，通过 NativeFn 包装暴露给 Kaubo 代码。
//! 设计原则：
//! - 核心函数用 Rust 实现（性能、系统调用）
//! - 扁平化设计：所有函数直接放在 std 下，不嵌套
//! - 启动时自动注册到 globals

use kaubo_ir::{
    CoroutineState, InterpretResult, NativeVmFn, ObjCoroutine, ObjList, ObjModule, ObjNative, ObjNativeVm,
    ObjString, Value, VM,
};
use crate::vm::VmRuntime;
use std::collections::HashMap;
use std::fs;
use std::path::Path;


/// 原生函数指针类型
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

// NativeVmFn 从 crate::core 导入

/// 创建标准库模块
pub fn create_stdlib_modules() -> Vec<(String, Box<ObjModule>)> {
    let mut exports = Vec::new();
    let mut name_to_shape = HashMap::new();

    // ===== 核心函数 (0-3) =====
    exports.push(create_native_value(print_fn, "print", 1));
    name_to_shape.insert("print".to_string(), 0u16);

    exports.push(create_native_value(assert_fn, "assert", 255)); // 255 = 变参
    name_to_shape.insert("assert".to_string(), 1u16);

    exports.push(create_native_value(type_fn, "type", 1));
    name_to_shape.insert("type".to_string(), 2u16);

    exports.push(create_native_value(to_string_fn, "to_string", 1));
    name_to_shape.insert("to_string".to_string(), 3u16);

    // ===== 数学函数 (4-8) =====
    exports.push(create_native_value(sqrt_fn, "sqrt", 1));
    name_to_shape.insert("sqrt".to_string(), 4u16);

    exports.push(create_native_value(sin_fn, "sin", 1));
    name_to_shape.insert("sin".to_string(), 5u16);

    exports.push(create_native_value(cos_fn, "cos", 1));
    name_to_shape.insert("cos".to_string(), 6u16);

    exports.push(create_native_value(floor_fn, "floor", 1));
    name_to_shape.insert("floor".to_string(), 7u16);

    exports.push(create_native_value(ceil_fn, "ceil", 1));
    name_to_shape.insert("ceil".to_string(), 8u16);

    // ===== 数学常量 (9-10) =====
    exports.push(Value::float(std::f64::consts::PI));
    name_to_shape.insert("PI".to_string(), 9u16);

    exports.push(Value::float(std::f64::consts::E));
    name_to_shape.insert("E".to_string(), 10u16);

    // ===== 协程函数 (11-13) - VM-aware 原生函数 =====
    let create_coro_fn: NativeVmFn = create_coroutine_fn;
    exports.push(create_native_vm_value(create_coro_fn, "create_coroutine", 1));
    name_to_shape.insert("create_coroutine".to_string(), 11u16);

    let resume_fn_typed: NativeVmFn = resume_fn;
    exports.push(create_native_vm_value(resume_fn_typed, "resume", 255)); // 变参
    name_to_shape.insert("resume".to_string(), 12u16);

    let status_fn: NativeVmFn = coroutine_status_fn;
    exports.push(create_native_vm_value(status_fn, "coroutine_status", 1));
    name_to_shape.insert("coroutine_status".to_string(), 13u16);

    // ===== 列表操作函数 (14-18) =====
    exports.push(create_native_value(len_fn, "len", 1));
    name_to_shape.insert("len".to_string(), 14u16);

    exports.push(create_native_value(push_fn, "push", 2));
    name_to_shape.insert("push".to_string(), 15u16);

    exports.push(create_native_value(is_empty_fn, "is_empty", 1));
    name_to_shape.insert("is_empty".to_string(), 16u16);

    // ===== 实用函数 (17-18) =====
    exports.push(create_native_value(range_fn, "range", 255)); // 变参 1-3
    name_to_shape.insert("range".to_string(), 17u16);

    exports.push(create_native_value(clone_fn, "clone", 1));
    name_to_shape.insert("clone".to_string(), 18u16);

    // ===== 文件系统函数 (19-23) =====
    exports.push(create_native_value(read_file_fn, "read_file", 1));
    name_to_shape.insert("read_file".to_string(), 19u16);

    exports.push(create_native_value(write_file_fn, "write_file", 2));
    name_to_shape.insert("write_file".to_string(), 20u16);

    exports.push(create_native_value(exists_fn, "exists", 1));
    name_to_shape.insert("exists".to_string(), 21u16);

    exports.push(create_native_value(is_file_fn, "is_file", 1));
    name_to_shape.insert("is_file".to_string(), 22u16);

    exports.push(create_native_value(is_dir_fn, "is_dir", 1));
    name_to_shape.insert("is_dir".to_string(), 23u16);

    // ===== 字符串函数 (24-27) =====
    exports.push(create_native_value(substring_fn, "substring", 3));
    name_to_shape.insert("substring".to_string(), 24u16);

    exports.push(create_native_value(contains_fn, "contains", 2));
    name_to_shape.insert("contains".to_string(), 25u16);

    exports.push(create_native_value(starts_with_fn, "starts_with", 2));
    name_to_shape.insert("starts_with".to_string(), 26u16);

    exports.push(create_native_value(ends_with_fn, "ends_with", 2));
    name_to_shape.insert("ends_with".to_string(), 27u16);

    // ===== 环境与时间 (28-29) =====
    exports.push(create_native_value(env_fn, "env", 1));
    name_to_shape.insert("env".to_string(), 28u16);

    exports.push(create_native_value(now_fn, "now", 0));
    name_to_shape.insert("now".to_string(), 29u16);

    let module = ObjModule::new("std".to_string(), exports, name_to_shape);
    vec![("std".to_string(), Box::new(module))]
}

/// 辅助函数：创建原生函数 Value
fn create_native_value(func: NativeFn, name: &str, arity: u8) -> Value {
    let native = Box::new(ObjNative::new(func, name.to_string(), arity));
    Value::native_fn(Box::into_raw(native))
}

/// 辅助函数：创建 VM-aware 原生函数 Value
fn create_native_vm_value(func: NativeVmFn, name: &str, arity: u8) -> Value {
    let native_vm = Box::new(ObjNativeVm::new(func, name.to_string(), arity));
    Value::native_vm_fn(Box::into_raw(native_vm))
}

// ===== 核心函数实现 =====

fn print_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "print() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }
    println!("{}", args[0]);
    Ok(Value::NULL)
}

fn assert_fn(args: &[Value]) -> Result<Value, String> {
    match args.len() {
        1 => {
            if !args[0].is_truthy() {
                return Err("Assertion failed".to_string());
            }
        }
        2 => {
            if !args[0].is_truthy() {
                let msg = if let Some(s) = args[1].as_string() {
                    unsafe { &(*s).chars }
                } else {
                    "Assertion failed"
                };
                return Err(msg.to_string());
            }
        }
        _ => {
            return Err(format!(
                "assert() takes 1 or 2 arguments ({} given)",
                args.len()
            ))
        }
    }
    Ok(Value::NULL)
}

fn type_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "type() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let type_name = if args[0].is_int() {
        "int"
    } else if args[0].is_float() {
        "float"
    } else if args[0].is_bool() {
        "bool"
    } else if args[0].is_null() {
        "null"
    } else if args[0].is_string() {
        "string"
    } else if args[0].is_list() {
        "list"
    } else if args[0].is_closure() || args[0].is_function() {
        "function"
    } else if args[0].is_module() {
        "module"
    } else if args[0].is_json() {
        "json"
    } else if args[0].is_coroutine() {
        "coroutine"
    } else {
        "unknown"
    };

    let string_obj = Box::new(ObjString::new(
        type_name.to_string(),
    ));
    let string_ptr = Box::into_raw(string_obj);
    Ok(Value::string(string_ptr))
}

fn to_string_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "to_string() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let s = format!("{}", args[0]);
    let string_obj = Box::new(ObjString::new(s));
    let string_ptr = Box::into_raw(string_obj);
    Ok(Value::string(string_ptr))
}

// ===== 数学函数实现 =====

fn sqrt_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "sqrt() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let x = to_f64(&args[0])?;
    if x < 0.0 {
        return Err("sqrt() domain error".to_string());
    }
    Ok(Value::float(x.sqrt()))
}

fn sin_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "sin() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let x = to_f64(&args[0])?;
    Ok(Value::float(x.sin()))
}

fn cos_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "cos() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let x = to_f64(&args[0])?;
    Ok(Value::float(x.cos()))
}

fn floor_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "floor() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let x = to_f64(&args[0])?;
    let result = x.floor();
    if result == result as i32 as f64 {
        Ok(Value::int(result as i32))
    } else {
        Ok(Value::float(result))
    }
}

fn ceil_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "ceil() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let x = to_f64(&args[0])?;
    let result = x.ceil();
    if result == result as i32 as f64 {
        Ok(Value::int(result as i32))
    } else {
        Ok(Value::float(result))
    }
}

/// 辅助函数：将 Value 转为 f64
fn to_f64(value: &Value) -> Result<f64, String> {
    if let Some(n) = value.as_int() {
        Ok(n as f64)
    } else if value.is_float() {
        Ok(value.as_float())
    } else {
        Err("Expected number, got unknown type".to_string())
    }
}

// ===== VM-aware 协程函数实现 =====



/// create_coroutine(closure) -> coroutine
fn create_coroutine_fn(_vm_ptr: *mut (), args: &[Value]) -> Result<Value, String> {
    let _vm = unsafe { &mut *(_vm_ptr as *mut VM) };
    if args.len() != 1 {
        return Err(format!(
            "create_coroutine() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    if let Some(closure_ptr) = args[0].as_closure() {
        let coroutine = Box::new(ObjCoroutine::new(closure_ptr));
        Ok(Value::coroutine(Box::into_raw(coroutine)))
    } else {
        Err("Coroutine must be created from a closure".to_string())
    }
}

/// resume(coroutine, ...values) -> yielded_value
fn resume_fn(vm_ptr: *mut (), args: &[Value]) -> Result<Value, String> {
    let vm = unsafe { &mut *(vm_ptr as *mut VM) };
    if args.is_empty() {
        return Err("resume() expects at least 1 argument (coroutine)".to_string());
    }

    let coro_val = args[0];
    if let Some(coro_ptr) = coro_val.as_coroutine() {
        let coro = unsafe { &mut *coro_ptr };

        // 检查协程状态
        if coro.state == CoroutineState::Dead {
            return Err("Cannot resume dead coroutine".to_string());
        }

        // 收集传入的参数（除了第一个协程参数）
        let arg_count = args.len() - 1;
        let resume_args: Vec<_> = args.iter().skip(1).copied().collect();

        // 如果协程是第一次运行，需要初始化调用帧
        if coro.state == CoroutineState::Suspended && coro.frames.is_empty() {
            let closure = coro.entry_closure;
            let func = unsafe { &*(*closure).function };

            if func.arity != arg_count as u8 {
                return Err(format!(
                    "Expected {} arguments but got {}",
                    func.arity, arg_count
                ));
            }

            // 创建初始调用帧
            coro.frames.push(kaubo_ir::CallFrame {
                closure,
                ip: func.chunk.code.as_ptr(),
                locals: resume_args,
                stack_base: 0,
            });
        }

        // 切换到协程上下文执行
        coro.state = CoroutineState::Running;

        // 保存当前 VM 状态
        let saved_stack = std::mem::take(vm.stack_mut());
        let saved_frames = std::mem::take(vm.frames_mut());
        let saved_upvalues = std::mem::take(vm.open_upvalues_mut());

        // 加载协程状态
        *vm.stack_mut() = std::mem::take(&mut coro.stack);
        *vm.frames_mut() = std::mem::take(&mut coro.frames);
        *vm.open_upvalues_mut() = std::mem::take(&mut coro.open_upvalues);

        // 执行协程
        let result = vm.run();

        // 保存协程状态
        coro.stack = std::mem::take(vm.stack_mut());
        coro.frames = std::mem::take(vm.frames_mut());
        coro.open_upvalues = std::mem::take(vm.open_upvalues_mut());

        // 恢复主 VM 状态
        *vm.stack_mut() = saved_stack;
        *vm.frames_mut() = saved_frames;
        *vm.open_upvalues_mut() = saved_upvalues;

        // 根据执行结果处理
        match result {
            InterpretResult::Ok => {
                coro.state = CoroutineState::Dead;
                // 协程正常结束，获取返回值
                let return_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                Ok(return_val)
            }
            InterpretResult::RuntimeError(msg) => {
                if msg == "yield" {
                    // 协程通过 yield 挂起
                    coro.state = CoroutineState::Suspended;
                    // 获取 yield 值（在协程栈顶）
                    let yield_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                    Ok(yield_val)
                } else {
                    coro.state = CoroutineState::Dead;
                    Err(msg)
                }
            }
            InterpretResult::CompileError(msg) => {
                coro.state = CoroutineState::Dead;
                Err(msg)
            }
        }
    } else {
        Err("Can only resume coroutines".to_string())
    }
}

/// coroutine_status(coroutine) -> int (0=Suspended, 1=Running, 2=Dead)
fn coroutine_status_fn(_vm_ptr: *mut (), args: &[Value]) -> Result<Value, String> {
    let _vm = unsafe { &mut *(_vm_ptr as *mut VM) };
    if args.len() != 1 {
        return Err(format!(
            "coroutine_status() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    if let Some(coro_ptr) = args[0].as_coroutine() {
        let coro = unsafe { &*coro_ptr };
        let status = match coro.state {
            CoroutineState::Suspended => 0i64,
            CoroutineState::Running => 1i64,
            CoroutineState::Dead => 2i64,
        };
        Ok(Value::smi(status as i32))
    } else {
        Err("Expected a coroutine".to_string())
    }
}

// ===== 列表操作函数实现 =====



/// len(list|string|json) -> int
fn len_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "len() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let len = if let Some(ptr) = args[0].as_string() {
        let s = unsafe { &(*ptr).chars };
        s.len() as i64
    } else if let Some(ptr) = args[0].as_list() {
        unsafe { (*ptr).len() as i64 }
    } else if let Some(ptr) = args[0].as_json() {
        unsafe { (*ptr).len() as i64 }
    } else {
        return Err("len() expects string, list, or json".to_string());
    };

    Ok(Value::smi(len as i32))
}

/// push(list, value) -> list (返回新列表)
fn push_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "push() takes exactly 2 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = args[0].as_list() {
        let list = unsafe { &*ptr };
        let mut new_elements = Vec::new();

        // 复制原列表元素
        for i in 0..list.len() {
            if let Some(val) = list.get(i) {
                new_elements.push(val);
            }
        }

        // 添加新元素
        new_elements.push(args[1]);

        // 创建新列表
        let new_list = Box::new(ObjList::from_vec(new_elements));
        Ok(Value::list(Box::into_raw(new_list)))
    } else {
        Err("push() first argument must be a list".to_string())
    }
}

/// is_empty(list|string|json) -> bool
fn is_empty_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "is_empty() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let is_empty = if let Some(ptr) = args[0].as_string() {
        let s = unsafe { &(*ptr).chars };
        s.is_empty()
    } else if let Some(ptr) = args[0].as_list() {
        unsafe { (*ptr).is_empty() }
    } else if let Some(ptr) = args[0].as_json() {
        unsafe { (*ptr).is_empty() }
    } else {
        return Err("is_empty() expects string, list, or json".to_string());
    };

    Ok(Value::bool_from(is_empty))
}

/// range(end) or range(start, end) or range(start, end, step) -> list
fn range_fn(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "range() takes 1 to 3 arguments ({} given)",
            args.len()
        ));
    }

    let (start, end, step) = match args.len() {
        1 => {
            let e = to_i64(&args[0])?;
            (0i64, e, 1i64)
        }
        2 => {
            let s = to_i64(&args[0])?;
            let e = to_i64(&args[1])?;
            (s, e, 1i64)
        }
        3 => {
            let s = to_i64(&args[0])?;
            let e = to_i64(&args[1])?;
            let st = to_i64(&args[2])?;
            if st == 0 {
                return Err("range() step cannot be zero".to_string());
            }
            (s, e, st)
        }
        _ => unreachable!(),
    };

    let mut elements = Vec::new();
    if step > 0 {
        let mut i = start;
        while i < end {
            elements.push(Value::smi(i as i32));
            i += step;
        }
    } else {
        let mut i = start;
        while i > end {
            elements.push(Value::smi(i as i32));
            i += step;
        }
    }

    let list = Box::new(ObjList::from_vec(elements));
    Ok(Value::list(Box::into_raw(list)))
}

/// clone(value) -> value (浅拷贝)
fn clone_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "clone() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    // 对于基本类型直接返回，对于容器类型创建新对象
    let cloned = if let Some(ptr) = args[0].as_list() {
        let list = unsafe { &*ptr };
        let mut new_elements = Vec::new();
        for i in 0..list.len() {
            if let Some(val) = list.get(i) {
                new_elements.push(val);
            }
        }
        let new_list = Box::new(ObjList::from_vec(new_elements));
        Value::list(Box::into_raw(new_list))
    } else if let Some(ptr) = args[0].as_string() {
        let s = unsafe { &*ptr };
        let new_str = Box::new(ObjString::new(s.chars.clone()));
        Value::string(Box::into_raw(new_str))
    } else {
        // 其他类型直接复制值
        args[0]
    };

    Ok(cloned)
}

/// 辅助函数：将 Value 转为 i64
fn to_i64(value: &Value) -> Result<i64, String> {
    if let Some(n) = value.as_int() {
        Ok(n as i64)
    } else {
        Err("Expected integer".to_string())
    }
}

// ===== 文件系统函数实现 =====

/// read_file(path) -> string
fn read_file_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "read_file() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let path = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("read_file() argument must be a string".to_string());
    };

    match fs::read_to_string(path) {
        Ok(content) => {
            let string_obj = Box::new(ObjString::new(content));
            Ok(Value::string(Box::into_raw(string_obj)))
        }
        Err(e) => Err(format!("read_file() failed: {e}")),
    }
}

/// write_file(path, content) -> null
fn write_file_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "write_file() takes exactly 2 arguments ({} given)",
            args.len()
        ));
    }

    let path = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("write_file() first argument must be a string".to_string());
    };

    let content = if let Some(ptr) = args[1].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("write_file() second argument must be a string".to_string());
    };

    match fs::write(path, content) {
        Ok(_) => Ok(Value::NULL),
        Err(e) => Err(format!("write_file() failed: {e}")),
    }
}

/// exists(path) -> bool
fn exists_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "exists() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let path = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("exists() argument must be a string".to_string());
    };

    Ok(Value::bool_from(Path::new(path).exists()))
}

/// is_file(path) -> bool
fn is_file_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "is_file() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let path = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("is_file() argument must be a string".to_string());
    };

    Ok(Value::bool_from(Path::new(path).is_file()))
}

/// is_dir(path) -> bool
fn is_dir_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "is_dir() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let path = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("is_dir() argument must be a string".to_string());
    };

    Ok(Value::bool_from(Path::new(path).is_dir()))
}

// ===== 字符串函数实现 =====

fn substring_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(format!(
            "substring() takes exactly 3 arguments ({} given)",
            args.len()
        ));
    }

    let s = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("substring() first argument must be a string".to_string());
    };

    let start = to_i64(&args[1])? as usize;
    let end = to_i64(&args[2])? as usize;

    let chars: Vec<char> = s.chars().collect();
    if start >= chars.len() || end > chars.len() || start > end {
        return Err("substring() index out of bounds".to_string());
    }
    let result: String = chars[start..end].iter().collect();
    let string_obj = Box::new(ObjString::new(result));
    Ok(Value::string(Box::into_raw(string_obj)))
}

fn contains_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "contains() takes exactly 2 arguments ({} given)",
            args.len()
        ));
    }

    let s = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("contains() first argument must be a string".to_string());
    };
    let substr = if let Some(ptr) = args[1].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("contains() second argument must be a string".to_string());
    };

    Ok(Value::bool_from(s.contains(substr.as_str())))
}

fn starts_with_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "starts_with() takes exactly 2 arguments ({} given)",
            args.len()
        ));
    }

    let s = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("starts_with() first argument must be a string".to_string());
    };
    let prefix = if let Some(ptr) = args[1].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("starts_with() second argument must be a string".to_string());
    };

    Ok(Value::bool_from(s.starts_with(prefix.as_str())))
}

fn ends_with_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "ends_with() takes exactly 2 arguments ({} given)",
            args.len()
        ));
    }

    let s = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("ends_with() first argument must be a string".to_string());
    };
    let suffix = if let Some(ptr) = args[1].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("ends_with() second argument must be a string".to_string());
    };

    Ok(Value::bool_from(s.ends_with(suffix.as_str())))
}

// ===== 环境与时间函数实现 =====

fn env_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "env() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let name = if let Some(ptr) = args[0].as_string() {
        unsafe { &(*ptr).chars }
    } else {
        return Err("env() argument must be a string".to_string());
    };

    match std::env::var(name.as_str()) {
        Ok(val) => {
            let string_obj = Box::new(ObjString::new(val));
            Ok(Value::string(Box::into_raw(string_obj)))
        }
        Err(_) => Ok(Value::NULL),
    }
}

fn now_fn(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("now() takes no arguments".to_string());
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(Value::float(ts as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== 核心函数测试 =====

    #[test]
    fn test_print_too_many_args() {
        let args = [Value::smi(1), Value::smi(2)];
        let result = print_fn(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_true() {
        let args = [Value::bool_from(true)];
        let result = assert_fn(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_false() {
        let args = [Value::bool_from(false)];
        let result = assert_fn(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_with_message() {
        let msg = Box::new(ObjString::new("fail".to_string()));
        let args = [Value::bool_from(false), Value::string(Box::into_raw(msg))];
        let result = assert_fn(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_wrong_arg_count() {
        let args: [Value; 0] = [];
        let result = assert_fn(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_int() {
        let args = [Value::smi(42)];
        let result = type_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "int");
    }

    #[test]
    fn test_type_float() {
        let args = [Value::float(3.14)];
        let result = type_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "float");
    }

    #[test]
    fn test_type_bool() {
        let args = [Value::bool_from(true)];
        let result = type_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "bool");
    }

    #[test]
    fn test_type_null() {
        let args = [Value::NULL];
        let result = type_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "null");
    }

    #[test]
    fn test_type_string() {
        let s = Box::new(ObjString::new("hello".to_string()));
        let args = [Value::string(Box::into_raw(s))];
        let result = type_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "string");
    }

    #[test]
    fn test_type_wrong_arg_count() {
        let args: [Value; 0] = [];
        assert!(type_fn(&args).is_err());
    }

    #[test]
    fn test_to_string_int() {
        let args = [Value::smi(42)];
        let result = to_string_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "42");
    }

    #[test]
    fn test_to_string_float() {
        let args = [Value::float(3.14)];
        let result = to_string_fn(&args).unwrap();
        assert!(unsafe { &(*(result.as_string().unwrap())).chars }.contains("3.14"));
    }

    // ===== 数学函数测试 =====

    #[test]
    fn test_sqrt() {
        let args = [Value::float(4.0)];
        let result = sqrt_fn(&args).unwrap();
        assert!((result.as_float() - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_sqrt_int() {
        let args = [Value::smi(9)];
        let result = sqrt_fn(&args).unwrap();
        assert!((result.as_float() - 3.0).abs() < 0.0001);
    }

    #[test]
    fn test_sqrt_negative() {
        let args = [Value::float(-1.0)];
        assert!(sqrt_fn(&args).is_err());
    }

    #[test]
    fn test_sqrt_wrong_args() {
        let args: [Value; 0] = [];
        assert!(sqrt_fn(&args).is_err());
    }

    #[test]
    fn test_sin() {
        let args = [Value::float(std::f64::consts::PI / 2.0)];
        let result = sin_fn(&args).unwrap();
        assert!((result.as_float() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cos() {
        let args = [Value::float(0.0)];
        let result = cos_fn(&args).unwrap();
        assert!((result.as_float() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_floor() {
        let args = [Value::float(3.7)];
        let result = floor_fn(&args).unwrap();
        assert_eq!(result.as_int(), Some(3));
    }

    #[test]
    fn test_ceil() {
        let args = [Value::float(3.2)];
        let result = ceil_fn(&args).unwrap();
        assert_eq!(result.as_int(), Some(4));
    }

    #[test]
    fn test_floor_negative() {
        let args = [Value::float(-3.7)];
        let result = floor_fn(&args).unwrap();
        assert_eq!(result.as_int(), Some(-4));
    }

    #[test]
    fn test_math_wrong_arg_type() {
        let args = [Value::NULL];
        assert!(sqrt_fn(&args).is_err());
    }

    // ===== 列表操作函数测试 =====

    #[test]
    fn test_len_empty_list() {
        let list = Box::new(ObjList::new());
        let args = [Value::list(Box::into_raw(list))];
        let result = len_fn(&args).unwrap();
        assert_eq!(result.as_int(), Some(0));
    }

    #[test]
    fn test_len_non_empty_list() {
        let list = ObjList::from_vec(vec![Value::smi(1), Value::smi(2), Value::smi(3)]);
        let args = [Value::list(Box::into_raw(Box::new(list)))];
        let result = len_fn(&args).unwrap();
        assert_eq!(result.as_int(), Some(3));
    }

    #[test]
    fn test_push() {
        let list = ObjList::from_vec(vec![Value::smi(1)]);
        let args = [Value::list(Box::into_raw(Box::new(list))), Value::smi(2)];
        let result = push_fn(&args).unwrap();
        let pushed_list = unsafe { &*(result.as_list().unwrap()) };
        assert_eq!(pushed_list.elements.len(), 2);
    }

    #[test]
    fn test_is_empty_true() {
        let list = Box::new(ObjList::new());
        let args = [Value::list(Box::into_raw(list))];
        let result = is_empty_fn(&args).unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_is_empty_false() {
        let list = ObjList::from_vec(vec![Value::smi(1)]);
        let args = [Value::list(Box::into_raw(Box::new(list)))];
        let result = is_empty_fn(&args).unwrap();
        assert!(!result.is_true());
    }

    // ===== 工具函数测试 =====

    #[test]
    fn test_range_single_arg() {
        let args = [Value::smi(3)];
        let result = range_fn(&args).unwrap();
        let list = unsafe { &*(result.as_list().unwrap()) };
        assert_eq!(list.elements.len(), 3);
        assert_eq!(list.elements[0].as_smi(), Some(0));
        assert_eq!(list.elements[2].as_smi(), Some(2));
    }

    #[test]
    fn test_range_two_args() {
        let args = [Value::smi(2), Value::smi(5)];
        let result = range_fn(&args).unwrap();
        let list = unsafe { &*(result.as_list().unwrap()) };
        assert_eq!(list.elements.len(), 3);
        assert_eq!(list.elements[0].as_smi(), Some(2));
    }

    #[test]
    fn test_range_three_args() {
        let args = [Value::smi(0), Value::smi(10), Value::smi(3)];
        let result = range_fn(&args).unwrap();
        let list = unsafe { &*(result.as_list().unwrap()) };
        assert_eq!(list.elements.len(), 4);
        assert_eq!(list.elements[1].as_smi(), Some(3));
    }

    #[test]
    fn test_range_invalid() {
        let args: [Value; 0] = [];
        assert!(range_fn(&args).is_err());
    }

    #[test]
    fn test_clone_int() {
        let args = [Value::smi(42)];
        let result = clone_fn(&args).unwrap();
        assert_eq!(result.as_smi(), Some(42));
    }

    #[test]
    fn test_clone_float() {
        let args = [Value::float(3.14)];
        let result = clone_fn(&args).unwrap();
        assert!((result.as_float() - 3.14).abs() < 0.0001);
    }

    // ===== 文件 I/O 测试 =====

    #[test]
    fn test_file_exists_positive() {
        let path = Box::new(ObjString::new("Cargo.toml".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        let result = exists_fn(&args).unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_file_exists_negative() {
        let path = Box::new(ObjString::new("nonexistent_file.xyz".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        let result = exists_fn(&args).unwrap();
        assert!(!result.is_true());
    }

    #[test]
    fn test_is_file() {
        let path = Box::new(ObjString::new("Cargo.toml".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        let result = is_file_fn(&args).unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_is_dir() {
        let path = Box::new(ObjString::new("src".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        let result = is_dir_fn(&args).unwrap();
        assert!(result.is_true());
    }

    #[test]
    fn test_is_dir_false() {
        let path = Box::new(ObjString::new("Cargo.toml".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        let result = is_dir_fn(&args).unwrap();
        assert!(!result.is_true());
    }

    #[test]
    fn test_read_file() {
        let path = Box::new(ObjString::new("Cargo.toml".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        let result = read_file_fn(&args).unwrap();
        let content = unsafe { &(*(result.as_string().unwrap())).chars };
        assert!(!content.is_empty());
    }

    #[test]
    fn test_read_file_not_found() {
        let path = Box::new(ObjString::new("nonexistent_file.xyz".to_string()));
        let args = [Value::string(Box::into_raw(path))];
        assert!(read_file_fn(&args).is_err());
    }

    #[test]
    fn test_write_and_read_file() {
        let tmp_path = "test_write_temp.txt";
        let path_val = Box::new(ObjString::new(tmp_path.to_string()));
        let content = Box::new(ObjString::new("hello kaubo".to_string()));

        let args = [
            Value::string(Box::into_raw(path_val)),
            Value::string(Box::into_raw(content)),
        ];
        let result = write_file_fn(&args).unwrap();
        assert!(result.is_null());

        let path_val2 = Box::new(ObjString::new(tmp_path.to_string()));
        let args2 = [Value::string(Box::into_raw(path_val2))];
        let result2 = read_file_fn(&args2).unwrap();
        let content2 = unsafe { &(*(result2.as_string().unwrap())).chars };
        assert_eq!(content2, "hello kaubo");

        std::fs::remove_file(tmp_path).ok();
    }

    // ===== 字符串函数测试 =====

    #[test]
    fn test_substring() {
        let s = Box::new(ObjString::new("hello world".to_string()));
        let args = [
            Value::string(Box::into_raw(s)),
            Value::smi(0),
            Value::smi(5),
        ];
        let result = substring_fn(&args).unwrap();
        assert_eq!(unsafe { &(*(result.as_string().unwrap())).chars }, "hello");
    }

    #[test]
    fn test_contains_true() {
        let s = Box::new(ObjString::new("hello world".to_string()));
        let sub = Box::new(ObjString::new("world".to_string()));
        let args = [Value::string(Box::into_raw(s)), Value::string(Box::into_raw(sub))];
        assert!(contains_fn(&args).unwrap().is_true());
    }

    #[test]
    fn test_contains_false() {
        let s = Box::new(ObjString::new("hello".to_string()));
        let sub = Box::new(ObjString::new("xyz".to_string()));
        let args = [Value::string(Box::into_raw(s)), Value::string(Box::into_raw(sub))];
        assert!(!contains_fn(&args).unwrap().is_true());
    }

    #[test]
    fn test_starts_with() {
        let s = Box::new(ObjString::new("hello world".to_string()));
        let prefix = Box::new(ObjString::new("hello".to_string()));
        let args = [Value::string(Box::into_raw(s)), Value::string(Box::into_raw(prefix))];
        assert!(starts_with_fn(&args).unwrap().is_true());
    }

    #[test]
    fn test_ends_with() {
        let s = Box::new(ObjString::new("hello world".to_string()));
        let suffix = Box::new(ObjString::new("world".to_string()));
        let args = [Value::string(Box::into_raw(s)), Value::string(Box::into_raw(suffix))];
        assert!(ends_with_fn(&args).unwrap().is_true());
    }

    #[test]
    fn test_now_returns_float() {
        let args: [Value; 0] = [];
        let result = now_fn(&args).unwrap();
        assert!(result.is_float() || result.is_int());
    }

    #[test]
    fn test_now_args_error() {
        let args = [Value::smi(1)];
        assert!(now_fn(&args).is_err());
    }
}
