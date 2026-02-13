//! 标准库实现
//!
//! std 模块是 Rust 原生实现，通过 NativeFn 包装暴露给 Kaubo 代码。
//! 设计原则：
//! - 核心函数用 Rust 实现（性能、系统调用）
//! - 扁平化设计：所有函数直接放在 std 下，不嵌套
//! - 启动时自动注册到 globals

use crate::runtime::object::{ObjModule, ObjNativeVm};
use crate::runtime::Value;
use crate::runtime::VM;
use std::collections::HashMap;

/// 原生函数指针类型
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

/// VM-aware 原生函数指针类型
pub type NativeVmFn = fn(&mut VM, &[Value]) -> Result<Value, String>;

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
    exports.push(create_native_vm_value(
        create_coroutine_fn,
        "create_coroutine",
        1,
    ));
    name_to_shape.insert("create_coroutine".to_string(), 11u16);

    exports.push(create_native_vm_value(resume_fn, "resume", 255)); // 变参
    name_to_shape.insert("resume".to_string(), 12u16);

    exports.push(create_native_vm_value(
        coroutine_status_fn,
        "coroutine_status",
        1,
    ));
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

    let module = ObjModule::new("std".to_string(), exports, name_to_shape);
    vec![("std".to_string(), Box::new(module))]
}

/// 辅助函数：创建原生函数 Value
fn create_native_value(func: NativeFn, name: &str, arity: u8) -> Value {
    use crate::runtime::object::ObjNative;
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

    let string_obj = Box::new(crate::runtime::object::ObjString::new(
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
    let string_obj = Box::new(crate::runtime::object::ObjString::new(s));
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
        Err(format!("Expected number, got unknown type"))
    }
}

// ===== VM-aware 协程函数实现 =====

use crate::runtime::object::{CoroutineState, ObjCoroutine};

/// create_coroutine(closure) -> coroutine
fn create_coroutine_fn(_vm: &mut VM, args: &[Value]) -> Result<Value, String> {
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
fn resume_fn(vm: &mut VM, args: &[Value]) -> Result<Value, String> {
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
        let mut resume_args = Vec::with_capacity(arg_count);
        for i in 1..args.len() {
            resume_args.push(args[i]);
        }

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
            coro.frames.push(crate::runtime::object::CallFrame {
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
            crate::runtime::InterpretResult::Ok => {
                coro.state = CoroutineState::Dead;
                // 协程正常结束，获取返回值
                let return_val = coro.stack.last().copied().unwrap_or(Value::NULL);
                Ok(return_val)
            }
            crate::runtime::InterpretResult::RuntimeError(msg) => {
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
            crate::runtime::InterpretResult::CompileError(msg) => {
                coro.state = CoroutineState::Dead;
                Err(msg)
            }
        }
    } else {
        Err("Can only resume coroutines".to_string())
    }
}

/// coroutine_status(coroutine) -> int (0=Suspended, 1=Running, 2=Dead)
fn coroutine_status_fn(_vm: &mut VM, args: &[Value]) -> Result<Value, String> {
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

use crate::runtime::object::ObjList;

/// len(list|string|json) -> int
fn len_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "len() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let len = if let Some(ptr) = args[0].as_string() {
        unsafe { (&(*ptr).chars).len() as i64 }
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
        unsafe { (&(*ptr).chars).is_empty() }
    } else if let Some(ptr) = args[0].as_list() {
        unsafe { (*ptr).len() == 0 }
    } else if let Some(ptr) = args[0].as_json() {
        unsafe { (*ptr).len() == 0 }
    } else {
        return Err("is_empty() expects string, list, or json".to_string());
    };

    Ok(Value::bool_from(is_empty))
}

/// range(end) or range(start, end) or range(start, end, step) -> list
fn range_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() < 1 || args.len() > 3 {
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
            i = i + step;
        }
    } else {
        let mut i = start;
        while i > end {
            elements.push(Value::smi(i as i32));
            i = i + step;
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
        let new_str = Box::new(crate::runtime::object::ObjString::new(s.chars.clone()));
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

use std::fs;
use std::path::Path;

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
            let string_obj = Box::new(crate::runtime::object::ObjString::new(content));
            Ok(Value::string(Box::into_raw(string_obj)))
        }
        Err(e) => Err(format!("read_file() failed: {}", e)),
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
        Err(e) => Err(format!("write_file() failed: {}", e)),
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
