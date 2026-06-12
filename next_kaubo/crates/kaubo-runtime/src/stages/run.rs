//! RunStage — 执行字节码
use kaubo_ir::{Chunk, InterpretResult, VM};
use crate::binary::VMExecuteBinary;
use crate::vm::VmRuntime;

pub struct RunStage;

impl RunStage {
    pub fn new() -> Self { Self }

    pub fn run_chunk(&self, chunk: &Chunk) -> Result<(), String> {
        let mut vm = VM::new();
        match vm.interpret(chunk) {
            InterpretResult::Ok => Ok(()),
            InterpretResult::CompileError(m) => Err(format!("compile error: {}", m)),
            InterpretResult::RuntimeError(m) => Err(format!("runtime error: {}", m)),
        }
    }
}
