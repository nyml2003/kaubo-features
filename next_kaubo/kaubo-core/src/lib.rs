//! Kaubo Core - Core compiler (pure logic, no IO)
//!
//! Contains lexer, parser, compiler, and virtual machine.
//! Only operates on in-memory data structures, no file IO or terminal output.
//!
//! Configuration is passed explicitly via parameters, not via global state.

extern crate alloc;

// ==================== Core 层（类型定义）====================

/// Core 类型定义模块
/// 
/// 这是三层架构的中间层，包含所有核心类型的纯定义。
/// - Core 层：类型定义（本模块）
/// - Runtime 层：实现逻辑（runtime/ 目录）
/// - API 层：对外接口（kaubo-api）
pub mod core;

// ==================== 核心导出（从 core 层）====================

/// Kaubo 值类型
pub use core::Value;

/// 字节码块
pub use core::Chunk;

/// 操作码
pub use core::OpCode;

/// 运算符
pub use core::Operator;

/// 虚拟机
pub use core::VM;

/// VM 配置
pub use core::VMConfig;

/// 解释执行结果
pub use core::InterpretResult;

/// 对象形状
pub use core::ObjShape;

/// 运行时错误
pub use core::RuntimeError;

// ==================== 实现层（Runtime）====================

/// Runtime 实现模块
/// 
/// 包含 VM 执行逻辑、Value 操作、对象方法等实现。
pub mod runtime;

// ==================== 编译器（Compiler）====================

/// 编译器模块（Lexer + Parser + Compiler）
pub mod compiler;

// ==================== 工具模块（Kit）====================

/// 底层工具模块
pub mod kit;

// ==================== 向后兼容导出 ====================

/// Lexer 构建器（高级用户）
pub mod lexer {
    //! Lexer 构建工具
    pub use crate::compiler::lexer::builder::{build_lexer, build_lexer_with_config, LexerConfig};
}

/// Parser（高级用户）
pub mod parser {
    //! 语法解析器
    pub use crate::compiler::parser::parser::Parser;
    pub use crate::compiler::parser::Module;
    pub use crate::compiler::parser::TypeChecker;
}

/// 底层工具（高级用户）
pub mod utils {
    //! 底层工具（Lexer 核心、环形缓冲区等）
    pub use crate::kit::lexer::{Lexer, SourcePosition};
}
