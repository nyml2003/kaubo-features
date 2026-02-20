//! Loader trait and types
//!
//! Loaders are responsible for reading input data from external sources
//! such as files, stdin, or network.

use crate::component::{Component, ComponentKind, ComponentMetadata, Capabilities};
use crate::error::LoaderError;
use std::path::PathBuf;

/// The kind of source to load from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    /// Load from a file
    File,
    /// Load from standard input
    Stdin,
    /// Load from memory buffer
    Memory,
    /// Load from network
    Network,
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceKind::File => write!(f, "file"),
            SourceKind::Stdin => write!(f, "stdin"),
            SourceKind::Memory => write!(f, "memory"),
            SourceKind::Network => write!(f, "network"),
        }
    }
}

use std::fmt;

/// A source to load from
#[derive(Debug, Clone)]
pub struct Source {
    /// The kind of source
    pub kind: SourceKind,
    /// Optional path (for file sources)
    pub path: Option<PathBuf>,
    /// Source name/identifier
    pub name: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Source {
    /// Create a new file source
    pub fn file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            kind: SourceKind::File,
            name: path.to_string_lossy().to_string(),
            path: Some(path),
            metadata: Default::default(),
        }
    }
    
    /// Create a new stdin source
    pub fn stdin() -> Self {
        Self {
            kind: SourceKind::Stdin,
            name: "stdin".to_string(),
            path: None,
            metadata: Default::default(),
        }
    }
    
    /// Create a new memory source
    pub fn memory(name: impl Into<String>) -> Self {
        Self {
            kind: SourceKind::Memory,
            name: name.into(),
            path: None,
            metadata: Default::default(),
        }
    }
    
    /// Get the path if this is a file source
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
    
    /// Require a path (returns error if not a file source)
    pub fn require_path(&self) -> Result<&PathBuf, LoaderError> {
        self.path.as_ref().ok_or_else(|| {
            LoaderError::InvalidSourceKind {
                expected: "file".to_string(),
                actual: self.kind.to_string(),
            }
        })
    }
}

/// Raw data loaded from a source
#[derive(Debug, Clone)]
pub enum RawData {
    /// Text content (UTF-8)
    Text(String),
    /// Binary content
    Binary(Vec<u8>),
}

impl RawData {
    /// Try to get as text
    pub fn as_text(&self) -> Option<&str> {
        match self {
            RawData::Text(s) => Some(s),
            RawData::Binary(_) => None,
        }
    }
    
    /// Try to get as bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RawData::Text(s) => s.as_bytes(),
            RawData::Binary(b) => b,
        }
    }
    
    /// Convert to string (lossy for binary)
    pub fn to_string_lossy(&self) -> String {
        match self {
            RawData::Text(s) => s.clone(),
            RawData::Binary(b) => String::from_utf8_lossy(b).to_string(),
        }
    }
}

impl From<String> for RawData {
    fn from(s: String) -> Self {
        RawData::Text(s)
    }
}

impl From<Vec<u8>> for RawData {
    fn from(b: Vec<u8>) -> Self {
        RawData::Binary(b)
    }
}

/// The Loader trait for reading input data
///
/// Implementors of this trait can read from various sources (files, stdin, etc.)
pub trait Loader: Component {
    /// Load data from the given source
    ///
    /// # Arguments
    /// * `source` - The source to load from
    ///
    /// # Returns
    /// The raw data loaded from the source
    fn load(&self, source: &Source) -> Result<RawData, LoaderError>;
}

/// Helper methods for Loaders
pub trait LoaderExt: Loader {
    /// Load from a file path
    fn load_file(&self, path: impl Into<PathBuf>) -> Result<RawData, LoaderError> {
        self.load(&Source::file(path))
    }
    
    /// Load from stdin
    fn load_stdin(&self) -> Result<RawData, LoaderError> {
        self.load(&Source::stdin())
    }
}

impl<T: Loader + ?Sized> LoaderExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_creation() {
        let file_src = Source::file("test.kaubo");
        assert_eq!(file_src.kind, SourceKind::File);
        assert_eq!(file_src.name, "test.kaubo");
        assert!(file_src.path().is_some());
        
        let stdin_src = Source::stdin();
        assert_eq!(stdin_src.kind, SourceKind::Stdin);
        assert_eq!(stdin_src.name, "stdin");
    }

    #[test]
    fn test_raw_data() {
        let text = RawData::Text("hello".to_string());
        assert_eq!(text.as_text(), Some("hello"));
        assert_eq!(text.to_string_lossy(), "hello");
        
        let binary = RawData::Binary(vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]);
        assert_eq!(binary.as_text(), None);
        assert_eq!(binary.to_string_lossy(), "hello");
    }
}
