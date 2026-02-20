//! Orchestrator Demo - 演示如何使用 kaubo-orchestrator
//!
//! 运行: cargo run --example orchestrator_demo

use kaubo_orchestrator::{
    FileLoader, FileEmitter, StdoutEmitter, BytecodeEmitter, SourceParser, NoOpPass,
    ParserPass, CodeGenPass, CompilePass,
    Orchestrator, Source, DataFormat,
};
use kaubo_config::VmConfig;
use std::sync::Arc;

fn main() {
    println!("=== Kaubo Orchestrator Demo ===\n");

    // 1. 创建编排器
    let config = VmConfig::default();
    let mut orchestrator = Orchestrator::new(config);
    println!("✓ 创建编排器");

    // 2. 创建 logger
    let logger: Arc<kaubo_log::Logger> = kaubo_log::Logger::new(kaubo_log::Level::Info);

    // 3. 注册组件
    orchestrator.register_loader(Box::new(FileLoader::new()));
    orchestrator.register_adaptive_parser(Box::new(SourceParser::new()));
    orchestrator.register_emitter(Box::new(FileEmitter::new()));
    orchestrator.register_emitter(Box::new(StdoutEmitter::new()));
    orchestrator.register_emitter(Box::new(BytecodeEmitter::new()));
    orchestrator.register_pass(Box::new(NoOpPass::new(
        "noop",
        DataFormat::Source,
        DataFormat::Source,
    )));
    
    // 注册 Core 适配器 Pass
    orchestrator.register_pass(Box::new(ParserPass::new(logger.clone())));
    orchestrator.register_pass(Box::new(CodeGenPass::new(logger.clone())));
    orchestrator.register_pass(Box::new(CompilePass::new(logger)));
    
    println!("✓ 注册组件");

    // 4. 显示已注册的组件
    println!("\n已注册的组件:");
    println!("  Loaders: {}", orchestrator.loaders().len());
    for name in orchestrator.loaders().names() {
        println!("    - {}", name);
    }
    println!("  Passes: {}", orchestrator.passes().len());
    for name in orchestrator.passes().names() {
        println!("    - {}", name);
    }
    println!("  Emitters: {}", orchestrator.emitters().len());
    for name in orchestrator.emitters().names() {
        println!("    - {}", name);
    }

    // 5. 尝试加载文件
    println!("\n尝试加载文件:");
    let source = Source::file("examples/hello/main.kaubo");
    match orchestrator.loaders().get("file_loader") {
        Some(loader) => {
            match loader.load(&source) {
                Ok(data) => {
                    println!("✓ 文件加载成功");
                    match data {
                        kaubo_orchestrator::RawData::Text(text) => {
                            println!("  内容长度: {} 字符", text.len());
                        }
                        kaubo_orchestrator::RawData::Binary(bytes) => {
                            println!("  内容长度: {} 字节", bytes.len());
                        }
                    }
                }
                Err(e) => {
                    println!("✗ 文件加载失败: {}", e);
                }
            }
        }
        None => {
            println!("✗ 未找到 file_loader");
        }
    }

    // 6. 显示 Pass 链
    println!("\n编译 Pass 链 (Source → Bytecode):");
    if let Some(compile_pass) = orchestrator.passes().get("compile") {
        println!("  ✓ compile: {} → {}", 
            compile_pass.input_format(), 
            compile_pass.output_format()
        );
    }

    println!("\n=== Demo 完成 ===");
}
