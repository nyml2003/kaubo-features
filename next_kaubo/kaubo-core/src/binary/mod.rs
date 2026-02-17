//! Kaubo 二进制格式支持
//!
//! 提供 .kaubod (Debug) 和 .kaubor (Release) 文件的读写支持。
//!
//! # 文件格式
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      File Header (128 bytes)                 │
//! ├─────────────────────────────────────────────────────────────┤
//! │                     Section Directory                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  String Pool Section  │  全局字符串池（去重）                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Module Table Section │  模块元数据表                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Chunk Data Section   │  字节码和常量池                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Shape Table Section  │  Struct shape 定义                    │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Export Table Section │  导出符号表                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Import Table Section │  导入依赖表                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Debug Info Section   │  调试信息（Release 可选剥离）           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Source Map Section   │  源码映射（可选分离到 .kmap）           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # 示例
//!
//! ```rust,ignore
//! use kaubo_core::binary::{BinaryWriter, BinaryReader, WriteOptions, BuildMode};
//!
//! // 写入
//! let options = WriteOptions {
//!     build_mode: BuildMode::Debug,
//!     compress: false,
//!     strip_debug: false,
//!     source_map_external: false,
//! };
//! let mut writer = BinaryWriter::new(options);
//! writer.write_section(SectionKind::StringPool, &data);
//! let bytes = writer.finish();
//!
//! // 读取
//! let reader = BinaryReader::from_bytes(bytes)?;
//! let data = reader.read_section(SectionKind::StringPool)?;
//! ```

mod chunk;
mod data;
mod debug_info;
mod e2e_tests;
mod header;
mod loader;
mod reader;
mod section;
mod writer;

// 公开导出
pub use chunk::{
    decode_chunk, decode_chunk_with_context, encode_chunk, encode_chunk_with_context,
    ChunkDecodeError, ChunkEncodeError, DecodeContext, EncodeContext,
};
pub use data::{
    ExportEntry, ExportKind, ExportTable, FunctionEntry, FunctionPool, ImportEntry, ImportKind,
    ImportTable, ModuleEntry, ModuleTable, StringPool,
};
pub use debug_info::{DebugInfo, LineEntry, LineTable, LocalNameEntry, LocalNameTable};
pub use header::{Arch, BuildMode, FeatureFlags, FileHeader, HeaderError, OS, HEADER_SIZE, MAGIC};
pub use loader::{BinaryLoader, LoadedModule, LoadError, VMExecuteBinary, PackageJson, CompilerConfig, execute_binary, execute_binary_file, load_module};
pub use reader::{BinaryReader, FileInfo, ReadError, read_binary_file};
pub use section::{SectionData, SectionDirectory, SectionEntry, SectionError, SectionKind};
pub use writer::{BinaryWriter, WriteOptions};

/// 文件扩展名常量
pub mod ext {
    /// 源码文件
    pub const SOURCE: &str = "kaubo";
    /// Debug 编译产物
    pub const DEBUG: &str = "kaubod";
    /// Release 编译产物
    pub const RELEASE: &str = "kaubor";
    /// Source Map 文件
    pub const SOURCE_MAP: &str = "kmap";
    /// 可执行包
    pub const PACKAGE: &str = "kpk";
}

/// 从文件扩展名检测构建模式
pub fn detect_build_mode_from_ext(path: impl AsRef<std::path::Path>) -> Option<BuildMode> {
    let path = path.as_ref();
    let ext = path.extension()?.to_str()?;

    match ext {
        "kaubod" => Some(BuildMode::Debug),
        "kaubor" | "kpk" => Some(BuildMode::Release),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_extensions() {
        assert_eq!(ext::SOURCE, "kaubo");
        assert_eq!(ext::DEBUG, "kaubod");
        assert_eq!(ext::RELEASE, "kaubor");
        assert_eq!(ext::SOURCE_MAP, "kmap");
        assert_eq!(ext::PACKAGE, "kpk");
    }

    #[test]
    fn test_detect_build_mode() {
        use std::path::Path;

        assert_eq!(
            detect_build_mode_from_ext(Path::new("test.kaubod")),
            Some(BuildMode::Debug)
        );
        assert_eq!(
            detect_build_mode_from_ext(Path::new("test.kaubor")),
            Some(BuildMode::Release)
        );
        assert_eq!(
            detect_build_mode_from_ext(Path::new("test.kpk")),
            Some(BuildMode::Release)
        );
        assert_eq!(
            detect_build_mode_from_ext(Path::new("test.kaubo")),
            None
        );
    }

    #[test]
    fn test_roundtrip() {
        // 完整的写入-读取测试
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);

        // 写入 String Pool
        let mut string_pool = StringPool::new();
        let idx1 = string_pool.add("hello");
        let idx2 = string_pool.add("world");
        writer.write_section(SectionKind::StringPool, &string_pool.serialize());

        // 写入 Module Table
        let mut module_table = ModuleTable::new();
        module_table.add(ModuleEntry {
            name_idx: idx1,
            source_path_idx: idx2,
            chunk_offset: 0,
            chunk_size: 100,
            shape_start: 0,
            shape_count: 0,
            export_start: 0,
            export_count: 0,
            import_start: 0,
            import_count: 0,
        });
        writer.write_section(SectionKind::ModuleTable, &module_table.serialize());

        writer.set_entry(0, 0);

        let bytes = writer.finish();

        // 读取
        let reader = BinaryReader::from_bytes(bytes).unwrap();

        assert_eq!(reader.header().build_mode, BuildMode::Debug);
        assert!(reader.has_section(SectionKind::StringPool));
        assert!(reader.has_section(SectionKind::ModuleTable));

        // 验证 String Pool
        let string_data = reader.read_section(SectionKind::StringPool).unwrap();
        let string_pool2 = StringPool::deserialize(&string_data).unwrap();
        assert_eq!(string_pool2.get(idx1), Some("hello"));
        assert_eq!(string_pool2.get(idx2), Some("world"));

        // 验证 Module Table
        let module_data = reader.read_section(SectionKind::ModuleTable).unwrap();
        let module_table2 = ModuleTable::deserialize(&module_data).unwrap();
        assert_eq!(module_table2.entries.len(), 1);
    }
}
