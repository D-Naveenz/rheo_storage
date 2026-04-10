use std::path::{Path, PathBuf};

use crate::error::StorageError;
use crate::info::DirectoryInfo;
use crate::operations::common::normalize_path;
use crate::operations::{
    DirectoryDeleteOptions, TransferOptions, copy_directory, copy_directory_with_options,
    create_directory, create_directory_all, delete_directory, delete_directory_with_options,
    move_directory, move_directory_with_options, rename_directory,
};

/// Rust-native handle for directory operations and metadata lookups.
///
/// The handle itself is lightweight and path-based. Expensive recursive summary
/// work remains opt-in through [`DirectoryInfo`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryStorage {
    path: PathBuf,
}

impl DirectoryStorage {
    /// Create a path-based directory handle without touching the file system.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self {
            path: normalize_path(path)?,
        })
    }

    /// Create a directory handle for an existing directory.
    pub fn from_existing(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = normalize_path(path)?;
        if !path.exists() {
            return Err(StorageError::NotFound { path });
        }
        if !path.is_dir() {
            return Err(StorageError::NotADirectory { path });
        }

        Ok(Self { path })
    }

    /// Absolute path represented by this handle.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Directory name.
    pub fn name(&self) -> Option<&str> {
        self.path.file_name().and_then(|value| value.to_str())
    }

    /// Create this directory if it does not already exist.
    pub fn create(&self) -> Result<Self, StorageError> {
        let path = create_directory(&self.path)?;
        Ok(Self { path })
    }

    /// Create this directory and any missing parents.
    pub fn create_all(&self) -> Result<Self, StorageError> {
        let path = create_directory_all(&self.path)?;
        Ok(Self { path })
    }

    /// Load cheap directory metadata without walking the tree.
    pub fn info(&self) -> Result<DirectoryInfo, StorageError> {
        DirectoryInfo::from_path(&self.path)
    }

    /// Load directory metadata and precompute the recursive summary in parallel.
    pub fn info_with_summary(&self) -> Result<DirectoryInfo, StorageError> {
        DirectoryInfo::from_path_with_summary(&self.path)
    }

    /// Copy the directory tree to an exact destination path.
    pub fn copy_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = copy_directory(&self.path, destination)?;
        Ok(Self { path })
    }

    /// Copy the directory tree with overwrite and progress control.
    pub fn copy_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path = copy_directory_with_options(&self.path, destination, options)?;
        Ok(Self { path })
    }

    /// Move the directory tree to an exact destination path.
    ///
    /// The original handle is not mutated; a new handle for the destination path is returned.
    pub fn move_to(&self, destination: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = move_directory(&self.path, destination)?;
        Ok(Self { path })
    }

    /// Move the directory tree with overwrite and progress control.
    pub fn move_to_with_options(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path = move_directory_with_options(&self.path, destination, options)?;
        Ok(Self { path })
    }

    /// Rename the directory inside its current parent directory.
    ///
    /// The original handle is not mutated; a new handle for the renamed path is returned.
    pub fn rename(&self, new_name: &str) -> Result<Self, StorageError> {
        let path = rename_directory(&self.path, new_name)?;
        Ok(Self { path })
    }

    /// Delete the directory recursively.
    pub fn delete(&self) -> Result<(), StorageError> {
        delete_directory(&self.path)
    }

    /// Delete the directory with explicit recursive control.
    pub fn delete_with_options(&self, options: DirectoryDeleteOptions) -> Result<(), StorageError> {
        delete_directory_with_options(&self.path, options)
    }
}

#[cfg(feature = "async-tokio")]
impl DirectoryStorage {
    /// Async variant of [`Self::create`].
    pub async fn create_async(&self) -> Result<Self, StorageError> {
        let path = crate::operations::create_directory_async(&self.path).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::create_all`].
    pub async fn create_all_async(&self) -> Result<Self, StorageError> {
        let path = crate::operations::create_directory_all_async(&self.path).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::copy_to_with_options`].
    pub async fn copy_to_async(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path =
            crate::operations::copy_directory_async(&self.path, destination, options).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::move_to_with_options`].
    pub async fn move_to_async(
        &self,
        destination: impl AsRef<Path>,
        options: TransferOptions,
    ) -> Result<Self, StorageError> {
        let path =
            crate::operations::move_directory_async(&self.path, destination, options).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::rename`].
    pub async fn rename_async(&self, new_name: impl Into<String>) -> Result<Self, StorageError> {
        let path = crate::operations::rename_directory_async(&self.path, new_name).await?;
        Ok(Self { path })
    }

    /// Async variant of [`Self::delete_with_options`].
    pub async fn delete_async(&self, options: DirectoryDeleteOptions) -> Result<(), StorageError> {
        crate::operations::delete_directory_async(&self.path, options).await
    }
}
