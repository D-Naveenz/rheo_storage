use std::collections::BTreeSet;

use once_cell::sync::Lazy;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::StorageError;
use crate::rpkg::{
    CompressionKind, PackageMetadata, PackagePurpose, RpkgDecodeError, RpkgEncodeError,
    RpkgEncodeOptions, SerializationKind, VerificationMode, decode as decode_rpkg,
    encode as encode_rpkg,
};

const BUNDLED_FILEDEFS_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/filedefs.rpkg"
));
pub const DEFINITION_PACKAGE_ID: [u8; 4] = *b"FDEF";
const CATCH_ALL_INDEX: usize = 256;

static PACKAGE: Lazy<Result<DefinitionPackage, String>> = Lazy::new(|| {
    DefinitionPackage::from_bytes(BUNDLED_FILEDEFS_BYTES).map_err(|err| err.to_string())
});
static DATABASE: Lazy<Result<DefinitionDatabase, String>> = Lazy::new(|| {
    bundled_definition_package()
        .map(DefinitionDatabase::from_package)
        .map_err(|err| err.to_string())
});

/// Errors that can occur while decoding an `rpkg` definitions package.
#[derive(Debug, Error)]
pub enum DefinitionPackageDecodeError {
    /// The container was not a valid RPKG v2 definitions package.
    #[error("failed to decode RPKG container: {0}")]
    Rpkg(#[from] RpkgDecodeError),

    /// The MessagePack payload was malformed.
    #[error("failed to decode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),

    /// The container held an unexpected package identifier.
    #[error("unexpected package identifier '{found}'")]
    InvalidPackageId { found: String },
}

/// Errors that can occur while encoding an `rpkg` definitions package.
#[derive(Debug, Error)]
pub enum DefinitionPackageEncodeError {
    /// The package could not be serialized to MessagePack.
    #[error("failed to encode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),

    /// The package could not be encoded into an `RPKG` container.
    #[error("failed to encode RPKG container: {0}")]
    Rpkg(#[from] RpkgEncodeError),
}

/// The serialized definitions package used by the runtime and builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPackage {
    /// The package format version label.
    pub package_version: String,
    /// Upstream source version inherited from TrID XML.
    pub source_version: String,
    /// Rheo-specific package revision for the current container scheme.
    pub package_revision: u16,
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
    /// Decode the raw MessagePack payload used inside an `RPKG` definitions container.
    pub fn from_messagepack_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        let RawPackage(package_version, _, tags, definitions): RawPackage =
            rmp_serde::from_slice(bytes)?;
        Ok(Self {
            package_version,
            source_version: String::new(),
            package_revision: 0,
            tags,
            definitions,
        })
    }

    /// Decode a definitions package from an `RPKG` v2 container.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DefinitionPackageDecodeError> {
        Self::from_bytes_with_verification(bytes, VerificationMode::Default)
    }

    /// Decode a definitions package from an `RPKG` v2 container with explicit verification behavior.
    pub fn from_bytes_with_verification(
        bytes: &[u8],
        verification_mode: VerificationMode,
    ) -> Result<Self, DefinitionPackageDecodeError> {
        let decoded = decode_rpkg(bytes, verification_mode)?;
        if decoded.package_id != DEFINITION_PACKAGE_ID {
            return Err(DefinitionPackageDecodeError::InvalidPackageId {
                found: decoded.package_id_string(),
            });
        }

        let mut package = Self::from_messagepack_bytes(&decoded.payload)?;
        package.package_version = decoded.metadata.package_version;
        package.source_version = decoded.metadata.source_version;
        package.package_revision = decoded.metadata.package_revision;
        Ok(package)
    }

    /// Encode this package into the raw MessagePack payload stored inside an `RPKG` container.
    pub fn to_messagepack_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&RawPackageOut(
            self.package_version.clone(),
            (),
            self.tags,
            self.definitions.clone(),
        ))
    }

    /// Encode this package into the default external `RPKG` v2 container.
    pub fn to_bytes(&self) -> Result<Vec<u8>, DefinitionPackageEncodeError> {
        self.to_bytes_with_purpose(PackagePurpose::External)
    }

    /// Encode this package into an `RPKG` v2 container with the requested package purpose.
    pub fn to_bytes_with_purpose(
        &self,
        purpose: PackagePurpose,
    ) -> Result<Vec<u8>, DefinitionPackageEncodeError> {
        let payload = self.to_messagepack_bytes()?;
        let metadata = PackageMetadata::new(
            self.package_version.clone(),
            self.source_version.clone(),
            self.package_revision,
        );
        let options = RpkgEncodeOptions {
            package_id: DEFINITION_PACKAGE_ID,
            serialization: SerializationKind::MessagePack,
            compression: CompressionKind::Lz4Frame,
            purpose,
            metadata,
        };
        Ok(encode_rpkg(&payload, &options)?)
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

/// Load the embedded definitions package bundled with `rheo_storage`.
pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, StorageError> {
    PACKAGE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

/// Decode a definitions package from bytes using purpose-aware checksum verification.
pub fn decode_definition_package(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    decode_definition_package_with_verification(bytes, VerificationMode::Default)
}

/// Decode a definitions package from bytes using the requested checksum verification mode.
pub fn decode_definition_package_with_verification(
    bytes: &[u8],
    verification_mode: VerificationMode,
) -> Result<DefinitionPackage, StorageError> {
    DefinitionPackage::from_bytes_with_verification(bytes, verification_mode).map_err(|err| {
        StorageError::DefinitionsLoad {
            message: err.to_string(),
        }
    })
}

/// Encode a definitions package into the default external `RPKG` v2 format.
pub fn encode_definition_package(package: &DefinitionPackage) -> Result<Vec<u8>, StorageError> {
    encode_definition_package_with_purpose(package, PackagePurpose::External)
}

/// Encode a definitions package into an `RPKG` v2 container with the requested package purpose.
pub fn encode_definition_package_with_purpose(
    package: &DefinitionPackage,
    purpose: PackagePurpose,
) -> Result<Vec<u8>, StorageError> {
    package
        .to_bytes_with_purpose(purpose)
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
    use crate::rpkg::{PackagePurpose, VerificationMode, RPKG_MAGIC};

    use super::{
        DefinitionPackage, bundled_definition_package, database, decode_definition_package,
        decode_definition_package_with_verification, encode_definition_package,
        encode_definition_package_with_purpose,
    };

    #[test]
    fn bundled_rpkg_loads_successfully() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        assert!(!package.definitions.is_empty());
    }

    #[test]
    fn png_header_returns_png_candidate() {
        let db = database().expect("bundled definitions package should deserialize");
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
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_definition_package(package).expect("package should encode");
        let decoded = decode_definition_package(&bytes).expect("encoded package should decode");

        assert_eq!(&decoded, package);
    }

    #[test]
    fn encoded_package_uses_rpkg_header() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = package.to_bytes().expect("package should encode");

        assert!(bytes.starts_with(RPKG_MAGIC));
    }

    #[test]
    fn legacy_plain_messagepack_is_rejected() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let plain = package
            .to_messagepack_bytes()
            .expect("plain MessagePack should encode");

        let error = DefinitionPackage::from_bytes(&plain).expect_err("legacy package should fail");
        assert!(error.to_string().contains("RPKG"));
    }

    #[test]
    fn old_lz4_magic_is_rejected() {
        let mut bytes = b"RPKGLZ4\x01".to_vec();
        bytes.extend_from_slice(b"not-a-v2-package");

        let error = DefinitionPackage::from_bytes(&bytes).expect_err("old format should fail");
        assert!(error.to_string().contains("wire version") || error.to_string().contains("RPKG"));
    }

    #[test]
    fn embedded_packages_skip_checksum_by_default_but_can_be_forced() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let mut bytes = encode_definition_package_with_purpose(package, PackagePurpose::Embedded)
            .expect("package should encode");
        let checksum_byte = bytes.len() - 1;
        bytes[checksum_byte] ^= 0x01;

        let decoded =
            decode_definition_package(&bytes).expect("embedded package should skip verification");
        assert_eq!(decoded.package_version, package.package_version);

        let error = decode_definition_package_with_verification(&bytes, VerificationMode::Always)
            .expect_err("forced verification should fail");
        assert!(error.to_string().contains("checksum"));
    }
}
