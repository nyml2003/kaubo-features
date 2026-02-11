//! CLI 配置
//!
//! 包含 CLI 特有的配置：日志配置和运行配置的组合

use tracing::Level;

/// CLI 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub global: Level,
    pub lexer: Option<Level>,
    pub parser: Option<Level>,
    pub compiler: Option<Level>,
    pub vm: Option<Level>,
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

impl LogConfig {
    /// Get log level for a specific target
    pub fn level_for(&self, target: &str) -> Level {
        match target {
            "kaubo::lexer" => self.lexer.unwrap_or(self.global),
            "kaubo::parser" => self.parser.unwrap_or(self.global),
            "kaubo::compiler" => self.compiler.unwrap_or(self.global),
            "kaubo::vm" => self.vm.unwrap_or(self.global),
            _ => self.global,
        }
    }
}
