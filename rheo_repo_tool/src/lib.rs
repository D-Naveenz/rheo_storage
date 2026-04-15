use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, value};
use xmltree::{Element, XMLNode};

pub const CONFIG_PATH: &str = "rheo.config.toml";
pub const ENV_EXAMPLE_PATH: &str = ".env.example";
pub const ENV_LOCAL_PATH: &str = ".env.local";
pub const ROOT_CARGO_TOML_PATH: &str = "Cargo.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoConfig {
    pub versions: VersionConfig,
    pub nuget: NuGetConfig,
    pub ci: CiConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionConfig {
    pub rust_workspace: String,
    pub nuget_package: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NuGetConfig {
    pub package_id: String,
    pub source: String,
    pub authors: Vec<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub readme: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub repository_url: String,
    pub project_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiConfig {
    pub smoke_project: String,
    pub package_project: String,
    pub tests_project: String,
    pub environment: String,
    pub runtime: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowOutput {
    pub config: RepoConfig,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionChannel {
    Rust,
    NuGet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionPart {
    Major,
    Minor,
    Patch,
}

pub fn load_config(repo_root: &Path) -> Result<RepoConfig> {
    let config_path = repo_root.join(CONFIG_PATH);
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    toml::from_str(&content).with_context(|| format!("failed to parse {}", config_path.display()))
}

pub fn load_env(repo_root: &Path) -> Result<BTreeMap<String, String>> {
    let env_path = repo_root.join(ENV_LOCAL_PATH);
    if !env_path.exists() {
        return Ok(BTreeMap::new());
    }

    let content = fs::read_to_string(&env_path)
        .with_context(|| format!("failed to read {}", env_path.display()))?;
    parse_env_content(&content)
}

pub fn show(repo_root: &Path) -> Result<String> {
    let output = ShowOutput {
        config: load_config(repo_root)?,
        env: load_env(repo_root)?,
    };
    toml::to_string_pretty(&output).context("failed to serialize configuration")
}

pub fn init_env(repo_root: &Path) -> Result<bool> {
    let example_path = repo_root.join(ENV_EXAMPLE_PATH);
    let local_path = repo_root.join(ENV_LOCAL_PATH);
    if local_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&example_path)
        .with_context(|| format!("failed to read {}", example_path.display()))?;
    fs::write(&local_path, content)
        .with_context(|| format!("failed to write {}", local_path.display()))?;
    Ok(true)
}

pub fn verify_release(repo_root: &Path) -> Result<()> {
    let config = load_config(repo_root)?;
    validate_config(repo_root, &config)
}

pub fn sync(repo_root: &Path) -> Result<()> {
    let config = load_config(repo_root)?;
    validate_config(repo_root, &config)?;

    let cargo_path = repo_root.join(ROOT_CARGO_TOML_PATH);
    let cargo_content = fs::read_to_string(&cargo_path)
        .with_context(|| format!("failed to read {}", cargo_path.display()))?;
    let updated_cargo = sync_cargo_toml(&cargo_content, &config.versions.rust_workspace)?;
    if updated_cargo != cargo_content {
        fs::write(&cargo_path, updated_cargo)
            .with_context(|| format!("failed to write {}", cargo_path.display()))?;
    }

    let csproj_path = repo_root.join(&config.ci.package_project);
    let csproj_content = fs::read_to_string(&csproj_path)
        .with_context(|| format!("failed to read {}", csproj_path.display()))?;
    let updated_csproj = sync_csproj(&csproj_content, &config)?;
    if updated_csproj != csproj_content {
        fs::write(&csproj_path, updated_csproj)
            .with_context(|| format!("failed to write {}", csproj_path.display()))?;
    }

    Ok(())
}

pub fn set_version(repo_root: &Path, channel: VersionChannel, version: &str) -> Result<()> {
    let parsed = Version::parse(version).with_context(|| format!("invalid semver: {version}"))?;
    let mut config = load_config(repo_root)?;
    match channel {
        VersionChannel::Rust => config.versions.rust_workspace = parsed.to_string(),
        VersionChannel::NuGet => config.versions.nuget_package = parsed.to_string(),
    }
    write_config(repo_root, &config)
}

pub fn bump_version(
    repo_root: &Path,
    channel: VersionChannel,
    part: VersionPart,
) -> Result<String> {
    let mut config = load_config(repo_root)?;
    let current = match channel {
        VersionChannel::Rust => &config.versions.rust_workspace,
        VersionChannel::NuGet => &config.versions.nuget_package,
    };
    let mut version =
        Version::parse(current).with_context(|| format!("invalid configured semver: {current}"))?;
    match part {
        VersionPart::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
        VersionPart::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        VersionPart::Patch => {
            version.patch += 1;
        }
    }
    let next = version.to_string();
    match channel {
        VersionChannel::Rust => config.versions.rust_workspace = next.clone(),
        VersionChannel::NuGet => config.versions.nuget_package = next.clone(),
    }
    write_config(repo_root, &config)?;
    Ok(next)
}

pub fn sync_cargo_toml(content: &str, version: &str) -> Result<String> {
    Version::parse(version)
        .with_context(|| format!("invalid rust workspace version: {version}"))?;
    let mut document = content
        .parse::<DocumentMut>()
        .context("failed to parse Cargo.toml")?;
    document["workspace"]["package"]["version"] = value(version);
    Ok(document.to_string())
}

pub fn sync_csproj(content: &str, config: &RepoConfig) -> Result<String> {
    let mut project =
        Element::parse(content.as_bytes()).context("failed to parse Rheo.Storage.csproj")?;
    let property_group = get_or_add_property_group(&mut project);

    set_or_add_property(property_group, "PackageId", &config.nuget.package_id);
    set_or_add_property(property_group, "Version", &config.versions.nuget_package);
    set_or_add_property(property_group, "Description", &config.nuget.description);
    set_or_add_property(
        property_group,
        "PackageReadmeFile",
        file_name(&config.nuget.readme)?,
    );
    set_or_add_property(
        property_group,
        "RepositoryUrl",
        &config.nuget.repository_url,
    );
    set_or_add_property(
        property_group,
        "PackageProjectUrl",
        &config.nuget.project_url,
    );
    set_or_add_property(property_group, "Authors", &config.nuget.authors.join(";"));
    set_or_add_property(property_group, "PackageTags", &config.nuget.tags.join(";"));

    if let Some(icon) = &config.nuget.icon {
        set_or_add_property(property_group, "PackageIcon", file_name(icon)?);
    }

    let readme_file = file_name(&config.nuget.readme)?.to_owned();
    normalize_pack_none_item(
        &mut project,
        &[config.nuget.readme.as_str(), readme_file.as_str()],
        &readme_file,
        "\\",
    );
    if let Some(icon) = &config.nuget.icon {
        let icon_file = file_name(icon)?.to_owned();
        normalize_pack_none_item(
            &mut project,
            &[icon.as_str(), icon_file.as_str()],
            &icon_file,
            "\\",
        );
    }
    prune_empty_item_groups(&mut project);

    let mut output = Vec::new();
    project
        .write_with_config(
            &mut output,
            xmltree::EmitterConfig::new()
                .perform_indent(true)
                .write_document_declaration(false),
        )
        .context("failed to render Rheo.Storage.csproj")?;
    String::from_utf8(output).context("generated csproj was not valid utf-8")
}

pub fn parse_env_content(content: &str) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (line_number, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = line.split_once('=').with_context(|| {
            format!(
                "invalid env entry on line {}: expected KEY=VALUE",
                line_number + 1
            )
        })?;
        values.insert(key.trim().to_owned(), value.trim().to_owned());
    }
    Ok(values)
}

pub fn validate_config(repo_root: &Path, config: &RepoConfig) -> Result<()> {
    Version::parse(&config.versions.rust_workspace).with_context(|| {
        format!(
            "invalid rust workspace version: {}",
            config.versions.rust_workspace
        )
    })?;
    Version::parse(&config.versions.nuget_package).with_context(|| {
        format!(
            "invalid nuget package version: {}",
            config.versions.nuget_package
        )
    })?;

    if config.nuget.package_id.trim().is_empty() {
        bail!("nuget.package_id must not be empty");
    }
    if config.nuget.authors.is_empty() {
        bail!("nuget.authors must not be empty");
    }
    if config.nuget.tags.is_empty() {
        bail!("nuget.tags must not be empty");
    }

    require_exists(repo_root, CONFIG_PATH)?;
    require_exists(repo_root, ROOT_CARGO_TOML_PATH)?;
    require_exists(repo_root, &config.ci.package_project)?;
    require_exists(repo_root, &config.ci.tests_project)?;
    require_exists(repo_root, &config.ci.smoke_project)?;
    require_exists(repo_root, &config.nuget.readme)?;
    if let Some(icon) = &config.nuget.icon {
        require_exists(repo_root, icon)?;
    }
    require_exists(repo_root, ENV_EXAMPLE_PATH)?;

    Ok(())
}

fn file_name(path: &str) -> Result<&str> {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .with_context(|| format!("path must end with a file name: {path}"))
}

fn write_config(repo_root: &Path, config: &RepoConfig) -> Result<()> {
    validate_config(repo_root, config)?;
    let content = toml::to_string_pretty(config).context("failed to serialize config")?;
    let config_path = repo_root.join(CONFIG_PATH);
    fs::write(&config_path, content)
        .with_context(|| format!("failed to write {}", config_path.display()))
}

fn require_exists(repo_root: &Path, relative_path: &str) -> Result<PathBuf> {
    let path = repo_root.join(relative_path);
    if !path.exists() {
        bail!("required path does not exist: {}", path.display());
    }
    Ok(path)
}

fn get_or_add_property_group(project: &mut Element) -> &mut Element {
    let index = project
        .children
        .iter()
        .position(
            |child| matches!(child, XMLNode::Element(element) if element.name == "PropertyGroup"),
        )
        .unwrap_or_else(|| {
            project
                .children
                .insert(0, XMLNode::Element(Element::new("PropertyGroup")));
            0
        });

    match project.children.get_mut(index) {
        Some(XMLNode::Element(element)) => element,
        _ => unreachable!("property group index always points to an element"),
    }
}

fn set_or_add_property(group: &mut Element, name: &str, value_text: &str) {
    if let Some(element) = group.children.iter_mut().find_map(|child| match child {
        XMLNode::Element(element) if element.name == name => Some(element),
        _ => None,
    }) {
        element.children.clear();
        element.children.push(XMLNode::Text(value_text.to_owned()));
        return;
    }

    let mut element = Element::new(name);
    element.children.push(XMLNode::Text(value_text.to_owned()));
    group.children.push(XMLNode::Element(element));
}

fn normalize_pack_none_item(
    project: &mut Element,
    aliases: &[&str],
    include: &str,
    package_path: &str,
) {
    for child in &mut project.children {
        let XMLNode::Element(group) = child else {
            continue;
        };
        if group.name != "ItemGroup" {
            continue;
        }

        group.children.retain(|item| {
            !matches!(
                item,
                XMLNode::Element(entry)
                    if entry.name == "None"
                        && entry
                            .attributes
                            .get("Include")
                            .is_some_and(|candidate| aliases.contains(&candidate.as_str()))
            )
        });
    }

    let mut none = Element::new("None");
    none.attributes
        .insert("Include".to_owned(), include.to_owned());
    none.attributes.insert("Pack".to_owned(), "true".to_owned());
    none.attributes
        .insert("PackagePath".to_owned(), package_path.to_owned());

    let mut group = Element::new("ItemGroup");
    group.children.push(XMLNode::Element(none));
    project.children.push(XMLNode::Element(group));
}

fn prune_empty_item_groups(project: &mut Element) {
    project.children.retain(|child| {
        !matches!(
            child,
            XMLNode::Element(group) if group.name == "ItemGroup" && group.children.is_empty()
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
                description: "High-level .NET bindings for the native Rheo Storage Rust runtime."
                    .to_owned(),
                tags: vec!["storage".to_owned(), "ffi".to_owned(), "rust".to_owned()],
                readme: "bindings/dotnet/Rheo.Storage/README.md".to_owned(),
                icon: Some("bindings/dotnet/Rheo.Storage/icon-small.png".to_owned()),
                repository_url: "https://github.com/D-Naveenz/rheo_storage".to_owned(),
                project_url: "https://github.com/D-Naveenz/rheo_storage".to_owned(),
            },
            ci: CiConfig {
                smoke_project:
                    "bindings/dotnet/Rheo.Storage.ConsumerSmoke/Rheo.Storage.ConsumerSmoke.csproj"
                        .to_owned(),
                package_project: "bindings/dotnet/Rheo.Storage/Rheo.Storage.csproj".to_owned(),
                tests_project: "bindings/dotnet/Rheo.Storage.Tests/Rheo.Storage.Tests.csproj"
                    .to_owned(),
                environment: "nuget-production".to_owned(),
                runtime: "win-x64".to_owned(),
            },
        }
    }

    #[test]
    fn parses_env_content() {
        let parsed = parse_env_content(
            r#"
NUGET_API_KEY=
# comment
NUGET_SOURCE=https://api.nuget.org/v3/index.json
"#,
        )
        .expect("env parsing should succeed");

        assert_eq!(parsed.get("NUGET_API_KEY"), Some(&String::new()));
        assert_eq!(
            parsed.get("NUGET_SOURCE"),
            Some(&"https://api.nuget.org/v3/index.json".to_owned())
        );
    }

    #[test]
    fn syncs_cargo_workspace_version() {
        let input = r#"[workspace]
members = ["foo"]

[workspace.package]
version = "0.1.0"
"#;
        let output = sync_cargo_toml(input, "0.2.0").expect("cargo sync should succeed");
        assert!(output.contains("version = \"0.2.0\""));
    }

    #[test]
    fn syncs_csproj_metadata() {
        let input = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net10.0</TargetFramework>
  </PropertyGroup>
</Project>"#;
        let output = sync_csproj(input, &sample_config()).expect("csproj sync should succeed");
        assert!(output.contains("<PackageId>Rheo.Storage</PackageId>"));
        assert!(output.contains("<Version>2.0.0</Version>"));
        assert!(output.contains("<PackageIcon>icon-small.png</PackageIcon>"));
        assert!(output.contains("Include=\"README.md\""));
        assert!(output.contains("Include=\"icon-small.png\""));
    }

    #[test]
    fn bumps_versions_by_channel() {
        let temp = tempdir().expect("tempdir should succeed");
        let repo_root = temp.path();
        fs::write(
            repo_root.join(CONFIG_PATH),
            toml::to_string_pretty(&sample_config()).expect("config should serialize"),
        )
        .expect("config write should succeed");
        fs::write(repo_root.join(ENV_EXAMPLE_PATH), "NUGET_API_KEY=\n").expect("env write");
        fs::write(
            repo_root.join(ROOT_CARGO_TOML_PATH),
            "[workspace.package]\nversion = \"0.2.0\"\n",
        )
        .expect("cargo write");
        fs::create_dir_all(repo_root.join("bindings/dotnet/Rheo.Storage")).expect("dirs");
        fs::create_dir_all(repo_root.join("bindings/dotnet/Rheo.Storage.Tests")).expect("dirs");
        fs::create_dir_all(repo_root.join("bindings/dotnet/Rheo.Storage.ConsumerSmoke"))
            .expect("dirs");
        fs::write(
            repo_root.join("bindings/dotnet/Rheo.Storage/Rheo.Storage.csproj"),
            "<Project />",
        )
        .expect("csproj write");
        fs::write(
            repo_root.join("bindings/dotnet/Rheo.Storage.Tests/Rheo.Storage.Tests.csproj"),
            "<Project />",
        )
        .expect("tests write");
        fs::write(
            repo_root.join(
                "bindings/dotnet/Rheo.Storage.ConsumerSmoke/Rheo.Storage.ConsumerSmoke.csproj",
            ),
            "<Project />",
        )
        .expect("smoke write");
        fs::write(
            repo_root.join("bindings/dotnet/Rheo.Storage/README.md"),
            "# readme",
        )
        .expect("readme write");
        fs::write(
            repo_root.join("bindings/dotnet/Rheo.Storage/icon-small.png"),
            "png",
        )
        .expect("icon write");

        let bumped = bump_version(repo_root, VersionChannel::NuGet, VersionPart::Minor)
            .expect("version bump should succeed");
        assert_eq!(bumped, "2.1.0");
    }

    #[test]
    fn validate_config_requires_existing_paths() {
        let temp = tempdir().expect("tempdir should succeed");
        let repo_root = temp.path();
        let error =
            validate_config(repo_root, &sample_config()).expect_err("validation should fail");
        assert!(error.to_string().contains("required path does not exist"));
    }
}
