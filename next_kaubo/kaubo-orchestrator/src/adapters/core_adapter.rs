//! Core Adapter - 将 kaubo-core 功能包装为 Pass 组件
//!
//! 这个模块提供了从 kaubo-core 到 orchestrator Pass 系统的适配。
//! 随着迁移进行，这些适配器将被原生 Pass 实现取代。

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::converter::{DataFormat, IR};
use crate::error::PassError;
use crate::pass::{Input, Output, Pass, PassContext};
use std::sync::Arc;

/// Parser Pass - 将源代码解析为 AST
///
/// 输入: Source (String)
/// 输出: Ast (crate::vm::core::Module)
pub struct ParserPass {
    logger: Arc<kaubo_log::Logger>,
}

impl ParserPass {
    /// 创建新的解析器 Pass
    pub fn new(logger: Arc<kaubo_log::Logger>) -> Self {
        Self { logger }
    }
}

impl Component for ParserPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "parser",
            "0.1.0",
            ComponentKind::Pass,
            Some("将源代码解析为 AST"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![DataFormat::Source], vec![DataFormat::Ast])
    }
}

impl Pass for ParserPass {
    fn input_format(&self) -> DataFormat {
        DataFormat::Source
    }

    fn output_format(&self) -> DataFormat {
        DataFormat::Ast
    }

    fn run(&self, input: Input, _ctx: &PassContext) -> Result<Output, PassError> {
        // 获取源代码
        let source = input.as_source().map_err(|e| PassError::InvalidInput {
            message: format!("ParserPass 需要 Source 输入: {}", e),
        })?;

        // 使用 kaubo-core 进行解析
        use crate::kit::lexer::Lexer;
        use crate::passes::parser::Parser;

        // 创建 lexer
        let mut lexer = Lexer::with_logger(4096, self.logger.clone());
        
        // 输入源代码
        lexer.feed(source.as_bytes()).map_err(|e| {
            PassError::TransformFailed(format!("Lexer feed error: {:?}", e))
        })?;
        
        // 标记输入结束
        lexer.terminate().map_err(|e| {
            PassError::TransformFailed(format!("Lexer terminate error: {:?}", e))
        })?;

        // 创建 parser 并解析
        let mut parser = Parser::with_logger(lexer, self.logger.clone());

        match parser.parse() {
            Ok(module) => Ok(Output::new(IR::Ast(module))),
            Err(e) => Err(PassError::TransformFailed(format!("解析失败: {:?}", e))),
        }
    }
}

/// CodeGen Pass - 将 AST 编译为字节码
///
/// 输入: Ast (crate::vm::core::Module)
/// 输出: Bytecode (crate::vm::core::Chunk)
pub struct CodeGenPass {
    logger: Arc<kaubo_log::Logger>,
}

impl CodeGenPass {
    /// 创建新的代码生成 Pass
    pub fn new(logger: Arc<kaubo_log::Logger>) -> Self {
        Self { logger }
    }
}

impl Component for CodeGenPass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "codegen",
            "0.1.0",
            ComponentKind::Pass,
            Some("将 AST 编译为字节码"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![DataFormat::Ast], vec![DataFormat::Bytecode])
    }
}

impl Pass for CodeGenPass {
    fn input_format(&self) -> DataFormat {
        DataFormat::Ast
    }

    fn output_format(&self) -> DataFormat {
        DataFormat::Bytecode
    }

    fn run(&self, input: Input, _ctx: &PassContext) -> Result<Output, PassError> {
        // 获取 AST
        let module = input.as_ast().map_err(|e| PassError::InvalidInput {
            message: format!("CodeGenPass 需要 Ast 输入: {}", e),
        })?;

        // 使用 kaubo-core 进行编译
        use crate::passes::codegen::compile_with_struct_info_and_logger;
        use std::collections::HashMap;

        match compile_with_struct_info_and_logger(
            module,
            HashMap::new(),
            self.logger.clone(),
        ) {
            Ok((chunk, _local_count)) => {
                let bytecode_ir = IR::Bytecode(chunk);
                Ok(Output::new(bytecode_ir))
            }
            Err(e) => Err(PassError::TransformFailed(format!("编译失败: {:?}", e))),
        }
    }
}

/// 完整的 Source→Bytecode 编译 Pass
///
/// 这是一个组合 Pass，内部使用 Parser + CodeGen
pub struct CompilePass {
    parser: ParserPass,
    codegen: CodeGenPass,
}

impl CompilePass {
    /// 创建新的编译 Pass
    pub fn new(logger: Arc<kaubo_log::Logger>) -> Self {
        Self {
            parser: ParserPass::new(logger.clone()),
            codegen: CodeGenPass::new(logger),
        }
    }
}

impl Component for CompilePass {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "compile",
            "0.1.0",
            ComponentKind::Pass,
            Some("完整编译：Source → Bytecode"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(vec![DataFormat::Source], vec![DataFormat::Bytecode])
    }
}

impl Pass for CompilePass {
    fn input_format(&self) -> DataFormat {
        DataFormat::Source
    }

    fn output_format(&self) -> DataFormat {
        DataFormat::Bytecode
    }

    fn run(&self, input: Input, ctx: &PassContext) -> Result<Output, PassError> {
        // 第一步：解析
        let ast_output = self.parser.run(input, ctx)?;

        // 第二步：代码生成
        let codegen_input = Input::new(ast_output.data);
        self.codegen.run(codegen_input, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_pass_metadata() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = ParserPass::new(logger);

        assert_eq!(pass.metadata().name, "parser");
        assert_eq!(pass.input_format(), DataFormat::Source);
        assert_eq!(pass.output_format(), DataFormat::Ast);
    }

    #[test]
    fn test_codegen_pass_metadata() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = CodeGenPass::new(logger);

        assert_eq!(pass.metadata().name, "codegen");
        assert_eq!(pass.input_format(), DataFormat::Ast);
        assert_eq!(pass.output_format(), DataFormat::Bytecode);
    }

    #[test]
    fn test_compile_pass_metadata() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = CompilePass::new(logger);

        assert_eq!(pass.metadata().name, "compile");
        assert_eq!(pass.input_format(), DataFormat::Source);
        assert_eq!(pass.output_format(), DataFormat::Bytecode);
    }

    #[test]
    fn test_parser_pass_simple() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = ParserPass::new(logger);
        
        // 创建一个简单的 PassContext（简化版）
        use crate::pass::PassContext;
        use kaubo_config::VmConfig;
        use kaubo_vfs::MemoryFileSystem;
        
        let ctx = PassContext::new(
            Arc::new(VmConfig::default()),
            Arc::new(MemoryFileSystem::new()),
            kaubo_log::Logger::new(kaubo_log::Level::Info),
        );
        
        // 测试解析简单代码
        let input = Input::new(IR::Source("var x = 42;".to_string()));
        let result = pass.run(input, &ctx);
        
        // 应该成功解析
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());
    }

    #[test]
    fn test_compile_pass_simple() {
        let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);
        let pass = CompilePass::new(logger);
        
        use crate::pass::PassContext;
        use kaubo_config::VmConfig;
        use kaubo_vfs::MemoryFileSystem;
        
        let ctx = PassContext::new(
            Arc::new(VmConfig::default()),
            Arc::new(MemoryFileSystem::new()),
            kaubo_log::Logger::new(kaubo_log::Level::Info),
        );
        
        // 测试编译简单代码
        let input = Input::new(IR::Source("return 42;".to_string()));
        let result = pass.run(input, &ctx);
        
        // 应该成功编译
        assert!(result.is_ok(), "Compile should succeed: {:?}", result.err());
        
        // 验证输出是 Bytecode
        if let Ok(output) = result {
            assert!(matches!(output.data, IR::Bytecode(_)));
        }
    }
}
