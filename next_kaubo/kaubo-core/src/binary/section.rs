//! Section 定义和管理
//!
//! Section Directory 管理文件中所有 section 的偏移和大小

/// Section 类型
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    /// 字符串池
    StringPool = 0x01,
    /// 模块表
    ModuleTable = 0x02,
    /// Chunk 数据
    ChunkData = 0x03,
    /// Shape 表
    ShapeTable = 0x04,
    /// 导出表
    ExportTable = 0x05,
    /// 导入表
    ImportTable = 0x06,
    /// 重定位信息
    Relocation = 0x07,
    /// 调试信息
    DebugInfo = 0x08,
    /// Source Map
    SourceMap = 0x09,
    /// 签名
    Signature = 0x0A,
    /// 函数池（存储函数字面量）
    FunctionPool = 0x0B,
}

impl SectionKind {
    /// 从 u8 转换
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(SectionKind::StringPool),
            0x02 => Some(SectionKind::ModuleTable),
            0x03 => Some(SectionKind::ChunkData),
            0x04 => Some(SectionKind::ShapeTable),
            0x05 => Some(SectionKind::ExportTable),
            0x06 => Some(SectionKind::ImportTable),
            0x07 => Some(SectionKind::Relocation),
            0x08 => Some(SectionKind::DebugInfo),
            0x09 => Some(SectionKind::SourceMap),
            0x0A => Some(SectionKind::Signature),
            0x0B => Some(SectionKind::FunctionPool),
            _ => None,
        }
    }
}

/// Section Directory 条目 (16 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionEntry {
    /// Section 类型 (1 byte)
    pub kind: SectionKind,
    /// 对齐填充 (1 byte)
    pub padding: u8,
    /// 标志 (2 bytes)
    pub flags: u16,
    /// 在文件中的偏移 (4 bytes)
    pub offset: u32,
    /// 解压后大小 (4 bytes)
    pub size: u32,
    /// 压缩后大小 (4 bytes): 0 表示未压缩
    pub compressed_size: u32,
}

impl SectionEntry {
    /// 条目大小: 16 bytes
    pub const ENTRY_SIZE: usize = 16;

    /// 创建新的 section 条目
    pub fn new(kind: SectionKind, offset: u32, size: u32) -> Self {
        Self {
            kind,
            padding: 0,
            flags: 0,
            offset,
            size,
            compressed_size: 0,
        }
    }

    /// 设置压缩大小
    pub fn set_compressed(&mut self, compressed_size: u32) {
        self.compressed_size = compressed_size;
        self.flags |= SectionFlags::COMPRESSED;
    }

    /// 是否已压缩
    pub fn is_compressed(&self) -> bool {
        (self.flags & SectionFlags::COMPRESSED) != 0
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0] = self.kind as u8;
        bytes[1] = self.padding;
        bytes[2..4].copy_from_slice(&self.flags.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.offset.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.size.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.compressed_size.to_le_bytes());
        bytes
    }

    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 16 {
            return Err(SectionError::TooShort);
        }

        let kind = SectionKind::from_u8(bytes[0])
            .ok_or(SectionError::InvalidKind(bytes[0]))?;
        let padding = bytes[1];
        let flags = u16::from_le_bytes([bytes[2], bytes[3]]);
        let offset = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let compressed_size = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        Ok(Self {
            kind,
            padding,
            flags,
            offset,
            size,
            compressed_size,
        })
    }
}

/// Section 标志位
pub struct SectionFlags;

impl SectionFlags {
    /// 已压缩
    pub const COMPRESSED: u16 = 0x0001;
    /// 加密（预留）
    pub const ENCRYPTED: u16 = 0x0002;
    /// 内存对齐要求（预留）
    pub const ALIGN_4K: u16 = 0x0004;
}

/// Section Directory
#[derive(Debug, Clone)]
pub struct SectionDirectory {
    /// Section 条目列表
    pub entries: Vec<SectionEntry>,
}

impl SectionDirectory {
    /// 创建空的 Section Directory
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 添加 section 条目
    pub fn add(&mut self, entry: SectionEntry) {
        self.entries.push(entry);
    }

    /// 查找指定类型的 section
    pub fn find(&self, kind: SectionKind) -> Option<&SectionEntry> {
        self.entries.iter().find(|e| e.kind == kind)
    }

    /// 查找指定类型的 section（可变）
    pub fn find_mut(&mut self, kind: SectionKind) -> Option<&mut SectionEntry> {
        self.entries.iter_mut().find(|e| e.kind == kind)
    }

    /// 获取 section 数量
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// 计算序列化后的大小
    pub fn serialized_size(&self) -> usize {
        self.entries.len() * SectionEntry::ENTRY_SIZE
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.serialized_size());
        for entry in &self.entries {
            bytes.extend_from_slice(&entry.to_bytes());
        }
        bytes
    }

    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() % SectionEntry::ENTRY_SIZE != 0 {
            return Err(SectionError::InvalidSize);
        }

        let count = bytes.len() / SectionEntry::ENTRY_SIZE;
        let mut entries = Vec::with_capacity(count);

        for i in 0..count {
            let start = i * SectionEntry::ENTRY_SIZE;
            let entry = SectionEntry::from_bytes(&bytes[start..start + SectionEntry::ENTRY_SIZE])?;
            entries.push(entry);
        }

        Ok(Self { entries })
    }

    /// 计算所有 section 数据结束后的文件大小
    pub fn total_file_size(&self) -> u32 {
        let mut max_end = 0u32;
        for entry in &self.entries {
            let end = entry.offset + if entry.compressed_size > 0 {
                entry.compressed_size
            } else {
                entry.size
            };
            if end > max_end {
                max_end = end;
            }
        }
        max_end
    }
}

impl Default for SectionDirectory {
    fn default() -> Self {
        Self::new()
    }
}

/// Section 错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionError {
    /// 数据太短
    TooShort,
    /// 无效的 section 类型
    InvalidKind(u8),
    /// 无效的数据大小
    InvalidSize,
}

impl std::fmt::Display for SectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SectionError::TooShort => write!(f, "Section data too short"),
            SectionError::InvalidKind(k) => write!(f, "Invalid section kind: {}", k),
            SectionError::InvalidSize => write!(f, "Invalid section data size"),
        }
    }
}

impl std::error::Error for SectionError {}

/// Section 数据 trait
///
/// 所有 section 数据类型需要实现这个 trait 来支持序列化和反序列化
pub trait SectionData: Sized {
    /// 序列化为字节数组
    fn serialize(&self) -> Vec<u8>;
    /// 从字节数组反序列化
    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError>;
    /// 获取 section 类型
    fn section_kind() -> SectionKind;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_entry_roundtrip() {
        let entry = SectionEntry::new(SectionKind::StringPool, 128, 1024);
        let bytes = entry.to_bytes();
        assert_eq!(bytes.len(), 16);

        let parsed = SectionEntry::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.kind, SectionKind::StringPool);
        assert_eq!(parsed.offset, 128);
        assert_eq!(parsed.size, 1024);
        assert_eq!(parsed.compressed_size, 0);
    }

    #[test]
    fn test_section_entry_compressed() {
        let mut entry = SectionEntry::new(SectionKind::ChunkData, 256, 2048);
        entry.set_compressed(512);

        assert!(entry.is_compressed());
        assert_eq!(entry.compressed_size, 512);

        let bytes = entry.to_bytes();
        let parsed = SectionEntry::from_bytes(&bytes).unwrap();
        assert!(parsed.is_compressed());
        assert_eq!(parsed.compressed_size, 512);
    }

    #[test]
    fn test_section_directory() {
        let mut dir = SectionDirectory::new();
        dir.add(SectionEntry::new(SectionKind::StringPool, 128, 256));
        dir.add(SectionEntry::new(SectionKind::ChunkData, 384, 1024));
        dir.add(SectionEntry::new(SectionKind::DebugInfo, 1408, 512));

        assert_eq!(dir.count(), 3);

        let chunk = dir.find(SectionKind::ChunkData);
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().offset, 384);

        let bytes = dir.to_bytes();
        assert_eq!(bytes.len(), 3 * 16);

        let parsed = SectionDirectory::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.count(), 3);
    }

    #[test]
    fn test_total_file_size() {
        let mut dir = SectionDirectory::new();
        dir.add(SectionEntry::new(SectionKind::StringPool, 128, 256));
        dir.add(SectionEntry::new(SectionKind::ChunkData, 384, 1024));
        dir.add(SectionEntry::new(SectionKind::DebugInfo, 1408, 512));

        // 最后一个 section 结束于 1408 + 512 = 1920
        assert_eq!(dir.total_file_size(), 1920);
    }

    #[test]
    fn test_total_file_size_with_compression() {
        let mut dir = SectionDirectory::new();
        let mut entry = SectionEntry::new(SectionKind::ChunkData, 128, 1024);
        entry.set_compressed(256);
        dir.add(entry);

        // 使用压缩后的大小
        assert_eq!(dir.total_file_size(), 128 + 256);
    }
}
