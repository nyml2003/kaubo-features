//! Kaubo Virtual File System
//!
//! A virtual file system abstraction with multiple backend implementations.
//!
//! # Features
//! - `std` (default): Enable standard library support
//!
//! # Usage
//! ```rust,ignore
//! use kaubo_vfs::{VirtualFileSystem, MemoryFileSystem};
//! use std::path::Path;
//!
//! let fs = MemoryFileSystem::new();
//! fs.write_file(Path::new("/test.txt"), b"hello").unwrap();
//! let content = fs.read_file(Path::new("/test.txt")).unwrap();
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

mod error;
mod memory;
mod native;
mod r#trait;

pub use error::{VfsError, VfsResult};
pub use memory::MemoryFileSystem;
pub use native::NativeFileSystem;
pub use r#trait::VirtualFileSystem;

/// Create a new memory-based file system.
pub fn memory_fs() -> MemoryFileSystem {
    MemoryFileSystem::new()
}

/// Create a new native file system.
pub fn native_fs() -> NativeFileSystem {
    NativeFileSystem::new()
}
