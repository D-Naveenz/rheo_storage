//! Rust-native storage analysis primitives for the Rheo rewrite.
//!
//! This crate currently focuses on milestone one: immutable file metadata and
//! content-based file analysis backed by the legacy `filedefs.rpkg` package.

pub mod analysis;
mod definitions;
pub mod error;
pub mod info;

pub use analysis::{AnalysisReport, ContentKind, DetectedDefinition, analyze_path, analyze_reader};
pub use error::StorageError;
pub use info::FileInfo;
