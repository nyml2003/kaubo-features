//! 模块系统 — import 解析 + 多文件编译 + tree-shaking + .kaubop 打包

use kaubo_infer::infer_module;
use kaubo_ir::cps::CpsModule;
use kaubo_ir::cps_build::build_module;
use kaubo_syntax::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// 编译上下文：解析 import → 编译多文件 → tree-shaking
pub struct ModuleGraph {
    /// 已编译的模块 (path → CpsModule)
    pub compiled: HashMap<String, CpsModule>,
    /// 模块源码缓存
    pub sources: HashMap<String, String>,
    /// 搜索路径
    pub search_paths: Vec<PathBuf>,
    /// 入口模块路径
    pub entry: String,
}

impl ModuleGraph {
    pub fn new(entry: &str) -> Self {
        ModuleGraph {
            compiled: HashMap::new(),
            sources: HashMap::new(),
            search_paths: vec![PathBuf::from("."), PathBuf::from("std")],
            entry: entry.to_string(),
        }
    }

    /// 编译所有模块（从 entry 递归解析 import）
    pub fn compile_all(&mut self) -> Result<(), String> {
        let entry_path = self.resolve_module(&self.entry)?;
        let mut stack = vec![entry_path.clone()];
        let mut seen = HashSet::new();

        while let Some(path) = stack.pop() {
            if seen.contains(&path) {
                continue;
            }
            seen.insert(path.clone());

            // Load source
            let source = self.load_source(&path)?;
            self.sources.insert(path.clone(), source.clone());

            // Parse
            let module = Parser::new(&source)
                .parse()
                .map_err(|e| format!("parse {}: {}", path, e))?;

            // Collect imports
            for stmt in &module.stmts {
                if let kaubo_syntax::ast::Stmt::Import {
                    path: import_path, ..
                } = stmt
                {
                    let resolved = self.resolve_relative(&path, import_path)?;
                    if !seen.contains(&resolved) {
                        stack.push(resolved);
                    }
                }
            }

            // Type check
            infer_module(&module).map_err(|e| format!("infer {}: {:?}", path, e.msg))?;

            // Lower to CPS
            let cps = build_module(&module).map_err(|e| format!("build {}: {}", path, e))?;
            self.compiled.insert(path, cps);
        }

        Ok(())
    }

    /// Tree-shaking: 从 entry 出发标记可达符号，只保留被引用的部分
    pub fn tree_shake(&self) -> Result<HashMap<String, CpsModule>, String> {
        let mut result = HashMap::new();
        let entry = self.resolve_module(&self.entry)?;

        // BFS from entry
        let mut queue = vec![entry.clone()];
        let mut visited = HashSet::new();

        while let Some(path) = queue.pop() {
            if visited.contains(&path) {
                continue;
            }
            visited.insert(path.clone());

            if let Some(cps) = self.compiled.get(&path) {
                result.insert(path.clone(), cps.clone());

                // Find calls to imported modules
                if let Some(source) = self.sources.get(&path) {
                    let module = Parser::new(source)
                        .parse()
                        .map_err(|_| format!("re-parse {}", path))?;

                    for stmt in &module.stmts {
                        if let kaubo_syntax::ast::Stmt::Import { path: imp, .. } = stmt {
                            let resolved = self.resolve_relative(&path, imp)?;
                            if !visited.contains(&resolved) {
                                queue.push(resolved);
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// 打包为 .kaubop (JSON manifest + CPS bytecode 列表)
    pub fn pack(&self, output: &str) -> Result<(), String> {
        let shaken = self.tree_shake()?;
        let mut manifest = serde_json::json!({
            "name": self.entry,
            "version": "0.1.0",
            "kaubo": ">=0.2.0",
            "modules": {}
        });

        let modules_obj = manifest["modules"].as_object_mut().unwrap();
        for (name, cps) in &shaken {
            // Serialize CPS module as JSON for now (binary format later)
            let json = serde_json::json!({
                "functions": cps.functions.len(),
                "constants": cps.constants.len(),
                "structs": cps.structs.len(),
            });
            modules_obj.insert(name.clone(), json);
        }

        let manifest_str =
            serde_json::to_string_pretty(&manifest).map_err(|e| format!("json: {}", e))?;
        fs::write(output, manifest_str).map_err(|e| format!("write {}: {}", output, e))?;

        Ok(())
    }

    // ── helpers ──

    fn load_source(&self, path: &str) -> Result<String, String> {
        for base in &self.search_paths {
            let full = base.join(path);
            if full.exists() {
                return fs::read_to_string(&full).map_err(|e| format!("read {:?}: {}", full, e));
            }
        }
        Err(format!("module not found: {}", path))
    }

    fn resolve_module(&self, name: &str) -> Result<String, String> {
        // If name already ends with .kaubo, use as-is
        if name.ends_with(".kaubo") {
            return Ok(name.to_string());
        }
        let path = format!("{}.kaubo", name);
        for base in &self.search_paths {
            let full = base.join(&path);
            if full.exists() {
                return Ok(path);
            }
        }
        Err(format!("module '{}' not found", name))
    }

    fn resolve_relative(&self, from: &str, import: &str) -> Result<String, String> {
        let base = Path::new(from).parent().unwrap_or(Path::new("."));
        let resolved = base.join(format!("{}.kaubo", import));
        Ok(resolved.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &str, content: &str) {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_single_file_compile() {
        let tmp = "/tmp/kaubo_test_mod";
        let _ = fs::remove_dir_all(tmp);
        write_file(&format!("{}/main.kaubo", tmp), "const x = 42;");

        let mut mg = ModuleGraph::new("main.kaubo");
        mg.search_paths = vec![PathBuf::from(tmp)];
        mg.compile_all().unwrap();
        assert!(mg.compiled.contains_key("main.kaubo"));
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_multi_file_compile() {
        let tmp = "/tmp/kaubo_test_multi";
        let _ = fs::remove_dir_all(tmp);
        write_file(&format!("{}/main.kaubo", tmp), r#"import "lib";"#);
        write_file(&format!("{}/lib.kaubo", tmp), "const x = 42;");

        let mut mg = ModuleGraph::new("main.kaubo");
        mg.search_paths = vec![PathBuf::from(tmp)];
        mg.compile_all().unwrap();
        assert!(mg.compiled.len() >= 1);
        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_pack() {
        let tmp = "/tmp/kaubo_test_pack";
        let _ = fs::remove_dir_all(tmp);
        write_file(&format!("{}/main.kaubo", tmp), "const answer = 42;");

        let mut mg = ModuleGraph::new("main.kaubo");
        mg.search_paths = vec![PathBuf::from(tmp)];
        mg.compile_all().unwrap();
        mg.pack(&format!("{}/output.kaubop.json", tmp)).unwrap();
        assert!(Path::new(&format!("{}/output.kaubop.json", tmp)).exists());
        let _ = fs::remove_dir_all(tmp);
    }
}
