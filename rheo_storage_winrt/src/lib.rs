//! WinRT-facing wrapper crate for Rheo.Storage.
//!
//! This crate intentionally stays thin over `rheo_storage_lib`. The current
//! implementation provides documented Rust-side wrapper types that can be used
//! as the basis for a packaged Windows Runtime component surface.

use rheo_storage_lib::{DirectoryStorage, FileStorage, StorageError};

/// WinRT-oriented wrapper for file operations.
#[derive(Debug, Clone)]
pub struct FileObject {
    inner: FileStorage,
}

impl FileObject {
    /// Open an existing file path.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        Ok(Self {
            inner: FileStorage::from_existing(path)?,
        })
    }

    /// Get the absolute file path.
    pub fn full_path(&self) -> &std::path::Path {
        self.inner.path()
    }
}

/// WinRT-oriented wrapper for directory operations.
#[derive(Debug, Clone)]
pub struct DirectoryObject {
    inner: DirectoryStorage,
}

impl DirectoryObject {
    /// Open an existing directory path.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        Ok(Self {
            inner: DirectoryStorage::from_existing(path)?,
        })
    }

    /// Get the absolute directory path.
    pub fn full_path(&self) -> &std::path::Path {
        self.inner.path()
    }
}
