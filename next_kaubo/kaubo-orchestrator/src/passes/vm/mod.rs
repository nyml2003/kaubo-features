//! VM Execution Pass

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::adaptive_parser::{DataFormat, IR};
use crate::error::PassError;
use crate::pass::{Input, Output, Pass, PassContext};
use crate::vm::core::{Chunk, InterpretResult, VM};
use std::sync::Arc;

/// VM Execution Pass
pub struct VmExecutionPass {
    logger: Arc<kaubo_log::Logger>,
}

impl VmExecutionPass {
    pub fn new(logger: Arc<kaubo_log::Logger>) -> Self {
        Self { logger }
    }
}

impl Component for VmExecutionPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "vm_execution",
            "0.1.0",
            ComponentKind::Emitter,
            Some("VM Execution Pass (Pipeline end)"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![DataFormat::Bytecode], vec![DataFormat::Result])
    }
}

impl Pass for VmExecutionPass {
    fn input_format(&self) -> DataFormat {
        DataFormat::Bytecode
    }

    fn output_format(&self) -> DataFormat {
        DataFormat::Result
    }

    fn run(&self, input: Input, _ctx: &PassContext) -> Result<Output, PassError> {
        let chunk = input.as_bytecode().map_err(|e| PassError::InvalidInput {
            message: format!("VMExecutionPass needs Bytecode input: {}", e),
        })?;

        let mut vm = VM::new();
        let result = vm.interpret(chunk);

        // Convert VM result to ExecutionResult
        let exec_result = match result {
            InterpretResult::Ok => crate::adaptive_parser::ExecutionResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
            InterpretResult::CompileError(msg) => crate::adaptive_parser::ExecutionResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Compile error: {}", msg),
            },
            InterpretResult::RuntimeError(msg) => crate::adaptive_parser::ExecutionResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Runtime error: {}", msg),
            },
        };

        Ok(Output::new(IR::Result(exec_result)))
    }
}
