use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by Rheo storage analysis and metadata operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The supplied path did not exist at the time of access.
    #[error("path does not exist: {path}")]
    NotFound { path: PathBuf },

    /// The supplied path existed but did not refer to a file.
    #[error("path is not a file: {path}")]
    NotAFile { path: PathBuf },

    /// The supplied path existed but did not refer to a directory.
    #[error("path is not a directory: {path}")]
    NotADirectory { path: PathBuf },

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
}
