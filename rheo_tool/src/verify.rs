use std::path::Path;

use anyhow::Result;
use rheo_repo_tool::{RepoConfig, verify_release};

use crate::package::{PackageOptions, verify as verify_package_flow};
use crate::process::run_command;

pub fn verify_release_config(repo_root: &Path) -> Result<()> {
    verify_release(repo_root)
}

pub fn verify_ci(repo_root: &Path, config: &RepoConfig) -> Result<()> {
    verify_release_config(repo_root)?;

    for package in [
        "rheo_storage",
        "rheo_storage_ffi",
        "rheo_repo_tool",
        "rheo_storage_def_builder",
        "rheo_tool",
    ] {
        run_command(
            "cargo",
            &[
                "fmt".to_owned(),
                "-p".to_owned(),
                package.to_owned(),
                "--check".to_owned(),
            ],
            repo_root,
        )?;
    }

    run_command(
        "cargo",
        &[
            "clippy".to_owned(),
            "-p".to_owned(),
            "rheo_storage".to_owned(),
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
            "--".to_owned(),
            "-D".to_owned(),
            "warnings".to_owned(),
        ],
        repo_root,
    )?;
    for package in [
        "rheo_storage_ffi",
        "rheo_repo_tool",
        "rheo_storage_def_builder",
        "rheo_tool",
    ] {
        run_command(
            "cargo",
            &[
                "clippy".to_owned(),
                "-p".to_owned(),
                package.to_owned(),
                "--all-targets".to_owned(),
                "--".to_owned(),
                "-D".to_owned(),
                "warnings".to_owned(),
            ],
            repo_root,
        )?;
    }

    run_command(
        "cargo",
        &[
            "test".to_owned(),
            "-p".to_owned(),
            "rheo_storage".to_owned(),
            "--all-features".to_owned(),
        ],
        repo_root,
    )?;
    for package in [
        "rheo_storage_ffi",
        "rheo_repo_tool",
        "rheo_storage_def_builder",
        "rheo_tool",
    ] {
        run_command(
            "cargo",
            &["test".to_owned(), "-p".to_owned(), package.to_owned()],
            repo_root,
        )?;
    }

    run_command(
        "dotnet",
        &["test".to_owned(), config.ci.tests_project.clone()],
        repo_root,
    )?;

    Ok(())
}

pub fn verify_docs(repo_root: &Path) -> Result<()> {
    run_command(
        "cargo",
        &[
            "doc".to_owned(),
            "-p".to_owned(),
            "rheo_storage".to_owned(),
            "--no-deps".to_owned(),
            "--all-features".to_owned(),
        ],
        repo_root,
    )?;
    for package in ["rheo_storage_ffi", "rheo_repo_tool", "rheo_tool"] {
        run_command(
            "cargo",
            &[
                "doc".to_owned(),
                "-p".to_owned(),
                package.to_owned(),
                "--no-deps".to_owned(),
            ],
            repo_root,
        )?;
    }
    Ok(())
}

pub fn verify_package(
    repo_root: &Path,
    config: &RepoConfig,
    options: &PackageOptions,
) -> Result<()> {
    verify_package_flow(repo_root, config, options)
}
