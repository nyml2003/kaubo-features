//! Multi-Module Pass - 多文件模块编译
//!
//! 使用 kaubo-core 的多文件编译器支持 import 语句。

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::adaptive_parser::{DataFormat, IR};
use crate::error::PassError;
use crate::pass::{Input, Output, Pass, PassContext};
use crate::pipeline::module::MultiFileCompiler;
use kaubo_vfs::NativeFileSystem;
use std::path::Path;
use std::sync::Arc;

/// 多模块编译 Pass
///
/// 输入: Source (入口文件路径)
/// 输出: Bytecode (合并后的字节码)
pub struct MultiModulePass {
    logger: Arc<kaubo_log::Logger>,
}

impl MultiModulePass {
    /// 创建新的多模块编译 Pass
    pub fn new(logger: Arc<kaubo_log::Logger>) -> Self {
        Self { logger }
    }
}

impl Component for MultiModulePass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "multi_module",
            "0.1.0",
            ComponentKind::Pass,
            Some("多文件模块编译 (支持 import)"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![DataFormat::Source], vec![DataFormat::Bytecode])
    }
}

impl Pass for MultiModulePass {
    fn input_format(&self) -> DataFormat {
        DataFormat::Source
    }

    fn output_format(&self) -> DataFormat {
        DataFormat::Bytecode
    }

    fn run(&self, input: Input, _ctx: &PassContext) -> Result<Output, PassError> {
        // 获取入口文件路径
        let entry_path = input.as_source().map_err(|e| PassError::InvalidInput {
            message: format!("MultiModulePass 需要 Source 输入 (入口路径): {}", e),
        })?;

        // 获取入口文件的目录作为根目录
        let entry_path = Path::new(entry_path);
        let root_dir = entry_path.parent().unwrap_or(Path::new("."));

        // 创建 VFS
        let vfs = Box::new(NativeFileSystem::new());

        // 创建多文件编译器
        let mut compiler = MultiFileCompiler::new(vfs, root_dir);

        // 编译入口文件
        let result = compiler.compile_entry(entry_path).map_err(|e| {
            PassError::TransformFailed(format!("多文件编译失败: {}", e))
        })?;

        // 目前我们只编译入口模块（简化版）
        // 完整的实现应该合并所有模块的字节码
        let entry_unit = result.units.last().ok_or_else(|| {
            PassError::TransformFailed("没有编译单元".to_string())
        })?;

        // 编译入口模块为字节码
        use crate::pipeline::codegen::compile_with_struct_info_and_logger;
        use std::collections::HashMap;

        let (chunk, _) = compile_with_struct_info_and_logger(
            &entry_unit.ast,
            HashMap::new(),
            self.logger.clone(),
        )
        .map_err(|e| PassError::TransformFailed(format!("代码生成失败: {:?}", e)))?;

        Ok(Output::new(IR::Bytecode(chunk)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_module_pass_metadata() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = MultiModulePass::new(logger);

        assert_eq!(pass.metadata().name, "multi_module");
        assert_eq!(pass.input_format(), DataFormat::Source);
        assert_eq!(pass.output_format(), DataFormat::Bytecode);
    }

    #[test]
    fn test_multi_module_capabilities() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = MultiModulePass::new(logger);
        let caps = pass.capabilities();

        assert!(caps.can_accept(&DataFormat::Source));
        assert!(caps.can_produce(&DataFormat::Bytecode));
    }
}
