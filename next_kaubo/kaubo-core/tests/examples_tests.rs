//! 示例代码解析测试
//! 
//! 验证 examples/ 目录下的所有示例代码可以正确解析

use kaubo_core::compiler::module::multi_file::MultiFileCompiler;
use kaubo_vfs::MemoryFileSystem;
use std::path::Path;

/// 验证多模块示例可以正确解析
#[test]
fn test_multi_module_example() {
    let fs = MemoryFileSystem::with_files(vec![
        ("math.kaubo", include_str!("../../examples/multi_module/math.kaubo").as_bytes().to_vec()),
        ("utils.kaubo", include_str!("../../examples/multi_module/utils.kaubo").as_bytes().to_vec()),
        ("main.kaubo", include_str!("../../examples/multi_module/main.kaubo").as_bytes().to_vec()),
    ]);

    let vfs = Box::new(fs);
    let mut compiler = MultiFileCompiler::new(vfs, Path::new(""));
    
    let result = compiler.compile_entry("main.kaubo");
    assert!(result.is_ok(), "Multi-module example should compile: {:?}", result.err());
    
    let compile_result = result.unwrap();
    assert_eq!(compile_result.units.len(), 3, "Should have 3 modules (math, utils, main)");
}

/// 验证导入链示例可以正确解析
#[test]
fn test_import_chain_example() {
    let fs = MemoryFileSystem::with_files(vec![
        ("logger.kaubo", include_str!("../../examples/import_chain/logger.kaubo").as_bytes().to_vec()),
        ("database.kaubo", include_str!("../../examples/import_chain/database.kaubo").as_bytes().to_vec()),
        ("app.kaubo", include_str!("../../examples/import_chain/app.kaubo").as_bytes().to_vec()),
        ("main.kaubo", include_str!("../../examples/import_chain/main.kaubo").as_bytes().to_vec()),
    ]);

    let vfs = Box::new(fs);
    let mut compiler = MultiFileCompiler::new(vfs, Path::new(""));
    
    let result = compiler.compile_entry("main.kaubo");
    assert!(result.is_ok(), "Import chain example should compile: {:?}", result.err());
    
    let compile_result = result.unwrap();
    assert_eq!(compile_result.units.len(), 4, "Should have 4 modules");
}

/// 验证菱形依赖示例可以正确解析
#[test]
fn test_diamond_deps_example() {
    let fs = MemoryFileSystem::with_files(vec![
        ("common.kaubo", include_str!("../../examples/diamond_deps/common.kaubo").as_bytes().to_vec()),
        ("math.kaubo", include_str!("../../examples/diamond_deps/math.kaubo").as_bytes().to_vec()),
        ("strings.kaubo", include_str!("../../examples/diamond_deps/strings.kaubo").as_bytes().to_vec()),
        ("main.kaubo", include_str!("../../examples/diamond_deps/main.kaubo").as_bytes().to_vec()),
    ]);

    let vfs = Box::new(fs);
    let mut compiler = MultiFileCompiler::new(vfs, Path::new(""));
    
    let result = compiler.compile_entry("main.kaubo");
    assert!(result.is_ok(), "Diamond deps example should compile: {:?}", result.err());
    
    let compile_result = result.unwrap();
    assert_eq!(compile_result.units.len(), 4, "Should have 4 modules");
}

/// 验证嵌套导入示例可以正确解析
#[test]
fn test_nested_import_example() {
    let fs = MemoryFileSystem::with_files(vec![
        ("std/list.kaubo", include_str!("../../examples/nested_import/std/list.kaubo").as_bytes().to_vec()),
        ("std/math.kaubo", include_str!("../../examples/nested_import/std/math.kaubo").as_bytes().to_vec()),
        ("app/utils.kaubo", include_str!("../../examples/nested_import/app/utils.kaubo").as_bytes().to_vec()),
        ("main.kaubo", include_str!("../../examples/nested_import/main.kaubo").as_bytes().to_vec()),
    ]);

    let vfs = Box::new(fs);
    let mut compiler = MultiFileCompiler::new(vfs, Path::new(""));
    
    let result = compiler.compile_entry("main.kaubo");
    assert!(result.is_ok(), "Nested import example should compile: {:?}", result.err());
    
    let compile_result = result.unwrap();
    assert!(compile_result.units.len() >= 4, "Should have at least 4 modules");
}
