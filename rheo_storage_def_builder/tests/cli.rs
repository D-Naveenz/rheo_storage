use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("trid_xml")
}

fn copy_fixture_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap();
        }
    }
}

#[test]
fn inspect_trid_xml_has_human_friendly_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_rheo_storage_def_builder"))
        .arg("inspect-trid-xml")
        .arg("--input")
        .arg(fixtures_root())
        .output()
        .expect("builder CLI should run");

    assert!(output.status.success(), "command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Transformation Preview"));
    assert!(stdout.contains("Total Parsed"));
    assert!(stdout.contains("Final Kept"));
}

#[test]
fn running_without_a_subcommand_prints_help_and_exits_cleanly() {
    let output = Command::new(env!("CARGO_BIN_EXE_rheo_storage_def_builder"))
        .output()
        .expect("builder CLI should run");

    assert!(output.status.success(), "command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Build, inspect, and normalize Rheo definitions packages."));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Interactive mode:"));
}

#[test]
fn silent_mode_suppresses_normal_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_rheo_storage_def_builder"))
        .arg("--silent")
        .arg("inspect-trid-xml")
        .arg("--input")
        .arg(fixtures_root())
        .output()
        .expect("builder CLI should run");

    assert!(output.status.success(), "command should succeed");
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty in silent mode"
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty in silent mode"
    );
}

#[test]
fn default_package_output_and_logs_directories_are_used() {
    let temp = tempdir().unwrap();
    let package_dir = temp.path().join("package");
    copy_fixture_tree(&fixtures_root().join("defs"), &package_dir.join("defs"));

    let output = Command::new(env!("CARGO_BIN_EXE_rheo_storage_def_builder"))
        .env("RHEO_STORAGE_DEF_BUILDER_BASE_DIR", temp.path())
        .arg("build-trid-xml")
        .output()
        .expect("builder CLI should run");

    assert!(output.status.success(), "command should succeed");
    assert!(temp.path().join("output").join("filedefs.rpkg").exists());
    let log_files = fs::read_dir(temp.path().join("logs"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(log_files.len(), 1);
    assert!(log_files[0].ends_with("_def_builder.log"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Build Complete"));
    assert!(stdout.contains("Log"));
}
