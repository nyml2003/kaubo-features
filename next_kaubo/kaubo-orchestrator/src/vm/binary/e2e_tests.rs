//! 端到端测试
//!
//! 测试完整流程：源码 -> 编译 -> 二进制 -> 执行

#[cfg(test)]
mod tests {
    use crate::vm::binary::{
        BinaryLoader, BinaryWriter, BuildMode, SectionKind, WriteOptions,
        VMExecuteBinary, SectionData, encode_chunk,
    };
    use crate::passes::module::{MultiFileCompiler, CompileUnit};
    use crate::passes::parser::Parser;
    use crate::passes::lexer::builder::build_lexer;
    use crate::vm::core::{Chunk, VM, InterpretResult};
    use kaubo_vfs::NativeFileSystem;
    use std::collections::HashMap;

    /// 编译单个 CompileUnit 为 Chunk
    fn compile_unit_to_chunk(unit: &CompileUnit) -> Chunk {
        use crate::passes::codegen::compile_with_struct_info;

        // 提取 struct 信息用于编译器
        let struct_infos = HashMap::new();
        // TODO: 从 AST 提取 struct 定义

        // 编译 AST 为 Chunk
        let (chunk, _) = compile_with_struct_info(&unit.ast, struct_infos)
            .expect("Failed to compile unit");
        
        chunk
    }

    /// 从源码编译并生成二进制
    fn compile_to_binary(entry_path: &str, root_dir: &str) -> Vec<u8> {
        // 1. 多文件编译（解析 AST）
        let vfs = NativeFileSystem::new();
        let mut compiler = MultiFileCompiler::new(Box::new(vfs), root_dir);
        
        let result = compiler.compile_entry(entry_path)
            .expect("Failed to compile entry");

        // 2. 为每个编译单元生成 Chunk
        let mut chunks = Vec::new();
        for unit in &result.units {
            let chunk = compile_unit_to_chunk(unit);
            chunks.push((unit.import_path.clone(), chunk));
        }

        // 3. 打包为二进制格式
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);

        // 创建 String Pool
        let mut string_pool = crate::vm::binary::StringPool::new();
        let mut module_entries = Vec::new();
        let mut chunk_data_vec = Vec::new();
        let mut chunk_offset = 0u32;

        for (_idx, (name, chunk)) in chunks.iter().enumerate() {
            let name_idx = string_pool.add(name);
            let path_idx = string_pool.add(&format!("{}.kaubo", name));
            
            // 编码 Chunk
            let encoded = encode_chunk(chunk).expect("Failed to encode chunk");
            let chunk_size = encoded.len() as u32;
            
            module_entries.push(crate::vm::binary::ModuleEntry {
                name_idx,
                source_path_idx: path_idx,
                chunk_offset,
                chunk_size,
                shape_start: 0,
                shape_count: 0,
                export_start: 0,
                export_count: 0,
                import_start: 0,
                import_count: 0,
            });
            
            chunk_data_vec.extend_from_slice(&encoded);
            chunk_offset += chunk_size;
        }

        // 写入 String Pool
        writer.write_section(SectionKind::StringPool, &string_pool.serialize());

        // 写入 Module Table
        let mut module_table = crate::vm::binary::ModuleTable::new();
        for entry in module_entries {
            module_table.add(entry);
        }
        writer.write_section(SectionKind::ModuleTable, &module_table.serialize());

        // 写入 Chunk Data
        writer.write_section(SectionKind::ChunkData, &chunk_data_vec);

        // 设置入口为最后一个模块（主模块）
        let entry_idx = (chunks.len() - 1) as u16;
        writer.set_entry(entry_idx, 0);

        writer.finish()
    }

    #[test]
    #[ignore = "需要 NativeFileSystem 支持"] // 暂时忽略，需要完整的文件系统支持
    fn test_e2e_hello_example() {
        // 编译 hello 示例
        let binary = compile_to_binary("main.kaubo", "examples/hello");
        
        // 验证二进制不为空
        assert!(binary.len() > 128); // 至少包含 Header
        
        // 加载并执行
        let mut vm = VM::new();
        let result = vm.execute_binary(binary);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), InterpretResult::Ok);
    }

    #[test]
    fn test_compile_simple_to_binary() {
        // 简单的端到端测试（不依赖文件系统）
        // 注意：使用纯数值运算，因为 Chunk 编码暂时不支持字符串常量
        let source = r#"
var x = 40;
var y = 2;
var result = x + y;
"#;

        // 1. 解析
        let mut lexer = build_lexer();
        lexer.feed(source.as_bytes()).unwrap();
        lexer.terminate().unwrap();
        let mut parser = Parser::new(lexer);
        let ast = parser.parse().expect("Failed to parse");

        // 2. 编译
        use crate::passes::codegen::compile;
        let (chunk, _) = compile(&ast).expect("Failed to compile");

        // 3. 编码为二进制
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);
        
        // String Pool
        let mut string_pool = crate::vm::binary::StringPool::new();
        let name_idx = string_pool.add("main");
        let path_idx = string_pool.add("main.kaubo");
        writer.write_section(SectionKind::StringPool, &string_pool.serialize());

        // Module Table
        let mut module_table = crate::vm::binary::ModuleTable::new();
        let chunk_data = encode_chunk(&chunk).unwrap();
        module_table.add(crate::vm::binary::ModuleEntry {
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

        // Chunk Data
        writer.write_section(SectionKind::ChunkData, &chunk_data);
        
        writer.set_entry(0, 0);
        let binary = writer.finish();

        // 4. 验证二进制
        assert!(binary.len() > 128);
        
        // 5. 加载并执行
        let mut vm = VM::new();
        let result = vm.execute_binary(binary);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), InterpretResult::Ok);
    }

    #[test]
    fn test_binary_roundtrip_with_execution() {
        // 创建一个简单的 Chunk
        use crate::vm::core::bytecode::OpCode;
        
        let mut chunk = Chunk::new();
        chunk.write_op(OpCode::LoadConst0, 1);
        chunk.add_constant(crate::vm::core::Value::int(42));
        chunk.write_op(OpCode::ReturnValue, 1);

        // 编码为二进制
        let options = WriteOptions {
            build_mode: BuildMode::Debug,
            compress: false,
            strip_debug: false,
            source_map_external: false,
        };

        let mut writer = BinaryWriter::new(options);
        
        let mut string_pool = crate::vm::binary::StringPool::new();
        let name_idx = string_pool.add("test");
        let path_idx = string_pool.add("test.kaubo");
        writer.write_section(SectionKind::StringPool, &string_pool.serialize());

        let mut module_table = crate::vm::binary::ModuleTable::new();
        let chunk_data = encode_chunk(&chunk).unwrap();
        module_table.add(crate::vm::binary::ModuleEntry {
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
        writer.write_section(SectionKind::ChunkData, &chunk_data);
        writer.set_entry(0, 0);
        
        let binary = writer.finish();

        // 加载并执行
        let loader = BinaryLoader::from_bytes(binary).unwrap();
        let module = loader.load_entry_module().unwrap();
        
        assert_eq!(module.name, "test");
        
        // 执行
        let mut vm = VM::new();
        let result = vm.interpret(&module.chunk);
        
        assert_eq!(result, InterpretResult::Ok);
    }
}
