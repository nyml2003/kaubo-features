//! 字节码块 (Core 层)
//!
//! 包含 Chunk 类型定义，不依赖运行时实现

use super::bytecode::{InlineCacheSlot, MethodTableEntry, OpCode, OperatorTableEntry};
use super::operators::InlineCacheEntry;
use super::value::Value;

/// 字节码块
#[derive(Clone)]
pub struct Chunk {
    /// 指令字节码
    pub code: Vec<u8>,
    /// 常量池
    pub constants: Vec<Value>,
    /// 行号信息
    pub lines: Vec<usize>,
    /// 方法表
    pub method_table: Vec<MethodTableEntry>,
    /// 运算符表
    pub operator_table: Vec<OperatorTableEntry>,
    /// 内联缓存槽位表
    pub inline_cache_slots: Vec<InlineCacheSlot>,
    /// 内联缓存条目
    pub inline_caches: Vec<InlineCacheEntry>,
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("code", &self.code)
            .field("constants", &self.constants)
            .field("lines", &self.lines)
            .field("method_table", &self.method_table)
            .field("operator_table", &self.operator_table)
            .field("inline_cache_slots", &self.inline_cache_slots)
            .field("inline_caches", &self.inline_caches)
            .finish()
    }
}

impl Chunk {
    /// 创建新的字节码块
    pub fn new() -> Self {
        Self::with_logger(kaubo_log::Logger::noop())
    }

    /// 创建新的字节码块（带 logger，为向后兼容保留）
    pub fn with_logger(_logger: std::sync::Arc<kaubo_log::Logger>) -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
            method_table: Vec::new(),
            operator_table: Vec::new(),
            inline_cache_slots: Vec::new(),
            inline_caches: Vec::new(),
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

    /// 写入 i16 操作数
    pub fn write_i16(&mut self, value: i16, line: usize) {
        let bytes = value.to_le_bytes();
        self.code.push(bytes[0]);
        self.code.push(bytes[1]);
        self.lines.push(line);
        self.lines.push(line);
    }

    /// 写入 u16 操作数
    pub fn write_u16(&mut self, value: u16, line: usize) {
        let bytes = value.to_le_bytes();
        self.code.push(bytes[0]);
        self.code.push(bytes[1]);
        self.lines.push(line);
        self.lines.push(line);
    }

    /// 写入带 u16 + u8 操作数的指令
    pub fn write_op_u16_u8(&mut self, op: OpCode, u16_val: u16, u8_val: u8, line: usize) {
        self.code.push(op as u8);
        let bytes = u16_val.to_le_bytes();
        self.code.push(bytes[0]);
        self.code.push(bytes[1]);
        self.code.push(u8_val);
        self.lines.push(line);
        self.lines.push(line);
        self.lines.push(line);
        self.lines.push(line);
    }

    /// 写入跳转指令（占位）
    pub fn write_jump(&mut self, op: OpCode, line: usize) -> usize {
        self.write_op(op, line);
        let offset = self.code.len();
        self.write_i16(-1i16, line);
        offset
    }

    /// 修补跳转偏移量
    pub fn patch_jump(&mut self, offset: usize) {
        let jump = self.code.len() - (offset + 2);
        let jump_i16 = jump as i16;
        let bytes = jump_i16.to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    /// 写入循环跳转
    pub fn write_loop(&mut self, loop_start: usize, line: usize) {
        self.write_op(OpCode::JumpBack, line);
        let offset = self.code.len() - loop_start + 2;
        let jump = -(offset as i16);
        self.write_i16(jump, line);
    }

    /// 添加常量
    pub fn add_constant(&mut self, value: Value) -> u8 {
        let idx = self.constants.len();
        if idx > 255 {
            panic!("Too many constants in one chunk");
        }
        self.constants.push(value);
        idx as u8
    }

    /// 添加常量（宽索引）
    pub fn add_constant_wide(&mut self, value: Value) -> u16 {
        let idx = self.constants.len();
        if idx > 65535 {
            panic!("Too many constants in one chunk");
        }
        self.constants.push(value);
        idx as u16
    }

    /// 获取当前代码位置
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// 分配内联缓存槽位
    pub fn allocate_inline_cache(&mut self) -> u8 {
        let cache_idx = self.inline_caches.len();
        if cache_idx > 255 {
            panic!("Too many inline caches in one chunk");
        }
        self.inline_caches.push(InlineCacheEntry::empty());
        self.inline_cache_slots.push(InlineCacheSlot {
            pc: 0,
            cache_idx: cache_idx as u8,
        });
        cache_idx as u8
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
        let offset = chunk.write_jump(OpCode::Jump, 1);
        chunk.write_op(OpCode::Pop, 1);
        chunk.patch_jump(offset);
        assert!(chunk.code.len() > 3);
    }
}
