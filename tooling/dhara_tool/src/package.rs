use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use zip::ZipArchive;

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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::tempdir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{inspect_package_entries, write_nuget_config};

    #[test]
    fn writes_expected_nuget_config() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("NuGet.config");
        write_nuget_config(&config_path, &[temp.path().join("local")]).unwrap();
        let content = std::fs::read_to_string(config_path).unwrap();
        assert!(content.contains("<clear />"));
        assert!(content.contains("source0"));
    }

    #[test]
    fn reads_zip_entries() {
        let temp = tempdir().unwrap();
        let package_path = temp.path().join("sample.nupkg");
        let file = File::create(&package_path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file(
                "lib/net10.0/Dhara.Storage.dll",
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"x").unwrap();
        writer.finish().unwrap();

        let entries = inspect_package_entries(&package_path).unwrap();
        assert_eq!(entries, vec!["lib/net10.0/Dhara.Storage.dll".to_owned()]);
    }
}
