//! 二进制文件写入器
//!
//! 负责将编译后的模块数据写入二进制文件格式 (.kaubod/.kaubor)

use super::header::{BuildMode, FileHeader, FeatureFlags, HEADER_SIZE};
use super::section::{SectionDirectory, SectionEntry, SectionKind};

/// 二进制写入器
pub struct BinaryWriter {
    /// 文件头
    header: FileHeader,
    /// Section directory
    sections: SectionDirectory,
    /// 当前写入位置
    current_offset: u32,
    /// 缓冲区
    buffer: Vec<u8>,
}

/// 写入选项
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// 构建模式
    pub build_mode: BuildMode,
    /// 是否压缩
    pub compress: bool,
    /// 是否剥离调试信息
    pub strip_debug: bool,
    /// Source Map 是否外部
    pub source_map_external: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        }
    }
}

impl BinaryWriter {
    /// 创建新的二进制写入器
    pub fn new(options: WriteOptions) -> Self {
        let mut header = FileHeader::new(options.build_mode);

        // 设置特性标志
        if !options.strip_debug {
            header.flags.insert(FeatureFlags::HAS_DEBUG_INFO);
        }
        if !options.source_map_external {
            header.flags.insert(FeatureFlags::HAS_SOURCE_MAP);
        }
        if options.compress {
            header.flags.insert(FeatureFlags::HAS_CHECKSUM); // 压缩时总是包含校验和
        }

        let mut buffer = Vec::with_capacity(4096);
        // 预留文件头空间
        buffer.resize(HEADER_SIZE, 0);

        Self {
            header,
            sections: SectionDirectory::new(),
            current_offset: HEADER_SIZE as u32,
            buffer,
        }
    }

    /// 获取当前写入位置
    pub fn current_offset(&self) -> u32 {
        self.current_offset
    }

    /// 对齐到指定边界
    pub fn align_to(&mut self, alignment: u32) {
        let rem = self.current_offset % alignment;
        if rem != 0 {
            let padding = alignment - rem;
            self.buffer.resize(self.buffer.len() + padding as usize, 0);
            self.current_offset += padding;
        }
    }

    /// 写入 section 数据
    ///
    /// 返回 section 在文件中的偏移
    pub fn write_section(&mut self, kind: SectionKind, data: &[u8]) -> u32 {
        self.align_to(8); // 8 字节对齐

        let offset = self.current_offset;
        let size = data.len() as u32;

        // 创建 section 条目
        let entry = SectionEntry::new(kind, offset, size);
        self.sections.add(entry);

        // 写入数据
        self.buffer.extend_from_slice(data);
        self.current_offset += size;

        offset
    }

    /// 写入压缩的 section 数据
    ///
    /// 如果压缩失败或压缩后更大，则写入未压缩数据
    pub fn write_section_compressed(
        &mut self,
        kind: SectionKind,
        data: &[u8],
    ) -> (u32, bool) {
        self.align_to(8);

        // 尝试压缩
        let compressed = compress_data(data);

        let (entry, was_compressed) = if let Some(compressed_data) = compressed {
            if compressed_data.len() < data.len() {
                // 使用压缩数据
                let mut entry = SectionEntry::new(kind, self.current_offset, data.len() as u32);
                entry.set_compressed(compressed_data.len() as u32);

                // 写入压缩数据
                self.buffer.extend_from_slice(&compressed_data);
                self.current_offset += compressed_data.len() as u32;

                (entry, true)
            } else {
                // 压缩后更大，使用原始数据
                (SectionEntry::new(kind, self.current_offset, data.len() as u32), false)
            }
        } else {
            // 压缩失败，使用原始数据
            (SectionEntry::new(kind, self.current_offset, data.len() as u32), false)
        };

        if !was_compressed {
            // 写入未压缩数据
            self.buffer.extend_from_slice(data);
            self.current_offset += data.len() as u32;
        }

        let offset = entry.offset;
        self.sections.add(entry);

        (offset, was_compressed)
    }

    /// 设置入口点
    pub fn set_entry(&mut self, module_idx: u16, chunk_idx: u16) {
        self.header.entry_module_idx = module_idx;
        self.header.entry_chunk_idx = chunk_idx;
        self.header.flags.insert(FeatureFlags::IS_EXECUTABLE);
    }

    /// 设置源码哈希
    pub fn set_source_hash(&mut self, hash: [u8; 16]) {
        self.header.source_hash = hash;
    }

    /// 设置 Source Map 信息
    pub fn set_source_map(&mut self, offset: u32, size: u32, external: bool) {
        self.header.source_map_offset = offset;
        self.header.source_map_size = size;
        self.header.source_map_external = if external { 1 } else { 0 };

        if external {
            self.header.flags.insert(FeatureFlags::SOURCE_MAP_EXTERNAL);
        }
    }

    /// 完成写入，生成最终字节数组
    ///
    /// 这会更新文件头和 section directory，并计算校验和
    pub fn finish(mut self) -> Vec<u8> {
        // 写入 section directory
        self.align_to(8);
        let section_dir_offset = self.current_offset;
        let section_dir_data = self.sections.to_bytes();
        let section_dir_size = section_dir_data.len() as u32;

        self.buffer.extend_from_slice(&section_dir_data);
        self.current_offset += section_dir_size;

        // 更新文件头
        self.header.section_count = self.sections.count() as u16;
        self.header.section_dir_offset = section_dir_offset;
        self.header.section_dir_size = section_dir_size;

        // 计算校验和 (Blake3)
        // 注意：校验和不包含 blake3_hash 字段本身
        let hash = compute_blake3_hash(&self.buffer, HEADER_SIZE - 32);
        self.header.blake3_hash = hash;

        // 写入文件头
        let header_bytes = self.header.to_bytes();
        self.buffer[..HEADER_SIZE].copy_from_slice(&header_bytes);

        self.buffer
    }

    /// 获取当前文件头（用于调试）
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// 获取当前 sections（用于调试）
    pub fn sections(&self) -> &SectionDirectory {
        &self.sections
    }
}

/// 压缩数据
///
/// 返回压缩后的数据，如果压缩失败返回 None
#[cfg(feature = "zstd")]
fn compress_data(data: &[u8]) -> Option<Vec<u8>> {
    use zstd::stream::encode_all;
    encode_all(data, 3).ok() // level 3 是速度和压缩率的平衡
}

#[cfg(not(feature = "zstd"))]
fn compress_data(data: &[u8]) -> Option<Vec<u8>> {
    // 未启用 zstd 特性，返回 None 表示不压缩
    let _ = data;
    None
}

/// 计算 Blake3 校验和
///
/// 跳过 header 中的 hash 字段本身（最后 32 字节）
fn compute_blake3_hash(data: &[u8], hash_field_offset: usize) -> [u8; 32] {
    #[cfg(feature = "blake3")]
    {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // 哈希 hash 字段之前的部分
        hasher.update(&data[..hash_field_offset]);

        // 跳过 32 字节的 hash 字段，哈希剩余部分
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
        // 未启用 blake3 特性，返回空校验和
        // 实际项目中应该添加依赖或 panic
        let _ = (data, hash_field_offset);
        [0u8; 32]
    }
}

/// 辅助函数：计算简单哈希（用于没有 blake3 时）
pub fn simple_hash(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_basic() {
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);

        // 写入一些 section
        writer.write_section(SectionKind::StringPool, b"test string data");
        writer.write_section(SectionKind::ChunkData, &[0x01, 0x02, 0x03, 0x04]);

        // 设置入口
        writer.set_entry(0, 0);

        // 完成
        let data = writer.finish();

        // 验证大小
        assert!(data.len() >= HEADER_SIZE);

        // 验证 magic
        assert_eq!(&data[0..4], b"KAUB");
    }

    #[test]
    fn test_writer_sections() {
        let options = WriteOptions::default();
        let mut writer = BinaryWriter::new(options);

        let offset1 = writer.write_section(SectionKind::StringPool, b"data1");
        let offset2 = writer.write_section(SectionKind::ChunkData, b"data2");

        assert!(offset2 > offset1);

        let sections = writer.sections();
        assert_eq!(sections.count(), 2);

        let chunk_section = sections.find(SectionKind::ChunkData);
        assert!(chunk_section.is_some());
        assert_eq!(chunk_section.unwrap().offset, offset2);
    }

    #[test]
    fn test_simple_hash() {
        let data1 = b"hello world";
        let data2 = b"hello world";
        let data3 = b"different";

        assert_eq!(simple_hash(data1), simple_hash(data2));
        assert_ne!(simple_hash(data1), simple_hash(data3));
    }
}
