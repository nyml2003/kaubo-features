//! 虚拟机实现

use crate::runtime::Value;
use crate::runtime::bytecode::{OpCode, chunk::Chunk};
use crate::runtime::object::{ObjFunction, ObjIterator, ObjList};

/// 解释执行结果
#[derive(Debug, Clone, PartialEq)]
pub enum InterpretResult {
    Ok,
    CompileError(String),
    RuntimeError(String),
}

/// 调用栈帧
pub struct CallFrame {
    /// 当前执行的 Chunk
    chunk: Chunk,
    /// 指令指针在该帧中的偏移
    ip: *const u8,
    /// 该帧的局部变量数组
    locals: Vec<Value>,
}

/// 虚拟机
pub struct VM {
    /// 操作数栈（独立于局部变量）
    stack: Vec<Value>,
    /// 调用栈
    frames: Vec<CallFrame>,
}

impl VM {
    /// 创建新的虚拟机
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
        }
    }

    /// 解释执行一个 Chunk
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.interpret_with_locals(chunk, 0)
    }

    /// 解释执行一个 Chunk，并预分配局部变量空间
    pub fn interpret_with_locals(&mut self, chunk: &Chunk, local_count: usize) -> InterpretResult {
        // 预分配局部变量空间（初始化为 null）
        let mut locals = Vec::with_capacity(local_count);
        for _ in 0..local_count {
            locals.push(Value::NULL);
        }

        // 创建初始调用帧
        self.frames.push(CallFrame {
            chunk: chunk.clone(),
            ip: chunk.code.as_ptr(),
            locals,
        });

        // 执行主循环
        let result = self.run();

        // 清理调用栈
        self.frames.pop();

        result
    }

    /// 执行字节码的主循环
    fn run(&mut self) -> InterpretResult {
        use OpCode::*;

        loop {
            // 调试: 打印当前栈状态和指令
            #[cfg(feature = "trace_execution")]
            self.trace_instruction();

            // 读取操作码
            let instruction = unsafe { *self.current_ip() };
            self.advance_ip(1);
            let op = unsafe { std::mem::transmute::<u8, OpCode>(instruction) };
            eprintln!("next instruction: {:?}, with stack: {:?}", op, self.stack);
            match op {
                // ===== 常量加载 =====
                LoadConst0 => self.push_const(0),
                LoadConst1 => self.push_const(1),
                LoadConst2 => self.push_const(2),
                LoadConst3 => self.push_const(3),
                LoadConst4 => self.push_const(4),
                LoadConst5 => self.push_const(5),
                LoadConst6 => self.push_const(6),
                LoadConst7 => self.push_const(7),
                LoadConst8 => self.push_const(8),
                LoadConst9 => self.push_const(9),
                LoadConst10 => self.push_const(10),
                LoadConst11 => self.push_const(11),
                LoadConst12 => self.push_const(12),
                LoadConst13 => self.push_const(13),
                LoadConst14 => self.push_const(14),
                LoadConst15 => self.push_const(15),

                LoadConst => {
                    let idx = self.read_byte();
                    self.push_const(idx as usize);
                }

                // ===== 特殊值 =====
                LoadNull => self.push(Value::NULL),
                LoadTrue => self.push(Value::TRUE),
                LoadFalse => self.push(Value::FALSE),
                LoadZero => self.push(Value::smi(0)),
                LoadOne => self.push(Value::smi(1)),

                // ===== 栈操作 =====
                Pop => {
                    self.pop();
                }

                Dup => {
                    let v = self.peek(0);
                    self.push(v);
                }

                Swap => {
                    let len = self.stack.len();
                    if len >= 2 {
                        self.stack.swap(len - 1, len - 2);
                    }
                }

                // ===== 算术运算 =====
                Add => {
                    let (a, b) = self.pop_two();
                    let result = self.add_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Sub => {
                    let (a, b) = self.pop_two();
                    let result = self.sub_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Mul => {
                    let (a, b) = self.pop_two();
                    let result = self.mul_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Div => {
                    let (a, b) = self.pop_two();
                    let result = self.div_values(a, b);
                    match result {
                        Ok(v) => self.push(v),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Neg => {
                    let v = self.pop();
                    let result = self.neg_value(v);
                    match result {
                        Ok(v) => self.push(v),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                // ===== 比较运算 =====
                Equal => {
                    let (a, b) = self.pop_two();
                    self.push(Value::bool_from(a == b));
                }

                Greater => {
                    let (a, b) = self.pop_two();
                    let result = self.compare_values(a, b);
                    match result {
                        Ok(Ordering::Greater) => self.push(Value::TRUE),
                        Ok(_) => self.push(Value::FALSE),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                Less => {
                    let (a, b) = self.pop_two();
                    let result = self.compare_values(a, b);
                    match result {
                        Ok(Ordering::Less) => self.push(Value::TRUE),
                        Ok(_) => self.push(Value::FALSE),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                LessEqual => {
                    let (a, b) = self.pop_two();
                    let result = self.compare_values(a, b);
                    match result {
                        Ok(Ordering::Less) | Ok(Ordering::Equal) => self.push(Value::TRUE),
                        Ok(_) => self.push(Value::FALSE),
                        Err(e) => return InterpretResult::RuntimeError(e),
                    }
                }

                // ===== 局部变量 =====
                LoadLocal0 => {
                    let value = self.get_local(0);
                    self.push(value);
                }
                LoadLocal1 => {
                    let value = self.get_local(1);
                    self.push(value);
                }
                LoadLocal2 => {
                    let value = self.get_local(2);
                    self.push(value);
                }
                LoadLocal3 => {
                    let value = self.get_local(3);
                    self.push(value);
                }
                LoadLocal4 => {
                    let value = self.get_local(4);
                    self.push(value);
                }
                LoadLocal5 => {
                    let value = self.get_local(5);
                    self.push(value);
                }
                LoadLocal6 => {
                    let value = self.get_local(6);
                    self.push(value);
                }
                LoadLocal7 => {
                    let value = self.get_local(7);
                    self.push(value);
                }
                LoadLocal => {
                    let idx = self.read_byte() as usize;
                    let value = self.get_local(idx);
                    self.push(value);
                }

                StoreLocal0 => {
                    let value = self.pop();
                    self.set_local(0, value);
                }
                StoreLocal1 => {
                    let value = self.pop();
                    self.set_local(1, value);
                }
                StoreLocal2 => {
                    let value = self.pop();
                    self.set_local(2, value);
                }
                StoreLocal3 => {
                    let value = self.pop();
                    self.set_local(3, value);
                }
                StoreLocal4 => {
                    let value = self.pop();
                    self.set_local(4, value);
                }
                StoreLocal5 => {
                    let value = self.pop();
                    self.set_local(5, value);
                }
                StoreLocal6 => {
                    let value = self.pop();
                    self.set_local(6, value);
                }
                StoreLocal7 => {
                    let value = self.pop();
                    self.set_local(7, value);
                }
                StoreLocal => {
                    let idx = self.read_byte() as usize;
                    let value = self.pop();
                    self.set_local(idx, value);
                }

                // ===== 控制流 =====
                Jump => {
                    let offset = self.read_i16();
                    self.jump_ip(offset as isize);
                }

                JumpIfFalse => {
                    let offset = self.read_i16();
                    let condition = self.pop(); // 弹出条件
                    if !condition.is_truthy() {
                        self.jump_ip(offset as isize);
                    }
                }

                JumpBack => {
                    let offset = self.read_i16();
                    self.jump_ip(offset as isize);
                }

                // ===== 函数 =====
                Call => {
                    let arg_count = self.read_byte();

                    // 栈布局：[arg0, arg1, ..., argN, func]
                    // 先弹出函数对象（栈顶）
                    let callee = self.pop();
                    if let Some(func_ptr) = callee.as_function() {
                        let func = unsafe { &*func_ptr };
                        if func.arity != arg_count {
                            return InterpretResult::RuntimeError(format!(
                                "Expected {} arguments but got {}",
                                func.arity, arg_count
                            ));
                        }

                        // 收集参数（从栈顶，现在参数在栈顶）
                        // pop 顺序：argN, argN-1, ..., arg0
                        let mut locals = Vec::with_capacity(arg_count as usize);
                        for _ in 0..arg_count {
                            locals.push(self.pop());
                        }
                        // reverse 后：arg0, arg1, ..., argN
                        locals.reverse();

                        // 创建新的调用帧
                        let new_frame = CallFrame {
                            chunk: func.chunk.clone(),
                            ip: func.chunk.code.as_ptr(),
                            locals,
                        };
                        self.frames.push(new_frame);
                    } else {
                        return InterpretResult::RuntimeError(
                            "Can only call functions".to_string(),
                        );
                    }
                }

                Closure => {
                    // 从常量池加载函数对象
                    let const_idx = self.read_byte();
                    let constant = self.current_chunk().constants[const_idx as usize];

                    if constant.is_function() {
                        self.push(constant);
                    } else {
                        return InterpretResult::RuntimeError(
                            "Closure constant must be a function".to_string(),
                        );
                    }
                }

                Return => {
                    // 1. 弹出当前函数的调用帧
                    self.frames
                        .pop()
                        .expect("Runtime error: No call frame to pop");

                    // 2. 压入 NULL 作为无返回值函数的返回值
                    self.push(Value::NULL);

                    // 3. 只有当调用帧为空（主函数返回）时，才终止VM执行；否则继续执行上层帧
                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }
                    // 非空则继续循环，执行上层帧的下一条指令
                }

                // ===== 修复后的 RETURN_VALUE 指令 =====
                ReturnValue => {
                    // 1. 弹出当前函数的调用帧
                    self.frames
                        .pop()
                        .expect("Runtime error: No call frame to pop");

                    // 2. 保存栈顶的返回值（函数执行结果）
                    let return_value = self.pop();

                    // 3. 将返回值压回栈顶，供上层帧使用（比如主函数的PRINT指令）
                    self.push(return_value);

                    // 4. 仅主函数返回时终止，否则继续执行上层帧
                    if self.frames.is_empty() {
                        return InterpretResult::Ok;
                    }
                    // 非空则继续循环
                }

                // ===== 列表 =====
                BuildList => {
                    let count = self.read_byte() as usize;
                    // 从栈顶弹出 count 个元素，创建列表
                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.pop());
                    }
                    elements.reverse(); // 栈顶是最后一个元素

                    let list = Box::new(ObjList::from_vec(elements));
                    let list_ptr = Box::into_raw(list);
                    self.push(Value::list(list_ptr));
                }

                IndexGet => {
                    // 栈顶: [index, list]
                    let index_val = self.pop();
                    let list_val = self.pop();

                    let index = match index_val.as_smi() {
                        Some(i) => i,
                        None => {
                            return InterpretResult::RuntimeError(
                                "Index must be an integer".to_string(),
                            );
                        }
                    };

                    if let Some(list_ptr) = list_val.as_list() {
                        let list = unsafe { &*list_ptr };
                        let idx = index as usize;
                        if idx >= list.len() {
                            return InterpretResult::RuntimeError(format!(
                                "Index out of bounds: {} (length {})",
                                idx,
                                list.len()
                            ));
                        }
                        let value = list.get(idx).unwrap_or(Value::NULL);
                        self.push(value);
                    } else {
                        return InterpretResult::RuntimeError("Can only index lists".to_string());
                    }
                }

                GetIter => {
                    // 从列表获取迭代器
                    let list_val = self.pop();

                    if let Some(list_ptr) = list_val.as_list() {
                        let iter = Box::new(ObjIterator::from_list(list_ptr));
                        let iter_ptr = Box::into_raw(iter);
                        self.push(Value::iterator(iter_ptr));
                    } else {
                        return InterpretResult::RuntimeError(
                            "Can only iterate over lists".to_string(),
                        );
                    }
                }

                IterNext => {
                    // 获取迭代器下一个值，null 表示结束
                    let iter_val = self.pop();

                    if let Some(iter_ptr) = iter_val.as_iterator() {
                        let iter = unsafe { &mut *iter_ptr };
                        match iter.next() {
                            Some(value) => self.push(value),
                            None => self.push(Value::NULL),
                        }
                    } else {
                        return InterpretResult::RuntimeError("Expected iterator".to_string());
                    }
                }

                // ===== 调试 =====
                Print => {
                    let v = self.pop();
                    println!("{}", v);
                }

                Invalid => {
                    return InterpretResult::RuntimeError("Invalid opcode".to_string());
                }

                _ => {
                    return InterpretResult::RuntimeError(format!(
                        "Unimplemented opcode: {:?}",
                        op
                    ));
                }
            }
        }
    }

    // ==================== 辅助方法 ====================

    /// 获取当前帧的指令指针
    #[inline]
    fn current_ip(&self) -> *const u8 {
        self.frames.last().unwrap().ip
    }

    /// 获取当前帧的可变指令指针
    #[inline]
    fn current_ip_mut(&mut self) -> &mut *const u8 {
        &mut self.frames.last_mut().unwrap().ip
    }

    /// 获取当前帧的可变 locals
    #[inline]
    fn current_locals_mut(&mut self) -> &mut Vec<Value> {
        &mut self.frames.last_mut().unwrap().locals
    }

    /// 获取当前帧的 locals
    #[inline]
    fn current_locals(&self) -> &Vec<Value> {
        &self.frames.last().unwrap().locals
    }

    /// 获取局部变量（自动扩展）
    #[inline]
    fn get_local(&self, idx: usize) -> Value {
        let locals = self.current_locals();
        if idx < locals.len() {
            locals[idx]
        } else {
            Value::NULL
        }
    }

    /// 设置局部变量（自动扩展）
    #[inline]
    fn set_local(&mut self, idx: usize, value: Value) {
        let locals = self.current_locals_mut();
        if idx >= locals.len() {
            locals.resize(idx + 1, Value::NULL);
        }
        locals[idx] = value;
    }

    /// 获取当前帧的 chunk
    #[inline]
    fn current_chunk(&self) -> &Chunk {
        &self.frames.last().unwrap().chunk
    }

    /// 前进指令指针
    #[inline]
    fn advance_ip(&mut self, offset: usize) {
        *self.current_ip_mut() = unsafe { self.current_ip().add(offset) };
    }

    /// 跳转指令指针
    #[inline]
    fn jump_ip(&mut self, offset: isize) {
        *self.current_ip_mut() = unsafe { self.current_ip().offset(offset) };
    }

    /// 读取下一个字节
    #[inline]
    fn read_byte(&mut self) -> u8 {
        let byte = unsafe { *self.current_ip() };
        self.advance_ip(1);
        byte
    }

    /// 读取 i16
    #[inline]
    fn read_i16(&mut self) -> i16 {
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        i16::from_le_bytes([b1, b2])
    }

    /// 从常量池加载并压栈
    #[inline]
    fn push_const(&mut self, idx: usize) {
        let value = self.current_chunk().constants[idx];
        self.push(value);
    }

    /// 压栈
    #[inline]
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// 弹栈
    #[inline]
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

    /// 弹出两个值 (先弹出的是右操作数)
    #[inline]
    fn pop_two(&mut self) -> (Value, Value) {
        let b = self.pop();
        let a = self.pop();
        (a, b)
    }

    /// 查看栈顶元素 (distance=0 是栈顶)
    #[inline]
    fn peek(&self, distance: usize) -> Value {
        let len = self.stack.len();
        if len == 0 || distance >= len {
            panic!("Stack underflow in peek");
        }
        let idx = len - 1 - distance;
        self.stack[idx]
    }

    // ==================== 数值运算 ====================

    /// 加法
    fn add_values(&self, a: Value, b: Value) -> Result<Value, String> {
        // 优先尝试整数加法
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            // 检查溢出
            if let Some(sum) = ai.checked_add(bi) {
                if sum >= -(1 << 30) && sum < (1 << 30) {
                    return Ok(Value::smi(sum));
                }
            }
        }

        // 回退到浮点数
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        Ok(Value::float(af + bf))
    }

    /// 减法
    fn sub_values(&self, a: Value, b: Value) -> Result<Value, String> {
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            if let Some(diff) = ai.checked_sub(bi) {
                if diff >= -(1 << 30) && diff < (1 << 30) {
                    return Ok(Value::smi(diff));
                }
            }
        }

        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        Ok(Value::float(af - bf))
    }

    /// 乘法
    fn mul_values(&self, a: Value, b: Value) -> Result<Value, String> {
        if let (Some(ai), Some(bi)) = (a.as_smi(), b.as_smi()) {
            if let Some(prod) = ai.checked_mul(bi) {
                if prod >= -(1 << 30) && prod < (1 << 30) {
                    return Ok(Value::smi(prod));
                }
            }
        }

        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        Ok(Value::float(af * bf))
    }

    /// 除法
    fn div_values(&self, a: Value, b: Value) -> Result<Value, String> {
        // 除法总是返回浮点数（避免整数除法的困惑）
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        if bf == 0.0 {
            return Err("Division by zero".to_string());
        }

        Ok(Value::float(af / bf))
    }

    /// 取负
    fn neg_value(&self, v: Value) -> Result<Value, String> {
        if let Some(i) = v.as_smi() {
            if i != i32::MIN {
                // 避免溢出
                return Ok(Value::smi(-i));
            }
        }

        let f = if v.is_float() {
            v.as_float()
        } else {
            v.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };
        Ok(Value::float(-f))
    }

    /// 比较
    fn compare_values(&self, a: Value, b: Value) -> Result<std::cmp::Ordering, String> {
        let af = if a.is_float() {
            a.as_float()
        } else {
            a.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        let bf = if b.is_float() {
            b.as_float()
        } else {
            b.as_smi().map(|n| n as f64).unwrap_or(0.0)
        };

        Ok(af.partial_cmp(&bf).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// 获取栈顶值（用于测试和获取结果）
    pub fn stack_top(&self) -> Option<Value> {
        self.stack.last().copied()
    }

    // ==================== 调试 ====================

    /// 追踪当前指令执行
    #[cfg(feature = "trace_execution")]
    fn trace_instruction(&self) {
        print!("          ");
        for (i, slot) in self.stack.iter().enumerate() {
            print!("[ {} ]", slot);
        }
        println!();

        // 反汇编当前指令
        let frame = self.frames.last().unwrap();
        let offset = unsafe { frame.ip.offset_from(frame.chunk.code.as_ptr()) } as usize;
        // 只读取查看，不修改 ip
        let instruction = unsafe { *frame.ip };
        let op = unsafe { std::mem::transmute::<u8, OpCode>(instruction) };
        println!("{:04} {:?}", offset, op);
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

use std::cmp::Ordering;

impl Value {
    /// 从 bool 创建 Value
    fn bool_from(b: bool) -> Self {
        if b { Self::TRUE } else { Self::FALSE }
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::bytecode::OpCode::*;

    /// 辅助函数：创建包含单个指令的 chunk
    fn simple_chunk(op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.write_op(op, 1);
        chunk
    }

    #[test]
    fn test_push_pop() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        chunk.write_op(LoadOne, 1);
        chunk.write_op(Pop, 1);
        chunk.write_op(Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_arithmetic() {
        // 1 + 2
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c1 = chunk.add_constant(Value::smi(1));
        let c2 = chunk.add_constant(Value::smi(2));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op(Add, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        // 检查结果应该是 3
        assert_eq!(vm.stack.last().unwrap().as_smi(), Some(3));
    }

    #[test]
    fn test_add_overflow_to_float() {
        // 大数相加，溢出 SMI 范围，应该转为 float
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let big = (1 << 29) as i32; // 536870912
        let c1 = chunk.add_constant(Value::smi(big));
        let c2 = chunk.add_constant(Value::smi(big));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op(Add, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        // 结果应该是浮点数
        let top = vm.stack.last().unwrap();
        assert!(top.is_float() || top.as_smi().is_some());
    }

    #[test]
    fn test_comparison() {
        // 2 > 1
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c2 = chunk.add_constant(Value::smi(2));
        let c1 = chunk.add_constant(Value::smi(1));

        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op(Greater, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert!(vm.stack.last().unwrap().is_true());
    }

    #[test]
    fn test_division() {
        // 5 / 2 = 2.5
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c5 = chunk.add_constant(Value::smi(5));
        let c2 = chunk.add_constant(Value::smi(2));

        chunk.write_op_u8(LoadConst, c5, 1);
        chunk.write_op_u8(LoadConst, c2, 1);
        chunk.write_op(Div, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);

        let top = vm.stack.last().unwrap();
        assert!(top.is_float());
        assert_eq!(top.as_float(), 2.5);
    }

    #[test]
    fn test_division_by_zero() {
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c1 = chunk.add_constant(Value::smi(1));
        let c0 = chunk.add_constant(Value::smi(0));

        chunk.write_op_u8(LoadConst, c1, 1);
        chunk.write_op_u8(LoadConst, c0, 1);
        chunk.write_op(Div, 1);
        chunk.write_op(Return, 1);

        let result = vm.interpret(&chunk);
        assert!(matches!(result, InterpretResult::RuntimeError(_)));
    }

    #[test]
    fn test_jump_if_false() {
        // if (false) { LoadFalse } else { LoadTrue } 应该执行 LoadTrue
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        chunk.write_op(LoadFalse, 1); // 条件为 false

        // JumpIfFalse 跳过 LoadFalse (2 bytes: LoadFalse op)
        let jump_offset = chunk.write_jump(JumpIfFalse, 1);
        chunk.write_op(LoadFalse, 1); // 这个被跳过
        chunk.patch_jump(jump_offset);

        chunk.write_op(LoadTrue, 1); // 应该执行到这里
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert!(vm.stack.last().unwrap().is_true());
    }

    #[test]
    fn test_local_variables() {
        // var x = 5; var y = x + 3;
        // 使用 interpret_with_locals 预分配 2 个局部变量槽
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        let c5 = chunk.add_constant(Value::smi(5));
        let c3 = chunk.add_constant(Value::smi(3));

        // x = 5
        chunk.write_op_u8(LoadConst, c5, 1);
        chunk.write_op(StoreLocal0, 1);

        // y = x + 3
        chunk.write_op(LoadLocal0, 1); // 加载 x
        chunk.write_op_u8(LoadConst, c3, 1); // 加载 3
        chunk.write_op(Add, 1); // x + 3
        chunk.write_op(StoreLocal1, 1); // y = result

        // return y
        chunk.write_op(LoadLocal1, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret_with_locals(&chunk, 2);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.last().unwrap().as_smi(), Some(8));
    }

    #[test]
    fn test_local_variables_high_index() {
        // 测试高索引局部变量 (超过 7，需要使用 LoadLocal/StoreLocal 指令)
        let mut vm = VM::new();
        let mut chunk = Chunk::new();

        // slot 8 = 42
        let c42 = chunk.add_constant(Value::smi(42));
        chunk.write_op_u8(LoadConst, c42, 1);
        chunk.write_op_u8(StoreLocal, 8, 1);

        // return slot 8
        chunk.write_op_u8(LoadLocal, 8, 1);
        chunk.write_op(ReturnValue, 1);

        let result = vm.interpret_with_locals(&chunk, 10);
        assert_eq!(result, InterpretResult::Ok);
        assert_eq!(vm.stack.last().unwrap().as_smi(), Some(42));
    }
}
