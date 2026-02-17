//! Section 数据结构定义
//!
//! 定义各个 section 的具体数据格式：
//! - String Pool: 全局字符串去重存储
//! - Module Table: 模块元数据表
//! - Chunk Data: 字节码和常量池
//! - Shape Table: Struct shape 定义
//! - Export Table: 导出符号表
//! - Import Table: 导入依赖表

use super::section::{SectionData, SectionError, SectionKind};

// ==================== String Pool ====================

/// 字符串池
#[derive(Debug, Clone)]
pub struct StringPool {
    /// 字符串数据（以 null 分隔）
    data: Vec<u8>,
    /// 字符串偏移映射：索引 -> 偏移
    offsets: Vec<u32>,
}

impl StringPool {
    /// 创建空的字符串池
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            offsets: Vec::new(),
        }
    }

    /// 添加字符串，返回索引
    pub fn add(&mut self, s: &str) -> u32 {
        let bytes = s.as_bytes();

        // 查找是否已存在
        for (idx, &offset) in self.offsets.iter().enumerate() {
            let existing = self.get_bytes_at(offset as usize);
            if existing == bytes {
                return idx as u32;
            }
        }

        // 添加新字符串
        let idx = self.offsets.len() as u32;
        let offset = self.data.len() as u32;
        self.offsets.push(offset);
        self.data.extend_from_slice(bytes);
        self.data.push(0); // null 终止

        idx
    }

    /// 获取字符串
    pub fn get(&self, idx: u32) -> Option<&str> {
        let idx = idx as usize;
        if idx >= self.offsets.len() {
            return None;
        }

        let offset = self.offsets[idx] as usize;
        let bytes = self.get_bytes_at(offset);
        std::str::from_utf8(bytes).ok()
    }

    /// 获取字节数组（内部使用）
    fn get_bytes_at(&self, offset: usize) -> &[u8] {
        let end = self.data[offset..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.data.len() - offset);
        &self.data[offset..offset + end]
    }

    /// 获取字符串数量
    pub fn len(&self) -> usize {
        self.offsets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }
}

impl SectionData for StringPool {
    fn serialize(&self) -> Vec<u8> {
        // 格式：
        // - count: u32 (字符串数量)
        // - offsets: [u32; count] (偏移数组)
        // - data_len: u32 (数据长度)
        // - data: [u8; data_len] (字符串数据)

        let mut result = Vec::new();

        // count
        result.extend_from_slice(&(self.offsets.len() as u32).to_le_bytes());

        // offsets
        for &offset in &self.offsets {
            result.extend_from_slice(&offset.to_le_bytes());
        }

        // data_len
        result.extend_from_slice(&(self.data.len() as u32).to_le_bytes());

        // data
        result.extend_from_slice(&self.data);

        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 8 {
            return Err(SectionError::TooShort);
        }

        let mut offset = 0;

        // count
        let count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // offsets
        if bytes.len() < offset + count * 4 + 4 {
            return Err(SectionError::TooShort);
        }

        let mut offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let off = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offsets.push(off);
            offset += 4;
        }

        // data_len
        let data_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // data
        if bytes.len() < offset + data_len {
            return Err(SectionError::TooShort);
        }

        let data = bytes[offset..offset + data_len].to_vec();

        Ok(Self { data, offsets })
    }

    fn section_kind() -> SectionKind {
        SectionKind::StringPool
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Module Table ====================

/// 模块表条目
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    /// 模块名（在字符串池中的索引）
    pub name_idx: u32,
    /// 源文件路径（在字符串池中的索引）
    pub source_path_idx: u32,
    /// Chunk 数据起始偏移（相对于 Chunk Data section）
    pub chunk_offset: u32,
    /// Chunk 数据大小
    pub chunk_size: u32,
    /// Shape 表起始索引
    pub shape_start: u32,
    /// Shape 数量
    pub shape_count: u32,
    /// 导出项起始索引
    pub export_start: u32,
    /// 导出项数量
    pub export_count: u32,
    /// 导入项起始索引
    pub import_start: u32,
    /// 导入项数量
    pub import_count: u32,
}

impl ModuleEntry {
    /// 条目大小：48 bytes
    pub const ENTRY_SIZE: usize = 48;

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 48] {
        let mut bytes = [0u8; 48];
        let mut offset = 0;

        bytes[offset..offset + 4].copy_from_slice(&self.name_idx.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.source_path_idx.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.chunk_offset.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.chunk_size.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.shape_start.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.shape_count.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.export_start.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.export_count.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.import_start.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.import_count.to_le_bytes());

        bytes
    }

    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 48 {
            return Err(SectionError::TooShort);
        }

        let mut offset = 0;

        let name_idx = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let source_path_idx = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let chunk_offset = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let chunk_size = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let shape_start = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let shape_count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let export_start = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let export_count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let import_start = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let import_count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);

        Ok(Self {
            name_idx,
            source_path_idx,
            chunk_offset,
            chunk_size,
            shape_start,
            shape_count,
            export_start,
            export_count,
            import_start,
            import_count,
        })
    }
}

/// 模块表
#[derive(Debug, Clone)]
pub struct ModuleTable {
    pub entries: Vec<ModuleEntry>,
}

impl ModuleTable {
    /// 创建空的模块表
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 添加模块条目
    pub fn add(&mut self, entry: ModuleEntry) -> u32 {
        let idx = self.entries.len() as u32;
        self.entries.push(entry);
        idx
    }

    /// 获取模块条目
    pub fn get(&self, idx: u32) -> Option<&ModuleEntry> {
        self.entries.get(idx as usize)
    }
}

impl SectionData for ModuleTable {
    fn serialize(&self) -> Vec<u8> {
        // count + entries
        let mut result = Vec::with_capacity(4 + self.entries.len() * ModuleEntry::ENTRY_SIZE);
        result.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            result.extend_from_slice(&entry.to_bytes());
        }
        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

        if bytes.len() < 4 + count * ModuleEntry::ENTRY_SIZE {
            return Err(SectionError::TooShort);
        }

        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let start = 4 + i * ModuleEntry::ENTRY_SIZE;
            let entry = ModuleEntry::from_bytes(&bytes[start..start + ModuleEntry::ENTRY_SIZE])?;
            entries.push(entry);
        }

        Ok(Self { entries })
    }

    fn section_kind() -> SectionKind {
        SectionKind::ModuleTable
    }
}

impl Default for ModuleTable {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Export Table ====================

/// 导出条目
#[derive(Debug, Clone)]
pub struct ExportEntry {
    /// 名称（在字符串池中的索引）
    pub name_idx: u32,
    /// 导出类型
    pub kind: ExportKind,
    /// 值类型（在字符串池中的索引，类型描述）
    pub type_idx: u32,
    /// 常量池索引（如果是函数/变量）
    pub const_idx: u32,
    /// 模块索引
    pub module_idx: u32,
}

/// 导出类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    /// 变量
    Var = 0x01,
    /// 函数
    Function = 0x02,
    /// 结构体
    Struct = 0x03,
    /// 模块
    Module = 0x04,
}

impl ExportEntry {
    /// 条目大小：20 bytes
    pub const ENTRY_SIZE: usize = 20;

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0..4].copy_from_slice(&self.name_idx.to_le_bytes());
        bytes[4] = self.kind as u8;
        bytes[5..8].copy_from_slice(&[0, 0, 0]); // padding
        bytes[8..12].copy_from_slice(&self.type_idx.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.const_idx.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.module_idx.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 20 {
            return Err(SectionError::TooShort);
        }

        let name_idx = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let kind = match bytes[4] {
            0x01 => ExportKind::Var,
            0x02 => ExportKind::Function,
            0x03 => ExportKind::Struct,
            0x04 => ExportKind::Module,
            n => return Err(SectionError::InvalidKind(n)),
        };
        let type_idx = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let const_idx = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let module_idx = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        Ok(Self {
            name_idx,
            kind,
            type_idx,
            const_idx,
            module_idx,
        })
    }
}

/// 导出表
#[derive(Debug, Clone)]
pub struct ExportTable {
    pub entries: Vec<ExportEntry>,
}

impl ExportTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: ExportEntry) -> u32 {
        let idx = self.entries.len() as u32;
        self.entries.push(entry);
        idx
    }
}

impl SectionData for ExportTable {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(4 + self.entries.len() * ExportEntry::ENTRY_SIZE);
        result.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            result.extend_from_slice(&entry.to_bytes());
        }
        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

        if bytes.len() < 4 + count * ExportEntry::ENTRY_SIZE {
            return Err(SectionError::TooShort);
        }

        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let start = 4 + i * ExportEntry::ENTRY_SIZE;
            let entry = ExportEntry::from_bytes(&bytes[start..start + ExportEntry::ENTRY_SIZE])?;
            entries.push(entry);
        }

        Ok(Self { entries })
    }

    fn section_kind() -> SectionKind {
        SectionKind::ExportTable
    }
}

impl Default for ExportTable {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Import Table ====================

/// 导入条目
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// 模块路径（在字符串池中的索引）
    pub module_path_idx: u32,
    /// 导入项名称（在字符串池中的索引，可选）
    pub name_idx: u32,
    /// 本地别名（在字符串池中的索引）
    pub alias_idx: u32,
    /// 导入类型
    pub kind: ImportKind,
    /// 目标模块索引（链接后填充）
    pub target_module_idx: u32,
}

/// 导入类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    /// 整个模块
    Module = 0x01,
    /// 特定项
    Item = 0x02,
}

impl ImportEntry {
    /// 条目大小：20 bytes
    pub const ENTRY_SIZE: usize = 20;

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0..4].copy_from_slice(&self.module_path_idx.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.name_idx.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.alias_idx.to_le_bytes());
        bytes[12] = self.kind as u8;
        bytes[13..16].copy_from_slice(&[0, 0, 0]); // padding
        bytes[16..20].copy_from_slice(&self.target_module_idx.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 20 {
            return Err(SectionError::TooShort);
        }

        let module_path_idx = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let name_idx = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let alias_idx = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let kind = match bytes[12] {
            0x01 => ImportKind::Module,
            0x02 => ImportKind::Item,
            n => return Err(SectionError::InvalidKind(n)),
        };
        let target_module_idx = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

        Ok(Self {
            module_path_idx,
            name_idx,
            alias_idx,
            kind,
            target_module_idx,
        })
    }
}

/// 导入表
#[derive(Debug, Clone)]
pub struct ImportTable {
    pub entries: Vec<ImportEntry>,
}

impl ImportTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: ImportEntry) -> u32 {
        let idx = self.entries.len() as u32;
        self.entries.push(entry);
        idx
    }
}

impl SectionData for ImportTable {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(4 + self.entries.len() * ImportEntry::ENTRY_SIZE);
        result.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            result.extend_from_slice(&entry.to_bytes());
        }
        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

        if bytes.len() < 4 + count * ImportEntry::ENTRY_SIZE {
            return Err(SectionError::TooShort);
        }

        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let start = 4 + i * ImportEntry::ENTRY_SIZE;
            let entry = ImportEntry::from_bytes(&bytes[start..start + ImportEntry::ENTRY_SIZE])?;
            entries.push(entry);
        }

        Ok(Self { entries })
    }

    fn section_kind() -> SectionKind {
        SectionKind::ImportTable
    }
}

impl Default for ImportTable {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Function Pool ====================

/// 函数字面量条目
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    /// 函数名（在字符串池中的索引，匿名函数可为 0）
    pub name_idx: u32,
    /// 参数数量
    pub arity: u8,
    /// Chunk 数据（已编码的 chunk 字节）
    pub chunk_data: Vec<u8>,
}

impl FunctionEntry {
    /// 条目头部大小：9 bytes (不含 chunk_data)
    pub const HEADER_SIZE: usize = 9;

    /// 序列化为字节数组
    /// 格式：name_idx(4) + arity(1) + chunk_len(4) + chunk_data
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(Self::HEADER_SIZE + self.chunk_data.len());
        result.extend_from_slice(&self.name_idx.to_le_bytes());
        result.push(self.arity);
        result.extend_from_slice(&(self.chunk_data.len() as u32).to_le_bytes());
        result.extend_from_slice(&self.chunk_data);
        result
    }

    /// 从字节数组反序列化
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err(SectionError::TooShort);
        }

        let name_idx = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let arity = bytes[4];
        let chunk_len = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;

        if bytes.len() < Self::HEADER_SIZE + chunk_len {
            return Err(SectionError::TooShort);
        }

        let chunk_data = bytes[Self::HEADER_SIZE..Self::HEADER_SIZE + chunk_len].to_vec();

        Ok(Self {
            name_idx,
            arity,
            chunk_data,
        })
    }
}

/// 函数池 - 存储所有函数字面量
#[derive(Debug, Clone)]
pub struct FunctionPool {
    pub entries: Vec<FunctionEntry>,
}

impl FunctionPool {
    /// 创建空的函数池
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 添加函数条目，返回索引
    pub fn add(&mut self, entry: FunctionEntry) -> u32 {
        let idx = self.entries.len() as u32;
        self.entries.push(entry);
        idx
    }

    /// 获取函数条目
    pub fn get(&self, idx: u32) -> Option<&FunctionEntry> {
        self.entries.get(idx as usize)
    }

    /// 获取函数数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl SectionData for FunctionPool {
    fn serialize(&self) -> Vec<u8> {
        // 格式：count(4) + [entry_size(4) + entry_data...]
        let mut result = Vec::new();
        result.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        for entry in &self.entries {
            let entry_bytes = entry.serialize();
            result.extend_from_slice(&(entry_bytes.len() as u32).to_le_bytes());
            result.extend_from_slice(&entry_bytes);
        }

        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

        let mut entries = Vec::with_capacity(count);
        let mut offset = 4;

        for _ in 0..count {
            if bytes.len() < offset + 4 {
                return Err(SectionError::TooShort);
            }

            let entry_len = u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]) as usize;
            offset += 4;

            if bytes.len() < offset + entry_len {
                return Err(SectionError::TooShort);
            }

            let entry = FunctionEntry::deserialize(&bytes[offset..offset + entry_len])?;
            entries.push(entry);
            offset += entry_len;
        }

        Ok(Self { entries })
    }

    fn section_kind() -> SectionKind {
        SectionKind::FunctionPool
    }
}

impl Default for FunctionPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_pool() {
        let mut pool = StringPool::new();

        let idx1 = pool.add("hello");
        let idx2 = pool.add("world");
        let idx3 = pool.add("hello"); // 重复

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // 返回已存在的索引

        assert_eq!(pool.get(idx1), Some("hello"));
        assert_eq!(pool.get(idx2), Some("world"));
        assert_eq!(pool.len(), 2);

        // 序列化/反序列化
        let bytes = pool.serialize();
        let pool2 = StringPool::deserialize(&bytes).unwrap();
        assert_eq!(pool2.get(0), Some("hello"));
        assert_eq!(pool2.get(1), Some("world"));
    }

    #[test]
    fn test_module_entry() {
        let entry = ModuleEntry {
            name_idx: 1,
            source_path_idx: 2,
            chunk_offset: 128,
            chunk_size: 256,
            shape_start: 0,
            shape_count: 3,
            export_start: 0,
            export_count: 2,
            import_start: 0,
            import_count: 1,
        };

        let bytes = entry.to_bytes();
        let entry2 = ModuleEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.name_idx, entry2.name_idx);
        assert_eq!(entry.chunk_offset, entry2.chunk_offset);
        assert_eq!(entry.chunk_size, entry2.chunk_size);
    }

    #[test]
    fn test_export_entry() {
        let entry = ExportEntry {
            name_idx: 1,
            kind: ExportKind::Function,
            type_idx: 2,
            const_idx: 3,
            module_idx: 0,
        };

        let bytes = entry.to_bytes();
        let entry2 = ExportEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.name_idx, entry2.name_idx);
        assert_eq!(entry.kind, entry2.kind);
        assert_eq!(entry.const_idx, entry2.const_idx);
    }

    #[test]
    fn test_import_entry() {
        let entry = ImportEntry {
            module_path_idx: 1,
            name_idx: 2,
            alias_idx: 3,
            kind: ImportKind::Item,
            target_module_idx: 0,
        };

        let bytes = entry.to_bytes();
        let entry2 = ImportEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.module_path_idx, entry2.module_path_idx);
        assert_eq!(entry.kind, entry2.kind);
    }

    #[test]
    fn test_export_table() {
        let mut table = ExportTable::new();
        table.add(ExportEntry {
            name_idx: 1,
            kind: ExportKind::Var,
            type_idx: 0,
            const_idx: 0,
            module_idx: 0,
        });
        table.add(ExportEntry {
            name_idx: 2,
            kind: ExportKind::Function,
            type_idx: 0,
            const_idx: 1,
            module_idx: 0,
        });

        let bytes = table.serialize();
        let table2 = ExportTable::deserialize(&bytes).unwrap();
        assert_eq!(table2.entries.len(), 2);
    }
}
