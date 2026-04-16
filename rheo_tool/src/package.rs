use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rheo_repo_tool::{RepoConfig, sync, verify_release};
use zip::ZipArchive;

use crate::process::{run_command, run_command_with_env};

#[derive(Debug, Clone)]
pub struct PackageOptions {
    pub configuration: String,
    pub version_override: Option<String>,
    pub source_override: Option<String>,
    pub api_key_env_override: Option<String>,
    pub output_dir: Option<PathBuf>,
    pub execute_publish: bool,
}

pub fn pack(repo_root: &Path, config: &RepoConfig, options: &PackageOptions) -> Result<PathBuf> {
    verify_release(repo_root)?;
    sync(repo_root)?;

    let version = effective_version(config, &options.version_override);
    let working_root = working_root(repo_root, options.output_dir.as_ref())?;
    let native_stage_root = working_root.join("native-stage");
    let nuget_output = working_root.join("nuget");
    reset_directory(&native_stage_root)?;
    reset_directory(&nuget_output)?;

    stage_native_assets(repo_root, config, options, &native_stage_root)?;

    run_command(
        "dotnet",
        &[
            "pack".to_owned(),
            config.ci.package_project.clone(),
            "--configuration".to_owned(),
            options.configuration.clone(),
            "--include-symbols".to_owned(),
            "-p:ContinuousIntegrationBuild=true".to_owned(),
            format!("-p:Version={version}"),
            format!("-p:StagedNativeRoot={}", native_stage_root.display()),
            "--output".to_owned(),
            nuget_output.display().to_string(),
        ],
        repo_root,
    )?;

    let package_path = nuget_output.join(format!("{}.{}.nupkg", config.nuget.package_id, version));
    inspect_package_contents(&package_path, config)?;
    Ok(package_path)
}

pub fn verify(repo_root: &Path, config: &RepoConfig, options: &PackageOptions) -> Result<()> {
    let version = effective_version(config, &options.version_override);
    let package_path = pack(repo_root, config, options)?;
    let working_root = package_path
        .parent()
        .and_then(Path::parent)
        .context("package output folder should have a parent")?;
    let local_config = working_root.join("local-package.nuget.config");
    write_nuget_config(
        &local_config,
        &[
            package_path
                .parent()
                .context("package path should have a parent")?
                .to_path_buf(),
            PathBuf::from(&config.nuget.source),
        ],
    )?;

    restore_smoke_consumer(repo_root, config, &version, &local_config, None, false)?;
    run_smoke_consumer(repo_root, config, &version)?;
    restore_smoke_consumer(
        repo_root,
        config,
        &version,
        &local_config,
        Some(&config.ci.aot_runtime_smoke),
        true,
    )?;
    publish_aot_smoke_consumer(
        repo_root,
        config,
        &version,
        &working_root.join("smoke-aot"),
        &config.ci.aot_runtime_smoke,
    )?;
    Ok(())
}

pub fn publish(repo_root: &Path, config: &RepoConfig, options: &PackageOptions) -> Result<()> {
    let version = effective_version(config, &options.version_override);
    let package_path = pack(repo_root, config, options)?;
    let working_root = package_path
        .parent()
        .and_then(Path::parent)
        .context("package output folder should have a parent")?;
    let local_config = working_root.join("local-package.nuget.config");
    write_nuget_config(
        &local_config,
        &[
            package_path
                .parent()
                .context("package path should have a parent")?
                .to_path_buf(),
            PathBuf::from(&config.nuget.source),
        ],
    )?;

    restore_smoke_consumer(repo_root, config, &version, &local_config, None, false)?;
    run_smoke_consumer(repo_root, config, &version)?;
    restore_smoke_consumer(
        repo_root,
        config,
        &version,
        &local_config,
        Some(&config.ci.aot_runtime_smoke),
        true,
    )?;
    publish_aot_smoke_consumer(
        repo_root,
        config,
        &version,
        &working_root.join("smoke-aot"),
        &config.ci.aot_runtime_smoke,
    )?;

    if !options.execute_publish {
        println!("Dry run complete. Package was verified but not published.");
        return Ok(());
    }

    let source = options
        .source_override
        .clone()
        .unwrap_or_else(|| config.nuget.source.clone());
    let api_key_env = options
        .api_key_env_override
        .clone()
        .unwrap_or_else(|| config.publish.api_key_env.clone());
    let api_key =
        std::env::var(&api_key_env).with_context(|| format!("{api_key_env} is not set"))?;
    run_command_with_env(
        "dotnet",
        &[
            "nuget".to_owned(),
            "push".to_owned(),
            package_path.display().to_string(),
            "--api-key".to_owned(),
            api_key,
            "--source".to_owned(),
            source.clone(),
            "--skip-duplicate".to_owned(),
        ],
        repo_root,
        &[],
    )?;

    let published_config = working_root.join("published-package.nuget.config");
    write_nuget_config(&published_config, &[PathBuf::from(source)])?;
    restore_smoke_consumer(repo_root, config, &version, &published_config, None, false)?;
    run_smoke_consumer(repo_root, config, &version)?;
    Ok(())
}

fn stage_native_assets(
    repo_root: &Path,
    config: &RepoConfig,
    options: &PackageOptions,
    stage_root: &Path,
) -> Result<()> {
    let profile_flag = if options.configuration.eq_ignore_ascii_case("Release") {
        "--release"
    } else {
        bail!("only Release packaging is currently supported");
    };

    for rid in &config.ci.native_runtimes {
        let target = config
            .targets
            .rust_targets
            .get(rid)
            .with_context(|| format!("missing rust target mapping for runtime '{rid}'"))?;
        run_command(
            "cargo",
            &[
                "build".to_owned(),
                "-p".to_owned(),
                "rheo_storage_ffi".to_owned(),
                profile_flag.to_owned(),
                "--target".to_owned(),
                target.clone(),
            ],
            repo_root,
        )?;

        let source_path = repo_root
            .join("target")
            .join(target)
            .join("release")
            .join("rheo_storage_ffi.dll");
        let destination_path = stage_root
            .join("runtimes")
            .join(rid)
            .join("native")
            .join("rheo_storage_ffi.dll");
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(&source_path, &destination_path).with_context(|| {
            format!(
                "failed to copy native asset from '{}' to '{}'",
                source_path.display(),
                destination_path.display()
            )
        })?;
    }

    Ok(())
}

fn inspect_package_contents(package_path: &Path, config: &RepoConfig) -> Result<()> {
    let file = fs::File::open(package_path)
        .with_context(|| format!("failed to open {}", package_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("failed to read {}", package_path.display()))?;

    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        entries.push(file.name().replace('\\', "/"));
    }

    if !entries
        .iter()
        .any(|entry| entry == "lib/net10.0/Rheo.Storage.dll")
    {
        bail!("managed assembly missing from package");
    }

    for rid in &config.ci.native_runtimes {
        let expected = format!("runtimes/{rid}/native/rheo_storage_ffi.dll");
        if !entries.iter().any(|entry| entry == &expected) {
            bail!("native asset missing from package: {expected}");
        }
    }

    Ok(())
}

fn write_nuget_config(path: &Path, sources: &[PathBuf]) -> Result<()> {
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

fn restore_smoke_consumer(
    repo_root: &Path,
    config: &RepoConfig,
    version: &str,
    nuget_config: &Path,
    runtime: Option<&str>,
    publish_aot: bool,
) -> Result<()> {
    let mut args = vec![
        "restore".to_owned(),
        config.ci.smoke_project.clone(),
        format!("-p:RheoStoragePackageVersion={version}"),
        format!("--configfile={}", nuget_config.display()),
        "--force-evaluate".to_owned(),
    ];
    if let Some(runtime) = runtime {
        args.push("--runtime".to_owned());
        args.push(runtime.to_owned());
    }
    if publish_aot {
        args.push("-p:PublishAot=true".to_owned());
    }
    run_command("dotnet", &args, repo_root)
}

fn run_smoke_consumer(repo_root: &Path, config: &RepoConfig, version: &str) -> Result<()> {
    run_command(
        "dotnet",
        &[
            "run".to_owned(),
            "--project".to_owned(),
            config.ci.smoke_project.clone(),
            "--configuration".to_owned(),
            "Release".to_owned(),
            "--no-restore".to_owned(),
            format!("-p:RheoStoragePackageVersion={version}"),
        ],
        repo_root,
    )
}

fn publish_aot_smoke_consumer(
    repo_root: &Path,
    config: &RepoConfig,
    version: &str,
    output_dir: &Path,
    runtime: &str,
) -> Result<()> {
    reset_directory(output_dir)?;
    run_command(
        "dotnet",
        &[
            "publish".to_owned(),
            config.ci.smoke_project.clone(),
            "--configuration".to_owned(),
            "Release".to_owned(),
            "--runtime".to_owned(),
            runtime.to_owned(),
            "--self-contained".to_owned(),
            "true".to_owned(),
            "--no-restore".to_owned(),
            "-p:PublishAot=true".to_owned(),
            format!("-p:RheoStoragePackageVersion={version}"),
            "--output".to_owned(),
            output_dir.display().to_string(),
        ],
        repo_root,
    )?;

    let executable = output_dir.join("Rheo.Storage.ConsumerSmoke.exe");
    run_command(
        executable
            .to_str()
            .context("published smoke consumer path was not valid utf-8")?,
        &[],
        repo_root,
    )
}

fn effective_version(config: &RepoConfig, override_value: &Option<String>) -> String {
    override_value
        .clone()
        .unwrap_or_else(|| config.versions.nuget_package.clone())
}

fn working_root(repo_root: &Path, override_value: Option<&PathBuf>) -> Result<PathBuf> {
    let root = override_value
        .cloned()
        .unwrap_or_else(|| repo_root.join(".artifacts").join("rheo_tool"));
    fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;
    Ok(root)
}

fn reset_directory(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::Write;

    use rheo_repo_tool::{
        CiConfig, NuGetConfig, PublishConfig, RepoConfig, TargetsConfig, VersionConfig,
    };
    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::inspect_package_contents;

    fn sample_config() -> RepoConfig {
        RepoConfig {
            versions: VersionConfig {
                rust_workspace: "0.2.0".to_owned(),
                nuget_package: "2.0.0".to_owned(),
            },
            nuget: NuGetConfig {
                package_id: "Rheo.Storage".to_owned(),
                source: "https://api.nuget.org/v3/index.json".to_owned(),
                authors: vec!["Naveen Dharmathunga".to_owned()],
                description: "desc".to_owned(),
                tags: vec!["storage".to_owned()],
                readme: "README.md".to_owned(),
                icon: None,
                repository_url: "https://example.com".to_owned(),
                project_url: "https://example.com".to_owned(),
            },
            ci: CiConfig {
                smoke_project: "smoke.csproj".to_owned(),
                package_project: "pkg.csproj".to_owned(),
                tests_project: "tests.csproj".to_owned(),
                native_runtimes: vec!["win-x64".to_owned(), "win-arm64".to_owned()],
                host_runtime_smoke: "win-x64".to_owned(),
                aot_runtime_smoke: "win-x64".to_owned(),
            },
            publish: PublishConfig {
                environment: "nuget-production".to_owned(),
                api_key_env: "NUGET_API_KEY".to_owned(),
            },
            targets: TargetsConfig {
                rust_targets: BTreeMap::from([
                    ("win-x64".to_owned(), "x86_64-pc-windows-msvc".to_owned()),
                    ("win-arm64".to_owned(), "aarch64-pc-windows-msvc".to_owned()),
                ]),
            },
        }
    }

    #[test]
    fn inspects_expected_nupkg_entries() {
        let temp = tempdir().unwrap();
        let package_path = temp.path().join("Rheo.Storage.2.0.0.nupkg");
        let file = File::create(&package_path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        for entry in [
            "lib/net10.0/Rheo.Storage.dll",
            "runtimes/win-x64/native/rheo_storage_ffi.dll",
            "runtimes/win-arm64/native/rheo_storage_ffi.dll",
        ] {
            writer.start_file(entry, options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();

        inspect_package_contents(&package_path, &sample_config()).unwrap();
    }
}
