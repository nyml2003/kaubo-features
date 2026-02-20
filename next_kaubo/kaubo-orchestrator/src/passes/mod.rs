//! Pass implementations

pub mod codegen;
pub mod lexer;
pub mod module;
pub mod parser;
pub mod vm;

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::converter::DataFormat;
use crate::error::PassError;
use crate::pass::{Input, Output, Pass, PassContext};

/// A simple pass that just passes through the input (for testing)
pub struct NoOpPass {
    name: &'static str,
    input_format: DataFormat,
    output_format: DataFormat,
}

impl NoOpPass {
    pub fn new(name: &'static str, input_format: DataFormat, output_format: DataFormat) -> Self {
        Self {
            name,
            input_format,
            output_format,
        }
    }
}

impl Component for NoOpPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            self.name,
            "0.1.0",
            ComponentKind::Pass,
            Some("No-op pass for testing"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![self.input_format.clone()], vec![self.output_format.clone()])
    }
}

impl Pass for NoOpPass {
    fn input_format(&self) -> DataFormat {
        self.input_format.clone()
    }

    fn output_format(&self) -> DataFormat {
        self.output_format.clone()
    }

    fn run(&self, input: Input, _ctx: &PassContext) -> Result<Output, PassError> {
        Ok(Output::new(input.data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_pass() {
        let pass = NoOpPass::new("test", DataFormat::Source, DataFormat::Source);

        assert_eq!(pass.metadata().name, "test");
        assert_eq!(pass.input_format(), DataFormat::Source);
        assert_eq!(pass.output_format(), DataFormat::Source);
    }
}
