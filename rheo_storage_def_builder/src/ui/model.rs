use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use crate::BuilderPaths;
use crate::builder::{TridBuildProgress, TridBuildStage};
use crate::runner::{BuilderAction, CommandReport, ReportStatus};

pub(crate) const MENU_ITEMS: [MenuAction; 8] = [
    MenuAction::Pack,
    MenuAction::BuildTridXml,
    MenuAction::Inspect,
    MenuAction::InspectTridXml,
    MenuAction::Normalize,
    MenuAction::Verify,
    MenuAction::ViewLogs,
    MenuAction::Exit,
];

pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const LOGO: &[&str] = &[
    "██████╗ ██╗  ██╗███████╗ ██████╗ ",
    "██╔══██╗██║  ██║██╔════╝██╔═══██╗",
    "██████╔╝███████║█████╗  ██║   ██║",
    "██╔══██╗██╔══██║██╔══╝  ██║   ██║",
    "██║  ██║██║  ██║███████╗╚██████╔╝",
    "╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝ ╚═════╝ ",
];

#[derive(Debug)]
pub(crate) enum Screen {
    Menu,
    Form(FormState),
    Running(RunningScreen),
    Result(ResultScreen),
}

impl Screen {
    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::Menu => "Main Menu",
            Self::Form(_) => "Configure Command",
            Self::Running(_) => "Running Command",
            Self::Result(_) => "Results",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Pack,
    BuildTridXml,
    Inspect,
    InspectTridXml,
    Normalize,
    Verify,
    ViewLogs,
    Exit,
}

impl MenuAction {
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Pack => "Pack bundled runtime package",
            Self::BuildTridXml => "Build reduced package from TrID XML",
            Self::Inspect => "Inspect package",
            Self::InspectTridXml => "Inspect TrID XML source",
            Self::Normalize => "Normalize package",
            Self::Verify => "Verify package equality",
            Self::ViewLogs => "View logs",
            Self::Exit => "Exit",
        }
    }

    pub(crate) fn subtitle(self, logs_visible: bool) -> &'static str {
        match self {
            Self::Pack => "Write bundled runtime package to output",
            Self::BuildTridXml => "Use package/ as default source and output/filedefs.rpkg",
            Self::Inspect => "Inspect the current package output",
            Self::InspectTridXml => "Preview reductions without writing output",
            Self::Normalize => "Decode and re-encode an existing package",
            Self::Verify => "Compare two packages semantically",
            Self::ViewLogs if logs_visible => "Hide the live log pane",
            Self::ViewLogs => "Show the live log pane",
            Self::Exit => "Leave the Rheo shell",
        }
    }

    pub(crate) fn form_title(self) -> &'static str {
        match self {
            Self::Pack => "Pack bundled runtime package",
            Self::BuildTridXml => "Build reduced package",
            Self::Inspect => "Inspect package",
            Self::InspectTridXml => "Inspect TrID XML source",
            Self::Normalize => "Normalize package",
            Self::Verify => "Verify package equality",
            Self::ViewLogs => "View logs",
            Self::Exit => "Exit",
        }
    }
}

#[derive(Debug)]
pub(crate) struct FormState {
    pub(crate) action: MenuAction,
    pub(crate) fields: Vec<FormField>,
    pub(crate) selected: usize,
    pub(crate) editing: bool,
}

impl FormState {
    pub(crate) fn new(action: MenuAction, paths: &BuilderPaths) -> Self {
        let fields = match action {
            MenuAction::Pack => vec![FormField::new(
                "Output",
                paths.default_package_output_path().display().to_string(),
                true,
                "Destination package file",
            )],
            MenuAction::BuildTridXml => vec![
                FormField::new(
                    "Input",
                    paths.default_trid_input_path().display().to_string(),
                    true,
                    "Source .7z, XML, or directory",
                ),
                FormField::new(
                    "Output",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "Destination package file",
                ),
            ],
            MenuAction::Inspect => vec![FormField::new(
                "Input",
                paths.default_package_output_path().display().to_string(),
                true,
                "Package file to inspect",
            )],
            MenuAction::InspectTridXml => vec![FormField::new(
                "Input",
                paths.default_trid_input_path().display().to_string(),
                true,
                "Source .7z, XML, or directory",
            )],
            MenuAction::Normalize => vec![
                FormField::new(
                    "Input",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "Package file to normalize",
                ),
                FormField::new(
                    "Output",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "Destination package file",
                ),
            ],
            MenuAction::Verify => vec![
                FormField::new(
                    "Left",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "First package file",
                ),
                FormField::new("Right", String::new(), true, "Second package file"),
            ],
            MenuAction::ViewLogs | MenuAction::Exit => Vec::new(),
        };

        Self {
            action,
            fields,
            selected: 0,
            editing: false,
        }
    }

    pub(crate) fn to_action(&self) -> Result<BuilderAction, String> {
        for field in &self.fields {
            if field.required && field.value.trim().is_empty() {
                return Err(format!("{} is required.", field.label));
            }
        }

        Ok(match self.action {
            MenuAction::Pack => BuilderAction::Pack {
                output: PathBuf::from(self.fields[0].value.trim()),
            },
            MenuAction::BuildTridXml => BuilderAction::BuildTridXml {
                input: PathBuf::from(self.fields[0].value.trim()),
                output: PathBuf::from(self.fields[1].value.trim()),
            },
            MenuAction::Inspect => BuilderAction::Inspect {
                input: PathBuf::from(self.fields[0].value.trim()),
            },
            MenuAction::InspectTridXml => BuilderAction::InspectTridXml {
                input: PathBuf::from(self.fields[0].value.trim()),
            },
            MenuAction::Normalize => BuilderAction::Normalize {
                input: PathBuf::from(self.fields[0].value.trim()),
                output: PathBuf::from(self.fields[1].value.trim()),
            },
            MenuAction::Verify => BuilderAction::Verify {
                left: PathBuf::from(self.fields[0].value.trim()),
                right: PathBuf::from(self.fields[1].value.trim()),
            },
            MenuAction::ViewLogs | MenuAction::Exit => {
                return Err("This menu item does not launch a command.".to_string());
            }
        })
    }
}

#[derive(Debug)]
pub(crate) struct FormField {
    pub(crate) label: &'static str,
    pub(crate) value: String,
    pub(crate) required: bool,
    pub(crate) help: &'static str,
}

impl FormField {
    fn new(label: &'static str, value: String, required: bool, help: &'static str) -> Self {
        Self {
            label,
            value,
            required,
            help,
        }
    }
}

#[derive(Debug)]
pub(crate) struct RunningScreen {
    pub(crate) title: String,
    pub(crate) progress: Option<TridBuildProgress>,
    pub(crate) receiver: Receiver<RunnerEvent>,
}

#[derive(Debug)]
pub(crate) enum RunnerEvent {
    Progress(TridBuildProgress),
    Done(Result<CommandReport, String>),
}

#[derive(Debug)]
pub(crate) struct ResultScreen {
    pub(crate) title: String,
    pub(crate) status: ReportStatus,
    pub(crate) lines: Vec<String>,
}

impl ResultScreen {
    pub(crate) fn from_report(report: CommandReport) -> Self {
        let mut lines = Vec::with_capacity(report.fields.len() + 2);
        lines.push(String::new());
        for field in &report.fields {
            lines.push(format!("{:<20} {}", field.label, field.value));
        }
        lines.push(String::new());
        lines.push("Press Enter to return to the main menu.".to_string());
        Self {
            title: report.title,
            status: report.status,
            lines,
        }
    }

    pub(crate) fn from_error(error: String) -> Self {
        Self {
            title: "Command Failed".to_string(),
            status: ReportStatus::Warning,
            lines: vec![
                String::new(),
                error,
                String::new(),
                "Press Enter to return to the main menu.".to_string(),
            ],
        }
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }
}

pub(crate) fn stage_name(stage: TridBuildStage) -> &'static str {
    match stage {
        TridBuildStage::LoadSource => "Loading source",
        TridBuildStage::ExtractArchive => "Extracting archive",
        TridBuildStage::ParseDefinitions => "Parsing definitions",
        TridBuildStage::ReduceDefinitions => "Reducing definitions",
        TridBuildStage::FinalizePackage => "Finalizing package",
    }
}
