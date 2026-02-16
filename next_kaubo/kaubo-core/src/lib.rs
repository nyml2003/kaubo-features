//! Kaubo Core - Core compiler (pure logic, no IO)
//!
//! Contains lexer, parser, compiler, and virtual machine.
//! Only operates on in-memory data structures, no file IO or terminal output.
//!
//! Configuration is passed explicitly via parameters, not via global state.
//!
//! # 主要导出
//!
//! - `Value` - Kaubo 运行时值类型
//! - `VM` - 字节码虚拟机
//! - `Chunk` - 字节码块
//! - `InterpretResult` - 解释执行结果

extern crate alloc;

// 模块声明
pub mod compiler;
pub mod kit;
pub mod runtime;

// ==================== 核心导出（所有用户）====================

/// Kaubo 值类型
pub use runtime::value::Value;

/// 字节码块
pub use runtime::bytecode::chunk::Chunk;

/// 虚拟机
pub use runtime::vm::VM;

/// VM 配置
pub use runtime::vm::VMConfig;

/// 解释执行结果
pub use runtime::vm::InterpretResult;

/// 对象形状（用于运算符重载和方法调用）
pub use runtime::object::ObjShape;

// ==================== 编译器导出（高级用户）====================

/// Lexer 构建器
pub mod lexer {
    //! Lexer 构建工具
    pub use crate::compiler::lexer::builder::{build_lexer, build_lexer_with_config, LexerConfig};
}

/// Parser
pub mod parser {
    //! 语法解析器
    pub use crate::compiler::parser::parser::Parser;
    pub use crate::compiler::parser::Module;
    pub use crate::compiler::parser::TypeChecker;
}

/// 底层工具模块
pub mod utils {
    //! 底层工具（Lexer 核心、环形缓冲区等）
    pub use crate::kit::lexer::{Lexer, SourcePosition};
}
