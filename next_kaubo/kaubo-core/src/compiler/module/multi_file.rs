//! 多文件编译支持
//!
//! 支持从入口文件开始，递归编译所有依赖的模块。

use kaubo_vfs::VirtualFileSystem;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::compiler::lexer::builder::build_lexer;
use crate::compiler::module::resolver::{ModuleResolver, ResolveError, ResolvedModule};
use crate::compiler::parser::module::Module;
use crate::compiler::parser::parser::Parser;
use crate::compiler::parser::stmt::{ImportStmt, StmtKind};

/// 多文件编译错误
#[derive(Debug, Clone)]
pub enum MultiFileError {
    /// 模块解析错误
    Resolve { path: String, error: ResolveError },
    /// 解析错误
    Parse { path: PathBuf, message: String },
    /// 循环依赖
    CircularDependency { chain: Vec<String> },
}

impl std::fmt::Display for MultiFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiFileError::Resolve { path, error } => {
                write!(f, "Failed to resolve '{}': {}", path, error)
            }
            MultiFileError::Parse { path, message } => {
                write!(f, "Failed to parse '{}': {}", path.display(), message)
            }
            MultiFileError::CircularDependency { chain } => {
                write!(f, "Circular dependency: {}", chain.join(" → "))
            }
        }
    }
}

impl std::error::Error for MultiFileError {}

/// 编译单元（一个文件）
#[derive(Debug, Clone)]
pub struct CompileUnit {
    /// 文件路径
    pub path: PathBuf,
    /// import 路径
    pub import_path: String,
    /// AST
    pub ast: Module,
    /// 源代码
    pub source: String,
    /// 依赖的模块（import 路径列表）
    pub dependencies: Vec<String>,
}

/// 多文件编译结果
#[derive(Debug)]
pub struct MultiFileCompileResult {
    /// 所有编译单元（按编译顺序）
    pub units: Vec<CompileUnit>,
    /// 入口模块
    pub entry: String,
}

/// 多文件编译器
pub struct MultiFileCompiler {
    /// 模块解析器
    resolver: ModuleResolver,
    /// 已解析的模块：import_path -> CompileUnit
    units: HashMap<String, CompileUnit>,
    /// 当前解析栈（用于循环依赖检测）
    resolve_stack: Vec<String>,
}

impl MultiFileCompiler {
    /// 创建新的多文件编译器
    pub fn new(vfs: Box<dyn VirtualFileSystem>, root_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            resolver: ModuleResolver::new(vfs, root_dir),
            units: HashMap::new(),
            resolve_stack: Vec::new(),
        }
    }

    /// 从入口文件开始编译
    ///
    /// # Arguments
    /// * `entry_path` - 入口文件路径（如 "main.kaubo" 或 "src/main.kaubo"）
    ///
    /// # Returns
    /// 编译结果，包含所有编译单元（按依赖顺序排列）
    pub fn compile_entry(
        &mut self,
        entry_path: impl AsRef<std::path::Path>,
    ) -> Result<MultiFileCompileResult, MultiFileError> {
        let entry_path = entry_path.as_ref();
        
        // 读取入口文件
        let entry_unit = self.load_and_parse_file(entry_path, "__entry__".to_string())?;
        let entry_imports = entry_unit.dependencies.clone();
        
        // 保存入口单元
        self.units.insert("__entry__".to_string(), entry_unit);
        
        // 递归解析所有依赖
        for import_path in entry_imports {
            self.resolve_dependency(&import_path)?;
        }
        
        // 拓扑排序获取编译顺序
        let sorted = self.topological_sort("__entry__")?;
        
        // 按顺序收集编译单元
        let mut units = Vec::new();
        for import_path in sorted {
            if import_path != "__entry__" {
                if let Some(unit) = self.units.get(&import_path) {
                    units.push(unit.clone());
                }
            }
        }
        
        // 入口单元放在最后
        if let Some(entry) = self.units.get("__entry__") {
            units.push(entry.clone());
        }
        
        Ok(MultiFileCompileResult {
            units,
            entry: "__entry__".to_string(),
        })
    }

    /// 递归解析依赖
    fn resolve_dependency(&mut self, import_path: &str) -> Result<(), MultiFileError> {
        // 已解析
        if self.units.contains_key(import_path) {
            return Ok(());
        }

        // 标准库是内置模块，不需要从文件系统加载
        // 它们由 VM 在运行时提供
        if import_path == "std" || import_path.starts_with("std.") {
            return Ok(());
        }

        // 检测循环依赖
        if self.resolve_stack.contains(&import_path.to_string()) {
            let mut chain = self.resolve_stack.clone();
            chain.push(import_path.to_string());
            return Err(MultiFileError::CircularDependency { chain });
        }

        // 推入栈
        self.resolve_stack.push(import_path.to_string());

        // 使用 resolver 解析模块
        let resolved = self
            .resolver
            .resolve(import_path)
            .map_err(|e| MultiFileError::Resolve {
                path: import_path.to_string(),
                error: e,
            })?;

        // 提取依赖
        let dependencies = extract_imports(&resolved.ast);

        // 创建编译单元
        let unit = CompileUnit {
            path: resolved.path.clone(),
            import_path: import_path.to_string(),
            ast: resolved.ast.clone(),
            source: resolved.source.clone(),
            dependencies: dependencies.clone(),
        };

        self.units.insert(import_path.to_string(), unit);

        // 递归解析子依赖
        for dep in dependencies {
            self.resolve_dependency(&dep)?;
        }

        // 弹出栈
        self.resolve_stack.pop();

        Ok(())
    }

    /// 从文件路径加载并解析（入口文件专用，直接读取）
    fn load_and_parse_file(
        &mut self,
        path: &std::path::Path,
        import_path: String,
    ) -> Result<CompileUnit, MultiFileError> {
        // 读取文件
        let content = self
            .resolver
            .resolve_direct(path, &import_path)
            .map_err(|e| MultiFileError::Resolve {
                path: import_path.clone(),
                error: e,
            })?;

        // 提取依赖
        let dependencies = extract_imports(&content.ast);

        Ok(CompileUnit {
            path: path.to_path_buf(),
            import_path,
            ast: content.ast,
            source: content.source,
            dependencies,
        })
    }

    /// 拓扑排序
    fn topological_sort(&self, entry: &str) -> Result<Vec<String>, MultiFileError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        fn visit(
            import_path: &str,
            units: &HashMap<String, CompileUnit>,
            visited: &mut HashSet<String>,
            temp_mark: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) -> Result<(), MultiFileError> {
            if temp_mark.contains(import_path) {
                // 发现循环依赖
                return Err(MultiFileError::CircularDependency {
                    chain: vec![import_path.to_string()],
                });
            }

            if visited.contains(import_path) {
                return Ok(());
            }

            temp_mark.insert(import_path.to_string());

            if let Some(unit) = units.get(import_path) {
                for dep in &unit.dependencies {
                    visit(dep, units, visited, temp_mark, result)?;
                }
            }

            temp_mark.remove(import_path);
            visited.insert(import_path.to_string());
            result.push(import_path.to_string());

            Ok(())
        }

        visit(entry, &self.units, &mut visited, &mut temp_mark, &mut result)?;

        Ok(result)
    }
}

/// 从 AST 提取所有 import 路径
fn extract_imports(ast: &Module) -> Vec<String> {
    let mut imports = Vec::new();

    for stmt in &ast.statements {
        if let StmtKind::Import(ImportStmt {
            module_path,
            items: _,
            alias: _,
        }) = stmt.as_ref()
        {
            imports.push(module_path.clone());
        }
    }

    imports
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_vfs::MemoryFileSystem;

    fn create_test_fs() -> MemoryFileSystem {
        MemoryFileSystem::with_files([
            ("/main.kaubo", b"import math; import utils; var x = 1;".to_vec()),
            ("/math.kaubo", b"var PI = 3.14;".to_vec()),
            ("/utils.kaubo", b"import math; var helper = 0;".to_vec()),
        ])
    }

    #[test]
    fn test_compile_entry() {
        let fs = create_test_fs();
        let mut compiler = MultiFileCompiler::new(Box::new(fs), "/");

        let result = compiler.compile_entry("/main.kaubo").unwrap();

        assert_eq!(result.units.len(), 3);
        
        // 验证顺序：math 和 utils 在 main 之前
        let paths: Vec<_> = result.units.iter().map(|u| u.import_path.clone()).collect();
        assert!(paths.contains(&"math".to_string()));
        assert!(paths.contains(&"utils".to_string()));
        assert_eq!(paths.last().unwrap(), "__entry__");
    }

    #[test]
    fn test_extract_imports() {
        let code = r#"
import math;
import std.list;
from utils import helper;
var x = 1;
"#;
        let mut lexer = build_lexer();
        lexer.feed(code.as_bytes()).unwrap();
        lexer.terminate().unwrap();
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().unwrap();

        let imports = extract_imports(&ast);
        assert_eq!(imports, vec!["math", "std.list", "utils"]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let fs = MemoryFileSystem::with_files([
            ("/a.kaubo", b"import b;".to_vec()),
            ("/b.kaubo", b"import c;".to_vec()),
            ("/c.kaubo", b"import a;".to_vec()), // 循环依赖
        ]);

        let mut compiler = MultiFileCompiler::new(Box::new(fs), "/");
        let result = compiler.compile_entry("/a.kaubo");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MultiFileError::CircularDependency { .. }));
    }
}
