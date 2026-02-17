//! 二进制模块加载器
//!
//! 将 .kaubod/.kaubor 文件加载为 VM 可执行的 Chunk。
//!
//! 支持从 package.json 加载配置，自动查找并应用编译/运行时配置。

use super::{
    BinaryReader, ReadError, SectionKind, SectionData, decode_chunk, decode_chunk_with_context,
    DecodeContext, ChunkDecodeError, StringPool, FunctionPool, ShapeTable, ModuleTable, ModuleEntry, 
    ExportTable, ImportTable, DebugInfo,
};
use crate::core::{Chunk, VM, InterpretResult, ObjShape};

/// package.json 中的配置
#[derive(Debug, Clone, Default)]
pub struct PackageJson {
    /// 包名
    pub name: Option<String>,
    /// 版本
    pub version: Option<String>,
    /// 入口文件（源码或二进制）
    pub entry: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 编译器配置
    pub compiler: Option<CompilerConfig>,
}

/// 编译器配置
#[derive(Debug, Clone, Default)]
pub struct CompilerConfig {
    /// 日志级别
    pub log_level: Option<String>,
    /// 是否显示源码
    pub show_source: bool,
    /// 是否显示步骤
    pub show_steps: bool,
    /// 是否转储字节码
    pub dump_bytecode: bool,
    /// 仅编译
    pub compile_only: bool,
}

impl PackageJson {
    /// 从当前目录或父目录查找 package.json
    pub fn find_and_load() -> Option<Self> {
        let mut current_dir = std::env::current_dir().ok()?;
        loop {
            let package_path = current_dir.join("package.json");
            if package_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&package_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        return Self::from_value(&json);
                    }
                }
            }
            if !current_dir.pop() {
                break;
            }
        }
        None
    }

    /// 从指定路径加载
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let json = serde_json::from_str::<serde_json::Value>(&content).ok()?;
        Self::from_value(&json)
    }

    /// 从 JSON Value 解析
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;
        
        let compiler = obj.get("compiler").and_then(|c| {
            let c_obj = c.as_object()?;
            Some(CompilerConfig {
                log_level: c_obj.get("log_level").and_then(|v| v.as_str().map(String::from)),
                show_source: c_obj.get("show_source").and_then(|v| v.as_bool()).unwrap_or(false),
                show_steps: c_obj.get("show_steps").and_then(|v| v.as_bool()).unwrap_or(false),
                dump_bytecode: c_obj.get("dump_bytecode").and_then(|v| v.as_bool()).unwrap_or(false),
                compile_only: c_obj.get("compile_only").and_then(|v| v.as_bool()).unwrap_or(false),
            })
        });
        
        Some(Self {
            name: obj.get("name").and_then(|v| v.as_str().map(String::from)),
            version: obj.get("version").and_then(|v| v.as_str().map(String::from)),
            entry: obj.get("entry").and_then(|v| v.as_str().map(String::from)),
            description: obj.get("description").and_then(|v| v.as_str().map(String::from)),
            compiler,
        })
    }

    /// 获取入口文件路径，默认 main.kaubo
    pub fn entry_file(&self) -> &str {
        self.entry.as_deref().unwrap_or("main.kaubo")
    }

    /// 获取对应的二进制文件路径（将 .kaubo 替换为 .kaubod）
    pub fn entry_binary(&self) -> String {
        let entry = self.entry_file();
        if entry.ends_with(".kaubo") {
            entry.replace(".kaubo", ".kaubod")
        } else {
            format!("{}.kaubod", entry)
        }
    }
}

/// 加载错误
#[derive(Debug, Clone)]
pub enum LoadError {
    /// 读取错误
    Read(ReadError),
    /// Chunk 解码错误
    ChunkDecode(ChunkDecodeError),
    /// 不支持的文件版本
    UnsupportedVersion {
        major: u8,
        minor: u8,
        patch: u8,
    },
    /// 缺少必需的 Section
    MissingSection(SectionKind),
    /// 数据损坏
    CorruptedData(String),
    /// 不支持的特性
    UnsupportedFeature(&'static str),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Read(e) => write!(f, "Read error: {}", e),
            LoadError::ChunkDecode(e) => write!(f, "Chunk decode error: {}", e),
            LoadError::UnsupportedVersion { major, minor, patch } => {
                write!(f, "Unsupported file version: {}.{}.{}", major, minor, patch)
            }
            LoadError::MissingSection(kind) => write!(f, "Missing required section: {:?}", kind),
            LoadError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
            LoadError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Read(e) => Some(e),
            LoadError::ChunkDecode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ReadError> for LoadError {
    fn from(e: ReadError) -> Self {
        LoadError::Read(e)
    }
}

impl From<ChunkDecodeError> for LoadError {
    fn from(e: ChunkDecodeError) -> Self {
        LoadError::ChunkDecode(e)
    }
}

/// 加载的模块
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// 模块名称
    pub name: String,
    /// 模块的 Chunk
    pub chunk: Chunk,
    /// 源文件路径
    pub source_path: String,
    /// 调试信息（可选）
    pub debug_info: Option<DebugInfo>,
}

/// 二进制加载器
pub struct BinaryLoader {
    reader: BinaryReader,
    string_pool: StringPool,
    function_pool: FunctionPool,
    shape_table: ShapeTable,
    module_table: ModuleTable,
    export_table: ExportTable,
    import_table: ImportTable,
}

impl BinaryLoader {
    /// 从字节数组创建加载器
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, LoadError> {
        let reader = BinaryReader::from_bytes(bytes)?;

        // 检查版本
        let header = reader.header();
        if header.version_major != super::header::VERSION_MAJOR {
            return Err(LoadError::UnsupportedVersion {
                major: header.version_major,
                minor: header.version_minor,
                patch: header.version_patch,
            });
        }

        // 加载 String Pool（必需）
        let string_pool = if reader.has_section(SectionKind::StringPool) {
            let data = reader.read_section(SectionKind::StringPool)?;
            StringPool::deserialize(&data)
                .map_err(|e| LoadError::CorruptedData(format!("String pool: {}", e)))?
        } else {
            return Err(LoadError::MissingSection(SectionKind::StringPool));
        };

        // 加载 Function Pool（可选，用于堆对象序列化）
        let function_pool = if reader.has_section(SectionKind::FunctionPool) {
            let data = reader.read_section(SectionKind::FunctionPool)?;
            FunctionPool::deserialize(&data)
                .map_err(|e| LoadError::CorruptedData(format!("Function pool: {}", e)))?
        } else {
            FunctionPool::new()
        };

        // 加载 Shape Table（可选，用于结构体序列化）
        let shape_table = if reader.has_section(SectionKind::ShapeTable) {
            let data = reader.read_section(SectionKind::ShapeTable)?;
            ShapeTable::deserialize(&data)
                .map_err(|e| LoadError::CorruptedData(format!("Shape table: {}", e)))?
        } else {
            ShapeTable::new()
        };

        // 加载 Module Table（必需）
        let module_table = if reader.has_section(SectionKind::ModuleTable) {
            let data = reader.read_section(SectionKind::ModuleTable)?;
            ModuleTable::deserialize(&data)
                .map_err(|e| LoadError::CorruptedData(format!("Module table: {}", e)))?
        } else {
            return Err(LoadError::MissingSection(SectionKind::ModuleTable));
        };

        // 加载 Export Table（可选）
        let export_table = if reader.has_section(SectionKind::ExportTable) {
            let data = reader.read_section(SectionKind::ExportTable)?;
            ExportTable::deserialize(&data)
                .map_err(|e| LoadError::CorruptedData(format!("Export table: {}", e)))?
        } else {
            ExportTable::new()
        };

        // 加载 Import Table（可选）
        let import_table = if reader.has_section(SectionKind::ImportTable) {
            let data = reader.read_section(SectionKind::ImportTable)?;
            ImportTable::deserialize(&data)
                .map_err(|e| LoadError::CorruptedData(format!("Import table: {}", e)))?
        } else {
            ImportTable::new()
        };

        Ok(Self {
            reader,
            string_pool,
            function_pool,
            shape_table,
            module_table,
            export_table,
            import_table,
        })
    }

    /// 从文件加载
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, LoadError> {
        let bytes = std::fs::read(path)
            .map_err(|e| LoadError::CorruptedData(format!("Failed to read file: {}", e)))?;
        Self::from_bytes(bytes)
    }

    /// 获取文件头信息
    pub fn file_info(&self) -> super::FileInfo {
        super::FileInfo::from_reader(&self.reader)
    }

    /// 获取模块数量
    pub fn module_count(&self) -> usize {
        self.module_table.entries.len()
    }

    /// 加载指定索引的模块
    pub fn load_module(&self, idx: usize) -> Result<LoadedModule, LoadError> {
        let entry = self.module_table.entries.get(idx)
            .ok_or_else(|| LoadError::CorruptedData(format!("Module index {} out of bounds", idx)))?;

        let name = self.string_pool.get(entry.name_idx)
            .ok_or_else(|| LoadError::CorruptedData("Invalid module name index".to_string()))?
            .to_string();

        let source_path = self.string_pool.get(entry.source_path_idx)
            .ok_or_else(|| LoadError::CorruptedData("Invalid source path index".to_string()))?
            .to_string();

        // 读取并解码 Chunk 数据
        let chunk = if self.reader.has_section(SectionKind::ChunkData) {
            let data = self.reader.read_section(SectionKind::ChunkData)?;
            // Chunk 数据格式：多个 Chunk 连续存储，通过 offset/size 定位
            let start = entry.chunk_offset as usize;
            let end = start + entry.chunk_size as usize;
            if end > data.len() {
                return Err(LoadError::CorruptedData("Chunk data out of bounds".to_string()));
            }
            
            // 使用 DecodeContext 支持堆对象序列化
            let ctx = DecodeContext::new(&self.string_pool, &self.function_pool, &self.shape_table);
            let mut offset = 0;
            decode_chunk_with_context(&data[start..end], &mut offset, &ctx)?
        } else {
            return Err(LoadError::MissingSection(SectionKind::ChunkData));
        };

        // 加载 Debug Info（可选）
        let debug_info = if self.reader.has_section(SectionKind::DebugInfo) {
            let data = self.reader.read_section(SectionKind::DebugInfo)?;
            // 每个模块有自己的 debug info，需要根据模块索引定位
            // 简化实现：假设只有一个模块，直接使用
            if idx == 0 {
                DebugInfo::deserialize(&data)
                    .map_err(|e| LoadError::CorruptedData(format!("Debug info: {}", e)))
                    .ok()
            } else {
                None
            }
        } else {
            None
        };

        Ok(LoadedModule {
            name,
            chunk,
            source_path,
            debug_info,
        })
    }

    /// 加载入口模块
    pub fn load_entry_module(&self) -> Result<LoadedModule, LoadError> {
        let entry_idx = self.reader.header().entry_module_idx as usize;
        self.load_module(entry_idx)
    }

    /// 获取所有模块名称
    pub fn module_names(&self) -> Vec<String> {
        self.module_table.entries.iter()
            .filter_map(|e| self.string_pool.get(e.name_idx))
            .map(|s| s.to_string())
            .collect()
    }
}

/// VM 扩展：从二进制文件执行
pub trait VMExecuteBinary {
    /// 执行二进制文件
    fn execute_binary(&mut self, bytes: Vec<u8>) -> Result<InterpretResult, LoadError>;

    /// 从文件执行二进制
    fn execute_binary_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<InterpretResult, LoadError>;

    /// 从 package.json 执行（自动查找入口）
    fn execute_package(&mut self) -> Result<InterpretResult, LoadError>;

    /// 从指定目录的 package.json 执行
    fn execute_package_in(&mut self, dir: impl AsRef<std::path::Path>) -> Result<InterpretResult, LoadError>;
}

impl VMExecuteBinary for VM {
    fn execute_binary(&mut self, bytes: Vec<u8>) -> Result<InterpretResult, LoadError> {
        // 初始化标准库（如果尚未初始化）
        self.init_stdlib();
        
        let loader = BinaryLoader::from_bytes(bytes)?;
        
        // 注册 Shape Table 中的所有 Shape 到 VM
        for entry in &loader.shape_table.entries {
            // 重建 ObjShape
            let name = loader.string_pool.get(entry.name_idx).unwrap_or("Struct").to_string();
            let field_names: Vec<String> = entry.field_name_indices.iter()
                .map(|&idx| loader.string_pool.get(idx).unwrap_or("").to_string())
                .collect();
            let field_types: Vec<String> = entry.field_type_indices.iter()
                .map(|&idx| loader.string_pool.get(idx).unwrap_or("").to_string())
                .collect();
            
            let shape = if field_types.is_empty() {
                ObjShape::new(entry.shape_id, name, field_names)
            } else {
                ObjShape::new_with_types(entry.shape_id, name, field_names, field_types)
            };
            
            let shape_ptr = Box::into_raw(Box::new(shape));
            unsafe {
                self.register_shape(shape_ptr);
            }
        }
        
        let module = loader.load_entry_module()?;
        
        // 使用 VM 的 interpret 方法执行 Chunk
        let result = self.interpret(&module.chunk);
        
        Ok(result)
    }

    fn execute_binary_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<InterpretResult, LoadError> {
        let bytes = std::fs::read(path)
            .map_err(|e| LoadError::CorruptedData(format!("Failed to read file: {}", e)))?;
        self.execute_binary(bytes)
    }

    fn execute_package(&mut self) -> Result<InterpretResult, LoadError> {
        let package = PackageJson::find_and_load()
            .ok_or_else(|| LoadError::CorruptedData("package.json not found".to_string()))?;
        
        let binary_path = package.entry_binary();
        self.execute_binary_file(&binary_path)
    }

    fn execute_package_in(&mut self, dir: impl AsRef<std::path::Path>) -> Result<InterpretResult, LoadError> {
        let package_path = dir.as_ref().join("package.json");
        let package = PackageJson::from_file(&package_path)
            .ok_or_else(|| LoadError::CorruptedData(
                format!("package.json not found in {}", dir.as_ref().display())
            ))?;
        
        let binary_path = dir.as_ref().join(package.entry_binary());
        self.execute_binary_file(binary_path)
    }
}

/// 便捷的加载函数

/// 从字节数组加载并执行
pub fn execute_binary(vm: &mut VM, bytes: Vec<u8>) -> Result<InterpretResult, LoadError> {
    vm.execute_binary(bytes)
}

/// 从文件加载并执行
pub fn execute_binary_file(vm: &mut VM, path: impl AsRef<std::path::Path>) -> Result<InterpretResult, LoadError> {
    vm.execute_binary_file(path)
}

/// 仅加载模块（不执行）
pub fn load_module(bytes: Vec<u8>) -> Result<LoadedModule, LoadError> {
    let loader = BinaryLoader::from_bytes(bytes)?;
    loader.load_entry_module()
}

#[cfg(test)]
mod tests {
    use super::super::{BinaryWriter, WriteOptions, BuildMode, SectionKind, SectionData};
    use super::*;
    use crate::core::Chunk;
    use crate::core::bytecode::OpCode;

    fn create_test_binary() -> Vec<u8> {
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);

        // 创建 String Pool
        let mut string_pool = StringPool::new();
        let name_idx = string_pool.add("main");
        let path_idx = string_pool.add("main.kaubo");
        writer.write_section(SectionKind::StringPool, &string_pool.serialize());

        // 创建 Chunk
        let mut chunk = Chunk::new();
        chunk.write_op(OpCode::LoadNull, 1);
        chunk.write_op(OpCode::Return, 1);
        let chunk_data = super::super::encode_chunk(&chunk).unwrap();

        // 创建 Module Table
        let mut module_table = ModuleTable::new();
        module_table.add(ModuleEntry {
            name_idx,
            source_path_idx: path_idx,
            chunk_offset: 0,
            chunk_size: chunk_data.len() as u32,
            shape_start: 0,
            shape_count: 0,
            export_start: 0,
            export_count: 0,
            import_start: 0,
            import_count: 0,
        });
        writer.write_section(SectionKind::ModuleTable, &module_table.serialize());

        // 写入 Chunk Data
        writer.write_section(SectionKind::ChunkData, &chunk_data);

        writer.set_entry(0, 0);
        writer.finish()
    }

    #[test]
    fn test_loader_from_bytes() {
        let binary = create_test_binary();
        let loader = BinaryLoader::from_bytes(binary).unwrap();

        assert_eq!(loader.module_count(), 1);
        
        let names = loader.module_names();
        assert_eq!(names, vec!["main"]);
    }

    #[test]
    fn test_load_module() {
        let binary = create_test_binary();
        let loader = BinaryLoader::from_bytes(binary).unwrap();
        
        let module = loader.load_module(0).unwrap();
        assert_eq!(module.name, "main");
        assert_eq!(module.source_path, "main.kaubo");
    }

    #[test]
    fn test_vm_execute_binary() {
        let binary = create_test_binary();
        let mut vm = VM::new();
        
        let result = vm.execute_binary(binary).unwrap();
        assert_eq!(result, InterpretResult::Ok);
    }

    #[test]
    fn test_invalid_magic() {
        let mut binary = create_test_binary();
        binary[0] = b'X';
        
        let result = BinaryLoader::from_bytes(binary);
        assert!(result.is_err());
    }

    #[test]
    fn test_package_json_parse() {
        let json = r#"{
            "name": "calculator",
            "version": "0.1.0",
            "entry": "main.kaubo",
            "description": "Simple calculator example",
            "compiler": {
                "log_level": "error",
                "show_source": true,
                "show_steps": false,
                "dump_bytecode": false,
                "compile_only": false
            }
        }"#;
        
        let package = PackageJson::from_value(&serde_json::from_str(json).unwrap()).unwrap();
        assert_eq!(package.name, Some("calculator".to_string()));
        assert_eq!(package.version, Some("0.1.0".to_string()));
        assert_eq!(package.entry, Some("main.kaubo".to_string()));
        assert_eq!(package.entry_binary(), "main.kaubod");
        
        let compiler = package.compiler.unwrap();
        assert_eq!(compiler.log_level, Some("error".to_string()));
        assert!(compiler.show_source);
    }

    #[test]
    fn test_package_json_default_entry() {
        let json = r#"{"name": "test"}"#;
        let package = PackageJson::from_value(&serde_json::from_str(json).unwrap()).unwrap();
        assert_eq!(package.entry_file(), "main.kaubo");
        assert_eq!(package.entry_binary(), "main.kaubod");
    }
}
