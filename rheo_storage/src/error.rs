use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by Rheo storage analysis and metadata operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The supplied path did not exist at the time of access.
    #[error("path does not exist: {path}")]
    NotFound { path: PathBuf },

    /// The destination path already existed and overwrite was disabled.
    #[error("path already exists: {path}")]
    AlreadyExists { path: PathBuf },

    /// The supplied path existed but did not refer to a file.
    #[error("path is not a file: {path}")]
    NotAFile { path: PathBuf },

    /// The supplied path existed but did not refer to a directory.
    #[error("path is not a directory: {path}")]
    NotADirectory { path: PathBuf },

    /// A file-system name argument was invalid for the requested operation.
    #[error("invalid {kind} name: {value}")]
    InvalidName { kind: &'static str, value: String },

    /// The requested operation could not be completed because the path shape conflicted with the API contract.
    #[error("path conflict at '{path}': {message}")]
    PathConflict {
        path: PathBuf,
        message: &'static str,
    },

    /// A path-based I/O operation failed.
    #[error("failed to {operation} '{path}': {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A reader-based I/O operation failed.
    #[error("failed to {operation} reader: {source}")]
    ReaderIo {
        operation: &'static str,
        #[source]
        source: io::Error,
    },

    /// The embedded legacy definitions package could not be decoded.
    #[error("failed to load embedded legacy file definitions: {message}")]
    DefinitionsLoad { message: String },

    /// An async runtime task failed before the storage operation completed.
    #[error("async runtime failed to {operation}: {message}")]
    AsyncRuntime {
        operation: &'static str,
        message: String,
    },

    /// A storage watcher failed to initialize or deliver events.
    #[error("watch error while attempting to {operation}: {message}")]
    Watch {
        operation: &'static str,
        message: String,
    },
}

impl StorageError {
    pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }

    pub(crate) fn reader_io(operation: &'static str, source: io::Error) -> Self {
        Self::ReaderIo { operation, source }
    }

    pub(crate) fn already_exists(path: impl Into<PathBuf>) -> Self {
        Self::AlreadyExists { path: path.into() }
    }

    pub(crate) fn invalid_name(kind: &'static str, value: impl Into<String>) -> Self {
        Self::InvalidName {
            kind,
            value: value.into(),
        }
    }

    pub(crate) fn path_conflict(path: impl Into<PathBuf>, message: &'static str) -> Self {
        Self::PathConflict {
            path: path.into(),
            message,
        }
    }

    #[cfg(feature = "async-tokio")]
    pub(crate) fn async_runtime(operation: &'static str, message: impl Into<String>) -> Self {
        Self::AsyncRuntime {
            operation,
            message: message.into(),
        }
    }

    pub(crate) fn watch(operation: &'static str, message: impl Into<String>) -> Self {
        Self::Watch {
            operation,
            message: message.into(),
        }
    }
}
