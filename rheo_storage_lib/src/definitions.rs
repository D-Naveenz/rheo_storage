use std::collections::BTreeSet;

use once_cell::sync::Lazy;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};

use crate::error::StorageError;

const LEGACY_FILEDEFS_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/filedefs.rpkg"
));
const CATCH_ALL_INDEX: usize = 256;

static PACKAGE: Lazy<Result<DefinitionPackage, String>> = Lazy::new(|| {
    DefinitionPackage::from_bytes(LEGACY_FILEDEFS_BYTES).map_err(|err| err.to_string())
});
static DATABASE: Lazy<Result<DefinitionDatabase, String>> = Lazy::new(|| {
    bundled_definition_package()
        .map(DefinitionDatabase::from_package)
        .map_err(|err| err.to_string())
});

/// The serialized definitions package used by the runtime and builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPackage {
    /// The package format version label.
    pub package_version: String,
    /// Reserved tags field carried forward from the legacy package.
    pub tags: u32,
    /// File signature definitions contained in the package.
    pub definitions: Vec<DefinitionRecord>,
}

/// A single file signature definition stored in a package.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionRecord {
    /// Human-readable file type label.
    pub file_type: String,
    /// Candidate filename extensions.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// MIME type associated with the definition.
    #[serde(default)]
    pub mime_type: String,
    /// Additional remarks preserved from legacy data.
    #[serde(default)]
    pub remarks: String,
    /// Signature data used during matching.
    #[serde(default)]
    pub signature: SignatureDefinition,
    /// Priority hint used to break ties between similar signatures.
    #[serde(default)]
    pub priority_level: i32,
}

/// Signature definition attached to a [`DefinitionRecord`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDefinition {
    /// Fixed-position byte patterns.
    #[serde(default)]
    pub patterns: Vec<SignaturePattern>,
    /// Arbitrary byte strings that must appear somewhere in the content.
    #[serde(default)]
    pub strings: Vec<Vec<u8>>,
}

/// A single positional byte pattern inside a [`SignatureDefinition`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePattern {
    /// Zero-based byte offset at which the pattern must appear.
    pub position: u16,
    /// Raw bytes to match.
    #[serde(default)]
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RawPackage(String, IgnoredAny, u32, Vec<DefinitionRecord>);

#[derive(Debug, Serialize)]
struct RawPackageOut(String, (), u32, Vec<DefinitionRecord>);

/// An indexed definitions database optimized for runtime analysis.
#[derive(Debug, Clone)]
pub struct DefinitionDatabase {
    definitions: Vec<DefinitionRecord>,
    pattern_index: Vec<Vec<usize>>,
}

impl DefinitionPackage {
    /// Decode a MessagePack definitions package.
    ///
    /// # Returns
    ///
    /// - `Result<DefinitionPackage, rmp_serde::decode::Error>` - The decoded package.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload is not a valid Rheo definitions package.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        let RawPackage(package_version, _, tags, definitions): RawPackage =
            rmp_serde::from_slice(bytes)?;
        Ok(Self {
            package_version,
            tags,
            definitions,
        })
    }

    /// Encode this package back into the legacy MessagePack wire format.
    ///
    /// # Returns
    ///
    /// - `Result<Vec<u8>, rmp_serde::encode::Error>` - The encoded package bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if MessagePack serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&RawPackageOut(
            self.package_version.clone(),
            (),
            self.tags,
            self.definitions.clone(),
        ))
    }
}

impl DefinitionDatabase {
    fn from_package(package: &DefinitionPackage) -> Self {
        let definitions = package.definitions.clone();
        let mut pattern_index = vec![Vec::new(); CATCH_ALL_INDEX + 1];
        for (idx, definition) in definitions.iter().enumerate() {
            if definition.signature.patterns.is_empty() {
                pattern_index[CATCH_ALL_INDEX].push(idx);
                continue;
            }

            for pattern in &definition.signature.patterns {
                if let Some(first_byte) = pattern.data.first() {
                    pattern_index[*first_byte as usize].push(idx);
                } else {
                    pattern_index[CATCH_ALL_INDEX].push(idx);
                }
            }
        }

        Self {
            definitions,
            pattern_index,
        }
    }

    pub(crate) fn candidate_indices(&self, header: &[u8]) -> Vec<usize> {
        let mut candidates = BTreeSet::new();

        for idx in &self.pattern_index[CATCH_ALL_INDEX] {
            candidates.insert(*idx);
        }

        for (position, byte) in header.iter().enumerate() {
            for definition_idx in &self.pattern_index[*byte as usize] {
                let definition = &self.definitions[*definition_idx];
                if definition
                    .signature
                    .patterns
                    .iter()
                    .any(|pattern| pattern.position as usize == position)
                {
                    candidates.insert(*definition_idx);
                }
            }
        }

        candidates.into_iter().collect()
    }

    pub(crate) fn definition(&self, idx: usize) -> &DefinitionRecord {
        &self.definitions[idx]
    }
}

/// Load the embedded definitions package bundled with `rheo_storage_lib`.
///
/// # Returns
///
/// - `Result<&'static DefinitionPackage, StorageError>` - The decoded embedded package.
///
/// # Errors
///
/// Returns [`StorageError::DefinitionsLoad`] when the embedded MessagePack payload
/// cannot be decoded.
pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, StorageError> {
    PACKAGE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

/// Decode a definitions package from bytes.
///
/// # Returns
///
/// - `Result<DefinitionPackage, StorageError>` - The decoded package.
///
/// # Errors
///
/// Returns [`StorageError::DefinitionsLoad`] when the payload is malformed.
pub fn decode_definition_package(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    DefinitionPackage::from_bytes(bytes).map_err(|err| StorageError::DefinitionsLoad {
        message: err.to_string(),
    })
}

/// Encode a definitions package into the legacy MessagePack format.
///
/// # Returns
///
/// - `Result<Vec<u8>, StorageError>` - The encoded package bytes.
///
/// # Errors
///
/// Returns [`StorageError::DefinitionsLoad`] when serialization fails.
pub fn encode_definition_package(package: &DefinitionPackage) -> Result<Vec<u8>, StorageError> {
    package
        .to_bytes()
        .map_err(|err| StorageError::DefinitionsLoad {
            message: err.to_string(),
        })
}

pub(crate) fn database() -> Result<&'static DefinitionDatabase, StorageError> {
    DATABASE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::{
        bundled_definition_package, database, decode_definition_package, encode_definition_package,
    };

    #[test]
    fn legacy_rpkg_loads_successfully() {
        let package =
            bundled_definition_package().expect("legacy definitions package should deserialize");
        assert!(!package.definitions.is_empty());
    }

    #[test]
    fn png_header_returns_png_candidate() {
        let db = database().expect("legacy definitions package should deserialize");
        let header = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

        let candidates = db
            .candidate_indices(&header)
            .into_iter()
            .map(|idx| db.definition(idx))
            .collect::<Vec<_>>();

        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|definition| {
            definition
                .extensions
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case(".png"))
        }));
    }

    #[test]
    fn package_roundtrip_is_semantically_stable() {
        let package =
            bundled_definition_package().expect("legacy definitions package should deserialize");
        let bytes = encode_definition_package(package).expect("package should encode");
        let decoded = decode_definition_package(&bytes).expect("encoded package should decode");

        assert_eq!(&decoded, package);
    }
}
