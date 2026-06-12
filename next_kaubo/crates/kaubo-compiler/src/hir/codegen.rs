//! HIR Codegen: HirModule → Chunk
//!
//! 将基本块 + 三地址码转换为平坦字节码。

use kaubo_ir::hir::{
    ConstantValue, HirBinaryOp, HirBlock, HirFunction, HirInstr, HirModule, HirOperand,
    HirTerminator, HirUnaryOp,
};
use kaubo_ir::Chunk;
use kaubo_ir::OpCode;
use kaubo_ir::Value;
use crate::codegen::CompileError;
use kaubo_runtime::vm::VmRuntime;

struct HirCodegen {
    chunk: Chunk,
    block_offsets: Vec<Option<usize>>, // block id → bytecode offset
    temp_to_local: Vec<Option<u8>>,     // temp ID → local index
}

impl HirCodegen {
    fn new(local_count: usize, block_count: usize) -> Self {
        Self {
            chunk: Chunk::new(),
            block_offsets: vec![None; block_count],
            temp_to_local: vec![None; local_count],
        }
    }

    fn alloc_local_for_temp(&mut self, temp: usize) -> u8 {
        if let Some(idx) = self.temp_to_local.get(temp).and_then(|x| *x) {
            return idx;
        }
        // Extend the temp_to_local vec if needed
        while self.temp_to_local.len() <= temp {
            self.temp_to_local.push(None);
        }
        // Assign next available local slot
        let idx = self.temp_to_local.iter().filter(|x| x.is_some()).count() as u8;
        self.temp_to_local[temp] = Some(idx);
        idx
    }

    fn get_operand(&mut self, op: &HirOperand) -> u8 {
        match op {
            HirOperand::Temp(t) => self.alloc_local_for_temp(*t),
            HirOperand::Const(idx) => {
                // Add constant to chunk's constant pool
                self.chunk.constants.len() as u8 // simplified
            }
            HirOperand::Immediate(val) => {
                let v = match val {
                    ConstantValue::Int(n) => Value::int(*n),
                    ConstantValue::Float(f) => Value::float(*f),
                    ConstantValue::Bool(b) => Value::bool_from(*b),
                    ConstantValue::String(s) => {
                        let obj = Box::new(kaubo_ir::object::ObjString::new(s.clone()));
                        Value::string(Box::into_raw(obj))
                    }
                    ConstantValue::Null => Value::NULL,
                };
                self.chunk.add_constant(v)
            }
            _ => 0,
        }
    }

    fn compile_function(&mut self, func: &HirFunction) {
        // Record block offsets
        for (i, block) in func.blocks.iter().enumerate() {
            self.block_offsets[block.id] = Some(self.chunk.code.len());

            for instr in &block.instrs {
                self.compile_instr(instr);
            }
            self.compile_terminator(&block.term, &func.blocks);
        }
    }

    fn compile_instr(&mut self, instr: &HirInstr) {
        match instr {
            HirInstr::LoadConst { dst, value } => {
                let idx = match value {
                    ConstantValue::Int(n) => {
                        self.chunk.add_constant(Value::int(*n))
                    }
                    ConstantValue::Float(f) => {
                        self.chunk.add_constant(Value::float(*f))
                    }
                    ConstantValue::Bool(b) => {
                        self.chunk.add_constant(Value::bool_from(*b))
                    }
                    ConstantValue::Null => {
                        self.chunk.add_constant(Value::NULL)
                    }
                    ConstantValue::String(s) => {
                        let obj = Box::new(kaubo_ir::object::ObjString::new(s.clone()));
                        self.chunk.add_constant(Value::string(Box::into_raw(obj)))
                    }
                };
                let local = self.alloc_local_for_temp(match dst { HirOperand::Temp(t) => *t, _ => 0 });
                self.chunk.write_op_u8(OpCode::LoadConst, idx, 0);
                self.chunk.write_op_u8(OpCode::StoreLocal, local, 0);
            }
            HirInstr::Move { dst, src } => {
                let src_local = self.get_operand(src);
                let dst_local = self.alloc_local_for_temp(match dst { HirOperand::Temp(t) => *t, _ => 0 });
                self.chunk.write_op_u8(OpCode::LoadLocal, src_local, 0);
                self.chunk.write_op_u8(OpCode::StoreLocal, dst_local, 0);
            }
            HirInstr::Binary { dst, op, left, right } => {
                let l = self.get_operand(left);
                let r = self.get_operand(right);
                let d = self.alloc_local_for_temp(match dst { HirOperand::Temp(t) => *t, _ => 0 });
                self.chunk.write_op_u8(OpCode::LoadLocal, l, 0);
                self.chunk.write_op_u8(OpCode::LoadLocal, r, 0);
                let oc = op.to_opcode();
                self.chunk.write_op_u8(oc, 0xFF, 0);
                self.chunk.write_op_u8(OpCode::StoreLocal, d, 0);
            }
            HirInstr::Print { value: _ } => {
                // Print the top of stack — simplified
                self.chunk.write_op(OpCode::Print, 0);
            }
            HirInstr::Nop => {}
            _ => {}
        }
    }

    fn compile_terminator(&mut self, term: &HirTerminator, blocks: &[HirBlock]) {
        match term {
            HirTerminator::Jump { target } => {
                // Calculate offset to target block
                let target_offset = self.block_offsets[*target].unwrap_or(0);
                let current = self.chunk.code.len();
                let jump = target_offset as i32 - current as i32;
                self.chunk.write_i16(jump as i16, 0);
            }
            HirTerminator::Branch { cond: _, true_target, false_target } => {
                let t_off = self.block_offsets[*true_target].unwrap_or(0);
                let current = self.chunk.code.len();
                let jump = t_off as i32 - current as i32;
                self.chunk.write_i16(jump as i16, 0);
                // Emit a jump for false path
                let f_off = self.block_offsets[*false_target].unwrap_or(0);
                let current = self.chunk.code.len();
                let jump = f_off as i32 - current as i32;
                self.chunk.write_i16(jump as i16, 0);
            }
            HirTerminator::Return { value: _ } => {
                self.chunk.write_op(OpCode::Return, 0);
            }
            HirTerminator::End => {
                self.chunk.write_op(OpCode::Return, 0);
            }
        }
    }

    fn finish(mut self) -> (Chunk, usize) {
        let max_locals = self.temp_to_local.iter().filter(|x| x.is_some()).count().max(1);
        (self.chunk, max_locals)
    }
}

/// Compile HIR module to Chunk
pub fn compile_hir(module: &HirModule) -> Result<(Chunk, usize), CompileError> {
    let func = module.functions.first().ok_or(CompileError::Unimplemented("empty module".into()))?;
    let mut cg = HirCodegen::new(func.local_count, func.blocks.len());
    cg.compile_function(func);
    Ok(cg.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_hir() {
        let mut module = HirModule::new();
        module.functions.push(HirFunction {
            name: Some("main".into()),
            arity: 0,
            local_count: 2,
            return_type: None,
            entry: 0,
            blocks: vec![HirBlock {
                id: 0,
                instrs: vec![
                    HirInstr::LoadConst {
                        dst: HirOperand::Temp(0),
                        value: ConstantValue::Int(42),
                    },
                ],
                term: HirTerminator::Return { value: Some(HirOperand::Temp(0)) },
            }],
        });

        let (chunk, _) = compile_hir(&module).unwrap();
        assert!(!chunk.code.is_empty(), "chunk should have bytecode");
    }
}
