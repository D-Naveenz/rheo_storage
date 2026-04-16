use std::path::Path;

use anyhow::Result;
use rheo_tool_core::{CommandResult, run_command};

use crate::{PackageOptions, RheoRepoConfig, verify_release};

pub fn verify_release_config(repo_root: &Path) -> Result<CommandResult> {
    verify_release(repo_root)?;
    Ok(CommandResult::success())
}

pub fn verify_ci(repo_root: &Path, config: &RheoRepoConfig) -> Result<CommandResult> {
    verify_release_config(repo_root)?;

    for package in [
        "rheo_storage",
        "rheo_storage_ffi",
        "rheo_tool_core",
        "rheo_tool_ui",
        "rheo_tool_rheo_storage",
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
        "rheo_tool_core",
        "rheo_tool_ui",
        "rheo_tool_rheo_storage",
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
        "rheo_tool_core",
        "rheo_tool_ui",
        "rheo_tool_rheo_storage",
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

    Ok(CommandResult::success())
}

pub fn verify_docs(repo_root: &Path) -> Result<CommandResult> {
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
    for package in [
        "rheo_storage_ffi",
        "rheo_tool_core",
        "rheo_tool_ui",
        "rheo_tool_rheo_storage",
        "rheo_tool",
    ] {
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

    Ok(CommandResult::success())
}

pub fn verify_package(
    repo_root: &Path,
    config: &RheoRepoConfig,
    options: &PackageOptions,
) -> Result<CommandResult> {
    crate::package_flow::verify(repo_root, config, options)
}
