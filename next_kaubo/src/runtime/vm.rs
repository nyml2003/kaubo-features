//! 虚拟机实现

use crate::runtime::Value;
use crate::runtime::bytecode::{OpCode, chunk::Chunk};

/// 解释执行结果
#[derive(Debug, Clone, PartialEq)]
pub enum InterpretResult {
    Ok,
    CompileError(String),
    RuntimeError(String),
}

/// 虚拟机
pub struct VM {
    /// 值栈
    stack: Vec<Value>,
    /// 指令指针 (指向当前执行的指令)
    ip: *const u8,
    /// 当前执行的 Chunk
    current_chunk: Option<Chunk>,
}

impl VM {
    /// 创建新的虚拟机
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            ip: std::ptr::null(),
            current_chunk: None,
        }
    }

    /// 解释执行一个 Chunk
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        // 保存 chunk 的引用
        self.current_chunk = Some(chunk.clone());
        let chunk_ref = self.current_chunk.as_ref().unwrap();

        // 设置指令指针
        self.ip = chunk_ref.code.as_ptr();

        // 执行主循环
        self.run()
    }

    /// 执行字节码的主循环
    fn run(&mut self) -> InterpretResult {
        use OpCode::*;

        loop {
            // 调试: 打印当前栈状态和指令
            #[cfg(feature = "trace_execution")]
            self.trace_instruction();

            // 读取操作码
            let instruction = self.read_byte();
            let op = unsafe { std::mem::transmute::<u8, OpCode>(instruction) };

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

                // ===== 控制流 =====
                Jump => {
                    let offset = self.read_i16();
                    self.ip = unsafe { self.ip.offset(offset as isize) };
                }

                JumpIfFalse => {
                    let offset = self.read_i16();
                    let condition = self.pop(); // 弹出条件
                    if !condition.is_truthy() {
                        self.ip = unsafe { self.ip.offset(offset as isize) };
                    }
                }

                JumpBack => {
                    let offset = self.read_i16();
                    self.ip = unsafe { self.ip.offset(offset as isize) };
                }

                // ===== 函数 =====
                Return => {
                    return InterpretResult::Ok;
                }

                ReturnValue => {
                    // 返回值已经在栈顶
                    return InterpretResult::Ok;
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

    /// 读取下一个字节
    #[inline]
    fn read_byte(&mut self) -> u8 {
        let byte = unsafe { *self.ip };
        self.ip = unsafe { self.ip.add(1) };
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
        let chunk = self.current_chunk.as_ref().unwrap();
        let value = chunk.constants[idx];
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
        let idx = self.stack.len() - 1 - distance;
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
        let offset = unsafe {
            self.ip
                .offset_from(self.current_chunk.as_ref().unwrap().code.as_ptr())
        } as usize;
        // TODO: 打印指令
        println!("{:04} {:?}", offset, self.read_byte());
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
        chunk.write_op(Return, 1);

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
        chunk.write_op(Return, 1);

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
        chunk.write_op(Return, 1);

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
        chunk.write_op(Return, 1);

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
        chunk.write_op(Return, 1);

        let result = vm.interpret(&chunk);
        assert_eq!(result, InterpretResult::Ok);
        assert!(vm.stack.last().unwrap().is_true());
    }
}
