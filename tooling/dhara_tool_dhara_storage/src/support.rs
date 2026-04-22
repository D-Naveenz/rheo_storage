use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use tracing::{debug, info};
use zip::ZipArchive;

fn command_display(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_owned()
    } else {
        format!("{program} {}", args.join(" "))
    }
}

pub fn run_command(program: &str, args: &[String], cwd: &Path) -> Result<()> {
    println!("> {}", command_display(program, args));
    info!(
        target: "dhara_tool_dhara_storage::support",
        program,
        args = args.join(" "),
        cwd = %cwd.display(),
        "running command"
    );
    let status = prepare_command(program, args, cwd, &[])?
        .status()
        .with_context(|| format!("failed to start '{program}'"))?;
    if !status.success() {
        bail!(
            "command failed with status {}: {}",
            status,
            command_display(program, args)
        );
    }
    debug!(
        target: "dhara_tool_dhara_storage::support",
        program,
        status = %status,
        "command completed successfully"
    );
    Ok(())
}

pub fn run_command_with_env(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&str, &str)],
) -> Result<()> {
    println!("> {}", command_display(program, args));
    info!(
        target: "dhara_tool_dhara_storage::support",
        program,
        args = args.join(" "),
        cwd = %cwd.display(),
        env_count = envs.len(),
        "running command with environment overrides"
    );
    let status = prepare_command(program, args, cwd, envs)?
        .status()
        .with_context(|| format!("failed to start '{program}'"))?;
    if !status.success() {
        bail!(
            "command failed with status {}: {}",
            status,
            command_display(program, args)
        );
    }
    debug!(
        target: "dhara_tool_dhara_storage::support",
        program,
        status = %status,
        "command completed successfully"
    );
    Ok(())
}

pub fn run_command_expect_failure(
    program: &str,
    args: &[String],
    cwd: &Path,
    expected_output: &str,
) -> Result<()> {
    println!("> {}", command_display(program, args));
    info!(
        target: "dhara_tool_dhara_storage::support",
        program,
        args = args.join(" "),
        cwd = %cwd.display(),
        expected_output,
        "running command that is expected to fail"
    );
    let output = prepare_command(program, args, cwd, &[])?
        .output()
        .with_context(|| format!("failed to start '{program}'"))?;
    if output.status.success() {
        bail!(
            "command unexpectedly succeeded: {}",
            command_display(program, args)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    if !combined.contains(expected_output) {
        bail!(
            "command failed without expected output '{}': {}",
            expected_output,
            command_display(program, args)
        );
    }

    debug!(
        target: "dhara_tool_dhara_storage::support",
        program,
        status = %output.status,
        expected_output,
        "command failed as expected"
    );
    Ok(())
}

pub fn write_nuget_config(path: &Path, sources: &[PathBuf]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut body = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
"#,
    );
    for (index, source) in sources.iter().enumerate() {
        body.push_str(&format!(
            "    <add key=\"source{index}\" value=\"{}\" />\n",
            source.display()
        ));
    }
    body.push_str("  </packageSources>\n</configuration>\n");
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn inspect_package_entries(package_path: &Path) -> Result<Vec<String>> {
    debug!(
        target: "dhara_tool_dhara_storage::support",
        package_path = %package_path.display(),
        "reading package entries"
    );
    let file = fs::File::open(package_path)
        .with_context(|| format!("failed to open {}", package_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("failed to read {}", package_path.display()))?;

    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        entries.push(file.name().replace('\\', "/"));
    }

    Ok(entries)
}

fn prepare_command<'a>(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&'a str, &'a str)],
) -> Result<Command> {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);
    for (key, value) in envs {
        command.env(key, value);
    }

    if program.eq_ignore_ascii_case("dotnet")
        && std::env::var_os("DOTNET_CLI_HOME").is_none()
        && !envs.iter().any(|(key, _)| *key == "DOTNET_CLI_HOME")
    {
        let dotnet_home = cwd.join(".dotnet");
        fs::create_dir_all(&dotnet_home)
            .with_context(|| format!("failed to create {}", dotnet_home.display()))?;
        command.env("DOTNET_CLI_HOME", dotnet_home);
    }

    if program.eq_ignore_ascii_case("dotnet")
        && std::env::var_os("DOTNET_SKIP_FIRST_TIME_EXPERIENCE").is_none()
        && !envs
            .iter()
            .any(|(key, _)| *key == "DOTNET_SKIP_FIRST_TIME_EXPERIENCE")
    {
        command.env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1");
    }

    Ok(command)
}
