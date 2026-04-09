use std::path::{Path, PathBuf};

use crate::analysis::{AnalysisReport, ContentKind, DetectedDefinition, analyze_path};
use crate::error::StorageError;

/// Immutable file metadata enriched with content-based analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    /// Full path to the file on disk.
    pub path: PathBuf,
    /// File size in bytes.
    pub size: u64,
    /// Extension derived from the filename, normalized without a leading dot.
    pub filename_extension: Option<String>,
    /// Top detected extension from content analysis, normalized without a leading dot.
    pub detected_extension: Option<String>,
    /// Top detected MIME type from content analysis.
    pub mime_type: Option<String>,
    /// Heuristic content kind derived from the scanned bytes.
    pub content_kind: ContentKind,
    /// Ranked content matches produced by the analyzer.
    pub matches: Vec<DetectedDefinition>,
}

impl FileInfo {
    /// Build immutable file metadata for the provided path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound {
                    path: path.to_path_buf(),
                }
            } else {
                StorageError::io("read metadata for", path.to_path_buf(), err)
            }
        })?;

        if !metadata.is_file() {
            return Err(StorageError::NotAFile {
                path: path.to_path_buf(),
            });
        }

        let analysis = analyze_path(path)?;
        Ok(Self::from_analysis(path, metadata.len(), analysis))
    }

    fn from_analysis(path: &Path, size: u64, analysis: AnalysisReport) -> Self {
        Self {
            path: path.to_path_buf(),
            size,
            filename_extension: path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
                .filter(|ext| !ext.is_empty()),
            detected_extension: analysis.top_detected_extension,
            mime_type: analysis.top_mime_type,
            content_kind: analysis.content_kind,
            matches: analysis.matches,
        }
    }
}
