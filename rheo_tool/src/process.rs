use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

fn command_display(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_owned()
    } else {
        format!("{program} {}", args.join(" "))
    }
}

pub fn run_command(program: &str, args: &[String], cwd: &Path) -> Result<()> {
    println!("> {}", command_display(program, args));
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
    Ok(())
}

pub fn run_command_with_env(
    program: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&str, &str)],
) -> Result<()> {
    println!("> {}", command_display(program, args));
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
    Ok(())
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
        && std::env::var_os("NUGET_PACKAGES").is_none()
        && !envs.iter().any(|(key, _)| *key == "NUGET_PACKAGES")
    {
        let packages_dir = cwd.join(".nuget").join("packages");
        fs::create_dir_all(&packages_dir)
            .with_context(|| format!("failed to create {}", packages_dir.display()))?;
        command.env("NUGET_PACKAGES", packages_dir);
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
