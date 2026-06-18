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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_variants_are_readable() {
        assert_eq!(
            VfsError::NotFound {
                path: "/missing".into()
            }
            .to_string(),
            "Path not found: /missing"
        );
        assert_eq!(
            VfsError::PermissionDenied {
                path: "/deny".into()
            }
            .to_string(),
            "Permission denied: /deny"
        );
        assert_eq!(
            VfsError::AlreadyExists {
                path: "/dup".into()
            }
            .to_string(),
            "Path already exists: /dup"
        );
        assert_eq!(
            VfsError::InvalidPath {
                path: "x".into(),
                reason: "bad".into()
            }
            .to_string(),
            "Invalid path 'x': bad"
        );
        assert_eq!(
            VfsError::Custom {
                message: "oops".into()
            }
            .to_string(),
            "oops"
        );
    }

    #[test]
    fn io_error_maps_to_io_variant() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
        let vfs_err: VfsError = err.into();

        assert!(matches!(vfs_err, VfsError::Io { message } if message.contains("nope")));
    }
}
