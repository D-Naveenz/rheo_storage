use std::collections::BTreeSet;

use once_cell::sync::Lazy;
use rheo_rpkg::{RpkgDecodeError, RpkgReadOptions, RpkgReader};
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::StorageError;

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

#[derive(Debug, Error)]
pub enum DefinitionPackageDecodeError {
    #[error("failed to decode RPKG container: {0}")]
    Rpkg(#[from] RpkgDecodeError),

    #[error("failed to decode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),

    #[error("unexpected package identifier '{found}'")]
    InvalidPackageId { found: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPackage {
    pub package_version: String,
    pub source_version: String,
    pub package_revision: u16,
    pub tags: u32,
    pub definitions: Vec<DefinitionRecord>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionRecord {
    pub file_type: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub remarks: String,
    #[serde(default)]
    pub signature: SignatureDefinition,
    #[serde(default)]
    pub priority_level: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDefinition {
    #[serde(default)]
    pub patterns: Vec<SignaturePattern>,
    #[serde(default)]
    pub strings: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePattern {
    pub position: u16,
    #[serde(default)]
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RawPackage(String, String, u16, IgnoredAny, u32, Vec<DefinitionRecord>);

#[derive(Debug, Clone)]
pub struct DefinitionDatabase {
    definitions: Vec<DefinitionRecord>,
    pattern_index: Vec<Vec<usize>>,
}

impl DefinitionPackage {
    pub fn from_messagepack_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        let RawPackage(package_version, source_version, package_revision, _, tags, definitions): RawPackage =
            rmp_serde::from_slice(bytes)?;
        Ok(Self {
            package_version,
            source_version,
            package_revision,
            tags,
            definitions,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DefinitionPackageDecodeError> {
        let header = RpkgReader::read_header(bytes)?;
        if header.package_id != DEFINITION_PACKAGE_ID {
            return Err(DefinitionPackageDecodeError::InvalidPackageId {
                found: String::from_utf8_lossy(&header.package_id).into_owned(),
            });
        }
        let payload = RpkgReader::read_payload_bytes(bytes)?;
        Self::from_messagepack_bytes(&payload).map_err(Into::into)
    }

    pub fn from_bytes_with_options(
        bytes: &[u8],
        options: &RpkgReadOptions,
    ) -> Result<Self, DefinitionPackageDecodeError> {
        let package = RpkgReader::read_package(bytes, options)?;
        if package.header.package_id != DEFINITION_PACKAGE_ID {
            return Err(DefinitionPackageDecodeError::InvalidPackageId {
                found: String::from_utf8_lossy(&package.header.package_id).into_owned(),
            });
        }
        Self::from_messagepack_bytes(&package.payload).map_err(Into::into)
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

pub fn bundled_definition_package() -> Result<&'static DefinitionPackage, StorageError> {
    PACKAGE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}

pub fn decode_definition_package(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    decode_definition_package_with_options(bytes, &RpkgReadOptions::default())
}

pub fn decode_definition_package_with_options(
    bytes: &[u8],
    options: &RpkgReadOptions,
) -> Result<DefinitionPackage, StorageError> {
    DefinitionPackage::from_bytes_with_options(bytes, options).map_err(|err| {
        StorageError::DefinitionsLoad {
            message: err.to_string(),
        }
    })
}

pub fn decode_definition_package_payload(bytes: &[u8]) -> Result<DefinitionPackage, StorageError> {
    DefinitionPackage::from_messagepack_bytes(bytes).map_err(|err| StorageError::DefinitionsLoad {
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
    use rheo_rpkg::{
        CompressionKind, IntegrityKind, PackagePurpose, RPKG_MAGIC, RpkgReadOptions,
        RpkgWriteOptions, RpkgWriter,
    };
    use serde::Serialize;

    use super::{
        DEFINITION_PACKAGE_ID, DefinitionPackage, bundled_definition_package, database,
        decode_definition_package, decode_definition_package_payload,
    };

    #[derive(Serialize)]
    struct RawPackageOut(String, String, u16, (), u32, Vec<super::DefinitionRecord>);

    fn encode_test_package(package: &DefinitionPackage, purpose: PackagePurpose) -> Vec<u8> {
        let payload = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();
        RpkgWriter::write_payload_bytes(
            &payload,
            &RpkgWriteOptions {
                package_id: DEFINITION_PACKAGE_ID,
                purpose,
                compression: CompressionKind::Lz4Frame,
                flags: 0,
                metadata: Some(rmp_serde::to_vec(&"filedefs").unwrap()),
                integrity: IntegrityKind::Sha256,
            },
        )
        .unwrap()
    }

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
    fn builder_style_package_roundtrip_is_semantically_stable() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_test_package(package, PackagePurpose::Standard);
        let decoded = decode_definition_package(&bytes).expect("encoded package should decode");

        assert_eq!(&decoded, package);
    }

    #[test]
    fn encoded_package_uses_rpkg_header() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_test_package(package, PackagePurpose::Standard);

        assert!(bytes.starts_with(RPKG_MAGIC));
    }

    #[test]
    fn legacy_plain_messagepack_is_rejected() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let plain = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();

        let error = DefinitionPackage::from_bytes(&plain).expect_err("legacy package should fail");
        assert!(error.to_string().contains("RPKG"));
    }

    #[test]
    fn wrong_package_id_is_rejected() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let payload = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();
        let bytes = RpkgWriter::write_payload_bytes(
            &payload,
            &RpkgWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::None,
                flags: 0,
                metadata: Some(rmp_serde::to_vec(&"meta").unwrap()),
                integrity: IntegrityKind::None,
            },
        )
        .unwrap();

        let error = DefinitionPackage::from_bytes(&bytes).expect_err("package id should fail");
        assert!(error.to_string().contains("identifier"));
    }

    #[test]
    fn decode_payload_helper_reads_raw_messagepack_payload() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let payload = rmp_serde::to_vec(&RawPackageOut(
            package.package_version.clone(),
            package.source_version.clone(),
            package.package_revision,
            (),
            package.tags,
            package.definitions.clone(),
        ))
        .unwrap();
        let decoded = decode_definition_package_payload(&payload).expect("payload should decode");
        assert_eq!(decoded.tags, package.tags);
        assert_eq!(decoded.definitions.len(), package.definitions.len());
    }

    #[test]
    fn fast_profile_filedefs_can_still_be_decoded_when_metadata_is_requested() {
        let package =
            bundled_definition_package().expect("bundled definitions package should deserialize");
        let bytes = encode_test_package(package, PackagePurpose::Embedded);
        let decoded = super::decode_definition_package_with_options(
            &bytes,
            &RpkgReadOptions {
                verify_integrity: true,
                load_metadata: true,
            },
        )
        .expect("package should decode");
        assert_eq!(decoded.package_version, package.package_version);
    }
}
