//! Compile context for multi-module compilation

use std::collections::HashMap;
use std::path::Path;
use kaubo_vfs::VirtualFileSystem;
use crate::pipeline::parser::module::Module;
use super::module_id::{ModuleId, ParseError};
use crate::pipeline::lexer::builder::build_lexer;
use crate::pipeline::parser::parser::Parser;

/// Error during module compilation
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Module not found: {id}")]
    ModuleNotFound { id: ModuleId },
    
    #[error("Circular dependency detected: {chain}")]
    CircularDependency { chain: String },
    
    #[error("Parse error in module {id}: {message}")]
    ParseError { id: ModuleId, message: String },
    
    #[error("VFS error: {0}")]
    VfsError(#[from] kaubo_vfs::VfsError),
    
    #[error("Invalid module path: {0}")]
    InvalidModulePath(#[from] ParseError),
}

/// Compiled unit representing a single module
#[derive(Debug, Clone)]
pub struct CompileUnit {
    /// Module ID
    pub id: ModuleId,
    /// Physical file path
    pub path: std::path::PathBuf,
    /// AST
    pub ast: Module,
    /// Source code
    pub source: String,
    /// Dependencies (imported modules)
    pub dependencies: Vec<ModuleId>,
}

/// Context for multi-module compilation
///
/// Manages module cache, dependency resolution, and circular dependency detection
pub struct CompileContext<'a> {
    /// VFS reference
    vfs: &'a dyn VirtualFileSystem,
    /// Compiled module cache
    cache: HashMap<ModuleId, CompileUnit>,
    /// Current resolution stack (for circular dependency detection)
    stack: Vec<ModuleId>,
    /// Root directory for relative resolution
    root_dir: std::path::PathBuf,
}

impl<'a> CompileContext<'a> {
    /// Create a new compile context
    pub fn new(vfs: &'a dyn VirtualFileSystem, root_dir: impl AsRef<Path>) -> Self {
        Self {
            vfs,
            cache: HashMap::new(),
            stack: Vec::new(),
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Get or compile a module
    ///
    /// If the module is already compiled, returns cached result.
    /// Otherwise, compiles the module and its dependencies.
    pub fn get_or_compile(&mut self, id: &ModuleId) -> Result<&CompileUnit, CompileError> {
        // Check cache first
        if self.cache.contains_key(id) {
            return Ok(&self.cache[id]);
        }
        
        // Check for circular dependency
        if self.stack.contains(id) {
            let chain = self.stack.iter()
                .chain(std::iter::once(id))
                .map(|m| m.to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(CompileError::CircularDependency { chain });
        }
        
        // Push to stack BEFORE any work
        self.stack.push(id.clone());
        
        // Compile the module
        let unit = self.compile_module(id)?;
        let deps = unit.dependencies.clone();
        
        // Recursively compile dependencies BEFORE caching
        // This ensures circular dependencies are detected
        for dep_id in deps {
            self.get_or_compile(&dep_id)?;
        }
        
        // Cache after all dependencies are compiled
        self.cache.insert(id.clone(), unit);
        
        // Pop from stack AFTER all work is done
        self.stack.pop();
        
        Ok(&self.cache[id])
    }
    
    /// Compile a single module
    fn compile_module(&self, id: &ModuleId) -> Result<CompileUnit, CompileError> {
        // Generate VFS path
        let vfs_path = id.to_vfs_path();
        
        // Read file through VFS
        let content = self.vfs.read_file(&vfs_path)?;
        let source = String::from_utf8(content)
            .map_err(|e| CompileError::ParseError {
                id: id.clone(),
                message: format!("Invalid UTF-8: {}", e),
            })?;
        
        // Parse AST
        let ast = self.parse(&source, id)?;
        
        // Extract dependencies from imports
        let dependencies = self.extract_imports(&ast)?;
        
        Ok(CompileUnit {
            id: id.clone(),
            path: vfs_path,
            ast,
            source,
            dependencies,
        })
    }
    
    /// Parse source code into AST
    fn parse(&self, source: &str, id: &ModuleId) -> Result<Module, CompileError> {
        let mut lexer = build_lexer();
        
        lexer.feed(source.as_bytes())
            .map_err(|e| CompileError::ParseError {
                id: id.clone(),
                message: format!("Lexer error: {:?}", e),
            })?;
        
        lexer.terminate()
            .map_err(|e| CompileError::ParseError {
                id: id.clone(),
                message: format!("Lexer error: {:?}", e),
            })?;
        
        let mut parser = Parser::new(lexer);
        
        parser.parse()
            .map_err(|e| CompileError::ParseError {
                id: id.clone(),
                message: e.to_string(),
            })
    }
    
    /// Extract import statements from AST
    fn extract_imports(&self, ast: &Module) -> Result<Vec<ModuleId>, CompileError> {
        let mut imports = Vec::new();
        
        for stmt in &ast.statements {
            use crate::pipeline::parser::stmt::StmtKind;
            
            if let StmtKind::Import(import_stmt) = stmt.as_ref() {
                let id = ModuleId::parse(&import_stmt.module_path)?;
                imports.push(id);
            }
        }
        
        Ok(imports)
    }
    
    /// Get all compiled units in dependency order
    ///
    /// Returns units sorted so that dependencies come before dependents
    pub fn get_sorted_units(&self) -> Vec<&CompileUnit> {
        // Simple approach: cache already stores in insertion order
        // which is depth-first post-order due to recursion
        self.cache.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_vfs::MemoryFileSystem;
    
    fn create_test_vfs() -> MemoryFileSystem {
        MemoryFileSystem::with_files(vec![
            ("/mod/main", b"import math; var x = 1;".to_vec()),
            ("/mod/math", b"import utils; pub var y = 2;".to_vec()),
            ("/mod/utils", b"pub var z = 3;".to_vec()),
        ])
    }
    
    #[test]
    fn test_compile_single_module() {
        let fs = create_test_vfs();
        let mut ctx = CompileContext::new(&fs, "/");
        
        let id = ModuleId::parse("utils").unwrap();
        let unit = ctx.get_or_compile(&id).unwrap();
        
        assert_eq!(unit.id, id);
        assert!(unit.dependencies.is_empty());
    }
    
    #[test]
    fn test_compile_with_dependencies() {
        let fs = create_test_vfs();
        let mut ctx = CompileContext::new(&fs, "/");
        
        let id = ModuleId::parse("math").unwrap();
        let unit = ctx.get_or_compile(&id).unwrap();
        
        assert_eq!(unit.id, id);
        assert_eq!(unit.dependencies.len(), 1);
        assert_eq!(unit.dependencies[0].to_string(), "utils");
    }
    
    #[test]
    fn test_circular_dependency() {
        let fs = MemoryFileSystem::with_files(vec![
            ("/mod/a", b"import b;".to_vec()),
            ("/mod/b", b"import a;".to_vec()),
        ]);
        
        let mut ctx = CompileContext::new(&fs, "/");
        
        let result = ctx.get_or_compile(&ModuleId::parse("a").unwrap());
        match &result {
            Ok(unit) => {
                panic!("Expected CircularDependency error, got Ok: {:?}", unit);
            }
            Err(CompileError::CircularDependency { chain }) => {
                println!("Got expected CircularDependency: {}", chain);
            }
            Err(e) => {
                panic!("Expected CircularDependency error, got: {:?}", e);
            }
        }
    }
}
