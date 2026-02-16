//! 编译期上下文和类型信息定义

use std::collections::HashMap;

/// 导出项信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Export {
    pub name: String,
    pub is_public: bool, // pub 修饰
    pub local_idx: u8,   // 对应的局部变量索引
    pub shape_id: u16,   // ShapeID（编译期确定的静态索引）
}

/// 模块信息（编译时）
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub exports: Vec<Export>,
    pub export_name_to_shape_id: HashMap<String, u16>, // 名称到 ShapeID 的映射
}

/// Struct 编译期信息
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub shape_id: u16,
    pub field_names: Vec<String>,  // 字段名列表（索引即字段位置）
    pub method_names: Vec<String>, // 方法名列表（来自 impl 块）
}

/// 变量类型信息
#[derive(Debug, Clone)]
pub enum VarType {
    Struct(String), // struct 类型名
}
