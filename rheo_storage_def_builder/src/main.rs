mod builder;
mod logging;
mod runner;
mod ui;

use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use color_eyre::eyre::{Result, eyre};

use crate::logging::{LoggingOptions, init_logging};
use crate::runner::{BuilderAction, print_report};

#[derive(Debug, Parser)]
#[command(name = "rheo_storage_def_builder")]
#[command(about = "Build, inspect, and normalize Rheo definitions packages.")]
#[command(version)]
#[command(next_line_help = true)]
#[command(
    after_help = "Interactive mode:\n  Launch without a subcommand in a real terminal to open the Rheo shell.\n  Use --silent to keep the classic non-interactive help/output behavior."
)]
struct Cli {
    /// Suppress normal command output and keep only errors.
    #[arg(short = 's', long, global = true)]
    silent: bool,

    /// Increase log verbosity. Repeat for more detail.
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    verbose: u8,

    /// Override the default package folder used for TrID XML source discovery.
    #[arg(long, global = true)]
    package_dir: Option<PathBuf>,

    /// Override the default output folder used for generated package files.
    #[arg(long, global = true)]
    output_dir: Option<PathBuf>,

    /// Override the default logs folder used for builder log files.
    #[arg(long, global = true)]
    logs_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write the bundled runtime package to an output path.
    Pack {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Build a reduced package from TrID XML definitions in a file, directory, or .7z archive.
    BuildTridXml {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print summary information about a package.
    Inspect {
        #[arg(short, long)]
        input: Option<PathBuf>,
    },
    /// Print transformation diagnostics for a TrID XML source.
    InspectTridXml {
        #[arg(short, long)]
        input: Option<PathBuf>,
    },
    /// Normalize an existing package by decoding and re-encoding it.
    Normalize {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Compare two package files for semantic equality.
    Verify {
        #[arg(long)]
        left: PathBuf,
        #[arg(long)]
        right: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuilderPaths {
    pub(crate) package_dir: PathBuf,
    pub(crate) output_dir: PathBuf,
    pub(crate) logs_dir: PathBuf,
}

impl BuilderPaths {
    fn from_cli(cli: &Cli) -> io::Result<Self> {
        let base_dir = builder_base_dir()?;
        Ok(Self {
            package_dir: cli
                .package_dir
                .clone()
                .unwrap_or_else(|| base_dir.join("package")),
            output_dir: cli
                .output_dir
                .clone()
                .unwrap_or_else(|| base_dir.join("output")),
            logs_dir: cli
                .logs_dir
                .clone()
                .unwrap_or_else(|| base_dir.join("logs")),
        })
    }

    pub(crate) fn default_trid_input_path(&self) -> PathBuf {
        resolve_default_trid_source(&self.package_dir)
    }

    pub(crate) fn default_package_output_path(&self) -> PathBuf {
        self.output_dir.join("filedefs.rpkg")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchMode {
    InteractiveShell,
    Direct,
    Help,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let exit_code = try_main()?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn try_main() -> Result<i32> {
    let cli = Cli::parse();
    let paths = BuilderPaths::from_cli(&cli)?;

    let launch_mode = determine_launch_mode(
        cli.command.as_ref(),
        cli.silent,
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
    );

    let logging = init_logging(LoggingOptions {
        silent: cli.silent,
        verbose: cli.verbose,
        logs_dir: paths.logs_dir.clone(),
        interactive: matches!(launch_mode, LaunchMode::InteractiveShell),
    })?;

    match launch_mode {
        LaunchMode::InteractiveShell => {
            ui::run_shell(paths.clone(), logging.log_path.clone())?;
            Ok(0)
        }
        LaunchMode::Direct => {
            let action = resolve_action(
                cli.command.expect("direct launch requires a command"),
                &paths,
            );
            run_direct_action(action, &logging.log_path, cli.silent)
        }
        LaunchMode::Help => {
            print_help()?;
            Ok(0)
        }
    }
}

fn determine_launch_mode(
    command: Option<&Command>,
    silent: bool,
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> LaunchMode {
    if command.is_some() {
        LaunchMode::Direct
    } else if !silent && stdin_is_terminal && stdout_is_terminal {
        LaunchMode::InteractiveShell
    } else {
        LaunchMode::Help
    }
}

fn resolve_action(command: Command, paths: &BuilderPaths) -> BuilderAction {
    match command {
        Command::Pack { output } => BuilderAction::Pack {
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        Command::BuildTridXml { input, output } => BuilderAction::BuildTridXml {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        Command::Inspect { input } => BuilderAction::Inspect {
            input: input.unwrap_or_else(|| paths.default_package_output_path()),
        },
        Command::InspectTridXml { input } => BuilderAction::InspectTridXml {
            input: input.unwrap_or_else(|| paths.default_trid_input_path()),
        },
        Command::Normalize { input, output } => BuilderAction::Normalize {
            input: input.unwrap_or_else(|| paths.default_package_output_path()),
            output: output.unwrap_or_else(|| paths.default_package_output_path()),
        },
        Command::Verify { left, right } => BuilderAction::Verify { left, right },
    }
}

fn run_direct_action(action: BuilderAction, log_path: &Path, silent: bool) -> Result<i32> {
    let report = crate::runner::execute_action(action, log_path, |_| {})
        .map_err(|error| eyre!(error.to_string()))?;
    if !silent {
        print_report(&report);
    }
    Ok(report.exit_code)
}

fn print_help() -> io::Result<()> {
    let mut cli = Cli::command();
    cli.print_help()?;
    println!();
    Ok(())
}

fn builder_base_dir() -> io::Result<PathBuf> {
    if let Some(override_dir) = std::env::var_os("RHEO_STORAGE_DEF_BUILDER_BASE_DIR") {
        return Ok(PathBuf::from(override_dir));
    }

    let exe_path = std::env::current_exe()?;
    exe_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("builder executable path has no parent directory"))
}

fn resolve_default_trid_source(package_dir: &Path) -> PathBuf {
    let preferred_archive = package_dir.join("triddefs_xml.7z");
    if preferred_archive.exists() {
        return preferred_archive;
    }

    let preferred_directory = package_dir.join("triddefs_xml");
    if preferred_directory.exists() {
        return preferred_directory;
    }

    if let Some(single_archive) = first_matching_file(package_dir, "7z") {
        return single_archive;
    }

    if let Some(single_xml) = first_matching_file(package_dir, "xml") {
        return single_xml;
    }

    package_dir.to_path_buf()
}

fn first_matching_file(directory: &Path, extension: &str) -> Option<PathBuf> {
    let mut matches = fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let is_match = path
                .extension()
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(extension));
            is_match.then_some(path)
        })
        .collect::<Vec<_>>();
    matches.sort();
    matches.into_iter().next()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use clap::Parser;
    use tempfile::tempdir;

    use super::{
        BuilderPaths, Cli, Command, LaunchMode, determine_launch_mode, resolve_default_trid_source,
    };

    #[test]
    fn default_source_prefers_trid_archive_when_present() {
        let temp = tempdir().unwrap();
        let package_dir = temp.path().join("package");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(package_dir.join("triddefs_xml.7z"), b"archive").unwrap();
        fs::write(package_dir.join("other.xml"), b"<xml />").unwrap();

        assert_eq!(
            resolve_default_trid_source(&package_dir),
            package_dir.join("triddefs_xml.7z")
        );
    }

    #[test]
    fn default_paths_use_base_dir_siblings() {
        let temp = tempdir().unwrap();
        unsafe {
            std::env::set_var("RHEO_STORAGE_DEF_BUILDER_BASE_DIR", temp.path());
        }
        let cli = Cli::parse_from(["builder"]);
        let paths = BuilderPaths::from_cli(&cli).unwrap();
        unsafe {
            std::env::remove_var("RHEO_STORAGE_DEF_BUILDER_BASE_DIR");
        }

        assert_eq!(paths.package_dir, temp.path().join("package"));
        assert_eq!(paths.output_dir, temp.path().join("output"));
        assert_eq!(paths.logs_dir, temp.path().join("logs"));
    }

    #[test]
    fn no_subcommand_with_tty_launches_shell() {
        let mode = determine_launch_mode(None, false, true, true);
        assert_eq!(mode, LaunchMode::InteractiveShell);
    }

    #[test]
    fn silent_without_subcommand_falls_back_to_help() {
        let mode = determine_launch_mode(None, true, true, true);
        assert_eq!(mode, LaunchMode::Help);
    }

    #[test]
    fn explicit_command_stays_direct() {
        let command = Command::Inspect { input: None };
        let mode = determine_launch_mode(Some(&command), false, true, true);
        assert_eq!(mode, LaunchMode::Direct);
    }
}
