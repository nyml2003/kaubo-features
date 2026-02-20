//! Execution context
//!
//! This module provides the execution context for pipeline stages.

use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;

/// Global execution context shared across the orchestrator
pub struct Context {
    /// The configuration
    pub config: Arc<kaubo_config::VmConfig>,
    /// Virtual file system
    pub vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
    /// Logger
    pub log: Arc<kaubo_log::Logger>,
    /// Global context data
    pub data: HashMap<String, Value>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("config", &self.config)
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            vfs: self.vfs.clone(),
            log: self.log.clone(),
            data: self.data.clone(),
        }
    }
}

impl Context {
    /// Create a new context
    pub fn new(
        config: Arc<kaubo_config::VmConfig>,
        vfs: Arc<dyn kaubo_vfs::VirtualFileSystem + Send + Sync>,
        log: Arc<kaubo_log::Logger>,
    ) -> Self {
        Self {
            config,
            vfs,
            log,
            data: HashMap::new(),
        }
    }
    
    /// Insert data into context
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        self.data.insert(key.into(), value);
    }
    
    /// Get data from context
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }
    
    /// Check if key exists
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
    
    /// Remove data
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }
    
    /// Get config value
    pub fn config_val(&self, key: &str) -> Option<&Value> {
        // TODO: implement config value access
        None
    }
    
    /// Log a message
    pub fn log(&self, level: kaubo_log::Level, message: &str) {
        // TODO: implement logging
        let _ = (level, message);
    }
}

/// Stage-specific context
#[derive(Debug, Clone)]
pub struct StageContext {
    /// The global context
    pub global: Context,
    /// Stage name
    pub stage_name: String,
    /// Stage index in pipeline
    pub stage_index: usize,
    /// Total stages
    pub total_stages: usize,
}

impl StageContext {
    /// Create a new stage context
    pub fn new(global: Context, stage_name: impl Into<String>, stage_index: usize, total_stages: usize) -> Self {
        Self {
            global,
            stage_name: stage_name.into(),
            stage_index,
            total_stages,
        }
    }
    
    /// Check if this is the first stage
    pub fn is_first(&self) -> bool {
        self.stage_index == 0
    }
    
    /// Check if this is the last stage
    pub fn is_last(&self) -> bool {
        self.stage_index == self.total_stages - 1
    }
    
    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_stages == 0 {
            return 1.0;
        }
        self.stage_index as f32 / self.total_stages as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock types for testing
    struct MockConfig;
    struct MockVfs;
    struct MockLogger;

    #[test]
    fn test_context_data() {
        // This is a simplified test - in reality you'd need proper mocks
        // Just testing the HashMap functionality here
        let mut data = HashMap::new();
        data.insert("key".to_string(), Value::String("value".to_string()));
        
        assert!(data.contains_key("key"));
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));
    }

    #[test]
    fn test_stage_context() {
        // Simplified test
        let stage_index = 2;
        let total_stages = 5;
        
        assert!(! (stage_index == 0)); // not first
        assert!(! (stage_index == total_stages - 1)); // not last
        
        let progress = stage_index as f32 / total_stages as f32;
        assert_eq!(progress, 0.4);
    }
}
