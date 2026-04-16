use std::path::PathBuf;

use anyhow::Result;
use rheo_storage_def_builder::{
    BuilderAction, LoggingOptions, execute_action, init_logging, print_report,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefsPaths {
    pub package_dir: PathBuf,
    pub output_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl DefsPaths {
    pub fn from_repo_root(
        repo_root: &std::path::Path,
        package_dir: Option<PathBuf>,
        output_dir: Option<PathBuf>,
        logs_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            package_dir: package_dir
                .unwrap_or_else(|| repo_root.join("rheo_storage_def_builder").join("package")),
            output_dir: output_dir.unwrap_or_else(|| repo_root.join("output")),
            logs_dir: logs_dir.unwrap_or_else(|| repo_root.join("logs")),
        }
    }

    pub fn default_trid_input_path(&self) -> PathBuf {
        let preferred_archive = self.package_dir.join("triddefs_xml.7z");
        if preferred_archive.exists() {
            return preferred_archive;
        }

        let preferred_directory = self.package_dir.join("triddefs_xml");
        if preferred_directory.exists() {
            return preferred_directory;
        }

        self.package_dir.clone()
    }

    pub fn default_package_output_path(&self) -> PathBuf {
        self.output_dir.join("filedefs.rpkg")
    }
}

#[derive(Debug, Clone)]
pub enum DefsCommand {
    Pack {
        output: Option<PathBuf>,
    },
    BuildTridXml {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Inspect {
        input: Option<PathBuf>,
    },
    InspectTridXml {
        input: Option<PathBuf>,
    },
    Normalize {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Verify {
        left: PathBuf,
        right: PathBuf,
    },
    SyncEmbedded {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
        check: bool,
    },
}

pub fn execute(
    command: DefsCommand,
    paths: &DefsPaths,
    silent: bool,
    verbose: u8,
    interactive: bool,
) -> Result<i32> {
    let logging = init_logging(LoggingOptions {
        silent,
        verbose,
        logs_dir: paths.logs_dir.clone(),
        interactive,
    })?;
    let action = resolve_action(command, paths);
    let report = execute_action(action, &logging.log_path, |_| {})
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    if !silent {
        print_report(&report);
    }
    Ok(report.exit_code())
}

pub fn print_defs_help() {
    println!("Defs commands:");
    println!("  pack [--output <path>]");
    println!("  build-trid-xml [--input <path>] [--output <path>]");
    println!("  inspect [--input <path>]");
    println!("  inspect-trid-xml [--input <path>]");
    println!("  normalize [--input <path>] [--output <path>]");
    println!("  verify --left <path> --right <path>");
    println!("  sync-embedded [--input <path>] [--output <path>] [--check]");
}

pub fn default_embedded_sync_paths(repo_root: &std::path::Path) -> (PathBuf, PathBuf) {
    (
        repo_root
            .join("rheo_storage_def_builder")
            .join("package")
            .join("triddefs_xml.7z"),
        repo_root
            .join("rheo_storage")
            .join("resources")
            .join("filedefs.rpkg"),
    )
}

fn resolve_action(command: DefsCommand, paths: &DefsPaths) -> BuilderAction {
    match command {
        DefsCommand::Pack { output } => BuilderAction::Pack {
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::BuildTridXml { input, output } => BuilderAction::BuildTridXml {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::Inspect { input } => BuilderAction::Inspect {
            input: input.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::InspectTridXml { input } => BuilderAction::InspectTridXml {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
        },
        DefsCommand::Normalize { input, output } => BuilderAction::Normalize {
            input: input.unwrap_or_else(|| paths.default_package_output_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        DefsCommand::Verify { left, right } => BuilderAction::Verify { left, right },
        DefsCommand::SyncEmbedded {
            input,
            output,
            check,
        } => {
            let repo_root = paths
                .package_dir
                .parent()
                .and_then(|parent| parent.parent())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let (default_input, default_output) = default_embedded_sync_paths(&repo_root);
            BuilderAction::SyncEmbedded {
                input: input.unwrap_or(default_input),
                output: output.unwrap_or(default_output),
                check,
            }
        }
    }
}
