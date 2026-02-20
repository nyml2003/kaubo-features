//! File Emitter - 输出到文件

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::adaptive_parser::DataFormat;
use crate::emitter::{Emitter, SerializedOutput, Target, TargetKind};
use crate::error::EmitterError;
use crate::pass::Output;

/// 文件发射器
pub struct FileEmitter;

impl FileEmitter {
    /// 创建新的文件发射器
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for FileEmitter {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "file_emitter",
            "0.1.0",
            ComponentKind::Emitter,
            Some("输出到文件"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(
            vec![DataFormat::Text, DataFormat::Binary, DataFormat::Result],
            vec![DataFormat::Custom("file".to_string())]
        )
    }
}

impl Emitter for FileEmitter {
    fn format(&self) -> &str {
        "file"
    }

    fn serialize(&self, output: &Output) -> Result<SerializedOutput, EmitterError> {
        // For now, just serialize the debug representation
        let data = format!("{:?}", output.data).into_bytes();
        Ok(SerializedOutput::new("text", data))
    }

    fn write(&self, data: &SerializedOutput, target: &Target) -> Result<(), EmitterError> {
        match &target.kind {
            TargetKind::File => {
                if let Some(path) = &target.path {
                    std::fs::write(path, &data.data)
                        .map_err(|e| EmitterError::WriteFailed(format!("写入文件失败: {}", e)))
                } else {
                    Err(EmitterError::TargetNotFound(
                        "File target requires path".to_string(),
                    ))
                }
            }
            _ => Err(EmitterError::WriteFailed(format!(
                "不支持的目标类型: {:?}",
                target.kind
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_emitter_metadata() {
        let emitter = FileEmitter::new();
        let metadata = emitter.metadata();

        assert_eq!(metadata.name, "file_emitter");
        assert_eq!(metadata.kind, ComponentKind::Emitter);
    }

    #[test]
    fn test_file_emitter_capabilities() {
        let emitter = FileEmitter::new();
        let caps = emitter.capabilities();

        assert!(caps.can_accept(&DataFormat::Binary));
        assert!(caps.can_produce(&DataFormat::Custom("file".to_string())));
    }

    #[test]
    fn test_file_emitter_format() {
        let emitter = FileEmitter::new();
        assert_eq!(emitter.format(), "file");
    }
}
