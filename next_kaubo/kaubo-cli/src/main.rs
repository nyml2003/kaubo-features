//! Kaubo CLI — direct stage pipeline using compiler+runtime crates

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "kaubo", about = "Kaubo language compiler", version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(value_name = "FILE", default_value = "package.json")]
    file: PathBuf,

    #[arg(short, long)] verbose: bool,
    #[arg(short, long)] compile_only: bool,
    #[arg(long)] emit_binary: bool,
    #[arg(long)] production: bool,
    #[arg(short, long, default_value = "auto")] mode: String,
}

#[derive(Subcommand)]
enum Commands {
    Compile { file: PathBuf, #[arg(short, long)] output: Option<PathBuf> },
    Run { file: PathBuf },
    Lex { file: PathBuf },
    Parse { file: PathBuf },
    Check { file: PathBuf },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Compile { file, output }) => cmd_compile(file, output.as_ref(), &cli),
        Some(Commands::Run { file }) => cmd_run(file),
        Some(Commands::Lex { file }) => cmd_lex(file),
        Some(Commands::Parse { file }) => cmd_parse(file),
        Some(Commands::Check { file }) => cmd_check(file),
        None => cmd_default(&cli.file, &cli),
    }
}

fn read_source(file: &PathBuf) -> String {
    if file.to_string_lossy().ends_with("package.json") {
        let content = std::fs::read_to_string(file).unwrap_or_else(|e| exit(&format!("read: {e}")));
        let pkg: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| exit(&format!("json: {e}")));
        let entry = pkg["entry"].as_str().unwrap_or("main.kaubo");
        let dir = file.parent().unwrap_or(std::path::Path::new("."));
        std::fs::read_to_string(dir.join(entry)).unwrap_or_else(|e| exit(&format!("read entry: {e}")))
    } else {
        std::fs::read_to_string(file).unwrap_or_else(|e| exit(&format!("read: {e}")))
    }
}

fn exit(msg: &str) -> ! { eprintln!("Error: {}", msg); process::exit(1); }

fn cmd_lex(file: &PathBuf) {
    let source = read_source(file);
    let result = kaubo_compiler::LexStage::new().run(&source).unwrap_or_else(|e| exit(&e));
    println!("{} tokens", result.count);
}

fn cmd_parse(file: &PathBuf) {
    let source = read_source(file);
    let m = kaubo_compiler::ParseStage::new().run(&source).unwrap_or_else(|e| exit(&e));
    println!("{} statements", m.statements.len());
}

fn cmd_check(file: &PathBuf) {
    let source = read_source(file);
    let m = kaubo_compiler::ParseStage::new().run(&source).unwrap_or_else(|e| exit(&e));
    kaubo_compiler::CheckStage::new().run(&m).unwrap_or_else(|e| exit(&e));
    println!("OK");
}

fn cmd_compile(file: &PathBuf, output: Option<&PathBuf>, cli: &Cli) {
    let chunk = compile_source(&read_source(file));
    let out = output.cloned().unwrap_or_else(|| { let mut p = file.clone(); p.set_extension("kaubod"); p });
    emit_binary(&chunk, &out, cli.production);
    println!("wrote {}", out.display());
}

fn cmd_run(file: &PathBuf) {
    let bytes = std::fs::read(file).unwrap_or_else(|e| exit(&format!("read binary: {e}")));
    use kaubo_runtime::binary::VMExecuteBinary;
    let mut vm = kaubo_ir::VM::new();
    let result = vm.execute_binary(bytes).unwrap_or_else(|e| exit(&format!("execute: {e:?}")));
    println!("{:?}", result);
}

fn cmd_default(file: &PathBuf, cli: &Cli) {
    let chunk = compile_source(&read_source(file));
    if !cli.compile_only {
        use kaubo_runtime::vm::VmRuntime;
        let mut vm = kaubo_ir::VM::new();
        let result = vm.interpret(&chunk);
        match result {
            kaubo_ir::InterpretResult::Ok => {}
            kaubo_ir::InterpretResult::CompileError(m) => exit(&format!("compile error: {m}")),
            kaubo_ir::InterpretResult::RuntimeError(m) => exit(&format!("runtime error: {m}")),
        }
    }
}

fn compile_source(source: &str) -> kaubo_ir::Chunk {
    let m = kaubo_compiler::ParseStage::new().run(source).unwrap_or_else(|e| exit(&e));
    kaubo_compiler::CheckStage::new().run(&m).unwrap_or_else(|e| exit(&e));
    kaubo_compiler::CodegenStage::new().run(&m).unwrap_or_else(|e| exit(&e))
}

fn emit_binary(chunk: &kaubo_ir::Chunk, path: &PathBuf, release: bool) {
    use kaubo_runtime::binary;
    let mut sp = binary::StringPool::new();
    let mut fp = binary::FunctionPool::new();
    let mut st = binary::ShapeTable::new();
    let mut ec = binary::EncodeContext::new(&mut sp, &mut fp, &mut st);
    let cd = binary::encode_chunk_with_context(chunk, &mut ec)
        .unwrap_or_else(|e| exit(&format!("encode: {:?}", e)));
    let bm = if release { binary::BuildMode::Release } else { binary::BuildMode::Debug };
    let opts = binary::WriteOptions { build_mode: bm, compress: release, strip_debug: release, source_map_external: false };
    let mut w = binary::BinaryWriter::new(opts);
    w.write_section(binary::SectionKind::StringPool, &vec![]);
    w.write_section(binary::SectionKind::FunctionPool, &vec![]);
    w.write_section(binary::SectionKind::ChunkData, &cd);
    w.set_entry(0, 0);
    std::fs::write(path, w.finish()).unwrap_or_else(|e| exit(&format!("write: {e}")));
}
