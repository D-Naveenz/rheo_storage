use std::path::Path;

use rheo_storage_lib::{
    DefinitionPackage, DefinitionRecord, SignatureDefinition, SignaturePattern,
};

use crate::{BuilderError, PackageSummary};

mod model;
mod source;

const LEGACY_VALIDATED_TAGS: u32 = 48;

/// Build a `filedefs.rpkg`-compatible package from a TrID XML source.
///
/// The source may be a single `.xml` definition file, a directory that contains
/// extracted TrID XML definitions, or a `.7z` archive containing the XML tree.
///
/// # Returns
///
/// - `Result<DefinitionPackage, BuilderError>` - A package compatible with `rheo_storage_lib`.
///
/// # Errors
///
/// Returns an error when the source cannot be opened, extracted, parsed, or
/// transformed into a valid package.
///
/// # Examples
///
/// ```no_run
/// use rheo_storage_def_builder::build_trid_xml_package;
///
/// let _ = build_trid_xml_package("temp/trid-defs/triddefs_xml.7z");
/// ```
pub fn build_trid_xml_package(source: impl AsRef<Path>) -> Result<DefinitionPackage, BuilderError> {
    let definitions = source::load_trid_definitions(source.as_ref())?
        .into_iter()
        .map(ParsedTridDefinition::into_definition_record)
        .collect::<Vec<_>>();

    let mut package = DefinitionPackage {
        package_version: String::new(),
        tags: LEGACY_VALIDATED_TAGS,
        definitions,
    };
    package.definitions.sort_by(|left, right| {
        right
            .priority_level
            .cmp(&left.priority_level)
            .then_with(|| left.file_type.cmp(&right.file_type))
            .then_with(|| left.mime_type.cmp(&right.mime_type))
            .then_with(|| left.extensions.cmp(&right.extensions))
            .then_with(|| left.remarks.cmp(&right.remarks))
    });

    Ok(package)
}

/// Inspect a TrID XML source without writing a package file.
///
/// # Returns
///
/// - `Result<PackageSummary, BuilderError>` - Summary metadata for the generated package.
///
/// # Errors
///
/// Returns an error when the source cannot be parsed into a package.
///
/// # Examples
///
/// ```no_run
/// use rheo_storage_def_builder::inspect_trid_xml_source;
///
/// let _ = inspect_trid_xml_source("temp/trid-defs/triddefs_xml.7z");
/// ```
pub fn inspect_trid_xml_source(source: impl AsRef<Path>) -> Result<PackageSummary, BuilderError> {
    let package = build_trid_xml_package(source)?;
    Ok(PackageSummary::from_package(&package))
}

#[derive(Debug, Clone)]
pub(crate) struct TridPattern {
    pub(crate) position: u16,
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct TridSignature {
    pub(crate) patterns: Vec<TridPattern>,
    pub(crate) strings: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedTridDefinition {
    pub(crate) file_type: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) mime_type: String,
    pub(crate) remarks: String,
    pub(crate) signature: TridSignature,
    pub(crate) file_count: u32,
}

impl ParsedTridDefinition {
    fn into_definition_record(self) -> DefinitionRecord {
        let priority_level = calculate_priority(
            self.file_count,
            self.signature.patterns.len(),
            self.signature.strings.len(),
        );

        DefinitionRecord {
            file_type: self.file_type,
            extensions: self.extensions,
            mime_type: self.mime_type,
            remarks: self.remarks,
            signature: SignatureDefinition {
                patterns: self
                    .signature
                    .patterns
                    .into_iter()
                    .map(|pattern| SignaturePattern {
                        position: pattern.position,
                        data: pattern.data,
                    })
                    .collect(),
                strings: self.signature.strings,
            },
            priority_level,
        }
    }
}

fn calculate_priority(file_count: u32, pattern_count: usize, string_count: usize) -> i32 {
    let pattern_score = pattern_count as i32 * 10;
    let string_score = string_count as i32 * 4;
    let identifiability_bonus = if pattern_count > 0 { 25 } else { 0 };

    file_count.min(i32::MAX as u32) as i32 + pattern_score + string_score + identifiability_bonus
}
