use std::fs;
use std::path::{Path, PathBuf};

use rheo_storage_lib::{
    DefinitionPackage, bundled_definition_package, decode_definition_package,
    encode_definition_package,
};
use thiserror::Error;

/// Errors produced by the definitions builder crate.
#[derive(Debug, Error)]
pub enum BuilderError {
    /// A file-system operation failed.
    #[error("failed to {operation} '{path}': {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A package payload could not be decoded or encoded.
    #[error("package error: {message}")]
    Package { message: String },
}

/// Summary information about a definitions package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSummary {
    /// Package format version string.
    pub package_version: String,
    /// Reserved tag count carried forward from the legacy package.
    pub tags: u32,
    /// Number of definitions in the package.
    pub definition_count: usize,
}

impl PackageSummary {
    /// Build a summary from a decoded package.
    pub fn from_package(package: &DefinitionPackage) -> Self {
        Self {
            package_version: package.package_version.clone(),
            tags: package.tags,
            definition_count: package.definitions.len(),
        }
    }
}

/// Load a package from a path on disk.
///
/// # Returns
///
/// - `Result<DefinitionPackage, BuilderError>` - The decoded definitions package.
///
/// # Errors
///
/// Returns [`BuilderError::Io`] if the file cannot be read or
/// [`BuilderError::Package`] if the payload is not a valid package.
pub fn load_package(path: impl AsRef<Path>) -> Result<DefinitionPackage, BuilderError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| BuilderError::Io {
        operation: "read package",
        path: path.to_path_buf(),
        source,
    })?;
    decode_definition_package(&bytes).map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })
}

/// Load the runtime package embedded in `rheo_storage_lib`.
///
/// # Returns
///
/// - `Result<DefinitionPackage, BuilderError>` - The embedded definitions package.
///
/// # Errors
///
/// Returns [`BuilderError::Package`] if the bundled package cannot be decoded.
pub fn load_bundled_package() -> Result<DefinitionPackage, BuilderError> {
    bundled_definition_package()
        .cloned()
        .map_err(|err| BuilderError::Package {
            message: err.to_string(),
        })
}

/// Write a normalized package to disk in the legacy MessagePack format.
///
/// # Returns
///
/// - `Result<PathBuf, BuilderError>` - The final output path.
///
/// # Errors
///
/// Returns [`BuilderError::Io`] when the output path cannot be created or written,
/// or [`BuilderError::Package`] when serialization fails.
pub fn write_package(
    package: &DefinitionPackage,
    path: impl AsRef<Path>,
) -> Result<PathBuf, BuilderError> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BuilderError::Io {
            operation: "create output directory for",
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let bytes = encode_definition_package(package).map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })?;
    fs::write(&path, bytes).map_err(|source| BuilderError::Io {
        operation: "write package",
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Normalize an input package by decoding and re-encoding it.
///
/// # Returns
///
/// - `Result<PathBuf, BuilderError>` - The output package path.
///
/// # Errors
///
/// Returns an error if either the input package cannot be decoded or the output
/// cannot be written.
pub fn normalize_package(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<PathBuf, BuilderError> {
    let package = load_package(input)?;
    write_package(&package, output)
}

/// Check semantic equality between two package files.
///
/// # Returns
///
/// - `Result<bool, BuilderError>` - `true` when the decoded package contents match.
///
/// # Errors
///
/// Returns an error if either package cannot be read or decoded.
pub fn packages_match(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
) -> Result<bool, BuilderError> {
    let left = load_package(left)?;
    let right = load_package(right)?;
    Ok(left == right)
}

/// Produce a compact summary for a package path.
///
/// # Returns
///
/// - `Result<PackageSummary, BuilderError>` - Summary metadata for the package.
///
/// # Errors
///
/// Returns an error if the package cannot be loaded.
pub fn inspect_package(path: impl AsRef<Path>) -> Result<PackageSummary, BuilderError> {
    let package = load_package(path)?;
    Ok(PackageSummary::from_package(&package))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        PackageSummary, load_bundled_package, normalize_package, packages_match, write_package,
    };

    #[test]
    fn bundled_package_has_expected_summary() {
        let package = load_bundled_package().expect("bundled package should load");
        let summary = PackageSummary::from_package(&package);

        assert!(summary.definition_count > 0);
    }

    #[test]
    fn normalize_roundtrip_preserves_semantics() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("original.rpkg");
        let normalized = temp.path().join("normalized.rpkg");
        let package = load_bundled_package().expect("bundled package should load");
        write_package(&package, &original).expect("original package should be written");

        normalize_package(&original, &normalized).expect("normalized package should be written");

        assert!(packages_match(&original, &normalized).expect("packages should compare"));
    }
}
