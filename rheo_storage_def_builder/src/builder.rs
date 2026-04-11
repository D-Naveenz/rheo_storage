use std::fs;
use std::path::{Path, PathBuf};

use rheo_storage::rpkg::{CompressionKind, PackagePurpose, SerializationKind, VerificationMode};
use rheo_storage::{
    DEFINITION_PACKAGE_ID, DefinitionPackage, PackageMetadata, bundled_definition_package,
    decode_definition_package_with_verification, decode_rpkg, encode_definition_package,
    encode_definition_package_with_purpose,
};
use thiserror::Error;
use tracing::{debug, info};

#[path = "trid_xml/mod.rs"]
mod trid_xml;

pub(crate) use trid_xml::{
    TridBuildProgress, TridBuildStage, TridTransformReport, build_trid_xml_package_with_progress,
};

/// Errors produced by the definitions builder CLI internals.
#[derive(Debug, Error)]
pub(crate) enum BuilderError {
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

    /// A TrID XML payload could not be parsed.
    #[error("failed to parse TrID XML '{path}': {message}")]
    Xml { path: PathBuf, message: String },

    /// A TrID XML definition contained an invalid hex byte sequence.
    #[error("invalid hex sequence '{value}' in '{path}'")]
    InvalidHex { path: PathBuf, value: String },

    /// A source path did not match a supported builder input kind.
    #[error("unsupported TrID source '{path}': expected a .7z archive, .xml file, or directory")]
    UnsupportedSource { path: PathBuf },

    /// A required archive tool was not available on the host.
    #[error("archive tool '{tool}' is not available on PATH")]
    ArchiveToolUnavailable { tool: &'static str },

    /// Extracting an archive failed.
    #[error("failed to {operation} archive '{path}': {message}")]
    ArchiveCommand {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },

    /// The TrID source did not expose any usable version values.
    #[error("failed to determine a usable TrID source version from: {versions}")]
    MissingSourceVersion { versions: String },
}

/// Summary information about a definitions package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageSummary {
    pub(crate) package_id: String,
    pub(crate) serialization: SerializationKind,
    pub(crate) compression: CompressionKind,
    pub(crate) purpose: PackagePurpose,
    pub(crate) package_version: String,
    pub(crate) source_version: String,
    pub(crate) package_revision: u16,
    pub(crate) checksum_verified: bool,
    pub(crate) tags: u32,
    pub(crate) definition_count: usize,
}

impl PackageSummary {
    pub(crate) fn from_loaded(loaded: &LoadedPackage) -> Self {
        Self {
            package_id: loaded.package_id_string(),
            serialization: loaded.serialization,
            compression: loaded.compression,
            purpose: loaded.purpose,
            package_version: loaded.metadata.package_version.clone(),
            source_version: loaded.metadata.source_version.clone(),
            package_revision: loaded.metadata.package_revision,
            checksum_verified: loaded.checksum_verified,
            tags: loaded.package.tags,
            definition_count: loaded.package.definitions.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedPackage {
    pub(crate) package: DefinitionPackage,
    pub(crate) package_id: [u8; 4],
    pub(crate) serialization: SerializationKind,
    pub(crate) compression: CompressionKind,
    pub(crate) purpose: PackagePurpose,
    pub(crate) metadata: PackageMetadata,
    pub(crate) checksum_verified: bool,
}

impl LoadedPackage {
    fn package_id_string(&self) -> String {
        String::from_utf8_lossy(&self.package_id).into_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SyncEmbeddedStatus {
    Skipped,
    UpToDate,
    Updated,
    NeedsUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncEmbeddedOutcome {
    pub(crate) input: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) status: SyncEmbeddedStatus,
    pub(crate) package_version: Option<String>,
    pub(crate) detail: String,
}

pub(crate) fn load_package(path: impl AsRef<Path>) -> Result<LoadedPackage, BuilderError> {
    load_package_with_verification(path, VerificationMode::Always)
}

pub(crate) fn load_package_with_verification(
    path: impl AsRef<Path>,
    verification_mode: VerificationMode,
) -> Result<LoadedPackage, BuilderError> {
    let path = path.as_ref();
    info!(path = %path.display(), "loading definitions package");
    let bytes = fs::read(path).map_err(|source| BuilderError::Io {
        operation: "read package",
        path: path.to_path_buf(),
        source,
    })?;
    decode_loaded_package(&bytes, verification_mode)
}

fn decode_loaded_package(
    bytes: &[u8],
    verification_mode: VerificationMode,
) -> Result<LoadedPackage, BuilderError> {
    let decoded = decode_rpkg(bytes, verification_mode).map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })?;
    let package =
        decode_definition_package_with_verification(bytes, verification_mode).map_err(|err| {
            BuilderError::Package {
                message: err.to_string(),
            }
        })?;
    Ok(LoadedPackage {
        package,
        package_id: decoded.package_id,
        serialization: decoded.serialization,
        compression: decoded.compression,
        purpose: decoded.purpose,
        metadata: decoded.metadata,
        checksum_verified: match verification_mode {
            VerificationMode::Always => true,
            VerificationMode::Skip => false,
            VerificationMode::Default => decoded.purpose == PackagePurpose::External,
        },
    })
}

pub(crate) fn load_bundled_package() -> Result<DefinitionPackage, BuilderError> {
    info!("loading bundled runtime definitions package");
    bundled_definition_package()
        .cloned()
        .map_err(|err| BuilderError::Package {
            message: err.to_string(),
        })
}

pub(crate) fn write_package(
    package: &DefinitionPackage,
    path: impl AsRef<Path>,
) -> Result<PathBuf, BuilderError> {
    write_package_with_purpose(package, path, PackagePurpose::External)
}

pub(crate) fn write_package_with_purpose(
    package: &DefinitionPackage,
    path: impl AsRef<Path>,
    purpose: PackagePurpose,
) -> Result<PathBuf, BuilderError> {
    let path = path.as_ref().to_path_buf();
    info!(path = %path.display(), ?purpose, "writing definitions package");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BuilderError::Io {
            operation: "create output directory for",
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let bytes = match purpose {
        PackagePurpose::External => encode_definition_package(package),
        PackagePurpose::Embedded => encode_definition_package_with_purpose(package, purpose),
    }
    .map_err(|err| BuilderError::Package {
        message: err.to_string(),
    })?;
    debug!(
        bytes = bytes.len(),
        definitions = package.definitions.len(),
        ?purpose,
        "encoded definitions package"
    );
    fs::write(&path, bytes).map_err(|source| BuilderError::Io {
        operation: "write package",
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub(crate) fn normalize_package(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<PathBuf, BuilderError> {
    info!("normalizing definitions package");
    let package = load_package(input)?;
    write_package_with_purpose(&package.package, output, package.purpose)
}

pub(crate) fn packages_match(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
) -> Result<bool, BuilderError> {
    info!("comparing definitions packages");
    let left = load_package(left)?;
    let right = load_package(right)?;
    Ok(left.package == right.package
        && left.package_id == right.package_id
        && left.serialization == right.serialization
        && left.compression == right.compression
        && left.purpose == right.purpose
        && left.metadata == right.metadata)
}

pub(crate) fn inspect_package(path: impl AsRef<Path>) -> Result<PackageSummary, BuilderError> {
    info!("inspecting definitions package");
    let package = load_package(path)?;
    Ok(PackageSummary::from_loaded(&package))
}

pub(crate) fn sync_embedded_package(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    check_only: bool,
) -> Result<SyncEmbeddedOutcome, BuilderError> {
    let input = input.as_ref().to_path_buf();
    let output = output.as_ref().to_path_buf();
    info!(
        input = %input.display(),
        output = %output.display(),
        check_only,
        "syncing embedded definitions package"
    );

    if !input.exists() {
        return Ok(SyncEmbeddedOutcome {
            input,
            output,
            status: SyncEmbeddedStatus::Skipped,
            package_version: None,
            detail: "TrID archive was not available; skipped embedded package refresh".to_string(),
        });
    }

    let build = trid_xml::build_trid_xml_package_with_report(&input)?;
    let desired_package = build.package;
    let desired_version = desired_package.package_version.clone();

    let current_state = match load_package(&output) {
        Ok(current) => current_validity_reason(&current, &desired_package),
        Err(error) if output.exists() => Some(format!("current embedded package is invalid: {error}")),
        Err(_) => Some("embedded package is missing".to_string()),
    };

    let Some(reason) = current_state else {
        return Ok(SyncEmbeddedOutcome {
            input,
            output,
            status: SyncEmbeddedStatus::UpToDate,
            package_version: Some(desired_version),
            detail: "embedded package is already current".to_string(),
        });
    };

    if check_only {
        return Ok(SyncEmbeddedOutcome {
            input,
            output,
            status: SyncEmbeddedStatus::NeedsUpdate,
            package_version: Some(desired_version),
            detail: reason,
        });
    }

    write_package_with_purpose(&desired_package, &output, PackagePurpose::Embedded)?;
    Ok(SyncEmbeddedOutcome {
        input,
        output,
        status: SyncEmbeddedStatus::Updated,
        package_version: Some(desired_version),
        detail: reason,
    })
}

fn current_validity_reason(
    current: &LoadedPackage,
    desired: &DefinitionPackage,
) -> Option<String> {
    if current.package_id != DEFINITION_PACKAGE_ID {
        return Some(format!(
            "embedded package id '{}' does not match 'FDEF'",
            current.package_id_string()
        ));
    }
    if current.purpose != PackagePurpose::Embedded {
        return Some("embedded package purpose is not marked as Embedded".to_string());
    }
    if current.metadata.package_version != desired.package_version {
        return Some(format!(
            "package version mismatch: current='{}', desired='{}'",
            current.metadata.package_version, desired.package_version
        ));
    }
    if current.metadata.source_version != desired.source_version {
        return Some(format!(
            "source version mismatch: current='{}', desired='{}'",
            current.metadata.source_version, desired.source_version
        ));
    }
    if current.metadata.package_revision != desired.package_revision {
        return Some(format!(
            "package revision mismatch: current='{}', desired='{}'",
            current.metadata.package_revision, desired.package_revision
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rheo_storage::{
        CompressionKind, PackagePurpose, SerializationKind, VerificationMode,
    };
    use tempfile::tempdir;

    use super::{
        PackageSummary, SyncEmbeddedStatus, load_bundled_package, normalize_package, packages_match,
        sync_embedded_package, trid_xml, write_package, write_package_with_purpose,
    };

    fn fixtures_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("trid_xml")
    }

    #[test]
    fn bundled_package_has_expected_summary() {
        let package = load_bundled_package().expect("bundled package should load");
        assert!(package.package_version.starts_with("trid-"));
        assert!(package.definitions.len() > 0);
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

    #[test]
    fn builds_package_from_fixture_directory() {
        let package = trid_xml::build_trid_xml_package(fixtures_root())
            .expect("fixture directory should build");

        assert_eq!(package.tags, 48);
        assert_eq!(package.source_version, "2.00");
        assert_eq!(package.package_revision, 1);
        assert_eq!(package.package_version, "trid-2.00+rpkg.1");
        assert_eq!(package.definitions.len(), 3);
        let png = package
            .definitions
            .iter()
            .find(|definition| definition.file_type == "Portable Network Graphics")
            .expect("png definition should be present");
        assert_eq!(png.extensions, vec!["png"]);
        assert_eq!(png.mime_type, "image/png");

        let zyzzyva = package
            .definitions
            .iter()
            .find(|definition| definition.file_type == "Zyzzyva Search")
            .expect("zyzzyva definition should be present");
        assert_eq!(zyzzyva.signature.strings.len(), 5);
        assert!(zyzzyva.priority_level >= png.priority_level);
        assert!(zyzzyva.remarks.contains(
            "Reference: http://www.scrabbleplayers.org/w/NASPA_Zyzzyva:_The_Last_Word_in_Word_Study"
        ));
    }

    #[test]
    fn builds_same_package_from_7z_archive() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should be available for archive creation");
        assert!(status.success(), "tar should create a 7z archive");

        let from_directory = trid_xml::build_trid_xml_package(fixtures_root())
            .expect("fixture directory should build");
        let from_archive =
            trid_xml::build_trid_xml_package(&archive_path).expect("fixture archive should build");

        assert_eq!(from_archive, from_directory);
    }

    #[test]
    fn inspects_trid_xml_source_without_writing_package() {
        let report = trid_xml::inspect_trid_xml_source(fixtures_root())
            .expect("fixture directory should be inspectable");

        assert_eq!(report.total_parsed, 3);
        assert_eq!(report.mime_corrected, 0);
        assert_eq!(report.mime_rejected, 0);
        assert_eq!(report.extension_rejected, 0);
        assert_eq!(report.signature_rejected, 0);
        assert_eq!(report.final_trimmed, 0);
        assert_eq!(report.final_kept, 3);
    }

    #[test]
    fn malformed_mime_types_are_corrected() {
        let temp = tempdir().expect("temporary directory should exist");
        let defs_dir = temp.path().join("defs").join("a");
        fs::create_dir_all(&defs_dir).expect("fixture directory should be created");
        fs::write(
            defs_dir.join("broken_pdf.trid.xml"),
            r#"<TrID ver="2.00">
    <Info>
        <FileType>Broken PDF</FileType>
        <Ext>PDF</Ext>
        <Mime>Applicaiton/PDF;</Mime>
        <ExtraInfo>
            <Rem></Rem>
            <RefURL>https://example.com/pdf</RefURL>
        </ExtraInfo>
    </Info>
    <General>
        <FileNum>42</FileNum>
    </General>
    <FrontBlock>
        <Pattern>
            <Bytes>255044462D</Bytes>
            <Pos>0</Pos>
        </Pattern>
    </FrontBlock>
</TrID>"#,
        )
        .expect("fixture XML should be written");

        let build = trid_xml::build_trid_xml_package_with_report(temp.path())
            .expect("fixture should build");
        let definition = build
            .package
            .definitions
            .first()
            .expect("definition should be present");

        assert_eq!(definition.mime_type, "application/pdf");
        assert_eq!(build.report.mime_corrected, 1);
        assert_eq!(build.report.final_kept, 1);
    }

    #[test]
    fn builder_writes_external_rpkg_readable_by_runtime() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.rpkg");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");

        write_package(&build.package, &output_path).expect("package should be written");
        let loaded = super::load_package(&output_path).expect("package should load");

        assert_eq!(loaded.package, build.package);
        assert_eq!(loaded.purpose, PackagePurpose::External);
        assert!(loaded.checksum_verified);
    }

    #[test]
    fn builder_writes_embedded_packages_that_skip_default_verification() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.rpkg");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");

        write_package_with_purpose(&build.package, &output_path, PackagePurpose::Embedded)
            .expect("embedded package should be written");
        let bytes = fs::read(&output_path).expect("written package should be readable");
        let decoded = rheo_storage::decode_definition_package(&bytes)
            .expect("runtime should decode embedded package");
        assert_eq!(decoded, build.package);

        let loaded = super::load_package_with_verification(&output_path, VerificationMode::Always)
            .expect("builder inspection should verify embedded packages");
        assert_eq!(loaded.purpose, PackagePurpose::Embedded);
    }

    #[test]
    fn inspect_package_reports_v2_metadata() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.rpkg");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");
        write_package(&build.package, &output_path).expect("package should be written");

        let summary = super::inspect_package(&output_path).expect("summary should load");
        assert_eq!(
            summary,
            PackageSummary {
                package_id: "FDEF".to_string(),
                serialization: SerializationKind::MessagePack,
                compression: CompressionKind::Lz4Frame,
                purpose: PackagePurpose::External,
                package_version: "trid-2.00+rpkg.1".to_string(),
                source_version: "2.00".to_string(),
                package_revision: 1,
                checksum_verified: true,
                tags: 48,
                definition_count: 3,
            }
        );
    }

    #[test]
    fn sync_embedded_skips_when_archive_is_missing() {
        let temp = tempdir().expect("temporary directory should exist");
        let outcome = sync_embedded_package(
            temp.path().join("missing.7z"),
            temp.path().join("filedefs.rpkg"),
            false,
        )
        .expect("sync should succeed");

        assert_eq!(outcome.status, SyncEmbeddedStatus::Skipped);
    }

    #[test]
    fn sync_embedded_updates_when_target_is_stale() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");
        let output_path = temp.path().join("filedefs.rpkg");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should create the fixture archive");
        assert!(status.success(), "tar should create a 7z archive");

        let package = load_bundled_package().expect("bundled package should load");
        write_package(&package, &output_path).expect("stale package should be written");

        let outcome =
            sync_embedded_package(&archive_path, &output_path, false).expect("sync should work");
        assert_eq!(outcome.status, SyncEmbeddedStatus::Updated);

        let loaded = super::load_package(&output_path).expect("updated package should load");
        assert_eq!(loaded.purpose, PackagePurpose::Embedded);
        assert_eq!(loaded.metadata.package_version, "trid-2.00+rpkg.1");
    }

    #[test]
    fn sync_embedded_check_reports_when_update_is_needed() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");
        let output_path = temp.path().join("filedefs.rpkg");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should create the fixture archive");
        assert!(status.success(), "tar should create a 7z archive");

        let output =
            sync_embedded_package(&archive_path, &output_path, true).expect("check should run");
        assert_eq!(output.status, SyncEmbeddedStatus::NeedsUpdate);
    }

    #[test]
    #[ignore = "slow smoke test over the real TrID XML archive"]
    fn full_archive_build_stays_within_target_range() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root should exist")
            .join("temp")
            .join("trid-defs")
            .join("triddefs_xml.7z");
        assert!(source.exists(), "real TrID archive should be available");

        let build = trid_xml::build_trid_xml_package_with_report(&source)
            .expect("real archive should be inspectable");
        let report = &build.report;

        assert!(
            report.final_kept >= 4_500,
            "final_kept={}",
            report.final_kept
        );
        assert!(
            report.final_kept <= 6_500,
            "final_kept={}",
            report.final_kept
        );
        assert!(build.package.definitions.iter().all(|definition| {
            !definition.mime_type.is_empty()
                && !definition.extensions.is_empty()
                && !definition.signature.patterns.is_empty()
        }));
    }

    #[test]
    fn decode_with_skip_verification_keeps_package_readable() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.rpkg");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");

        write_package_with_purpose(&build.package, &output_path, PackagePurpose::Embedded)
            .expect("embedded package should be written");
        let bytes = fs::read(&output_path).expect("written package should be readable");
        let loaded = super::decode_loaded_package(&bytes, VerificationMode::Skip)
            .expect("decode should succeed");
        assert_eq!(loaded.package, build.package);
    }

    #[test]
    fn sync_embedded_check_does_not_touch_output() {
        let temp = tempdir().expect("temporary directory should exist");
        let archive_path = temp.path().join("triddefs_xml.7z");
        let output_path = temp.path().join("filedefs.rpkg");

        let status = Command::new("tar")
            .arg("-a")
            .arg("-cf")
            .arg(&archive_path)
            .arg("-C")
            .arg(fixtures_root())
            .arg("defs")
            .status()
            .expect("tar should create the fixture archive");
        assert!(status.success(), "tar should create a 7z archive");

        let outcome =
            sync_embedded_package(&archive_path, &output_path, true).expect("check should work");
        assert_eq!(outcome.status, SyncEmbeddedStatus::NeedsUpdate);
        assert!(!output_path.exists());
    }
}
