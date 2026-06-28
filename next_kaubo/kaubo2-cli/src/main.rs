//! kaubo2 — v2 direct driver: parse → infer → build → flatten → execute
use std::env;
use std::fs;
use std::time::Instant;

fn render_run(outcome: &kaubo_driver::RunOutcome) {
    for line in &outcome.output {
        println!("{line}");
    }
}

/// Parse CLI arguments and build a RunConfig.
///
/// Recognized flags (position-independent, before or after subcommand):
///   --log-level <LEVEL>        trace|debug|info|warn|error
///   --max-loop-iterations <N>  override the default 1_000_000 loop limit
///
/// Priority: CLI --log-level > KAUBO_LOG env var > default (no logging).
fn build_config(args: &[String]) -> kaubo_driver::RunConfig {
    let mut config = kaubo_driver::RunConfig::default();

    // Read KAUBO_LOG env var first (lower priority than CLI flag)
    let env_handler = kaubo_log_handlers::init_from_env();

    let mut cli_level: Option<kaubo_log::Severity> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--log-level" => {
                if let Some(val) = args.get(i + 1) {
                    cli_level = kaubo_log_handlers::parse_severity(val);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--max-loop-iterations" => {
                if let Some(val) = args.get(i + 1) {
                    if let Ok(n) = val.parse::<u64>() {
                        config.max_loop_iterations = n;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    // CLI --log-level overrides env var
    if let Some(level) = cli_level {
        config.events = Some(Box::new(kaubo_log_handlers::make_handler(level)));
    } else if let Some(handler) = env_handler {
        config.events = Some(Box::new(handler));
    }

    config
}

/// Collect positional (non-flag) arguments, skipping known flags and their values.
fn positional_args(args: &[String]) -> Vec<&str> {
    let mut pos = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--log-level" | "--max-loop-iterations" => {
                i += 2; // skip flag + value
            }
            other => {
                if !other.starts_with("--") {
                    pos.push(other);
                }
                i += 1;
            }
        }
    }
    pos
}

fn run_args(args: &[String]) -> Result<(), String> {
    let pos = positional_args(args);

    let (sub, file) = match pos.as_slice() {
        ["compile" | "run" | "bench" | "mod", f, ..] => (pos[0], *f),
        [f, ..] if !matches!(*f, "compile" | "run" | "bench" | "mod") => ("run", *f),
        _ => {
            return Err(
                "Usage: kaubo2 [--log-level <LEVEL>] [--max-loop-iterations <N>] [compile|run|bench|mod] <file> [iterations] [warmup]"
                    .to_string(),
            );
        }
    };

    let config = build_config(args);

    match sub {
        "compile" => {
            let source = fs::read_to_string(file).map_err(|e| format!("read {file}: {e}"))?;
            let cps =
                kaubo_driver::compile_source_with_config(&source, &config)
                    .map_err(|e| e.to_string())?;
            let out = file.replace(".kaubo", ".kauboc");
            let bytes = kaubo_driver::encode_module(&cps);
            let len = bytes.len();
            fs::write(&out, bytes).map_err(|e| format!("write {out}: {e}"))?;
            println!("Compiled: ({:.1}KB)", len as f64 / 1024.0);
        }
        "bench" => {
            let iterations: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
            let warmup: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2);
            let source = fs::read_to_string(file).map_err(|e| format!("read {file}: {e}"))?;

            // Compile once
            let t0 = Instant::now();
            let cps =
                kaubo_driver::compile_source_with_config(&source, &config)
                    .map_err(|e| e.to_string())?;
            let compile_ms = t0.elapsed().as_secs_f64() * 1000.0;

            let last_func = cps.functions.len() - 1;
            let reg_count = cps.functions[last_func].reg_count;
            let instr_count: usize = cps
                .functions
                .iter()
                .flat_map(|f| &f.blocks)
                .filter(|b| b.id != usize::MAX)
                .map(|b| b.instrs.len())
                .sum();

            let events = config.events.as_ref().map(|h| h.as_ref());

            // Warmup
            for _ in 0..warmup {
                let mut vm = kaubo_vm::VM::new();
                vm.max_loop_iterations = config.max_loop_iterations;
                vm.load(&cps).map_err(|e| format!("load: {e}"))?;
                let _ = vm.execute(last_func, reg_count, events);
            }

            // Measure
            let mut times = Vec::with_capacity(iterations);
            for _ in 0..iterations {
                let mut vm = kaubo_vm::VM::new();
                vm.max_loop_iterations = config.max_loop_iterations;
                vm.load(&cps).map_err(|e| format!("load: {e}"))?;
                let t0 = Instant::now();
                let result = vm
                    .execute(last_func, reg_count, events)
                    .map_err(|e| format!("{e:?}"))?;
                let run_ms = t0.elapsed().as_secs_f64() * 1000.0;
                times.push(run_ms);
            }

            let avg_us = times.iter().sum::<f64>() / times.len() as f64 * 1000.0;
            // Single-line output: avg_us instr_count compile_ms
            println!("{} {} {}", avg_us, instr_count, compile_ms);
        }
        "mod" => {
            // 多文件模块模式：以 file 所在目录为 root，file 为入口
            let abs = std::path::Path::new(file)
                .canonicalize()
                .map_err(|e| format!("cannot resolve {file}: {e}"))?;
            let root = abs.parent().unwrap_or_else(|| std::path::Path::new("."));
            let entry_name = abs
                .file_name()
                .unwrap()
                .to_str()
                .ok_or_else(|| format!("invalid entry file: {file}"))?;

            let vfs = kaubo_vfs::FsVfs::new(root);
            let loader = kaubo_driver::module_loader::FileLoader::new(Box::new(vfs));

            let mut coord = kaubo_driver::Coordinator::new()
                .with_max_loop_iterations(config.max_loop_iterations);

            let outcome = coord
                .run_file(entry_name, &loader)
                .map_err(|e| e.to_string())?;
            render_run(&outcome);
        }
        "run" => {
            if file.ends_with(".kauboc") {
                let bytes = fs::read(file).map_err(|e| format!("read {file}: {e}"))?;
                let cps = kaubo_driver::decode_module(&bytes).map_err(|e| e.to_string())?;
                let outcome =
                    kaubo_driver::run_module_with_config(&cps, &config)
                        .map_err(|e| e.to_string())?;
                render_run(&outcome);
            } else {
                let source =
                    fs::read_to_string(file).map_err(|e| format!("read {file}: {e}"))?;
                let outcome =
                    kaubo_driver::run_source_with_config(&source, &config)
                        .map_err(|e| e.to_string())?;
                render_run(&outcome);
            }
        }
        _ => {
            let source = fs::read_to_string(file).map_err(|e| format!("read {file}: {e}"))?;
            let outcome =
                kaubo_driver::run_source_with_config(&source, &config)
                    .map_err(|e| e.to_string())?;
            render_run(&outcome);
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    run_args(&args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_stem(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("kaubo_cli_{}_{}", name, std::process::id()))
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn run_src(src: &str) -> Result<i64, String> {
        kaubo_driver::run_source(src)
            .map(|outcome| outcome.result)
            .map_err(|e| e.to_string())
    }

    fn run_src_with_output(src: &str) -> Result<(i64, Vec<String>), String> {
        kaubo_driver::run_source(src)
            .map(|outcome| (outcome.result, outcome.output))
            .map_err(|e| e.to_string())
    }

    #[test]
    fn e2e_lit() {
        assert_eq!(run_src("const x = 42;").unwrap(), 42);
    }
    #[test]
    fn e2e_add() {
        assert_eq!(run_src("const x = 40 + 2;").unwrap_or(-1), 42);
    }
    #[test]
    fn e2e_sub() {
        assert_eq!(run_src("const x = 50 - 8;").unwrap_or(-1), 42);
    }
    #[test]
    fn e2e_mul() {
        assert_eq!(run_src("const x = 6 * 7;").unwrap_or(-1), 42);
    }
    #[test]
    fn e2e_if_else() {
        assert_eq!(
            run_src("const x = if (true) { 42 } else { 0 };").unwrap_or(-1),
            42
        );
    }
    #[test]
    fn e2e_if_false() {
        assert_eq!(
            run_src("const x = if (false) { 0 } else { 42 };").unwrap_or(-1),
            42
        );
    }

    // ── Phase 1: 新 e2e 测试（先红）──

    #[test]
    fn e2e_var_multi_stmt() {
        assert_eq!(run_src("var x = 10; var y = 32; x + y;").unwrap_or(-1), 42);
    }

    #[test]
    fn e2e_const_multi_stmt() {
        assert_eq!(
            run_src("const x = 10; const y = x + 22; y;").unwrap_or(-1),
            32
        );
    }

    #[test]
    fn e2e_while_skip() {
        assert_eq!(
            run_src("while (false) { const x = 0; }; const r = 5; r;").unwrap_or(-1),
            5
        );
    }

    #[test]
    fn e2e_while_count() {
        assert_eq!(
            run_src("var n = 0; while (n < 3) { n = n + 1; }; n;").unwrap_or(-1),
            3
        );
    }

    #[test]
    fn e2e_lambda_call() {
        assert_eq!(run_src("const f = |x| { x + 1 }; f(41);").unwrap_or(-1), 42);
    }

    #[test]
    fn e2e_print() {
        let result = run_src_with_output("print(\"hi\");").unwrap();
        assert!(
            result.1.iter().any(|s| s.contains("hi")),
            "output should contain 'hi', got {:?}",
            result.1
        );
    }

    #[test]
    fn cli_reports_usage_without_file() {
        let err = run_args(&args(&["kaubo2"])).unwrap_err();
        assert!(err.contains("Usage:"));
    }

    #[test]
    fn cli_compile_writes_binary_file() {
        let src = temp_stem("compile").with_extension("kaubo");
        let out = temp_stem("compile").with_extension("kauboc");
        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&out);
        fs::write(&src, "const x = 42;").unwrap();

        run_args(&args(&["kaubo2", "compile", src.to_str().unwrap()])).unwrap();

        assert!(out.exists());
        assert!(!fs::read(&out).unwrap().is_empty());

        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&out);
    }

    #[test]
    fn cli_run_source_file() {
        let src = temp_stem("run_source").with_extension("kaubo");
        let _ = fs::remove_file(&src);
        fs::write(&src, "const x = 42;").unwrap();

        run_args(&args(&["kaubo2", "run", src.to_str().unwrap()])).unwrap();

        let _ = fs::remove_file(&src);
    }

    #[test]
    fn cli_run_compiled_file() {
        let src = temp_stem("run_compiled").with_extension("kaubo");
        let out = temp_stem("run_compiled").with_extension("kauboc");
        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&out);
        fs::write(&src, "const x = 42;").unwrap();

        run_args(&args(&["kaubo2", "compile", src.to_str().unwrap()])).unwrap();
        run_args(&args(&["kaubo2", "run", out.to_str().unwrap()])).unwrap();

        let _ = fs::remove_file(&src);
        let _ = fs::remove_file(&out);
    }

    #[test]
    fn cli_default_subcommand_runs_file() {
        let src = temp_stem("default_run").with_extension("kaubo");
        let _ = fs::remove_file(&src);
        fs::write(&src, "const x = 42;").unwrap();

        run_args(&args(&["kaubo2", src.to_str().unwrap()])).unwrap();

        let _ = fs::remove_file(&src);
    }

    #[test]
    fn cli_read_error_mentions_file() {
        let missing = temp_stem("missing").with_extension("kaubo");
        let _ = fs::remove_file(&missing);

        let err =
            run_args(&args(&["kaubo2", "run", missing.to_str().unwrap()])).unwrap_err();
        assert!(err.contains("read"));
        assert!(err.contains(missing.to_str().unwrap()));
    }

    #[test]
    fn cli_log_level_flag_is_parsed() {
        // --log-level with --max-loop-iterations should not crash
        let src = temp_stem("log_level").with_extension("kaubo");
        let _ = fs::remove_file(&src);
        fs::write(&src, "const x = 42;").unwrap();

        run_args(&args(&[
            "kaubo2",
            "--log-level",
            "debug",
            "--max-loop-iterations",
            "5000",
            "run",
            src.to_str().unwrap(),
        ]))
        .unwrap();

        let _ = fs::remove_file(&src);
    }

    #[test]
    fn debug_impl_dis() {
        let src = r#"
	struct Point { x: Int64, y: Int64 };
	impl Point {
	  dis: |self: Point, other: Point| -> Float64 {
	    const dx = (self.x - other.x);
	    const dy = (self.y - other.y);
	    return sqrt((dx*dx + dy*dy).to_float());
	  }
	};
	const p1 = Point { x: 200, y: 300 };
	const p2 = Point { x: 300, y: 400 };
	print(p1.dis(p2).to_string());
	"#;
        let cps = kaubo_driver::compile_source(src).unwrap();
        eprintln!("=== CPS DUMP ===");
        for (fi, func) in cps.functions.iter().enumerate() {
            eprintln!(
                "fn {} '{}' entry={} regs={}",
                fi, func.name, func.entry, func.reg_count
            );
            for b in func.blocks.iter().filter(|b| b.id != usize::MAX) {
                eprintln!(
                    "  blk{} {}: {:?} | {:?}",
                    b.id,
                    if b.id == func.entry { "(entry)" } else { "" },
                    b.instrs,
                    b.term
                );
            }
        }
        if cps.functions.is_empty() {
            panic!("no funcs");
        }
        let mut vm = kaubo_vm::VM::new();
        vm.load(&cps).unwrap();
        let e = cps.functions.len() - 1;
        let r = vm.execute(e, cps.functions[e].reg_count, None);
        eprintln!("[DEBUG] result={:?} output={:?}", r, vm.output);
    }
}
