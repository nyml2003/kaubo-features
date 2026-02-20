//! 二进制文件头定义
//!
//! 128 字节固定大小的文件头，包含 Magic、版本、构建信息、Section Directory 位置等

/// 文件头魔数: "KAUB"
pub const MAGIC: [u8; 4] = [b'K', b'A', b'U', b'B'];

/// 当前文件格式版本
pub const VERSION_MAJOR: u8 = 0;
pub const VERSION_MINOR: u8 = 1;
pub const VERSION_PATCH: u8 = 0;

/// 文件头大小: 128 字节
pub const HEADER_SIZE: usize = 128;

/// 构建模式
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    /// Debug 模式
    Debug = 0x01,
    /// Release 模式
    Release = 0x02,
}

/// 目标架构
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    Unknown = 0,
    X86_64 = 1,
    Aarch64 = 2,
    Wasm32 = 3,
}

/// 目标操作系统
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OS {
    Unknown = 0,
    Windows = 1,
    MacOS = 2,
    Linux = 3,
}

/// 特性标志位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureFlags(pub u32);

impl FeatureFlags {
    /// 包含调试信息
    pub const HAS_DEBUG_INFO: u32 = 0x0001;
    /// 包含 Source Map
    pub const HAS_SOURCE_MAP: u32 = 0x0002;
    /// Source Map 在独立文件
    pub const SOURCE_MAP_EXTERNAL: u32 = 0x0004;
    /// 包含重定位信息（动态链接）
    pub const HAS_RELOCATIONS: u32 = 0x0008;
    /// 可执行（非库）
    pub const IS_EXECUTABLE: u32 = 0x0010;
    /// 标准库模块
    pub const IS_STDLIB: u32 = 0x0020;
    /// 包含签名/校验
    pub const HAS_CHECKSUM: u32 = 0x0040;
    /// Release: 剥离源码路径
    pub const STRIP_SOURCE: u32 = 0x0080;

    /// 创建空的特性标志
    pub fn empty() -> Self {
        Self(0)
    }

    /// 检查是否包含指定标志
    pub fn contains(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    /// 添加标志
    pub fn insert(&mut self, flag: u32) {
        self.0 |= flag;
    }

    /// 移除标志
    pub fn remove(&mut self, flag: u32) {
        self.0 &= !flag;
    }
}

/// 文件头 (128 字节)
#[derive(Debug, Clone)]
pub struct FileHeader {
    /// Magic (4 bytes): "KAUB"
    pub magic: [u8; 4],
    /// 文件格式版本 (3 bytes)
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    /// 保留 (1 byte)
    pub reserved1: u8,

    /// 构建模式 (1 byte)
    pub build_mode: BuildMode,
    /// 目标架构 (1 byte)
    pub target_arch: Arch,
    /// 目标操作系统 (1 byte)
    pub target_os: OS,
    /// 保留 (1 byte)
    pub reserved2: u8,
    /// 构建时间戳 (4 bytes): Unix timestamp
    pub build_timestamp: u32,

    /// 特性标志 (4 bytes)
    pub flags: FeatureFlags,
    /// 保留 (4 bytes)
    pub reserved3: u32,

    /// Section 数量 (2 bytes)
    pub section_count: u16,
    /// Section Directory 偏移 (4 bytes)
    pub section_dir_offset: u32,
    /// Section Directory 大小 (4 bytes)
    pub section_dir_size: u32,
    /// 保留 (2 bytes)
    pub reserved4: u16,

    /// 入口模块索引 (2 bytes)
    pub entry_module_idx: u16,
    /// 入口 Chunk 索引 (2 bytes)
    pub entry_chunk_idx: u16,
    /// 保留 (4 bytes)
    pub reserved5: u32,
    /// 保留 (8 bytes)
    pub reserved6: u64,

    /// 源码哈希 (16 bytes): 用于增量编译检测
    pub source_hash: [u8; 16],
    /// Source Map 偏移 (4 bytes): 0 = 无
    pub source_map_offset: u32,
    /// Source Map 大小 (4 bytes)
    pub source_map_size: u32,
    /// Source Map 是否外部 (1 byte)
    pub source_map_external: u8,
    /// 保留 (11 bytes)
    pub reserved7: [u8; 11],

    /// 文件校验和 (32 bytes): Blake3 哈希（除本字段外）
    pub blake3_hash: [u8; 32],
}

impl FileHeader {
    /// 创建新的文件头
    pub fn new(build_mode: BuildMode) -> Self {
        Self {
            magic: MAGIC,
            version_major: VERSION_MAJOR,
            version_minor: VERSION_MINOR,
            version_patch: VERSION_PATCH,
            reserved1: 0,
            build_mode,
            target_arch: Arch::X86_64,
            target_os: detect_os(),
            reserved2: 0,
            build_timestamp: current_timestamp(),
            flags: FeatureFlags::empty(),
            reserved3: 0,
            section_count: 0,
            section_dir_offset: 0,
            section_dir_size: 0,
            reserved4: 0,
            entry_module_idx: 0,
            entry_chunk_idx: 0,
            reserved5: 0,
            reserved6: 0,
            source_hash: [0; 16],
            source_map_offset: 0,
            source_map_size: 0,
            source_map_external: 0,
            reserved7: [0; 11],
            blake3_hash: [0; 32],
        }
    }

    /// 序列化为字节数组
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        let mut offset = 0;

        // Magic & Version (8 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.magic);
        offset += 4;
        bytes[offset] = self.version_major;
        offset += 1;
        bytes[offset] = self.version_minor;
        offset += 1;
        bytes[offset] = self.version_patch;
        offset += 1;
        bytes[offset] = self.reserved1;
        offset += 1;

        // Build Info (8 bytes)
        bytes[offset] = self.build_mode as u8;
        offset += 1;
        bytes[offset] = self.target_arch as u8;
        offset += 1;
        bytes[offset] = self.target_os as u8;
        offset += 1;
        bytes[offset] = self.reserved2;
        offset += 1;
        bytes[offset..offset + 4].copy_from_slice(&self.build_timestamp.to_le_bytes());
        offset += 4;

        // Feature Flags (8 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.flags.0.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.reserved3.to_le_bytes());
        offset += 4;

        // Section Directory Info (16 bytes)
        bytes[offset..offset + 2].copy_from_slice(&self.section_count.to_le_bytes());
        offset += 2;
        bytes[offset..offset + 4].copy_from_slice(&self.section_dir_offset.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.section_dir_size.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 2].copy_from_slice(&self.reserved4.to_le_bytes());
        offset += 2;

        // Entry Point (16 bytes)
        bytes[offset..offset + 2].copy_from_slice(&self.entry_module_idx.to_le_bytes());
        offset += 2;
        bytes[offset..offset + 2].copy_from_slice(&self.entry_chunk_idx.to_le_bytes());
        offset += 2;
        bytes[offset..offset + 4].copy_from_slice(&self.reserved5.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 8].copy_from_slice(&self.reserved6.to_le_bytes());
        offset += 8;

        // Source Info (32 bytes)
        bytes[offset..offset + 16].copy_from_slice(&self.source_hash);
        offset += 16;
        bytes[offset..offset + 4].copy_from_slice(&self.source_map_offset.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.source_map_size.to_le_bytes());
        offset += 4;
        bytes[offset] = self.source_map_external;
        offset += 1;
        bytes[offset..offset + 11].copy_from_slice(&self.reserved7);
        offset += 11;

        // Checksum (32 bytes)
        bytes[offset..offset + 32].copy_from_slice(&self.blake3_hash);

        bytes
    }

    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HeaderError> {
        if bytes.len() < HEADER_SIZE {
            return Err(HeaderError::TooShort);
        }

        let mut offset = 0;

        // Magic
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[offset..offset + 4]);
        offset += 4;

        if magic != MAGIC {
            return Err(HeaderError::InvalidMagic(magic));
        }

        // Version
        let version_major = bytes[offset];
        offset += 1;
        let version_minor = bytes[offset];
        offset += 1;
        let version_patch = bytes[offset];
        offset += 1;
        let reserved1 = bytes[offset];
        offset += 1;

        // Build Info
        let build_mode = match bytes[offset] {
            0x01 => BuildMode::Debug,
            0x02 => BuildMode::Release,
            n => return Err(HeaderError::InvalidBuildMode(n)),
        };
        offset += 1;

        let target_arch = match bytes[offset] {
            0 => Arch::Unknown,
            1 => Arch::X86_64,
            2 => Arch::Aarch64,
            3 => Arch::Wasm32,
            n => return Err(HeaderError::InvalidArch(n)),
        };
        offset += 1;

        let target_os = match bytes[offset] {
            0 => OS::Unknown,
            1 => OS::Windows,
            2 => OS::MacOS,
            3 => OS::Linux,
            n => return Err(HeaderError::InvalidOS(n)),
        };
        offset += 1;

        let reserved2 = bytes[offset];
        offset += 1;

        let build_timestamp = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // Feature Flags
        let flags = FeatureFlags(u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]));
        offset += 4;

        let reserved3 = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // Section Directory Info
        let section_count = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let section_dir_offset = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;
        let section_dir_size = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;
        let reserved4 = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;

        // Entry Point
        let entry_module_idx = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let entry_chunk_idx = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let reserved5 = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;
        let reserved6 = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        // Source Info
        let mut source_hash = [0u8; 16];
        source_hash.copy_from_slice(&bytes[offset..offset + 16]);
        offset += 16;

        let source_map_offset = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let source_map_size = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        let source_map_external = bytes[offset];
        offset += 1;

        let mut reserved7 = [0u8; 11];
        reserved7.copy_from_slice(&bytes[offset..offset + 11]);
        offset += 11;

        // Checksum
        let mut blake3_hash = [0u8; 32];
        blake3_hash.copy_from_slice(&bytes[offset..offset + 32]);

        Ok(Self {
            magic,
            version_major,
            version_minor,
            version_patch,
            reserved1,
            build_mode,
            target_arch,
            target_os,
            reserved2,
            build_timestamp,
            flags,
            reserved3,
            section_count,
            section_dir_offset,
            section_dir_size,
            reserved4,
            entry_module_idx,
            entry_chunk_idx,
            reserved5,
            reserved6,
            source_hash,
            source_map_offset,
            source_map_size,
            source_map_external,
            reserved7,
            blake3_hash,
        })
    }

    /// 验证文件头
    pub fn validate(&self) -> Result<(), HeaderError> {
        if self.magic != MAGIC {
            return Err(HeaderError::InvalidMagic(self.magic));
        }

        if self.version_major != VERSION_MAJOR {
            return Err(HeaderError::UnsupportedVersion {
                major: self.version_major,
                minor: self.version_minor,
                patch: self.version_patch,
            });
        }

        Ok(())
    }
}

/// 文件头错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// 数据太短
    TooShort,
    /// 无效的 Magic
    InvalidMagic([u8; 4]),
    /// 无效的构建模式
    InvalidBuildMode(u8),
    /// 无效的架构
    InvalidArch(u8),
    /// 无效的操作系统
    InvalidOS(u8),
    /// 不支持的版本
    UnsupportedVersion {
        major: u8,
        minor: u8,
        patch: u8,
    },
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::TooShort => write!(f, "Header too short"),
            HeaderError::InvalidMagic(m) => {
                write!(f, "Invalid magic: {:02x?} (expected KAUB)", m)
            }
            HeaderError::InvalidBuildMode(n) => write!(f, "Invalid build mode: {}", n),
            HeaderError::InvalidArch(n) => write!(f, "Invalid architecture: {}", n),
            HeaderError::InvalidOS(n) => write!(f, "Invalid OS: {}", n),
            HeaderError::UnsupportedVersion { major, minor, patch } => {
                write!(f, "Unsupported version: {}.{}.{}", major, minor, patch)
            }
        }
    }
}

impl std::error::Error for HeaderError {}

/// 检测当前操作系统
fn detect_os() -> OS {
    #[cfg(target_os = "windows")]
    return OS::Windows;
    #[cfg(target_os = "macos")]
    return OS::MacOS;
    #[cfg(target_os = "linux")]
    return OS::Linux;
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return OS::Unknown;
}

/// 获取当前 Unix 时间戳
fn current_timestamp() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = FileHeader::new(BuildMode::Debug);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);

        let parsed = FileHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.magic, MAGIC);
        assert_eq!(parsed.version_major, VERSION_MAJOR);
        assert_eq!(parsed.build_mode, BuildMode::Debug);
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"XXXX");
        let result = FileHeader::from_bytes(&bytes);
        assert!(matches!(result, Err(HeaderError::InvalidMagic(_))));
    }

    #[test]
    fn test_feature_flags() {
        let mut flags = FeatureFlags::empty();
        assert!(!flags.contains(FeatureFlags::HAS_DEBUG_INFO));

        flags.insert(FeatureFlags::HAS_DEBUG_INFO);
        assert!(flags.contains(FeatureFlags::HAS_DEBUG_INFO));

        flags.remove(FeatureFlags::HAS_DEBUG_INFO);
        assert!(!flags.contains(FeatureFlags::HAS_DEBUG_INFO));
    }
}
