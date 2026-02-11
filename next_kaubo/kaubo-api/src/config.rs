//! API 层配置
//!
//! 包含执行配置 RunConfig 和全局单例（供 CLI 使用）

use kaubo_config::{CompilerConfig, LimitConfig};
use once_cell::sync::OnceCell;

/// Execution configuration
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Whether to show execution steps
    pub show_steps: bool,
    /// Whether to dump bytecode after compilation
    pub dump_bytecode: bool,
    /// Compiler configuration
    pub compiler: CompilerConfig,
    /// Execution limits
    pub limits: LimitConfig,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            show_steps: false,
            dump_bytecode: false,
            compiler: CompilerConfig::default(),
            limits: LimitConfig::default(),
        }
    }
}

// Global config singleton for CLI convenience
static GLOBAL_CONFIG: OnceCell<RunConfig> = OnceCell::new();

/// Initialize global configuration (must be called once before any operation)
///
/// # Panics
/// If config is already initialized
pub fn init(config: RunConfig) {
    GLOBAL_CONFIG
        .set(config)
        .expect("Config already initialized");
}

/// Get global config reference
///
/// # Panics
/// If config is not initialized
pub fn config() -> &'static RunConfig {
    GLOBAL_CONFIG.get().expect("Config not initialized")
}

/// Check if config is initialized
pub fn is_initialized() -> bool {
    GLOBAL_CONFIG.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_run_config() {
        let cfg = RunConfig::default();
        assert!(!cfg.show_steps);
        assert!(!cfg.dump_bytecode);
    }
}
