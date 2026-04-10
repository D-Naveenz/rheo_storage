use std::fs;
use std::path::{Path, PathBuf};

use rheo_storage::{
    DefinitionPackage, bundled_definition_package, decode_definition_package,
    encode_definition_package,
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
}

/// Summary information about a definitions package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageSummary {
    /// Package format version string.
    pub(crate) package_version: String,
    /// Reserved tag count carried forward from the legacy package.
    pub(crate) tags: u32,
    /// Number of definitions in the package.
    pub(crate) definition_count: usize,
}

impl PackageSummary {
    pub(crate) fn from_package(package: &DefinitionPackage) -> Self {
        Self {
            package_version: package.package_version.clone(),
            tags: package.tags,
            definition_count: package.definitions.len(),
        }
    }
}

pub(crate) fn load_package(path: impl AsRef<Path>) -> Result<DefinitionPackage, BuilderError> {
    let path = path.as_ref();
    info!(path = %path.display(), "loading definitions package");
    let bytes = fs::read(path).map_err(|source| BuilderError::Io {
        operation: "read package",
        path: path.to_path_buf(),
        source,
    })?;
    decode_definition_package(&bytes).map_err(|err| BuilderError::Package {
        message: err.to_string(),
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
    let path = path.as_ref().to_path_buf();
    info!(path = %path.display(), "writing definitions package");
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
    debug!(
        bytes = bytes.len(),
        definitions = package.definitions.len(),
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
    write_package(&package, output)
}

pub(crate) fn packages_match(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
) -> Result<bool, BuilderError> {
    info!("comparing definitions packages");
    let left = load_package(left)?;
    let right = load_package(right)?;
    Ok(left == right)
}

pub(crate) fn inspect_package(path: impl AsRef<Path>) -> Result<PackageSummary, BuilderError> {
    info!("inspecting definitions package");
    let package = load_package(path)?;
    Ok(PackageSummary::from_package(&package))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rheo_storage::decode_definition_package;
    use rheo_storage::definitions::is_compressed_definition_package;
    use tempfile::tempdir;

    use super::{
        PackageSummary, load_bundled_package, normalize_package, packages_match, trid_xml,
        write_package,
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

    #[test]
    fn builds_package_from_fixture_directory() {
        let package = trid_xml::build_trid_xml_package(fixtures_root())
            .expect("fixture directory should build");

        assert_eq!(package.tags, 48);
        assert_eq!(package.package_version, "");
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
    fn builder_writes_compressed_rpkg_readable_by_runtime() {
        let temp = tempdir().expect("temporary directory should exist");
        let output_path = temp.path().join("filedefs.rpkg");
        let build = trid_xml::build_trid_xml_package_with_report(fixtures_root())
            .expect("fixture should build");

        write_package(&build.package, &output_path).expect("package should be written");
        let bytes = fs::read(&output_path).expect("written package should be readable");

        assert!(is_compressed_definition_package(&bytes));

        let decoded =
            decode_definition_package(&bytes).expect("runtime should decode compressed rpkg");
        assert_eq!(decoded, build.package);
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
}
