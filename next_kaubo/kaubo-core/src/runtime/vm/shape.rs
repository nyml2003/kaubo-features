//! Shape 注册和查找

use crate::core::{ObjFunction, ObjShape, OperatorTableEntry, VM};

/// 注册 Shape 到 VM
///
/// # Safety
/// `shape` 必须是有效的、非空的指向 `ObjShape` 的指针
pub unsafe fn register_shape(vm: &mut VM, shape: *const ObjShape) {
    let shape_id = (*shape).shape_id;
    vm.shapes.insert(shape_id, shape);
}

/// 通过 ID 获取 Shape
pub fn get_shape(vm: &VM, shape_id: u16) -> *const ObjShape {
    vm.shapes
        .get(&shape_id)
        .copied()
        .unwrap_or(std::ptr::null())
}

/// 注册方法到 Shape 的方法表
pub fn register_method_to_shape(
    vm: &mut VM,
    shape_id: u16,
    method_idx: u8,
    func: *mut ObjFunction,
) {
    if let Some(&shape) = vm.shapes.get(&shape_id) {
        unsafe {
            let shape_mut = shape as *mut ObjShape;
            let methods = &mut (*shape_mut).methods;
            if method_idx as usize >= methods.len() {
                methods.resize(method_idx as usize + 1, std::ptr::null_mut());
            }
            methods[method_idx as usize] = func;
        }
    }
}

/// 从 Chunk 的 operator_table 注册运算符到 Shape
pub fn register_operators_from_chunk(
    vm: &mut VM,
    chunk: &crate::core::Chunk,
) {
    for entry in &chunk.operator_table {
        let OperatorTableEntry {
            shape_id,
            operator_name,
            const_idx,
        } = entry;

        // 获取函数值
        if let Some(function_value) = chunk.constants.get(*const_idx as usize) {
            if let Some(function_ptr) = function_value.as_function() {
                use crate::core::{ObjClosure, Operator};
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
                    if let Some(op) = Operator::from_method_name(operator_name) {
                        (*(*shape_ptr as *mut ObjShape)).register_operator(op, closure);
                    }
                }
            }
        }
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ObjShape, VM};

    #[test]
    fn test_shape_registration() {
        let mut vm = VM::new();

        // 创建一个 Shape
        let shape = Box::into_raw(Box::new(ObjShape::new(
            100,
            "TestShape".to_string(),
            vec!["field1".to_string()],
        )));

        // 注册 Shape
        unsafe {
            register_shape(&mut vm, shape);
        }

        // 验证 Shape 数量
        assert_eq!(vm.shapes.len(), 1);

        // 获取 Shape
        let retrieved = get_shape(&vm, 100);
        assert!(!retrieved.is_null());
        unsafe {
            assert_eq!((*retrieved).shape_id, 100);
            assert_eq!((*retrieved).name, "TestShape");
        }
    }

    #[test]
    fn test_get_nonexistent_shape() {
        let vm = VM::new();

        let shape = get_shape(&vm, 999);
        assert!(shape.is_null());
    }
}
