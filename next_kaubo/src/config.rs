//! 全局配置系统
//!
//! 提供线程安全的全局配置单例，支持日志级别、执行限制等配置。
//!
//! # 使用示例
//! ```
//! use kaubo::config::{Config, LogConfig, init, config};
//! use tracing::Level;
//!
//! let cfg = Config {
//!     log: LogConfig {
//!         global: Level::DEBUG,
//!         ..Default::default()
//!     },
//!     ..Default::default()
//! };
//!
//! init(cfg);
//! // 之后通过 config() 全局访问
//! ```

use once_cell::sync::OnceCell;
use tracing::Level;

static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

/// 初始化全局配置（必须在任何操作前调用一次）
///
/// # Panics
/// 如果配置已经初始化，会 panic
pub fn init(config: Config) {
    GLOBAL_CONFIG
        .set(config)
        .expect("Config already initialized");
}

/// 获取全局配置引用
///
/// # Panics
/// 如果配置未初始化，会 panic
pub fn config() -> &'static Config {
    GLOBAL_CONFIG.get().expect("Config not initialized")
}

/// 检查配置是否已初始化
pub fn is_initialized() -> bool {
    GLOBAL_CONFIG.get().is_some()
}

/// 全局配置结构
#[derive(Debug, Clone)]
pub struct Config {
    /// 日志配置
    pub log: LogConfig,
    /// 执行限制配置
    pub limits: LimitConfig,
    /// 编译器配置
    pub compiler: CompilerConfig,
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 全局默认日志级别
    pub global: Level,
    /// Lexer 日志级别（None 表示使用 global）
    pub lexer: Option<Level>,
    /// Parser 日志级别
    pub parser: Option<Level>,
    /// Compiler 日志级别
    pub compiler: Option<Level>,
    /// VM 日志级别
    pub vm: Option<Level>,
}

/// 执行限制配置
#[derive(Debug, Clone)]
pub struct LimitConfig {
    /// 最大栈大小
    pub max_stack_size: usize,
    /// 最大递归深度
    pub max_recursion_depth: usize,
}

/// 编译器配置
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// 是否生成调试信息
    pub emit_debug_info: bool,
}

/// 执行阶段枚举
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Lexer,
    Parser,
    Compiler,
    Vm,
}

impl LogConfig {
    /// 获取指定阶段的实际日志级别
    ///
    /// 如果该阶段有特定配置则返回特定级别，否则返回全局级别
    pub fn level_for(&self, phase: Phase) -> Level {
        let specific = match phase {
            Phase::Lexer => self.lexer,
            Phase::Parser => self.parser,
            Phase::Compiler => self.compiler,
            Phase::Vm => self.vm,
        };
        specific.unwrap_or(self.global)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log: LogConfig::default(),
            limits: LimitConfig::default(),
            compiler: CompilerConfig::default(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            global: Level::INFO,
            lexer: None,
            parser: None,
            compiler: None,
            vm: None,
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

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            emit_debug_info: true,
        }
    }
}

impl Phase {
    /// 获取阶段的字符串名称
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Lexer => "lexer",
            Phase::Parser => "parser",
            Phase::Compiler => "compiler",
            Phase::Vm => "vm",
        }
    }

    /// 获取阶段的日志目标名称
    pub fn target(&self) -> String {
        format!("kaubo::{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.log.global, Level::INFO);
        assert_eq!(cfg.limits.max_stack_size, 1024);
    }

    #[test]
    fn test_log_level_for() {
        let cfg = LogConfig {
            global: Level::WARN,
            lexer: Some(Level::DEBUG),
            parser: None,
            compiler: None,
            vm: None,
        };

        assert_eq!(cfg.level_for(Phase::Lexer), Level::DEBUG);
        assert_eq!(cfg.level_for(Phase::Parser), Level::WARN);
    }

    #[test]
    fn test_phase_as_str() {
        assert_eq!(Phase::Lexer.as_str(), "lexer");
        assert_eq!(Phase::Vm.target(), "kaubo::vm");
    }
}
