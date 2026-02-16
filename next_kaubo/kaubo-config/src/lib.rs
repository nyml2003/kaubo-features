//! Kaubo Config - Pure configuration data structures
//!
//! This crate contains only data structures, no logic or global state.
//! It serves as the shared configuration vocabulary across all Kaubo crates.

use serde::{Deserialize, Serialize};

/// Configuration for compiler behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// Whether to emit debug information
    #[serde(default = "default_emit_debug_info")]
    pub emit_debug_info: bool,
}

fn default_emit_debug_info() -> bool {
    true
}

/// Configuration for execution limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitConfig {
    /// Maximum stack size
    #[serde(default = "default_max_stack_size")]
    pub max_stack_size: usize,
    /// Maximum recursion depth
    #[serde(default = "default_max_recursion_depth")]
    pub max_recursion_depth: usize,
}

fn default_max_stack_size() -> usize {
    10240
}

fn default_max_recursion_depth() -> usize {
    256
}

/// Configuration for lexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexerConfig {
    /// Input buffer size in bytes
    #[serde(default = "default_lexer_buffer_size")]
    pub buffer_size: usize,
}

fn default_lexer_buffer_size() -> usize {
    102400 // 100KB
}

/// Configuration for VM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Initial stack capacity (slots)
    #[serde(default = "default_vm_initial_stack_size")]
    pub initial_stack_size: usize,
    /// Initial call frames capacity
    #[serde(default = "default_vm_initial_frames_capacity")]
    pub initial_frames_capacity: usize,
    /// Inline cache capacity
    #[serde(default = "default_vm_inline_cache_capacity")]
    pub inline_cache_capacity: usize,
}

fn default_vm_initial_stack_size() -> usize {
    256
}

fn default_vm_initial_frames_capacity() -> usize {
    64
}

fn default_vm_inline_cache_capacity() -> usize {
    64
}

/// Configuration for coroutine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoroutineConfig {
    /// Initial stack capacity (slots)
    #[serde(default = "default_coroutine_initial_stack_size")]
    pub initial_stack_size: usize,
    /// Initial call frames capacity
    #[serde(default = "default_coroutine_initial_frames_capacity")]
    pub initial_frames_capacity: usize,
}

fn default_coroutine_initial_stack_size() -> usize {
    256
}

fn default_coroutine_initial_frames_capacity() -> usize {
    64
}

/// Log level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogLevel {
    Error,
    #[default]
    Warn,
    Info,
    Debug,
    Trace,
}


impl LogLevel {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

/// Log targets configuration for different components
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogTargets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexer: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm: Option<LogLevel>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Global log level
    #[serde(default)]
    pub level: LogLevel,
    /// Per-component log levels
    #[serde(default)]
    pub targets: LogTargets,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Warn,
            targets: LogTargets::default(),
        }
    }
}

/// Runtime configuration options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeOptions {
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub limits: LimitConfig,
    #[serde(default)]
    pub lexer: LexerConfig,
    #[serde(default)]
    pub vm: VmConfig,
    #[serde(default)]
    pub coroutine: CoroutineConfig,
}

/// Built-in profiles
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Profile {
    Silent,
    #[default]
    Default,
    Dev,
    Debug,
    Trace,
}


impl Profile {
    /// Get the name of the profile
    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Silent => "silent",
            Profile::Default => "default",
            Profile::Dev => "dev",
            Profile::Debug => "debug",
            Profile::Trace => "trace",
        }
    }

    /// Apply profile defaults to runtime options
    pub fn apply(&self, options: &mut RuntimeOptions) {
        let (level, stack_mult, frame_mult) = match self {
            Profile::Silent => (LogLevel::Error, 1, 1),
            Profile::Default => (LogLevel::Warn, 1, 1),
            Profile::Dev => (LogLevel::Info, 1, 1),
            Profile::Debug => (LogLevel::Debug, 2, 2),
            Profile::Trace => (LogLevel::Trace, 4, 4),
        };

        // Apply to logging
        options.logging.level = level;
        options.logging.targets = LogTargets {
            lexer: Some(level),
            parser: Some(level),
            compiler: Some(level),
            vm: Some(level),
        };

        // Apply multipliers to VM and coroutine
        options.vm.initial_stack_size = default_vm_initial_stack_size() * stack_mult;
        options.vm.initial_frames_capacity = default_vm_initial_frames_capacity() * frame_mult;
        options.coroutine.initial_stack_size = default_coroutine_initial_stack_size() * stack_mult;
        options.coroutine.initial_frames_capacity = default_coroutine_initial_frames_capacity() * frame_mult;
    }
}

/// Kaubo configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KauboConfig {
    /// Configuration format version
    #[serde(default = "default_version")]
    pub version: String,
    /// Profile for default values
    #[serde(default)]
    pub profile: Profile,
    /// Compiler options
    #[serde(default)]
    pub compiler_options: CompilerConfig,
    /// Runtime options
    #[serde(default)]
    pub runtime_options: RuntimeOptions,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Default for KauboConfig {
    fn default() -> Self {
        let mut config = Self {
            version: default_version(),
            profile: Profile::Default,
            compiler_options: CompilerConfig::default(),
            runtime_options: RuntimeOptions::default(),
        };
        // Apply default profile
        config.profile.apply(&mut config.runtime_options);
        config
    }
}

impl RuntimeOptions {
    /// Merge another RuntimeOptions into self, preferring other's values
    fn merge(&mut self, other: &RuntimeOptions) {
        // Merge logging
        if other.logging.level != LogLevel::default() {
            self.logging.level = other.logging.level;
        }
        if other.logging.targets.lexer.is_some() {
            self.logging.targets.lexer = other.logging.targets.lexer;
        }
        if other.logging.targets.parser.is_some() {
            self.logging.targets.parser = other.logging.targets.parser;
        }
        if other.logging.targets.compiler.is_some() {
            self.logging.targets.compiler = other.logging.targets.compiler;
        }
        if other.logging.targets.vm.is_some() {
            self.logging.targets.vm = other.logging.targets.vm;
        }
        
        // Merge limits
        if other.limits.max_stack_size != default_max_stack_size() {
            self.limits.max_stack_size = other.limits.max_stack_size;
        }
        if other.limits.max_recursion_depth != default_max_recursion_depth() {
            self.limits.max_recursion_depth = other.limits.max_recursion_depth;
        }
        
        // Merge lexer
        if other.lexer.buffer_size != default_lexer_buffer_size() {
            self.lexer.buffer_size = other.lexer.buffer_size;
        }
        
        // Merge VM
        if other.vm.initial_stack_size != default_vm_initial_stack_size() {
            self.vm.initial_stack_size = other.vm.initial_stack_size;
        }
        if other.vm.initial_frames_capacity != default_vm_initial_frames_capacity() {
            self.vm.initial_frames_capacity = other.vm.initial_frames_capacity;
        }
        if other.vm.inline_cache_capacity != default_vm_inline_cache_capacity() {
            self.vm.inline_cache_capacity = other.vm.inline_cache_capacity;
        }
        
        // Merge coroutine
        if other.coroutine.initial_stack_size != default_coroutine_initial_stack_size() {
            self.coroutine.initial_stack_size = other.coroutine.initial_stack_size;
        }
        if other.coroutine.initial_frames_capacity != default_coroutine_initial_frames_capacity() {
            self.coroutine.initial_frames_capacity = other.coroutine.initial_frames_capacity;
        }
    }
}

impl KauboConfig {
    /// Create config from a specific profile
    pub fn from_profile(profile: Profile) -> Self {
        let mut config = Self {
            version: default_version(),
            profile,
            compiler_options: CompilerConfig::default(),
            runtime_options: RuntimeOptions::default(),
        };
        profile.apply(&mut config.runtime_options);
        config
    }

    /// Load config from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let user_config: KauboConfig = serde_json::from_str(json)?;
        
        // Start with profile defaults
        let mut config = Self::from_profile(user_config.profile);
        
        // Apply compiler options from user config
        config.compiler_options = user_config.compiler_options;
        
        // Merge runtime options (user values override profile defaults)
        config.runtime_options.merge(&user_config.runtime_options);
        
        Ok(config)
    }

    /// Load config from file
    pub fn from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_json(&content)?)
    }

    /// Find and load config from current directory or parents
    pub fn find_and_load() -> Option<Self> {
        let mut current_dir = std::env::current_dir().ok()?;
        loop {
            let config_path = current_dir.join("kaubo.json");
            if config_path.exists() {
                return Self::from_file(&config_path).ok();
            }
            if !current_dir.pop() {
                break;
            }
        }
        None
    }
}

// Implement Default for all config structs
impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            emit_debug_info: default_emit_debug_info(),
        }
    }
}

impl Default for LimitConfig {
    fn default() -> Self {
        Self {
            max_stack_size: default_max_stack_size(),
            max_recursion_depth: default_max_recursion_depth(),
        }
    }
}

impl Default for LexerConfig {
    fn default() -> Self {
        Self {
            buffer_size: default_lexer_buffer_size(),
        }
    }
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            initial_stack_size: default_vm_initial_stack_size(),
            initial_frames_capacity: default_vm_initial_frames_capacity(),
            inline_cache_capacity: default_vm_inline_cache_capacity(),
        }
    }
}

impl Default for CoroutineConfig {
    fn default() -> Self {
        Self {
            initial_stack_size: default_coroutine_initial_stack_size(),
            initial_frames_capacity: default_coroutine_initial_frames_capacity(),
        }
    }
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
        assert_eq!(cfg.max_stack_size, 10240);
        assert_eq!(cfg.max_recursion_depth, 256);
    }

    #[test]
    fn test_phase_as_str() {
        assert_eq!(Phase::Lexer.as_str(), "lexer");
        assert_eq!(Phase::Vm.target(), "kaubo::vm");
    }

    #[test]
    fn test_profile_apply() {
        let mut options = RuntimeOptions::default();
        Profile::Debug.apply(&mut options);
        assert_eq!(options.logging.level, LogLevel::Debug);
        assert_eq!(options.vm.initial_stack_size, 512); // 2x
    }

    #[test]
    fn test_config_from_json() {
        let json = r#"{
            "profile": "dev",
            "runtime_options": {
                "logging": {
                    "level": "debug",
                    "targets": {
                        "lexer": "trace"
                    }
                }
            }
        }"#;
        let config = KauboConfig::from_json(json).unwrap();
        assert!(matches!(config.profile, Profile::Dev));
        assert_eq!(config.runtime_options.logging.level, LogLevel::Debug);
        assert_eq!(config.runtime_options.logging.targets.lexer, Some(LogLevel::Trace));
    }

    #[test]
    fn test_config_from_profile() {
        let config = KauboConfig::from_profile(Profile::Silent);
        assert_eq!(config.runtime_options.logging.level, LogLevel::Error);
    }
}
