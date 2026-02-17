//! VFS Error Types

use std::fmt;

/// Result type for VFS operations
pub type VfsResult<T> = Result<T, VfsError>;

/// Error type for VFS operations
#[derive(Debug, Clone, PartialEq)]
pub enum VfsError {
    /// File or directory not found
    NotFound { path: String },

    /// Permission denied
    PermissionDenied { path: String },

    /// Path already exists
    AlreadyExists { path: String },

    /// Invalid path
    InvalidPath { path: String, reason: String },

    /// IO error
    Io { message: String },

    /// Custom error message
    Custom { message: String },
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::NotFound { path } => write!(f, "Path not found: {}", path),
            VfsError::PermissionDenied { path } => write!(f, "Permission denied: {}", path),
            VfsError::AlreadyExists { path } => write!(f, "Path already exists: {}", path),
            VfsError::InvalidPath { path, reason } => {
                write!(f, "Invalid path '{}': {}", path, reason)
            }
            VfsError::Io { message } => write!(f, "IO error: {}", message),
            VfsError::Custom { message } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for VfsError {}

impl From<std::io::Error> for VfsError {
    fn from(err: std::io::Error) -> Self {
        VfsError::Io {
            message: err.to_string(),
        }
    }
}
