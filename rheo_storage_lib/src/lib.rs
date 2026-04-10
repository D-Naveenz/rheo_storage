//! Rust-native storage analysis primitives for the Rheo rewrite.
//!
//! This crate currently focuses on the first two rewrite milestones:
//! immutable metadata and content-based analysis backed by the legacy
//! `filedefs.rpkg` package, plus a Rust-native file and directory operations
//! layer with optional async wrappers.

pub mod analysis;
pub mod definitions;
pub mod error;
pub mod info;
pub mod operations;
pub mod storage;
pub mod watch;

pub use analysis::{AnalysisReport, ContentKind, DetectedDefinition, analyze_path, analyze_reader};
pub use definitions::{
    DefinitionPackage, DefinitionRecord, SignatureDefinition, SignaturePattern,
    bundled_definition_package, decode_definition_package, encode_definition_package,
};
pub use error::StorageError;
pub use info::{DirectoryInfo, DirectorySummary, FileInfo, SizeUnit, StorageMetadata, format_size};
pub use operations::{
    DirectoryDeleteOptions, ProgressReporter, ReadOptions, SharedProgressReporter, StorageProgress,
    TransferOptions, WriteOptions, copy_directory, copy_directory_with_options, copy_file,
    copy_file_with_options, create_directory, create_directory_all, delete_directory,
    delete_directory_with_options, delete_file, move_directory, move_directory_with_options,
    move_file, move_file_with_options, read_file, read_file_to_string, rename_directory,
    rename_file, write_file, write_file_from_reader, write_file_string,
};
#[cfg(feature = "async-tokio")]
pub use operations::{
    copy_directory_async, copy_file_async, create_directory_all_async, create_directory_async,
    delete_directory_async, delete_file_async, move_directory_async, move_file_async,
    read_file_async, read_file_to_string_async, rename_directory_async, rename_file_async,
    write_file_async, write_file_from_reader_async, write_file_string_async,
};
pub use storage::{DirectoryStorage, FileStorage, SearchScope, StorageEntry};
pub use watch::{DirectoryWatchHandle, StorageChangeEvent, StorageChangeType, StorageWatchConfig};
