//! 内置类型方法表
//!
//! 为 List、String、JSON 等内置类型提供方法支持。
//! 设计目标：支持 `list.push(100)` 语法，和 Struct 方法调用保持一致。
//!
//! # 核心设计
//!
//! - 编译期确定方法索引（0=push, 1=len...）
//! - LoadMethod 指令 peek receiver，压入编码方法标识（SMI）
//! - Call 指令统一从栈取 receiver，内置类型走原生分发
//! - 零堆分配，和 Struct 方法完全一致

// use crate::vm::core::object::ObjList;
use crate::vm::core::value::Value;
use crate::vm::core::VM;
use crate::vm::runtime::vm::call_operator_closure;

// ==================== 类型常量 ====================

/// 内置类型标签
pub mod builtin_types {
    pub const LIST: u8 = 0;
    pub const STRING: u8 = 1;
    pub const JSON: u8 = 2;
}

/// List 方法索引（编译期确定）
pub mod list_methods {
    pub const PUSH: u8 = 0;
    pub const LEN: u8 = 1;
    pub const REMOVE: u8 = 2;
    pub const CLEAR: u8 = 3;
    pub const IS_EMPTY: u8 = 4;
    // 函数式方法
    pub const FOREACH: u8 = 5;
    pub const MAP: u8 = 6;
    pub const FILTER: u8 = 7;
    pub const REDUCE: u8 = 8;
    pub const FIND: u8 = 9;
    pub const ANY: u8 = 10;
    pub const ALL: u8 = 11;

    pub const COUNT: usize = 12;
}

/// String 方法索引（预留）
pub mod string_methods {
    pub const LEN: u8 = 0;
    pub const IS_EMPTY: u8 = 1;

    pub const COUNT: usize = 2;
}

/// JSON 方法索引（预留）
pub mod json_methods {
    pub const LEN: u8 = 0;
    pub const IS_EMPTY: u8 = 1;

    pub const COUNT: usize = 2;
}

// ==================== 方法函数类型 ====================

/// 内置类型方法分发函数类型
///
/// # 参数
/// - vm: VM 实例（用于调用闭包）
/// - receiver: 方法接收者（self）
/// - args: 方法参数列表（不包含 self）
///
/// # 返回
/// - Ok(Value): 方法返回值
/// - Err(String): 运行时错误信息
pub type BuiltinMethodFn = fn(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String>;

// ==================== SMI 编码 ====================

/// 编码: 高 4 位 = 类型，低 4 位 = 方法索引
/// 范围: 0x0100 ~ 0x01FF（避开普通小整数）
///
/// 编码方案：
/// - 0x0100 ~ 0x010F: List 方法 (type=0)
/// - 0x0110 ~ 0x011F: String 方法 (type=1)
/// - 0x0120 ~ 0x012F: JSON 方法 (type=2)
pub fn encode_method(type_tag: u8, method_idx: u8) -> i32 {
    debug_assert!(type_tag <= 0xF, "Type tag overflow: {}", type_tag);
    debug_assert!(method_idx <= 0xF, "Method index overflow: {}", method_idx);
    0x0100 + ((type_tag as i32) << 4) + (method_idx as i32)
}

/// 解码方法标识
///
/// # 返回
/// - Some((type_tag, method_idx)): 解码成功
/// - None: 不是有效的内置方法编码
pub fn decode_method(value: Value) -> Option<(u8, u8)> {
    let n = value.as_int()?;
    if n >= 0x0100 && n < 0x0200 {
        let type_tag = ((n - 0x0100) >> 4) as u8;
        let method_idx = ((n - 0x0100) & 0xF) as u8;
        Some((type_tag, method_idx))
    } else {
        None
    }
}

/// 检查 Value 是否为内置方法编码
pub fn is_builtin_method(value: Value) -> bool {
    value.as_int().map_or(false, |n| n >= 0x0100 && n < 0x0200)
}

// ==================== List 方法实现 ====================

/// list.push(item) -> list
///
/// 向列表末尾添加一个元素，返回 receiver 支持链式调用
fn list_push(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "push() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_list() {
        // SAFETY: List 对象由 GC 管理，且当前是单线程 VM
        let list = unsafe { &mut *ptr };
        list.elements.push(args[0]);
        Ok(receiver) // 返回 receiver 支持链式调用
    } else {
        Err("push() receiver must be a list".to_string())
    }
}

/// list.len() -> int
///
/// 返回列表元素个数
fn list_len(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "len() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_list() {
        let len = unsafe { (*ptr).len() as i32 };
        Ok(Value::smi(len))
    } else {
        Err("len() receiver must be a list".to_string())
    }
}

/// list.remove(index: int) -> any
///
/// 移除并返回指定索引处的元素
fn list_remove(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "remove() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let idx = args[0]
        .as_int()
        .ok_or("remove() argument must be an integer")? as usize;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &mut *ptr };
        if idx >= list.len() {
            return Err(format!(
                "remove() index out of bounds: {} (length {})",
                idx,
                list.len()
            ));
        }
        Ok(list.elements.remove(idx))
    } else {
        Err("remove() receiver must be a list".to_string())
    }
}

/// list.clear() -> list
///
/// 清空列表所有元素，返回 receiver 支持链式调用
fn list_clear(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "clear() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &mut *ptr };
        list.elements.clear();
        Ok(receiver)
    } else {
        Err("clear() receiver must be a list".to_string())
    }
}

/// list.is_empty() -> bool
///
/// 检查列表是否为空
fn list_is_empty(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "is_empty() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_list() {
        let is_empty = unsafe { (*ptr).is_empty() };
        Ok(Value::bool_from(is_empty))
    } else {
        Err("is_empty() receiver must be a list".to_string())
    }
}

// ==================== List 函数式方法实现 ====================

/// list.foreach(callback: |item| -> void) -> list
///
/// 遍历列表，对每个元素调用回调函数，返回 receiver 支持链式调用
fn list_foreach(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "foreach() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("foreach() argument must be a function")?;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        for elem in &list.elements {
            call_operator_closure(vm, closure, &[*elem])?;
        }
        Ok(receiver)
    } else {
        Err("foreach() receiver must be a list".to_string())
    }
}

/// list.map(callback: |item| -> any) -> List
///
/// 遍历列表，对每个元素调用回调函数，返回新列表
fn list_map(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "map() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("map() argument must be a function")?;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        let mut results = Vec::with_capacity(list.len());
        for elem in &list.elements {
            let result = call_operator_closure(vm, closure, &[*elem])?;
            results.push(result);
        }
        // 创建新列表
        let new_list = crate::vm::core::object::ObjList::from_vec(results);
        let new_ptr = Box::into_raw(Box::new(new_list));
        Ok(Value::list(new_ptr))
    } else {
        Err("map() receiver must be a list".to_string())
    }
}

/// list.filter(predicate: |item| -> bool) -> List
///
/// 遍历列表，返回满足条件的元素组成的新列表
fn list_filter(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "filter() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("filter() argument must be a function")?;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        let mut results = Vec::new();
        for elem in &list.elements {
            let keep = call_operator_closure(vm, closure, &[*elem])?;
            if keep.is_truthy() {
                results.push(*elem);
            }
        }
        // 创建新列表
        let new_list = crate::vm::core::object::ObjList::from_vec(results);
        let new_ptr = Box::into_raw(Box::new(new_list));
        Ok(Value::list(new_ptr))
    } else {
        Err("filter() receiver must be a list".to_string())
    }
}

/// list.reduce(callback: |acc, item| -> any, initial: any) -> any
///
/// 遍历列表，累积计算单一结果
fn list_reduce(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "reduce() takes exactly 2 arguments ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("reduce() first argument must be a function")?;
    let mut acc = args[1];

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        for elem in &list.elements {
            acc = call_operator_closure(vm, closure, &[acc, *elem])?;
        }
        Ok(acc)
    } else {
        Err("reduce() receiver must be a list".to_string())
    }
}

/// list.find(predicate: |item| -> bool) -> any|null
///
/// 返回第一个满足条件的元素，未找到返回 null
fn list_find(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "find() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("find() argument must be a function")?;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        for elem in &list.elements {
            let found = call_operator_closure(vm, closure, &[*elem])?;
            if found.is_truthy() {
                return Ok(*elem);
            }
        }
        Ok(Value::NULL)
    } else {
        Err("find() receiver must be a list".to_string())
    }
}

/// list.any(predicate: |item| -> bool) -> bool
///
/// 检查是否有任意元素满足条件
fn list_any(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "any() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("any() argument must be a function")?;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        for elem in &list.elements {
            let matched = call_operator_closure(vm, closure, &[*elem])?;
            if matched.is_truthy() {
                return Ok(Value::TRUE);
            }
        }
        Ok(Value::FALSE)
    } else {
        Err("any() receiver must be a list".to_string())
    }
}

/// list.all(predicate: |item| -> bool) -> bool
///
/// 检查是否所有元素都满足条件
fn list_all(vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "all() takes exactly 1 argument ({} given)",
            args.len()
        ));
    }

    let closure = args[0]
        .as_closure()
        .ok_or("all() argument must be a function")?;

    if let Some(ptr) = receiver.as_list() {
        let list = unsafe { &*ptr };
        for elem in &list.elements {
            let matched = call_operator_closure(vm, closure, &[*elem])?;
            if !matched.is_truthy() {
                return Ok(Value::FALSE);
            }
        }
        Ok(Value::TRUE)
    } else {
        Err("all() receiver must be a list".to_string())
    }
}

// ==================== String 方法实现 ====================

/// string.len() -> int
fn string_len(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "len() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_string() {
        let len = unsafe { (*ptr).chars.len() as i32 };
        Ok(Value::smi(len))
    } else {
        Err("len() receiver must be a string".to_string())
    }
}

/// string.is_empty() -> bool
fn string_is_empty(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "is_empty() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_string() {
        let is_empty = unsafe { (*ptr).chars.is_empty() };
        Ok(Value::bool_from(is_empty))
    } else {
        Err("is_empty() receiver must be a string".to_string())
    }
}

// ==================== JSON 方法实现 ====================

/// json.len() -> int
fn json_len(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "len() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_json() {
        let len = unsafe { (*ptr).len() as i32 };
        Ok(Value::smi(len))
    } else {
        Err("len() receiver must be a json".to_string())
    }
}

/// json.is_empty() -> bool
fn json_is_empty(_vm: &mut VM, receiver: Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "is_empty() takes exactly 0 arguments ({} given)",
            args.len()
        ));
    }

    if let Some(ptr) = receiver.as_json() {
        let is_empty = unsafe { (*ptr).is_empty() };
        Ok(Value::bool_from(is_empty))
    } else {
        Err("is_empty() receiver must be a json".to_string())
    }
}

// ==================== 方法表 ====================

/// 内置类型方法表
///
/// 管理所有内置类型的方法注册和查找
pub struct BuiltinMethodTable;

impl BuiltinMethodTable {
    /// 创建新的方法表
    pub fn new() -> Self {
        Self
    }

    /// 获取 List 方法表
    pub fn list_methods(&self) -> &'static [BuiltinMethodFn; list_methods::COUNT] {
        &LIST_METHOD_TABLE
    }

    /// 获取 String 方法表
    pub fn string_methods(&self) -> &'static [BuiltinMethodFn; string_methods::COUNT] {
        &STRING_METHOD_TABLE
    }

    /// 获取 JSON 方法表
    pub fn json_methods(&self) -> &'static [BuiltinMethodFn; json_methods::COUNT] {
        &JSON_METHOD_TABLE
    }

    /// 根据类型标签和方法索引查找方法
    ///
    /// # 参数
    /// - type_tag: 类型标签（LIST=0, STRING=1, JSON=2）
    /// - method_idx: 方法索引
    ///
    /// # 返回
    /// - Some(BuiltinMethodFn): 找到的方法
    /// - None: 未找到方法
    pub fn find_method(&self, type_tag: u8, method_idx: u8) -> Option<BuiltinMethodFn> {
        match type_tag {
            builtin_types::LIST => {
                LIST_METHOD_TABLE.get(method_idx as usize).copied()
            }
            builtin_types::STRING => {
                STRING_METHOD_TABLE.get(method_idx as usize).copied()
            }
            builtin_types::JSON => {
                JSON_METHOD_TABLE.get(method_idx as usize).copied()
            }
            _ => None,
        }
    }

    /// 编译期方法名解析：List
    pub fn resolve_list_method(name: &str) -> Option<u8> {
        match name {
            "push" => Some(list_methods::PUSH),
            "len" => Some(list_methods::LEN),
            "remove" => Some(list_methods::REMOVE),
            "clear" => Some(list_methods::CLEAR),
            "is_empty" => Some(list_methods::IS_EMPTY),
            "foreach" => Some(list_methods::FOREACH),
            "map" => Some(list_methods::MAP),
            "filter" => Some(list_methods::FILTER),
            "reduce" => Some(list_methods::REDUCE),
            "find" => Some(list_methods::FIND),
            "any" => Some(list_methods::ANY),
            "all" => Some(list_methods::ALL),
            _ => None,
        }
    }

    /// 编译期方法名解析：String
    pub fn resolve_string_method(name: &str) -> Option<u8> {
        match name {
            "len" => Some(string_methods::LEN),
            "is_empty" => Some(string_methods::IS_EMPTY),
            _ => None,
        }
    }

    /// 编译期方法名解析：JSON
    pub fn resolve_json_method(name: &str) -> Option<u8> {
        match name {
            "len" => Some(json_methods::LEN),
            "is_empty" => Some(json_methods::IS_EMPTY),
            _ => None,
        }
    }
}

impl Default for BuiltinMethodTable {
    fn default() -> Self {
        Self::new()
    }
}

/// 静态 List 方法表
static LIST_METHOD_TABLE: [BuiltinMethodFn; list_methods::COUNT] = [
    list_push,      // idx 0
    list_len,       // idx 1
    list_remove,    // idx 2
    list_clear,     // idx 3
    list_is_empty,  // idx 4
    list_foreach,   // idx 5
    list_map,       // idx 6
    list_filter,    // idx 7
    list_reduce,    // idx 8
    list_find,      // idx 9
    list_any,       // idx 10
    list_all,       // idx 11
];

/// 静态 String 方法表
static STRING_METHOD_TABLE: [BuiltinMethodFn; string_methods::COUNT] = [
    string_len,       // idx 0
    string_is_empty,  // idx 1
];

/// 静态 JSON 方法表
static JSON_METHOD_TABLE: [BuiltinMethodFn; json_methods::COUNT] = [
    json_len,       // idx 0
    json_is_empty,  // idx 1
];

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::core::object::ObjList;
    use crate::vm::core::VM;

    #[test]
    fn test_encode_decode() {
        // List.push
        let encoded = encode_method(builtin_types::LIST, list_methods::PUSH);
        assert_eq!(encoded, 0x0100);
        
        let decoded = decode_method(Value::smi(encoded)).unwrap();
        assert_eq!(decoded, (builtin_types::LIST, list_methods::PUSH));

        // String.len
        let encoded = encode_method(builtin_types::STRING, string_methods::LEN);
        assert_eq!(encoded, 0x0110);
        
        let decoded = decode_method(Value::smi(encoded)).unwrap();
        assert_eq!(decoded, (builtin_types::STRING, string_methods::LEN));

        // JSON.is_empty
        let encoded = encode_method(builtin_types::JSON, json_methods::IS_EMPTY);
        assert_eq!(encoded, 0x0121);
        
        let decoded = decode_method(Value::smi(encoded)).unwrap();
        assert_eq!(decoded, (builtin_types::JSON, json_methods::IS_EMPTY));
    }

    #[test]
    fn test_is_builtin_method() {
        assert!(is_builtin_method(Value::smi(0x0100)));
        assert!(is_builtin_method(Value::smi(0x0110)));
        assert!(is_builtin_method(Value::smi(0x01FF)));
        
        assert!(!is_builtin_method(Value::smi(0x00FF)));
        assert!(!is_builtin_method(Value::smi(0x0200)));
        assert!(!is_builtin_method(Value::smi(42)));
        assert!(!is_builtin_method(Value::NULL));
    }

    #[test]
    fn test_list_push() {
        let mut vm = VM::new();
        let list = Box::new(ObjList::new());
        let list_ptr = Box::into_raw(list);
        let receiver = Value::list(list_ptr);

        // push(42)
        let result = list_push(&mut vm, receiver, &[Value::smi(42)]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), receiver); // 返回 receiver

        // 验证列表长度
        let len_result = list_len(&mut vm, receiver, &[]);
        assert_eq!(len_result.unwrap().as_int(), Some(1));

        // 清理
        unsafe {
            let _ = Box::from_raw(list_ptr);
        }
    }

    #[test]
    fn test_list_methods() {
        let mut vm = VM::new();
        let list = Box::new(ObjList::from_vec(vec![
            Value::smi(1),
            Value::smi(2),
            Value::smi(3),
        ]));
        let list_ptr = Box::into_raw(list);
        let receiver = Value::list(list_ptr);

        // len
        let len = list_len(&mut vm, receiver, &[]);
        assert_eq!(len.unwrap().as_int(), Some(3));

        // is_empty
        let is_empty = list_is_empty(&mut vm, receiver, &[]);
        assert_eq!(is_empty.unwrap().as_bool(), Some(false));

        // remove(0)
        let removed = list_remove(&mut vm, receiver, &[Value::smi(0)]);
        assert_eq!(removed.unwrap().as_int(), Some(1));

        // len after remove
        let len = list_len(&mut vm, receiver, &[]);
        assert_eq!(len.unwrap().as_int(), Some(2));

        // clear
        let _ = list_clear(&mut vm, receiver, &[]);
        let len = list_len(&mut vm, receiver, &[]);
        assert_eq!(len.unwrap().as_int(), Some(0));

        // is_empty after clear
        let is_empty = list_is_empty(&mut vm, receiver, &[]);
        assert_eq!(is_empty.unwrap().as_bool(), Some(true));

        // 清理
        unsafe {
            let _ = Box::from_raw(list_ptr);
        }
    }

    #[test]
    fn test_find_method() {
        let table = BuiltinMethodTable::new();

        // List.push
        let method = table.find_method(builtin_types::LIST, list_methods::PUSH);
        assert!(method.is_some());

        // String.len
        let method = table.find_method(builtin_types::STRING, string_methods::LEN);
        assert!(method.is_some());

        // JSON.is_empty
        let method = table.find_method(builtin_types::JSON, json_methods::IS_EMPTY);
        assert!(method.is_some());

        // Invalid
        let method = table.find_method(99, 0);
        assert!(method.is_none());

        let method = table.find_method(builtin_types::LIST, 99);
        assert!(method.is_none());
    }

    #[test]
    fn test_resolve_methods() {
        // List
        assert_eq!(
            BuiltinMethodTable::resolve_list_method("push"),
            Some(list_methods::PUSH)
        );
        assert_eq!(
            BuiltinMethodTable::resolve_list_method("len"),
            Some(list_methods::LEN)
        );
        assert!(BuiltinMethodTable::resolve_list_method("nonexistent").is_none());

        // String
        assert_eq!(
            BuiltinMethodTable::resolve_string_method("len"),
            Some(string_methods::LEN)
        );
        assert!(BuiltinMethodTable::resolve_string_method("nonexistent").is_none());

        // JSON
        assert_eq!(
            BuiltinMethodTable::resolve_json_method("is_empty"),
            Some(json_methods::IS_EMPTY)
        );
        assert!(BuiltinMethodTable::resolve_json_method("nonexistent").is_none());
    }
}
