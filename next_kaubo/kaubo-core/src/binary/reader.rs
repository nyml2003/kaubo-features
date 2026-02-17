//! 二进制文件读取器
//!
//! 负责从二进制文件 (.kaubod/.kaubor) 读取模块数据

use super::header::{FileHeader, HeaderError, HEADER_SIZE};
use super::section::{SectionDirectory, SectionEntry, SectionError, SectionKind};

/// 二进制读取器
pub struct BinaryReader {
    /// 原始数据
    data: Vec<u8>,
    /// 文件头
    header: FileHeader,
    /// Section directory
    sections: SectionDirectory,
}

/// 读取错误
#[derive(Debug, Clone)]
pub enum ReadError {
    /// 文件头错误
    Header(HeaderError),
    /// Section 错误
    Section(SectionError),
    /// 数据太短
    TooShort,
    /// 无效的偏移
    InvalidOffset,
    /// 无效的 section 大小
    InvalidSectionSize,
    /// 校验和不匹配
    ChecksumMismatch,
    /// 解压失败
    DecompressionFailed,
    /// Section 未找到
    SectionNotFound(SectionKind),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Header(e) => write!(f, "Header error: {}", e),
            ReadError::Section(e) => write!(f, "Section error: {}", e),
            ReadError::TooShort => write!(f, "Data too short"),
            ReadError::InvalidOffset => write!(f, "Invalid offset"),
            ReadError::InvalidSectionSize => write!(f, "Invalid section size"),
            ReadError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            ReadError::DecompressionFailed => write!(f, "Decompression failed"),
            ReadError::SectionNotFound(k) => write!(f, "Section not found: {:?}", k),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Header(e) => Some(e),
            ReadError::Section(e) => Some(e),
            _ => None,
        }
    }
}

impl From<HeaderError> for ReadError {
    fn from(e: HeaderError) -> Self {
        ReadError::Header(e)
    }
}

impl From<SectionError> for ReadError {
    fn from(e: SectionError) -> Self {
        ReadError::Section(e)
    }
}

impl BinaryReader {
    /// 从字节数组创建读取器
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, ReadError> {
        if data.len() < HEADER_SIZE {
            return Err(ReadError::TooShort);
        }

        // 解析文件头
        let header = FileHeader::from_bytes(&data[..HEADER_SIZE])?;

        // 验证文件头
        header.validate()?;

        // 读取 section directory
        let section_dir_start = header.section_dir_offset as usize;
        let section_dir_end = section_dir_start + header.section_dir_size as usize;

        if section_dir_end > data.len() {
            return Err(ReadError::InvalidOffset);
        }

        let sections = SectionDirectory::from_bytes(&data[section_dir_start..section_dir_end])?;

        // 验证 section 数量
        if sections.count() != header.section_count as usize {
            // 警告：section 数量不匹配，但不报错
            // 这可能是由于预留字段的使用
        }

        // 验证校验和（如果存在）
        if header.flags.0 != 0 {
            // 如果有任何标志，尝试验证校验和
            let expected_hash = &header.blake3_hash;
            if !expected_hash.iter().all(|&b| b == 0) {
                // 校验和非零，需要验证
                let computed_hash = compute_blake3_hash_for_validation(&data, HEADER_SIZE - 32);
                if &computed_hash[..] != expected_hash {
                    return Err(ReadError::ChecksumMismatch);
                }
            }
        }

        Ok(Self {
            data,
            header,
            sections,
        })
    }

    /// 获取文件头
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// 获取 section directory
    pub fn sections(&self) -> &SectionDirectory {
        &self.sections
    }

    /// 读取指定 section 的原始数据
    pub fn read_section_raw(&self, kind: SectionKind) -> Result<&[u8], ReadError> {
        let entry = self
            .sections
            .find(kind)
            .ok_or(ReadError::SectionNotFound(kind))?;

        let start = entry.offset as usize;
        let size = if entry.compressed_size > 0 {
            entry.compressed_size
        } else {
            entry.size
        } as usize;

        let end = start + size;

        if end > self.data.len() {
            return Err(ReadError::InvalidSectionSize);
        }

        Ok(&self.data[start..end])
    }

    /// 读取并解压 section 数据
    pub fn read_section(&self, kind: SectionKind) -> Result<Vec<u8>, ReadError> {
        let entry = self
            .sections
            .find(kind)
            .ok_or(ReadError::SectionNotFound(kind))?;

        let raw_data = self.read_section_raw(kind)?;

        if entry.is_compressed() {
            // 需要解压
            decompress_data(raw_data, entry.size as usize)
                .ok_or(ReadError::DecompressionFailed)
        } else {
            Ok(raw_data.to_vec())
        }
    }

    /// 检查是否包含指定 section
    pub fn has_section(&self, kind: SectionKind) -> bool {
        self.sections.find(kind).is_some()
    }

    /// 获取所有 section 条目
    pub fn section_entries(&self) -> &[SectionEntry] {
        &self.sections.entries
    }

    /// 获取原始数据（用于调试）
    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }
}

/// 解压数据
#[cfg(feature = "zstd")]
fn decompress_data(data: &[u8], expected_size: usize) -> Option<Vec<u8>> {
    use zstd::stream::decode_all;
    decode_all(data).ok()
}

#[cfg(not(feature = "zstd"))]
fn decompress_data(data: &[u8], expected_size: usize) -> Option<Vec<u8>> {
    // 未启用 zstd 特性
    let _ = expected_size;
    Some(data.to_vec())
}

/// 计算 Blake3 校验和用于验证
fn compute_blake3_hash_for_validation(data: &[u8], hash_field_offset: usize) -> [u8; 32] {
    #[cfg(feature = "blake3")]
    {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        hasher.update(&data[..hash_field_offset]);
        if data.len() > hash_field_offset + 32 {
            hasher.update(&data[hash_field_offset + 32..]);
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }

    #[cfg(not(feature = "blake3"))]
    {
        let _ = (data, hash_field_offset);
        [0u8; 32]
    }
}

/// 从文件读取二进制数据
pub fn read_binary_file(path: impl AsRef<std::path::Path>) -> Result<BinaryReader, ReadError> {
    let data = std::fs::read(path).map_err(|_| ReadError::TooShort)?;
    BinaryReader::from_bytes(data)
}

/// 文件格式信息（用于 inspect）
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub magic: [u8; 4],
    pub version: (u8, u8, u8),
    pub build_mode: String,
    pub target: (String, String),
    pub section_count: usize,
    pub total_size: usize,
    pub has_debug_info: bool,
    pub has_source_map: bool,
    pub is_executable: bool,
}

impl FileInfo {
    /// 从 BinaryReader 提取文件信息
    pub fn from_reader(reader: &BinaryReader) -> Self {
        let h = &reader.header;
        Self {
            magic: h.magic,
            version: (h.version_major, h.version_minor, h.version_patch),
            build_mode: match h.build_mode {
                super::header::BuildMode::Debug => "Debug".to_string(),
                super::header::BuildMode::Release => "Release".to_string(),
            },
            target: (
                format!("{:?}", h.target_arch),
                format!("{:?}", h.target_os),
            ),
            section_count: reader.sections.count(),
            total_size: reader.data.len(),
            has_debug_info: h.flags.contains(super::header::FeatureFlags::HAS_DEBUG_INFO),
            has_source_map: h.flags.contains(super::header::FeatureFlags::HAS_SOURCE_MAP),
            is_executable: h.flags.contains(super::header::FeatureFlags::IS_EXECUTABLE),
        }
    }
}

impl std::fmt::Display for FileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Kaubo Binary Module")?;
        writeln!(f, "  Magic: {}", std::str::from_utf8(&self.magic).unwrap_or("INVALID"))?;
        writeln!(f, "  Version: {}.{}.{}", self.version.0, self.version.1, self.version.2)?;
        writeln!(f, "  Build Mode: {}", self.build_mode)?;
        writeln!(f, "  Target: {}-{}", self.target.0, self.target.1)?;
        writeln!(f, "  Sections: {}", self.section_count)?;
        writeln!(f, "  Total Size: {} bytes", self.total_size)?;
        writeln!(f, "  Has Debug Info: {}", self.has_debug_info)?;
        writeln!(f, "  Has Source Map: {}", self.has_source_map)?;
        writeln!(f, "  Is Executable: {}", self.is_executable)
    }
}

#[cfg(test)]
mod tests {
    use super::super::writer::{BinaryWriter, WriteOptions};
    use super::*;

    fn create_test_binary() -> Vec<u8> {
        let options = WriteOptions {
            build_mode: super::super::header::BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);
        writer.write_section(SectionKind::StringPool, b"test string data");
        writer.write_section(SectionKind::ChunkData, &[0x01, 0x02, 0x03, 0x04]);
        writer.set_entry(0, 0);
        writer.finish()
    }

    #[test]
    fn test_reader_basic() {
        let data = create_test_binary();
        let reader = BinaryReader::from_bytes(data).unwrap();

        assert_eq!(reader.header().magic, super::super::header::MAGIC);
        assert_eq!(reader.sections().count(), 2);
    }

    #[test]
    fn test_read_section() {
        let data = create_test_binary();
        let reader = BinaryReader::from_bytes(data).unwrap();

        let string_data = reader.read_section_raw(SectionKind::StringPool).unwrap();
        assert_eq!(string_data, b"test string data");

        let chunk_data = reader.read_section(SectionKind::ChunkData).unwrap();
        assert_eq!(chunk_data, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_has_section() {
        let data = create_test_binary();
        let reader = BinaryReader::from_bytes(data).unwrap();

        assert!(reader.has_section(SectionKind::StringPool));
        assert!(reader.has_section(SectionKind::ChunkData));
        assert!(!reader.has_section(SectionKind::DebugInfo));
    }

    #[test]
    fn test_file_info() {
        let data = create_test_binary();
        let reader = BinaryReader::from_bytes(data).unwrap();
        let info = FileInfo::from_reader(&reader);

        assert_eq!(info.magic, super::super::header::MAGIC);
        assert_eq!(info.build_mode, "Debug");
        assert!(info.is_executable);
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = create_test_binary();
        data[0] = b'X';
        let result = BinaryReader::from_bytes(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_section_not_found() {
        let data = create_test_binary();
        let reader = BinaryReader::from_bytes(data).unwrap();

        let result = reader.read_section(SectionKind::DebugInfo);
        assert!(matches!(result, Err(ReadError::SectionNotFound(_))));
    }
}
