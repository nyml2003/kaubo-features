mod compiler;
mod kit;

use std::fs;

fn main() {
    // 默认使用项目目录下的 assets/a.txt
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| r"C:\Users\nyml\code\kaubo-features\next_kaubo\assets\a.txt".to_string());
    
    println!("Kaubo VM - 字节码执行演示");
    println!("==========================");
    println!();
    
    // 读取源码文件
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: 无法读取文件 '{}': {}", path, e);
            std::process::exit(1);
        }
    };
    
    println!("文件: {}", path);
    println!();
    println!("[源码]");
    for (i, line) in content.lines().enumerate() {
        println!("{:3} | {}", i + 1, line);
    }
    println!();
    
    // 步骤 1: 词法分析
    println!("[步骤 1] 词法分析 (Lexer)...");
    let tokens = next_kaubo::tokenize(&content);
    print!("Tokens: ");
    for token in tokens.iter().take(10) {
        print!("{:?} ", token.kind);
    }
    if tokens.len() > 10 {
        print!("... (共 {} 个)", tokens.len());
    }
    println!();
    println!();
    
    // 步骤 2: 语法分析
    println!("[步骤 2] 语法分析 (Parser)...");
    use next_kaubo::compiler::lexer::builder::build_lexer;
    use next_kaubo::compiler::parser::parser::Parser;
    
    let mut lexer = build_lexer();
    let _ = lexer.feed(&content.as_bytes().to_vec());
    let _ = lexer.terminate();
    
    let mut parser = Parser::new(lexer);
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    };
    println!("AST 节点数: {}", ast.statements.len());
    println!();
    
    // 步骤 3: 编译为字节码
    println!("[步骤 3] 编译为字节码 (Compiler)...");
    use next_kaubo::runtime::compile;
    
    let (chunk, local_count) = match compile(&ast) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Compile error: {:?}", e);
            std::process::exit(1);
        }
    };
    
    println!("常量池: {} 个", chunk.constants.len());
    println!("字节码: {} bytes", chunk.code.len());
    println!("局部变量: {} 个", local_count);
    println!();
    
    // 打印反汇编
    println!("[字节码反汇编]");
    chunk.disassemble("main");
    println!();
    
    // 步骤 4: 执行
    println!("[步骤 4] 执行字节码 (VM)...");
    use next_kaubo::runtime::VM;
    
    let mut vm = VM::new();
    let result = vm.interpret_with_locals(&chunk, local_count);
    
    match result {
        next_kaubo::runtime::InterpretResult::Ok => {
            println!();
            println!("✅ 执行成功!");
            if let Some(value) = vm.stack_top() {
                println!("返回值: {:?} ({})", value, value);
            }
        }
        next_kaubo::runtime::InterpretResult::RuntimeError(msg) => {
            eprintln!("❌ 运行时错误: {}", msg);
            std::process::exit(1);
        }
        next_kaubo::runtime::InterpretResult::CompileError(msg) => {
            eprintln!("❌ 编译错误: {}", msg);
            std::process::exit(1);
        }
    }
}
