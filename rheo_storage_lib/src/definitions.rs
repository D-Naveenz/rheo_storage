use std::collections::BTreeSet;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::error::StorageError;

const LEGACY_FILEDEFS_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../temp/rheo-storage/Rheo.Storage/Assets/filedefs.rpkg"
));
const CATCH_ALL_INDEX: usize = 256;

static DATABASE: Lazy<Result<DefinitionDatabase, String>> = Lazy::new(|| {
    DefinitionDatabase::from_bytes(LEGACY_FILEDEFS_BYTES).map_err(|err| err.to_string())
});

#[derive(Debug, Clone)]
pub(crate) struct DefinitionDatabase {
    definitions: Vec<LegacyDefinition>,
    pattern_index: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct LegacyDefinition {
    pub(crate) file_type: String,
    #[serde(default)]
    pub(crate) extensions: Vec<String>,
    #[serde(default)]
    pub(crate) mime_type: String,
    #[serde(default)]
    pub(crate) _remarks: String,
    #[serde(default)]
    pub(crate) signature: Signature,
    #[serde(default)]
    pub(crate) priority_level: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct Signature {
    #[serde(default)]
    pub(crate) patterns: Vec<Pattern>,
    #[serde(default)]
    pub(crate) strings: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct Pattern {
    pub(crate) position: u16,
    #[serde(default)]
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RawPackage(String, IgnoredAny, u32, Vec<LegacyDefinition>);

impl DefinitionDatabase {
    fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        let RawPackage(_package_version, _, _tags, definitions): RawPackage =
            rmp_serde::from_slice(bytes)?;

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

        Ok(Self {
            definitions,
            pattern_index,
        })
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

    pub(crate) fn definition(&self, idx: usize) -> &LegacyDefinition {
        &self.definitions[idx]
    }
}

pub(crate) fn database() -> Result<&'static DefinitionDatabase, StorageError> {
    DATABASE
        .as_ref()
        .map_err(|message| StorageError::DefinitionsLoad {
            message: message.clone(),
        })
}
