//! 标准库实现
//!
//! std 模块是 Rust 原生实现，通过 NativeFn 包装暴露给 Kaubo 代码。
//! 设计原则：
//! - 核心函数用 Rust 实现（性能、系统调用）
//! - 扁平化设计：所有函数直接放在 std 下，不嵌套
//! - 启动时自动注册到 globals

use crate::core::runtime::Value;
use crate::core::runtime::object::ObjModule;
use std::collections::HashMap;

/// 原生函数指针类型
pub type NativeFn = fn(&[Value]) -> Result<Value, String>;

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

    let module = ObjModule::new("std".to_string(), exports, name_to_shape);
    vec![("std".to_string(), Box::new(module))]
}

/// 辅助函数：创建原生函数 Value
fn create_native_value(func: NativeFn, name: &str, arity: u8) -> Value {
    use crate::core::runtime::object::ObjNative;
    let native = Box::new(ObjNative::new(func, name.to_string(), arity));
    Value::native_fn(Box::into_raw(native))
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

    let string_obj = Box::new(crate::core::runtime::object::ObjString::new(type_name.to_string()));
    let string_ptr = Box::into_raw(string_obj);
    Ok(Value::string(string_ptr))
}

fn to_string_fn(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("to_string() takes exactly 1 argument ({} given)", args.len()));
    }

    let s = format!("{}", args[0]);
    let string_obj = Box::new(crate::core::runtime::object::ObjString::new(s));
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
