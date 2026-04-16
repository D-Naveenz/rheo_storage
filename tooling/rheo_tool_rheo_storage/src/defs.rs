use std::path::{Path, PathBuf};

use anyhow::Result;
use rheo_tool_core::{CommandResult, ReportField, StructuredReport, ToolContext};

use crate::{BuilderAction, LoggingOptions, execute_action, init_logging};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefsPaths {
    pub package_dir: PathBuf,
    pub output_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl DefsPaths {
    pub fn from_context(context: &ToolContext) -> Self {
        Self::from_repo_root(
            &context.repo_root,
            context.package_dir.clone(),
            context.output_dir.clone(),
            context.logs_dir.clone(),
        )
    }

    pub fn from_repo_root(
        repo_root: &Path,
        package_dir: Option<PathBuf>,
        output_dir: Option<PathBuf>,
        logs_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            package_dir: package_dir.unwrap_or_else(|| {
                repo_root
                    .join("tooling")
                    .join("rheo_tool_rheo_storage")
                    .join("package")
            }),
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

pub fn execute(command: DefsCommand, context: &ToolContext) -> Result<CommandResult> {
    let paths = DefsPaths::from_context(context);
    let logging = init_logging(LoggingOptions {
        silent: context.silent,
        verbose: context.verbose,
        logs_dir: paths.logs_dir.clone(),
        interactive: false,
    })?;
    let action = resolve_action(command, &paths);
    let report = execute_action(action, &logging.log_path, |_| {})
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(CommandResult {
        exit_code: report.exit_code(),
        report: Some(StructuredReport {
            title: report.title().to_owned(),
            fields: report
                .fields()
                .iter()
                .map(|field| ReportField {
                    label: field.label().to_owned(),
                    value: field.value().to_owned(),
                })
                .collect(),
        }),
        message: None,
    })
}

pub fn print_defs_help() -> String {
    [
        "Defs commands:",
        "  defs pack [--output <path>]",
        "  defs build-trid-xml [--input <path>] [--output <path>]",
        "  defs inspect [--input <path>]",
        "  defs inspect-trid-xml [--input <path>]",
        "  defs normalize [--input <path>] [--output <path>]",
        "  defs verify --left <path> --right <path>",
        "  defs sync-embedded [--input <path>] [--output <path>] [--check]",
    ]
    .join("\n")
}

pub fn default_embedded_sync_paths(repo_root: &Path) -> (PathBuf, PathBuf) {
    (
        repo_root
            .join("tooling")
            .join("rheo_tool_rheo_storage")
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
                .and_then(|parent| parent.parent())
                .unwrap_or_else(|| Path::new("."));
            let (default_input, default_output) = default_embedded_sync_paths(repo_root);
            BuilderAction::SyncEmbedded {
                input: input.unwrap_or(default_input),
                output: output.unwrap_or(default_output),
                check,
            }
        }
    }
}
