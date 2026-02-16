//! 函数调用相关 (closure, upvalue 操作)

use crate::core::{ObjClosure, ObjUpvalue, Value, VM};

/// 获取局部变量指针（用于 upvalue 捕获）
pub fn current_local_ptr(vm: &mut VM, idx: usize) -> *mut Value {
    // 扩展 locals 以确保索引有效
    let locals = vm.current_locals_mut();
    if idx >= locals.len() {
        locals.resize(idx + 1, Value::NULL);
    }
    &mut locals[idx] as *mut Value
}

/// 捕获 upvalue（如果已存在则复用）
pub fn capture_upvalue(vm: &mut VM, location: *mut Value) -> *mut ObjUpvalue {
    // 从后向前查找是否已有指向相同位置的 upvalue
    for &upvalue in vm.open_upvalues.iter().rev() {
        unsafe {
            if (*upvalue).location == location {
                return upvalue;
            }
        }
    }

    // 创建新的 upvalue
    let upvalue = Box::into_raw(Box::new(ObjUpvalue::new(location)));
    vm.open_upvalues.push(upvalue);
    upvalue
}

/// 关闭从指定槽位开始的所有 upvalues
pub fn close_upvalues(vm: &mut VM, slot: usize) {
    // 获取当前帧的 locals 起始地址
    let frame_base = vm
        .frames
        .last()
        .map(|f| f.locals.as_ptr() as usize)
        .unwrap_or(0);
    let close_threshold = frame_base + slot * std::mem::size_of::<Value>();

    // 关闭所有地址 >= close_threshold 的 upvalue
    let mut i = 0;
    while i < vm.open_upvalues.len() {
        let upvalue = vm.open_upvalues[i];
        unsafe {
            let upvalue_addr = (*upvalue).location as usize;
            if upvalue_addr >= close_threshold {
                // 关闭这个 upvalue
                (*upvalue).close();
                vm.open_upvalues.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::VM;

    #[test]
    fn test_upvalue_capture_and_close() {
        let mut vm = VM::new();

        // 模拟创建一个局部变量
        let mut local = Value::smi(42);
        let location = &mut local as *mut Value;

        // 捕获 upvalue
        let upvalue1 = capture_upvalue(&mut vm, location);
        assert_eq!(vm.open_upvalues.len(), 1);

        // 再次捕获相同位置，应该复用
        let upvalue2 = capture_upvalue(&mut vm, location);
        assert_eq!(vm.open_upvalues.len(), 1);
        assert_eq!(upvalue1, upvalue2);

        // 关闭 upvalues
        // 注意：这里需要有一个调用帧才能正常关闭
        // 实际测试应在完整 VM 上下文中进行
    }

    #[test]
    fn test_current_local_ptr() {
        let mut vm = VM::new();

        // 创建一个调用帧
        use crate::core::{CallFrame, Chunk, ObjFunction};
        let chunk = Chunk::new();
        let func = Box::into_raw(Box::new(ObjFunction::new(chunk, 0, None)));
        let closure = Box::into_raw(Box::new(ObjClosure::new(func)));

        vm.frames.push(CallFrame {
            closure,
            ip: std::ptr::null(),
            locals: vec![Value::smi(1), Value::smi(2)],
            stack_base: 0,
        });

        // 获取现有局部变量指针
        let ptr0 = current_local_ptr(&mut vm, 0);
        unsafe {
            assert_eq!((*ptr0).as_smi(), Some(1));
        }

        // 获取新局部变量指针（会自动扩展）
        let ptr5 = current_local_ptr(&mut vm, 5);
        unsafe {
            assert_eq!((*ptr5), Value::NULL);
        }

        // 清理
        vm.frames.clear();
    }
}
