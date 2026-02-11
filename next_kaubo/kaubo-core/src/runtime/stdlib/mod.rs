//! 标准库实现
//!
//! std 模块是 Rust 原生实现，通过 NativeFn 包装暴露给 Kaubo 代码。
//! 设计原则：
//! - 核心函数用 Rust 实现（性能、系统调用）
//! - 扁平化设计：所有函数直接放在 std 下，不嵌套
//! - 启动时自动注册到 globals

use crate::runtime::Value;
use crate::runtime::object::{ObjModule, ObjNativeVm};
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

    exports.push(create_native_value(assert_fn, "assert", 255));  // 255 = 变参
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
    exports.push(create_native_vm_value(create_coroutine_fn, "create_coroutine", 1));
    name_to_shape.insert("create_coroutine".to_string(), 11u16);

    exports.push(create_native_vm_value(resume_fn, "resume", 255)); // 变参
    name_to_shape.insert("resume".to_string(), 12u16);

    exports.push(create_native_vm_value(coroutine_status_fn, "coroutine_status", 1));
    name_to_shape.insert("coroutine_status".to_string(), 13u16);

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
        return Err(format!("print() takes exactly 1 argument ({} given)", args.len()));
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
        _ => return Err(format!("assert() takes 1 or 2 arguments ({} given)", args.len())),
    }
    Ok(Value::NULL)
}

fn type_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("type() takes exactly 1 argument ({} given)", args.len()));
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

    let string_obj = Box::new(crate::runtime::object::ObjString::new(type_name.to_string()));
    let string_ptr = Box::into_raw(string_obj);
    Ok(Value::string(string_ptr))
}

fn to_string_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("to_string() takes exactly 1 argument ({} given)", args.len()));
    }

    let s = format!("{}", args[0]);
    let string_obj = Box::new(crate::runtime::object::ObjString::new(s));
    let string_ptr = Box::into_raw(string_obj);
    Ok(Value::string(string_ptr))
}

// ===== 数学函数实现 =====

fn sqrt_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("sqrt() takes exactly 1 argument ({} given)", args.len()));
    }

    let x = to_f64(&args[0])?;
    if x < 0.0 {
        return Err("sqrt() domain error".to_string());
    }
    Ok(Value::float(x.sqrt()))
}

fn sin_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("sin() takes exactly 1 argument ({} given)", args.len()));
    }

    let x = to_f64(&args[0])?;
    Ok(Value::float(x.sin()))
}

fn cos_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("cos() takes exactly 1 argument ({} given)", args.len()));
    }

    let x = to_f64(&args[0])?;
    Ok(Value::float(x.cos()))
}

fn floor_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("floor() takes exactly 1 argument ({} given)", args.len()));
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
        return Err(format!("ceil() takes exactly 1 argument ({} given)", args.len()));
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

use crate::runtime::object::{ObjCoroutine, CoroutineState};

/// create_coroutine(closure) -> coroutine
fn create_coroutine_fn(_vm: &mut VM, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("create_coroutine() takes exactly 1 argument ({} given)", args.len()));
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
        return Err(format!("coroutine_status() takes exactly 1 argument ({} given)", args.len()));
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
