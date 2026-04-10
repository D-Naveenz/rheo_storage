use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("trid_xml")
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
