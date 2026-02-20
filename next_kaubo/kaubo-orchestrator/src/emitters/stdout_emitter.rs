//! Stdout Emitter - 输出到标准输出

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::converter::DataFormat;
use crate::emitter::{Emitter, SerializedOutput, Target, TargetKind};
use crate::error::EmitterError;
use crate::pass::Output;

/// 标准输出发射器
pub struct StdoutEmitter;

impl StdoutEmitter {
    /// 创建新的标准输出发射器
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdoutEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for StdoutEmitter {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "stdout_emitter",
            "0.1.0",
            ComponentKind::Emitter,
            Some("输出到标准输出"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(
            vec![DataFormat::Text, DataFormat::Result],
            vec![DataFormat::Custom("stdout".to_string())]
        )
    }
}

impl Emitter for StdoutEmitter {
    fn format(&self) -> &str {
        "stdout"
    }

    fn serialize(&self, output: &Output) -> Result<SerializedOutput, EmitterError> {
        // For now, just serialize the debug representation
        let data = format!("{:?}", output.data).into_bytes();
        Ok(SerializedOutput::new("text", data))
    }

    fn write(&self, data: &SerializedOutput, target: &Target) -> Result<(), EmitterError> {
        if target.kind != TargetKind::Stdout {
            return Err(EmitterError::WriteFailed(format!(
                "不支持的目标类型: {:?}",
                target.kind
            )));
        }

        // Try to output as UTF-8 text
        match String::from_utf8(data.data.clone()) {
            Ok(text) => {
                println!("{}", text);
                Ok(())
            }
            Err(_) => Err(EmitterError::InvalidOutputFormat(
                "无法将二进制数据转换为文本输出".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_emitter_metadata() {
        let emitter = StdoutEmitter::new();
        let metadata = emitter.metadata();

        assert_eq!(metadata.name, "stdout_emitter");
        assert_eq!(metadata.kind, ComponentKind::Emitter);
    }

    #[test]
    fn test_stdout_emitter_capabilities() {
        let emitter = StdoutEmitter::new();
        let caps = emitter.capabilities();

        assert!(caps.can_accept(&DataFormat::Text));
        assert!(caps.can_produce(&DataFormat::Custom("stdout".to_string())));
    }

    #[test]
    fn test_stdout_emitter_format() {
        let emitter = StdoutEmitter::new();
        assert_eq!(emitter.format(), "stdout");
    }
}
