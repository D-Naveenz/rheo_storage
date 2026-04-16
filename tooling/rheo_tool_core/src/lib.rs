mod package;
mod process;
mod registry;

use std::path::PathBuf;

pub use package::{inspect_package_entries, write_nuget_config};
pub use process::{run_command, run_command_with_env};
pub use registry::{CommandRegistry, CommandSpec, SectionSpec, ToolCapability};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub repo_root: PathBuf,
    pub silent: bool,
    pub verbose: u8,
    pub package_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportField {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredReport {
    pub title: String,
    pub fields: Vec<ReportField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub exit_code: i32,
    pub report: Option<StructuredReport>,
    pub message: Option<String>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            report: None,
            message: None,
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            report: None,
            message: Some(message.into()),
        }
    }

    pub fn with_report(report: StructuredReport) -> Self {
        Self {
            exit_code: 0,
            report: Some(report),
            message: None,
        }
    }

    pub fn from_exit_code(exit_code: i32) -> Self {
        Self {
            exit_code,
            report: None,
            message: None,
        }
    }

    pub fn print(&self, silent: bool) {
        if silent {
            return;
        }

        if let Some(message) = &self.message {
            println!("{message}");
        }

        if let Some(report) = &self.report {
            println!("{}", report.title);
            for field in &report.fields {
                println!("{:<20} {}", field.label, field.value);
            }
        }
    }
}

pub type CommandHandler = fn(&ToolContext, &[String]) -> anyhow::Result<CommandResult>;
