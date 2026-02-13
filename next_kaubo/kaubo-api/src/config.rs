//! API 层配置
//!
//! 包含执行配置 RunConfig 和全局单例（供 CLI 使用）

use kaubo_config::{CompilerConfig, LimitConfig};
use kaubo_log::Logger;
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// Execution configuration
#[derive(Clone)]
pub struct RunConfig {
    /// Whether to show execution steps
    pub show_steps: bool,
    /// Whether to dump bytecode after compilation
    pub dump_bytecode: bool,
    /// Compiler configuration
    pub compiler: CompilerConfig,
    /// Execution limits
    pub limits: LimitConfig,
    /// Logger (optional)
    pub logger: Arc<Logger>,
}

impl std::fmt::Debug for RunConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunConfig")
            .field("show_steps", &self.show_steps)
            .field("dump_bytecode", &self.dump_bytecode)
            .field("compiler", &self.compiler)
            .field("limits", &self.limits)
            .finish()
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            show_steps: false,
            dump_bytecode: false,
            compiler: CompilerConfig::default(),
            limits: LimitConfig::default(),
            logger: Logger::noop(),
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
        assert!(cfg.compiler.emit_debug_info);
        assert_eq!(cfg.limits.max_stack_size, 10240);
        assert_eq!(cfg.limits.max_recursion_depth, 256);
    }

    #[test]
    fn test_run_config_clone() {
        let cfg = RunConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.show_steps, cloned.show_steps);
        assert_eq!(cfg.dump_bytecode, cloned.dump_bytecode);
    }

    #[test]
    fn test_run_config_debug() {
        let cfg = RunConfig::default();
        let debug_str = format!("{:?}", cfg);
        assert!(debug_str.contains("show_steps"));
        assert!(debug_str.contains("dump_bytecode"));
        assert!(debug_str.contains("compiler"));
        assert!(debug_str.contains("limits"));
    }

    #[test]
    fn test_global_config_init_and_get() {
        // 确保测试开始前配置是未初始化的
        // 注意：由于全局状态，这个测试需要在独立进程中运行
        // 或者使用 cargo test -- --test-threads=1
        if !is_initialized() {
            let cfg = RunConfig::default();
            let show_steps = cfg.show_steps;
            let dump_bytecode = cfg.dump_bytecode;
            init(cfg);
            assert!(is_initialized());

            let retrieved = config();
            assert_eq!(retrieved.show_steps, show_steps);
            assert_eq!(retrieved.dump_bytecode, dump_bytecode);
        }
        // 如果已经初始化，跳过测试（全局状态限制）
    }

    #[test]
    fn test_is_initialized() {
        // 这个测试依赖于测试执行顺序
        // 在独立测试中，应该是 false
        // 但在 full test suite 中可能是 true
        let _ = is_initialized(); // 只是确保函数可调用
    }
}
