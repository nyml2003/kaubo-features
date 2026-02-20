//! Emitter implementations

mod bytecode_emitter;
mod file_emitter;
mod stdout_emitter;

pub use bytecode_emitter::BytecodeEmitter;
pub use file_emitter::FileEmitter;
pub use stdout_emitter::StdoutEmitter;
