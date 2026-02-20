//! Emitter trait and types
//!
//! Emitters write output to targets such as files, stdout, or network.

use crate::component::{Component, ComponentKind, ComponentMetadata, Capabilities};
use crate::pass::Output;
use crate::error::EmitterError;
use std::path::PathBuf;

/// The kind of target to emit to
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetKind {
    /// Write to a file
    File,
    /// Write to standard output
    Stdout,
    /// Write to standard error
    Stderr,
    /// Write to memory buffer
    Memory,
    /// Write over network
    Network,
    /// Call a callback function
    Callback,
}

impl fmt::Display for TargetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetKind::File => write!(f, "file"),
            TargetKind::Stdout => write!(f, "stdout"),
            TargetKind::Stderr => write!(f, "stderr"),
            TargetKind::Memory => write!(f, "memory"),
            TargetKind::Network => write!(f, "network"),
            TargetKind::Callback => write!(f, "callback"),
        }
    }
}

use std::fmt;

/// A target to emit to
#[derive(Debug, Clone)]
pub struct Target {
    /// The kind of target
    pub kind: TargetKind,
    /// Optional path (for file targets)
    pub path: Option<PathBuf>,
    /// Additional options
    pub options: std::collections::HashMap<String, String>,
}

impl Target {
    /// Create a new file target
    pub fn file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            kind: TargetKind::File,
            path: Some(path),
            options: Default::default(),
        }
    }
    
    /// Create a stdout target
    pub fn stdout() -> Self {
        Self {
            kind: TargetKind::Stdout,
            path: None,
            options: Default::default(),
        }
    }
    
    /// Create a stderr target
    pub fn stderr() -> Self {
        Self {
            kind: TargetKind::Stderr,
            path: None,
            options: Default::default(),
        }
    }
    
    /// Create a memory target
    pub fn memory() -> Self {
        Self {
            kind: TargetKind::Memory,
            path: None,
            options: Default::default(),
        }
    }
    
    /// Get the path if this is a file target
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
    
    /// Require a path (returns error if not a file target)
    pub fn require_path(&self) -> Result<&PathBuf, EmitterError> {
        self.path.as_ref().ok_or_else(|| {
            EmitterError::TargetNotFound("file path not specified".to_string())
        })
    }
    
    /// Add an option
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}

/// Serialized output data
#[derive(Debug, Clone)]
pub struct SerializedOutput {
    /// The format of the serialized data
    pub format: String,
    /// The serialized bytes
    pub data: Vec<u8>,
    /// Content type (MIME type)
    pub content_type: Option<String>,
}

impl SerializedOutput {
    /// Create new serialized output
    pub fn new(format: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            format: format.into(),
            data,
            content_type: None,
        }
    }
    
    /// Set content type
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
    
    /// Convert to string (UTF-8)
    pub fn to_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.data.clone())
    }
    
    /// Convert to string lossy
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }
}

/// The Emitter trait for writing output
///
/// Implementors serialize IR to a format and write to a target.
pub trait Emitter: Component {
    /// Get the format this emitter produces
    fn format(&self) -> &str;
    
    /// Serialize the output
    ///
    /// # Arguments
    /// * `output` - The output to serialize
    ///
    /// # Returns
    /// The serialized output data
    fn serialize(&self, output: &Output) -> Result<SerializedOutput, EmitterError>;
    
    /// Write serialized data to a target
    ///
    /// # Arguments
    /// * `data` - The serialized data
    /// * `target` - The target to write to
    fn write(&self, data: &SerializedOutput, target: &Target) -> Result<(), EmitterError>;
    
    /// Convenience method: serialize and emit in one call
    fn emit(&self, output: &Output, target: &Target) -> Result<(), EmitterError> {
        let serialized = self.serialize(output)?;
        self.write(&serialized, target)
    }
}

/// Helper methods for Emitters
pub trait EmitterExt: Emitter {
    /// Emit to a file
    fn emit_file(&self, output: &Output, path: impl Into<PathBuf>) -> Result<(), EmitterError> {
        self.emit(output, &Target::file(path))
    }
    
    /// Emit to stdout
    fn emit_stdout(&self, output: &Output) -> Result<(), EmitterError> {
        self.emit(output, &Target::stdout())
    }
    
    /// Serialize to string
    fn serialize_to_string(&self, output: &Output) -> Result<String, EmitterError> {
        let serialized = self.serialize(output)?;
        serialized.to_string().map_err(|e| {
            EmitterError::InvalidOutputFormat(format!("UTF-8 conversion failed: {}", e))
        })
    }
}

impl<T: Emitter + ?Sized> EmitterExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_creation() {
        let file_target = Target::file("output.txt");
        assert_eq!(file_target.kind, TargetKind::File);
        assert!(file_target.path().is_some());
        
        let stdout_target = Target::stdout();
        assert_eq!(stdout_target.kind, TargetKind::Stdout);
        assert!(stdout_target.path().is_none());
    }

    #[test]
    fn test_serialized_output() {
        let serialized = SerializedOutput::new("json", vec![123, 125]);
        assert_eq!(serialized.format, "json");
        assert_eq!(serialized.data, vec![123, 125]);
        
        let with_ct = serialized.with_content_type("application/json");
        assert_eq!(with_ct.content_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_serialized_to_string() {
        let serialized = SerializedOutput::new("text", b"hello".to_vec());
        assert_eq!(serialized.to_string().unwrap(), "hello");
    }
}
