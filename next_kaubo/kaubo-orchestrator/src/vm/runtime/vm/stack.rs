//! 栈操作

use crate::vm::core::{Value, VM};

/// 压栈
#[inline]
pub fn push(vm: &mut VM, value: Value) {
    vm.stack.push(value);
}

/// 弹栈
#[inline]
pub fn pop(vm: &mut VM) -> Value {
    vm.stack.pop().expect("Stack underflow")
}

/// 弹出两个值 (先弹出的是右操作数)
#[inline]
pub fn pop_two(vm: &mut VM) -> (Value, Value) {
    let b = pop(vm);
    let a = pop(vm);
    (a, b)
}

/// 查看栈顶元素 (distance=0 是栈顶)
#[inline]
pub fn peek(vm: &VM, distance: usize) -> Value {
    let len = vm.stack.len();
    if len == 0 || distance >= len {
        panic!("Stack underflow in peek");
    }
    let idx = len - 1 - distance;
    vm.stack[idx]
}

/// 获取栈顶值（用于测试和获取结果）
pub fn stack_top(vm: &VM) -> Option<Value> {
    vm.stack.last().copied()
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::core::VM;

    #[test]
    fn test_stack_operations() {
        let mut vm = VM::new();

        // 测试 push 和 pop
        push(&mut vm, Value::smi(1));
        push(&mut vm, Value::smi(2));
        push(&mut vm, Value::smi(3));

        assert_eq!(vm.stack.len(), 3);
        assert_eq!(peek(&vm, 0).as_smi(), Some(3)); // 栈顶
        assert_eq!(peek(&vm, 1).as_smi(), Some(2));
        assert_eq!(peek(&vm, 2).as_smi(), Some(1));

        assert_eq!(pop(&mut vm).as_smi(), Some(3));
        assert_eq!(pop(&mut vm).as_smi(), Some(2));
        assert_eq!(pop(&mut vm).as_smi(), Some(1));
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn test_pop_two() {
        let mut vm = VM::new();

        push(&mut vm, Value::smi(10));
        push(&mut vm, Value::smi(20));

        let (a, b) = pop_two(&mut vm);
        assert_eq!(a.as_smi(), Some(10));
        assert_eq!(b.as_smi(), Some(20));
    }

    #[test]
    fn test_stack_top() {
        let mut vm = VM::new();

        assert!(stack_top(&vm).is_none());

        push(&mut vm, Value::smi(42));
        assert_eq!(stack_top(&vm).unwrap().as_smi(), Some(42));

        push(&mut vm, Value::smi(100));
        assert_eq!(stack_top(&vm).unwrap().as_smi(), Some(100));
    }
}
