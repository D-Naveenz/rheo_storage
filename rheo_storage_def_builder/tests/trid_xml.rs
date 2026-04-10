use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rheo_storage_def_builder::{
    build_trid_xml_package, build_trid_xml_package_with_report, inspect_trid_xml_source,
    write_package,
};
use rheo_storage_lib::decode_definition_package;
use rheo_storage_lib::definitions::is_compressed_definition_package;
use tempfile::tempdir;

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("trid_xml")
}

#[test]
fn builds_package_from_fixture_directory() {
    let package = build_trid_xml_package(fixtures_root()).expect("fixture directory should build");

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

    let from_directory =
        build_trid_xml_package(fixtures_root()).expect("fixture directory should build");
    let from_archive = build_trid_xml_package(&archive_path).expect("fixture archive should build");

    assert_eq!(from_archive, from_directory);
}

#[test]
fn inspects_trid_xml_source_without_writing_package() {
    let report =
        inspect_trid_xml_source(fixtures_root()).expect("fixture directory should be inspectable");

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

    let build = build_trid_xml_package_with_report(temp.path()).expect("fixture should build");
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
    let build = build_trid_xml_package_with_report(fixtures_root()).expect("fixture should build");

    write_package(&build.package, &output_path).expect("package should be written");
    let bytes = fs::read(&output_path).expect("written package should be readable");

    assert!(is_compressed_definition_package(&bytes));

    let decoded = decode_definition_package(&bytes).expect("runtime should decode compressed rpkg");
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

    let build =
        build_trid_xml_package_with_report(&source).expect("real archive should be inspectable");
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
