//! 多文件编译端到端测试

use kaubo_core::compiler::module::{MultiFileCompiler, MultiFileCompileResult};
use kaubo_vfs::MemoryFileSystem;

/// 创建测试用的内存文件系统
fn create_test_fs(files: Vec<(&str, &str)>) -> MemoryFileSystem {
    let files_with_content: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .map(|(path, content)| (path.to_string(), content.as_bytes().to_vec()))
        .collect();
    
    MemoryFileSystem::with_files(
        files_with_content
            .into_iter()
            .map(|(p, c)| (p, c))
    )
}

/// 编译入口文件并返回结果
fn compile_entry(files: Vec<(&str, &str)>, entry: &str) -> MultiFileCompileResult {
    let fs = create_test_fs(files);
    let mut compiler = MultiFileCompiler::new(Box::new(fs), "/");
    compiler.compile_entry(entry).expect("Compilation failed")
}

#[test]
fn test_single_file_no_imports() {
    let result = compile_entry(
        vec![
            ("/main.kaubo", "var x = 42; return x;"),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 1);
    assert_eq!(result.units[0].import_path, "__entry__");
}

#[test]
fn test_two_files_simple_import() {
    let result = compile_entry(
        vec![
            ("/main.kaubo", "import math; return math.PI;"),
            ("/math.kaubo", "pub var PI = 3.14159;"),
        ],
        "/main.kaubo",
    );

    // 应该有两个单元：math 和 __entry__
    assert_eq!(result.units.len(), 2);
    
    // 依赖项在前
    assert_eq!(result.units[0].import_path, "math");
    assert_eq!(result.units[1].import_path, "__entry__");
}

#[test]
fn test_nested_module_path() {
    let result = compile_entry(
        vec![
            ("/main.kaubo", "import utils.string; var s = utils.string.HELLO;"),
            ("/utils/string.kaubo", "pub var HELLO = \"hello\";"),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 2);
    assert_eq!(result.units[0].import_path, "utils.string");
}

#[test]
fn test_multiple_imports() {
    let result = compile_entry(
        vec![
            ("/main.kaubo", r#"
                import math;
                import string_utils;
                var pi = math.PI;
                var msg = string_utils.greet("world");
                return pi;
            "#),
            ("/math.kaubo", "pub var PI = 3.14;"),
            ("/string_utils.kaubo", r#"
                pub var greet = |name: string| -> string {
                    return "Hello, " + name;
                };
            "#),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 3);
    
    let paths: Vec<_> = result.units.iter().map(|u| u.import_path.clone()).collect();
    assert!(paths.contains(&"math".to_string()));
    assert!(paths.contains(&"string_utils".to_string()));
    assert_eq!(paths.last().unwrap(), "__entry__");
}

#[test]
fn test_transitive_dependency() {
    // A -> B -> C (A 依赖 B，B 依赖 C)
    let result = compile_entry(
        vec![
            ("/main.kaubo", "import b; return b.value;"),
            ("/b.kaubo", "import c; pub var value = c.DEEP_VALUE;"),
            ("/c.kaubo", "pub var DEEP_VALUE = 100;"),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 3);
    
    // 编译顺序应该满足依赖关系
    let paths: Vec<_> = result.units.iter().map(|u| u.import_path.clone()).collect();
    
    // c 应该在 b 之前
    let c_idx = paths.iter().position(|p| p == "c").unwrap();
    let b_idx = paths.iter().position(|p| p == "b").unwrap();
    assert!(c_idx < b_idx, "c should be compiled before b");
    
    // __entry__ 应该最后
    assert_eq!(paths.last().unwrap(), "__entry__");
}

#[test]
fn test_diamond_dependency() {
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let result = compile_entry(
        vec![
            ("/main.kaubo", "import b; import c; return b.val + c.val;"),
            ("/b.kaubo", "import d; pub var val = d.BASE;"),
            ("/c.kaubo", "import d; pub var val = d.BASE * 2;"),
            ("/d.kaubo", "pub var BASE = 10;"),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 4);
    
    // d 应该在 b 和 c 之前
    let paths: Vec<_> = result.units.iter().map(|u| u.import_path.clone()).collect();
    let d_idx = paths.iter().position(|p| p == "d").unwrap();
    let b_idx = paths.iter().position(|p| p == "b").unwrap();
    let c_idx = paths.iter().position(|p| p == "c").unwrap();
    
    assert!(d_idx < b_idx, "d should be compiled before b");
    assert!(d_idx < c_idx, "d should be compiled before c");
}

#[test]
fn test_module_with_exports() {
    // 测试带导出的模块
    let result = compile_entry(
        vec![
            ("/main.kaubo", r#"
                import constants;
                return constants.PI;
            "#),
            ("/constants.kaubo", r#"
                pub var PI = 3.14159;
                pub var E = 2.71828;
            "#),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 2);
    
    // 验证 constants 模块正确解析
    let constants_unit = result.units.iter().find(|u| u.import_path == "constants").unwrap();
    assert!(constants_unit.source.contains("PI"));
    assert!(constants_unit.source.contains("E"));
}

#[test]
fn test_circular_dependency_error() {
    let fs = create_test_fs(vec![
        ("/a.kaubo", "import b;"),
        ("/b.kaubo", "import c;"),
        ("/c.kaubo", "import a;"), // 循环依赖
    ]);

    let mut compiler = MultiFileCompiler::new(Box::new(fs), "/");
    let result = compiler.compile_entry("/a.kaubo");

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Circular dependency") || err_msg.contains("circular"),
            "Expected circular dependency error, got: {}", err_msg);
}

#[test]
fn test_module_not_found_error() {
    let fs = create_test_fs(vec![
        ("/main.kaubo", "import nonexistent;"),
    ]);

    let mut compiler = MultiFileCompiler::new(Box::new(fs), "/");
    let result = compiler.compile_entry("/main.kaubo");

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("NotFound"),
            "Expected 'not found' error, got: {}", err_msg);
}

#[test]
fn test_parse_error_in_imported_module() {
    let fs = create_test_fs(vec![
        ("/main.kaubo", "import broken;"),
        ("/broken.kaubo", "var x = ;"), // 语法错误
    ]);

    let mut compiler = MultiFileCompiler::new(Box::new(fs), "/");
    let result = compiler.compile_entry("/main.kaubo");

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("parse") || err_msg.contains("ParseError"),
            "Expected parse error, got: {}", err_msg);
}

#[test]
fn test_shared_module_loaded_once() {
    // A 和 B 都依赖 C，C 应该只加载一次
    let result = compile_entry(
        vec![
            ("/main.kaubo", "import a; import b; return a.val + b.val;"),
            ("/a.kaubo", "import shared; pub var val = shared.counter;"),
            ("/b.kaubo", "import shared; pub var val = shared.counter * 2;"),
            ("/shared.kaubo", "pub var counter = 1;"),
        ],
        "/main.kaubo",
    );

    // 验证 shared 只出现一次
    let shared_count = result.units.iter()
        .filter(|u| u.import_path == "shared")
        .count();
    assert_eq!(shared_count, 1, "shared module should only be loaded once");
    
    // 总共应该有 4 个单元
    assert_eq!(result.units.len(), 4);
}

#[test]
fn test_empty_import_list() {
    let result = compile_entry(
        vec![
            ("/main.kaubo", "var x = 1; return x;"),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 1);
    assert!(result.units[0].dependencies.is_empty());
}

#[test]
fn test_complex_project_structure() {
    // 模拟一个真实项目结构
    let result = compile_entry(
        vec![
            ("/main.kaubo", r#"
                import math.vector;
                import math.matrix;
                import utils.logger;
                
                var x = 1;
                var y = 2;
                
                return x + y;
            "#),
            ("/math/vector.kaubo", r#"
                pub var VERSION = "1.0";
            "#),
            ("/math/matrix.kaubo", r#"
                import math.vector;
                pub var ID = 42;
            "#),
            ("/utils/logger.kaubo", r#"
                pub var enabled = true;
            "#),
        ],
        "/main.kaubo",
    );

    assert_eq!(result.units.len(), 4);
    
    // 验证所有模块都被加载
    let paths: Vec<_> = result.units.iter().map(|u| u.import_path.clone()).collect();
    assert!(paths.contains(&"math.vector".to_string()));
    assert!(paths.contains(&"math.matrix".to_string()));
    assert!(paths.contains(&"utils.logger".to_string()));
}
