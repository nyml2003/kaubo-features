//! Output handling for orchestrated execution
//!
//! This module provides an abstraction for capturing and routing output
//! from various components (VM print, show_source, etc.) through the
//! Emitter system.

use std::sync::{Arc, Mutex};

/// A single output entry
#[derive(Debug, Clone)]
pub enum OutputEntry {
    /// Print output from VM
    Print(String),
    /// Source code display (show_source)
    Source(String),
    /// Bytecode dump
    Bytecode(String),
    /// Generic info message
    Info(String),
    /// Error message
    Error(String),
}

/// Output buffer for capturing and routing output
pub trait OutputBuffer: Send + Sync {
    /// Push an output entry
    fn push(&self, entry: OutputEntry);
    
    /// Drain all entries (returns and clears)
    fn drain(&self) -> Vec<OutputEntry>;
    
    /// Check if empty
    fn is_empty(&self) -> bool;
}

/// In-memory output buffer implementation
pub struct MemoryOutputBuffer {
    entries: Mutex<Vec<OutputEntry>>,
}

impl MemoryOutputBuffer {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }
}

impl Default for MemoryOutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputBuffer for MemoryOutputBuffer {
    fn push(&self, entry: OutputEntry) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);
        }
    }
    
    fn drain(&self) -> Vec<OutputEntry> {
        if let Ok(mut entries) = self.entries.lock() {
            std::mem::take(&mut *entries)
        } else {
            Vec::new()
        }
    }
    
    fn is_empty(&self) -> bool {
        self.entries
            .lock()
            .map(|entries| entries.is_empty())
            .unwrap_or(true)
    }
}

/// Shared output buffer handle
pub type OutputHandle = Arc<dyn OutputBuffer>;

/// Create a new output buffer
pub fn new_output_buffer() -> OutputHandle {
    Arc::new(MemoryOutputBuffer::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_buffer() {
        let buffer = new_output_buffer();
        
        buffer.push(OutputEntry::Print("Hello".to_string()));
        buffer.push(OutputEntry::Info("World".to_string()));
        
        assert!(!buffer.is_empty());
        
        let entries = buffer.drain();
        assert_eq!(entries.len(), 2);
        assert!(buffer.is_empty());
    }
}
