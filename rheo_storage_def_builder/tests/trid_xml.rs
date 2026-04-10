use std::path::{Path, PathBuf};
use std::process::Command;

use rheo_storage_def_builder::{build_trid_xml_package, inspect_trid_xml_source};
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
    assert_eq!(
        package.definitions[0].file_type,
        "Portable Network Graphics"
    );
    assert_eq!(package.definitions[0].extensions, vec!["png"]);
    assert_eq!(package.definitions[0].mime_type, "image/png");
    assert!(package.definitions[0].priority_level > package.definitions[1].priority_level);

    let zyzzyva = package
        .definitions
        .iter()
        .find(|definition| definition.file_type == "Zyzzyva Search")
        .expect("zyzzyva definition should be present");
    assert_eq!(zyzzyva.signature.strings.len(), 5);
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
    let summary =
        inspect_trid_xml_source(fixtures_root()).expect("fixture directory should be inspectable");

    assert_eq!(summary.package_version, "");
    assert_eq!(summary.tags, 48);
    assert_eq!(summary.definition_count, 3);
}
