//! 模块导入/导出数据结构。
//!
//! 提供导出表（ExportTable）、导入表（ImportTable）及其组成类型。
//! 这些数据结构被 ModuleGraph、ModuleCompiler、LinkStage 和 infer 共享。

use kaubo_infer::Type;
use std::collections::HashMap;
use std::sync::Arc;

// Re-use kaubo_ir's re-export of kaubo_cps to avoid adding a new dependency.
use kaubo_ir::cps::CpsModule;

// ── 导出项 ──

/// 单个导出项——按种类区分，携带完整的跨模块信息。
#[derive(Debug, Clone)]
pub enum ExportEntry {
    Const {
        name: String,
        ty: Type,
        /// 源模块中的 const_idx
        const_idx: usize,
    },
    Function {
        name: String,
        ty: Type,
        /// 源模块中的 func_idx（局部索引）
        func_idx: usize,
    },
    Struct {
        name: String,
        /// 完整的字段列表（跨模块边界不丢失）
        fields: Vec<(String, Type)>,
        /// 源模块中的 struct_id（局部索引）
        struct_id: usize,
    },
    Interface {
        name: String,
        /// 方法签名列表：(方法名, [(参数名, 参数类型)], 返回类型可选)
        methods: Vec<(String, Vec<(String, Type)>, Option<Type>)>,
    },
}

impl ExportEntry {
    /// 返回导出项的公开名称。
    pub fn export_name(&self) -> &str {
        match self {
            ExportEntry::Const { name, .. } => name,
            ExportEntry::Function { name, .. } => name,
            ExportEntry::Struct { name, .. } => name,
            ExportEntry::Interface { name, .. } => name,
        }
    }
}

// ── 导出表 ──

/// 整个模块的导出表。
///
/// 被导入方只暴露这张表，不暴露内部细节。
#[derive(Debug, Clone)]
pub struct ExportTable {
    /// 源模块路径
    pub source_path: String,
    /// 导出项列表（按声明顺序）
    pub entries: Vec<ExportEntry>,
    /// 本模块的导入表（LinkStage 用它解析本模块的 CallExternal）
    pub import_table: ImportTable,
    /// 源模块的 CpsModule（Arc 共享，避免深拷贝）
    pub cps_module: Arc<CpsModule>,
}

impl ExportTable {
    /// 按名称查找导出项。
    pub fn find_export(&self, name: &str) -> Option<&ExportEntry> {
        self.entries.iter().find(|e| e.export_name() == name)
    }
}

// ── 原始导入请求 ──

/// 原始导入请求（Parser 产物，不做路径解析）。
#[derive(Debug, Clone)]
pub struct RawImport {
    /// 导入名列表（源码中 `import { a, b } from …` 的名字）
    pub names: Vec<String>,
    /// 来源路径（源码字面量，未解析）
    pub source_path: String,
}

// ── 解析后的导入引用 ──

/// 解析后的导入引用（Infer 阶段产物）。
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// 本地名
    pub local_name: String,
    /// 来源路径（已解析的规范路径）
    pub source_path: String,
    /// 被导入条目（从 ExportTable 复制）
    pub entry: ExportEntry,
}

// ── 导入表 ──

/// 整个模块的导入表——双重索引。
#[derive(Debug, Clone)]
pub struct ImportTable {
    /// 按句柄索引（CPS/Link 阶段用）：handle → ResolvedImport
    pub entries: Vec<ResolvedImport>,
    /// 按本地名查找（Infer 阶段注入类型时用）：local_name → handle
    pub by_name: HashMap<String, usize>,
}

impl ImportTable {
    /// 创建空的导入表。
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    /// 检查导入表是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── 全局符号引用 ──

/// 链接后的全局符号引用类型。
#[derive(Debug, Clone)]
pub enum GlobalRef {
    Func(usize),
    Struct(usize),
    Const(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_cps() -> Arc<CpsModule> {
        Arc::new(CpsModule {
            functions: vec![],
            constants: vec![],
            structs: vec![],
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        })
    }

    #[test]
    fn export_entry_name() {
        let e = ExportEntry::Const {
            name: "answer".into(),
            ty: Type::Int64,
            const_idx: 0,
        };
        assert_eq!(e.export_name(), "answer");

        let e = ExportEntry::Function {
            name: "add".into(),
            ty: Type::Int64,
            func_idx: 1,
        };
        assert_eq!(e.export_name(), "add");
    }

    #[test]
    fn export_table_find_export() {
        let table = ExportTable {
            source_path: "math.kb".into(),
            entries: vec![
                ExportEntry::Const {
                    name: "PI".into(),
                    ty: Type::Float64,
                    const_idx: 0,
                },
                ExportEntry::Function {
                    name: "add".into(),
                    ty: Type::Arrow(Box::new(Type::Int64), Box::new(Type::Int64)),
                    func_idx: 0,
                },
            ],
            import_table: ImportTable::empty(),
            cps_module: dummy_cps(),
        };

        assert!(table.find_export("PI").is_some());
        assert!(table.find_export("add").is_some());
        assert!(table.find_export("nope").is_none());
    }

    #[test]
    fn import_table_empty() {
        let t = ImportTable::empty();
        assert!(t.is_empty());
        assert_eq!(t.entries.len(), 0);
        assert_eq!(t.by_name.len(), 0);
    }
}
