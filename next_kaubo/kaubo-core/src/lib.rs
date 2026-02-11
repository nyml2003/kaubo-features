//! Kaubo Core - Core compiler (pure logic, no IO)
//!
//! Contains lexer, parser, compiler, and virtual machine.
//! Only operates on in-memory data structures, no file IO or terminal output.
//!
//! Configuration is passed explicitly via parameters, not via global state.

pub mod compiler;
pub mod kit;
pub mod runtime;

// Re-export common types
pub use runtime::value::Value;
pub use runtime::bytecode::chunk::Chunk;
pub use runtime::vm::{VM, InterpretResult};

// Re-export config types from kaubo-config
pub use kaubo_config::{CompilerConfig, LimitConfig, Phase};
