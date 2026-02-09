//! 字节码块实现

use super::OpCode;
use crate::runtime::Value;

/// 字节码块
#[derive(Debug, Clone)]
pub struct Chunk {
    /// 指令字节码
    pub code: Vec<u8>,
    /// 常量池
    pub constants: Vec<Value>,
    /// 行号信息 (与 code 一一对应)
    pub lines: Vec<usize>,
}

impl Chunk {
    /// 创建新的字节码块
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// 写入单字节指令
    pub fn write_op(&mut self, op: OpCode, line: usize) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    /// 写入带 u8 操作数的指令
    pub fn write_op_u8(&mut self, op: OpCode, operand: u8, line: usize) {
        self.code.push(op as u8);
        self.code.push(operand);
        self.lines.push(line);
        self.lines.push(line);
    }

    /// 写入带 i16 操作数的指令 (跳转用)
    pub fn write_i16(&mut self, value: i16, line: usize) {
        let bytes = value.to_le_bytes();
        self.code.push(bytes[0]);
        self.code.push(bytes[1]);
        self.lines.push(line);
        self.lines.push(line);
    }

    /// 写入跳转指令 (占位，稍后 patch)
    pub fn write_jump(&mut self, op: OpCode, line: usize) -> usize {
        self.write_op(op, line);
        let offset = self.code.len();
        self.write_i16(-1i16, line); // 占位符
        offset
    }

    /// 修补跳转偏移量
    /// offset: i16 操作数的起始位置
    /// 计算从操作数之后到当前位置的偏移
    pub fn patch_jump(&mut self, offset: usize) {
        // VM 执行完跳转指令后，ip 指向操作数之后 (offset + 2)
        // 跳转偏移 = 目标位置 - (offset + 2)
        let jump = self.code.len() - (offset + 2);
        let jump_i16 = jump as i16;
        let bytes = jump_i16.to_le_bytes();

        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    /// 写入循环跳转 (负向跳转)
    pub fn write_loop(&mut self, loop_start: usize, line: usize) {
        self.write_op(OpCode::JumpBack, line);
        // 计算从当前位置回到 loop_start 的偏移 (负值)
        let offset = self.code.len() - loop_start + 2; // +2 为 i16 操作数
        let jump = -(offset as i16);
        self.write_i16(jump, line);
    }

    /// 添加常量，返回索引
    pub fn add_constant(&mut self, value: Value) -> u8 {
        let idx = self.constants.len();
        if idx > 255 {
            panic!("Too many constants in one chunk");
        }
        self.constants.push(value);
        idx as u8
    }

    /// 添加常量 (宽索引，支持更多常量)
    pub fn add_constant_wide(&mut self, value: Value) -> u16 {
        let idx = self.constants.len();
        if idx > 65535 {
            panic!("Too many constants in one chunk");
        }
        self.constants.push(value);
        idx as u16
    }

    /// 获取当前代码位置 (用于计算跳转)
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// 反汇编打印 (调试用)
    pub fn disassemble(&self, name: &str) {
        println!("== {} ==", name);
        println!("Constants:");
        for (i, constant) in self.constants.iter().enumerate() {
            println!("  [{:3}] {:?}", i, constant);
        }
        println!("\nBytecode:");

        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    /// 反汇编单条指令
    fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);

        // 打印行号
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:4} ", self.lines[offset]);
        }

        let instruction = self.code[offset];
        let opcode = OpCode::from(instruction);

        match opcode {
            // 无操作数指令
            op if op.operand_size() == 0 => {
                println!("{}", op.name());
                offset + 1
            }

            // u8 操作数
            OpCode::LoadConst => {
                let idx = self.code[offset + 1];
                println!(
                    "{} {:3} {:?}",
                    opcode.name(),
                    idx,
                    self.constants[idx as usize]
                );
                offset + 2
            }

            OpCode::Closure => {
                let idx = self.code[offset + 1];
                let constant = &self.constants[idx as usize];
                println!("{} {:3} {:?}", opcode.name(), idx, constant);
                // 如果是函数对象，反汇编函数体
                if let Some(func_ptr) = constant.as_function() {
                    let func = unsafe { &*func_ptr };
                    println!("  --- Function (arity: {}) ---", func.arity);
                    println!("Constants:");
                    for (i, constant) in func.chunk.constants.iter().enumerate() {
                        println!("  [{:3}] {:?}", i, constant);
                    }
                    println!("\nBytecode:");
                    let mut offset = 0;
                    while offset < func.chunk.code.len() {
                        offset = func.chunk.disassemble_instruction(offset);
                    }
                    println!("  --- End Function ---");
                }
                offset + 2
            }

            OpCode::LoadLocal
            | OpCode::StoreLocal
            | OpCode::LoadGlobal
            | OpCode::StoreGlobal
            | OpCode::DefineGlobal
            | OpCode::Call
            | OpCode::GetUpvalue
            | OpCode::SetUpvalue
            | OpCode::BuildList
            | OpCode::Resume
            | OpCode::CoroutineStatus => {
                let operand = self.code[offset + 1];
                println!("{} {}", opcode.name(), operand);
                offset + 2
            }
            
            OpCode::CreateCoroutine | OpCode::Yield => {
                println!("{}", opcode.name());
                offset + 1
            }

            // i16 操作数 (跳转)
            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpBack => {
                let jump = i16::from_le_bytes([self.code[offset + 1], self.code[offset + 2]]);
                let target = if jump >= 0 {
                    offset + 3 + jump as usize
                } else {
                    offset + 3 - (-jump) as usize
                };
                println!("{} {} (to {})", opcode.name(), jump, target);
                offset + 3
            }

            OpCode::LoadConstWide => {
                let idx = u16::from_le_bytes([self.code[offset + 1], self.code[offset + 2]]);
                println!(
                    "{} {:3} {:?}",
                    opcode.name(),
                    idx,
                    self.constants[idx as usize]
                );
                offset + 3
            }

            _ => {
                println!("Unknown opcode {}", instruction);
                offset + 1
            }
        }
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_op() {
        let mut chunk = Chunk::new();
        chunk.write_op(OpCode::Add, 1);
        assert_eq!(chunk.code.len(), 1);
        assert_eq!(chunk.lines[0], 1);
    }

    #[test]
    fn test_constant() {
        let mut chunk = Chunk::new();
        let idx = chunk.add_constant(Value::smi(42));
        assert_eq!(idx, 0);
        assert_eq!(chunk.constants[0].as_smi(), Some(42));
    }

    #[test]
    fn test_jump() {
        let mut chunk = Chunk::new();
        let jump_offset = chunk.write_jump(OpCode::Jump, 1);
        chunk.write_op(OpCode::Pop, 1);
        chunk.patch_jump(jump_offset);

        // 反汇编验证
        chunk.disassemble("test");
    }

    #[test]
    fn test_loop() {
        let mut chunk = Chunk::new();
        let loop_start = chunk.current_offset();
        chunk.write_op(OpCode::Pop, 1);
        chunk.write_loop(loop_start, 1);

        chunk.disassemble("loop test");
    }
}
