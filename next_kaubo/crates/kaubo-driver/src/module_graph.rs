//! 模块依赖图——纯语法 DFS + 拓扑排序 + 循环检测。
//!
//! `ModuleGraph` 只做轻量 Parser 提取 import 语句——不涉及类型检查、
//! CPS、缓存。图执行（`ModuleCompiler`）利用拓扑序按固定顺序编译。

use crate::export_table::RawImport;
use crate::module_loader::ModuleLoader;
use crate::protocol::BuildError;
use kaubo_ast::{Module, Stmt};
use kaubo_syntax::parser::Parser;
use std::collections::{HashMap, HashSet};

/// 模块依赖图。
///
/// 包含拓扑排序后的模块列表、各模块源码和导入信息。
#[derive(Debug, Clone)]
pub struct ModuleGraph {
    /// 拓扑序（叶子→根），保证每个模块在被编译时其依赖已就绪。
    pub order: Vec<String>,
    /// 路径 → 源码
    pub sources: HashMap<String, String>,
    /// 路径 → 原始导入列表
    pub imports: HashMap<String, Vec<RawImport>>,
    /// 路径 → 直接依赖路径列表
    pub deps: HashMap<String, Vec<String>>,
}

impl ModuleGraph {
    /// 从入口文件构建模块图。
    ///
    /// `entry` 是入口模块路径（相对于 loader）。
    /// `loader` 用于读取和解析路径。
    ///
    /// DFS 遍历所有传递依赖，检测循环，生成拓扑序。
    pub fn build(entry: &str, loader: &dyn ModuleLoader) -> Result<Self, BuildError> {
        let mut graph = Self {
            order: Vec::new(),
            sources: HashMap::new(),
            imports: HashMap::new(),
            deps: HashMap::new(),
        };
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        graph.dfs(entry, loader, &mut visited, &mut stack)?;
        // 后序即拓扑序（叶子在前），无需反转
        Ok(graph)
    }

    /// 深度优先遍历。
    fn dfs(
        &mut self,
        path: &str,
        loader: &dyn ModuleLoader,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
    ) -> Result<(), BuildError> {
        // 循环检测
        if let Some(pos) = stack.iter().position(|p| p == path) {
            let cycle: Vec<String> = stack[pos..].to_vec();
            return Err(BuildError::CircularImport { cycle });
        }

        // 已访问则跳过
        if visited.contains(path) {
            return Ok(());
        }

        stack.push(path.to_string());

        // 读取源文件
        let source = loader.read(path)?;

        // ★ 仅语法解析——不涉及类型检查/CPS
        let module = parse_for_imports(&source)?;

        // 收集原始导入信息
        let raw_imports = collect_raw_imports(&module);

        // 存储
        self.sources.insert(path.to_string(), source);
        self.imports.insert(path.to_string(), raw_imports.clone());

        // 对每个 import，解析路径，记录依赖，递归 DFS
        for imp in &raw_imports {
            let (dep_path, _) = loader.resolve(path, &imp.source_path)?;
            self.deps
                .entry(path.to_string())
                .or_default()
                .push(dep_path.clone());
            self.dfs(&dep_path, loader, visited, stack)?;
        }

        stack.pop();
        visited.insert(path.to_string());
        self.order.push(path.to_string());
        Ok(())
    }
}

/// 仅解析源码为 AST（纯语法，不做类型推导/CPS）。
///
/// 注意：图发现阶段可能遇到导入 struct 的字面量语法（如 `Point { x: 1 }`），
/// 此时 parser 尚未知道 Point 是 struct。为容忍此情况，
/// 使用文本扫描预先收集 struct 定义名称并注册到 parser。
fn parse_for_imports(source: &str) -> Result<Module, BuildError> {
    let mut parser = Parser::new(source);
    // 预扫描 struct 定义——收集文件中定义的 struct 名称
    let struct_names = scan_struct_defs(source);
    for name in &struct_names {
        parser.register_struct_name(name);
    }
    parser.parse().map_err(|e| BuildError::Parse(e.to_string()))
}

/// 文本扫描——提取源文件中 `struct Name { ... }` 的名称。
/// 这是一个轻量辅助函数，不做完整解析。
fn scan_struct_defs(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut chars = source.chars().peekable();
    let s: Vec<char> = source.chars().collect();

    // 简单状态机：查找 "struct" 关键字后的标识符
    let mut i = 0;
    while i < s.len() {
        // 跳过空白
        while i < s.len() && s[i].is_whitespace() {
            i += 1;
        }
        // 检查 "struct" 关键字
        if i + 6 <= s.len() && s[i..i + 6].iter().collect::<String>() == "struct" {
            // 确保是完整的单词（后面是空白）
            if i + 6 < s.len() && s[i + 6].is_whitespace() {
                i += 6;
                // 跳过空白
                while i < s.len() && s[i].is_whitespace() {
                    i += 1;
                }
                // 读取标识符
                let start = i;
                while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                    i += 1;
                }
                if i > start {
                    names.push(s[start..i].iter().collect());
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    names
}

/// 从 AST 收集原始导入请求。
///
/// 目前支持两种 import 形式：
/// - `import { names } from "path"` → RawImport { names: [...], source_path: "path" }
/// - `import "path" [as alias]` → 对带 alias 的不产生 RawImport（整个模块导入按 alias 使用）
pub fn collect_raw_imports(module: &Module) -> Vec<RawImport> {
    module
        .stmts
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Import { path, names, .. } if !names.is_empty() => Some(RawImport {
                names: names.clone(),
                source_path: path.clone(),
            }),
            // 整个模块导入（`import "path" as alias`）：暂不处理，Phase 3b 不做通配符
            Stmt::Import { .. } => None,
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_loader::MemLoader;

    #[test]
    fn single_file_no_imports() {
        let mut loader = MemLoader::new();
        loader.insert("main.kb", "const x = 42;");

        let graph = ModuleGraph::build("main.kb", &loader).unwrap();
        assert_eq!(graph.order, vec!["main.kb"]);
        assert!(graph.imports["main.kb"].is_empty());
        assert!(graph.deps.get("main.kb").map_or(true, |d| d.is_empty()));
    }

    #[test]
    fn simple_import_chain() {
        let mut loader = MemLoader::new();
        loader.insert("main.kb", "import { PI } from \"./math.kb\"; PI;");
        loader.insert("math.kb", "export const PI = 3.14;");

        let graph = ModuleGraph::build("main.kb", &loader).unwrap();
        // 叶子（math.kb）在前，根（main.kb）在后
        assert_eq!(graph.order[0], "math.kb");
        assert_eq!(graph.order[1], "main.kb");
        assert_eq!(graph.deps["main.kb"], vec!["math.kb"]);
    }

    #[test]
    fn diamond_dependency() {
        let mut loader = MemLoader::new();
        loader.insert(
            "main.kb",
            "import { a } from \"./A.kb\"; import { b } from \"./B.kb\";",
        );
        loader.insert(
            "A.kb",
            "import { c } from \"./base.kb\"; export const a = c;",
        );
        loader.insert(
            "B.kb",
            "import { c } from \"./base.kb\"; export const b = c;",
        );
        loader.insert("base.kb", "export const c = 42;");

        let graph = ModuleGraph::build("main.kb", &loader).unwrap();
        // base 必须是第一个（被 A 和 B 依赖）
        assert_eq!(graph.order[0], "base.kb");
        // A 和 B 在 base 之后，main 在最后
        assert_eq!(graph.order[3], "main.kb");
        // main 依赖 A 和 B
        assert!(graph.deps["main.kb"].contains(&"A.kb".to_string()));
        assert!(graph.deps["main.kb"].contains(&"B.kb".to_string()));
    }

    #[test]
    fn circular_dependency_detected() {
        let mut loader = MemLoader::new();
        loader.insert("a.kb", "import { b } from \"./b.kb\"; export const a = b;");
        loader.insert("b.kb", "import { a } from \"./a.kb\"; export const b = a;");

        let err = ModuleGraph::build("a.kb", &loader).unwrap_err();
        match err {
            BuildError::CircularImport { cycle } => {
                assert!(cycle.contains(&"a.kb".to_string()));
                assert!(cycle.contains(&"b.kb".to_string()));
            }
            _ => panic!("expected CircularImport error, got: {err:?}"),
        }
    }

    #[test]
    fn file_not_found() {
        let loader = MemLoader::new();
        let err = ModuleGraph::build("missing.kb", &loader).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn collect_raw_imports_extracts_named_imports() {
        let module = parse_for_imports("import { a, b } from \"./math.kb\";").unwrap();
        let raw = collect_raw_imports(&module);
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].names, vec!["a", "b"]);
        assert_eq!(raw[0].source_path, "./math.kb");
    }
}
