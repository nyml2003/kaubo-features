//! Kaubo Config - Pure configuration data structures
//!
//! This crate contains only data structures, no logic or global state.
//! It serves as the shared configuration vocabulary across all Kaubo crates.

/// Configuration for compiler behavior
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Whether to emit debug information
    pub emit_debug_info: bool,
}

/// Configuration for execution limits
#[derive(Debug, Clone)]
pub struct LimitConfig {
    /// Maximum stack size
    pub max_stack_size: usize,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
}

/// Execution phase enum for phase-specific configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Lexer,
    Parser,
    Compiler,
    Vm,
}

impl Phase {
    /// Get the string name of the phase
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Lexer => "lexer",
            Phase::Parser => "parser",
            Phase::Compiler => "compiler",
            Phase::Vm => "vm",
        }
    }

    /// Get the log target name for this phase
    pub fn target(&self) -> String {
        format!("kaubo::{}", self.as_str())
    }
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            emit_debug_info: true,
        }
    }
}

impl Default for LimitConfig {
    fn default() -> Self {
        Self {
            max_stack_size: 1024,
            max_recursion_depth: 256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_compiler_config() {
        let cfg = CompilerConfig::default();
        assert!(cfg.emit_debug_info);
    }

    #[test]
    fn test_default_limit_config() {
        let cfg = LimitConfig::default();
        assert_eq!(cfg.max_stack_size, 1024);
        assert_eq!(cfg.max_recursion_depth, 256);
    }

    #[test]
    fn test_phase_as_str() {
        assert_eq!(Phase::Lexer.as_str(), "lexer");
        assert_eq!(Phase::Vm.target(), "kaubo::vm");
    }
}
