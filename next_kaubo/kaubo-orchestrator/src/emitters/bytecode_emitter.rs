//! Bytecode Emitter - 输出字节码

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::adaptive_parser::DataFormat;
use crate::emitter::{Emitter, SerializedOutput, Target, TargetKind};
use crate::error::EmitterError;
use crate::pass::Output;

/// Bytecode 发射器 - 用于 emit 阶段处理 Bytecode 格式
pub struct BytecodeEmitter;

impl BytecodeEmitter {
    /// Create a new BytecodeEmitter
    pub fn new() -> Self {
        Self
    }
}

impl Default for BytecodeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BytecodeEmitter {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "bytecode_emitter",
            "0.1.0",
            ComponentKind::Emitter,
            Some("输出字节码"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(
            vec![DataFormat::Bytecode],
            vec![DataFormat::Binary]
        )
    }
}

impl Emitter for BytecodeEmitter {
    fn format(&self) -> &str {
        "bytecode"
    }

    fn serialize(&self, output: &Output) -> Result<SerializedOutput, EmitterError> {
        // Serialize bytecode to binary format
        use crate::adaptive_parser::IR;
        match &output.data {
            IR::Bytecode(chunk) => {
                // Simple serialization - just use chunk's code and constants
                let mut data = Vec::new();
                // Add code length as u32
                data.extend_from_slice(&(chunk.code.len() as u32).to_le_bytes());
                // Add code
                data.extend_from_slice(&chunk.code);
                Ok(SerializedOutput::new("bytecode", data).with_content_type("application/octet-stream"))
            }
            _ => Err(EmitterError::InvalidOutputFormat(
                format!("Expected Bytecode, got {:?}", output.data)
            )),
        }
    }

    fn write(&self, data: &SerializedOutput, target: &Target) -> Result<(), EmitterError> {
        match &target.kind {
            TargetKind::File => {
                if let Some(path) = &target.path {
                    std::fs::write(path, &data.data)
                        .map_err(|e| EmitterError::WriteFailed(format!("写入文件失败: {}", e)))
                } else {
                    Err(EmitterError::TargetNotFound("文件路径未指定".to_string()))
                }
            }
            TargetKind::Stdout => {
                // For stdout, we don't write binary data
                Ok(())
            }
            TargetKind::Memory => {
                // Memory target - nothing to do, data is already in SerializedOutput
                Ok(())
            }
            _ => Err(EmitterError::TargetNotFound(format!(
                "不支持的目标类型: {:?}",
                target.kind
            ))),
        }
    }
}
