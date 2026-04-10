mod logging;

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use crossterm::event::{Event, read};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use rheo_storage_def_builder::{
    TridBuildProgress, TridBuildStage, TridTransformReport, build_trid_xml_package_with_progress,
    inspect_package, load_bundled_package, normalize_package, packages_match, write_package,
};
use tracing::{error, info};

use crate::logging::{LoggingOptions, init_logging};

#[derive(Debug, Parser)]
#[command(name = "rheo_storage_def_builder")]
#[command(about = "Build, inspect, and normalize Rheo definitions packages.")]
#[command(version)]
#[command(next_line_help = true)]
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

#[derive(Debug, Clone)]
struct Output {
    silent: bool,
    log_path: PathBuf,
}

impl Output {
    fn new(silent: bool, log_path: PathBuf) -> Self {
        Self { silent, log_path }
    }

    fn section(&self, title: &str) {
        if self.silent {
            return;
        }
        println!("{title}");
    }

    fn field(&self, label: &str, value: impl std::fmt::Display) {
        if self.silent {
            return;
        }
        println!("{label:<20} {value}");
    }

    fn blank_line(&self) {
        if self.silent {
            return;
        }
        println!();
    }

    fn log_location(&self) {
        self.field("Log", self.log_path.display());
    }
}

struct BuildUi {
    multi: MultiProgress,
    stage: ProgressBar,
    detail: ProgressBar,
    silent: bool,
}

impl BuildUi {
    fn new(silent: bool) -> Self {
        let multi = MultiProgress::new();
        if silent {
            multi.set_draw_target(ProgressDrawTarget::hidden());
        }

        let stage = multi.add(ProgressBar::new_spinner());
        stage.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}").expect("valid spinner template"),
        );
        stage.enable_steady_tick(std::time::Duration::from_millis(100));

        let detail = multi.add(ProgressBar::new(0));
        detail.set_style(
            ProgressStyle::with_template("{bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .expect("valid progress template")
                .progress_chars("##-"),
        );
        detail.set_message("Waiting");

        Self {
            multi,
            stage,
            detail,
            silent,
        }
    }

    fn update(&self, progress: TridBuildProgress) {
        if self.silent {
            return;
        }

        self.stage.set_message(stage_title(progress.stage));
        match progress.total {
            Some(total) => {
                self.detail.set_length(total as u64);
                self.detail.set_position(progress.current as u64);
                self.detail.set_message(progress.message);
            }
            None => {
                self.detail.set_length(0);
                self.detail.set_position(0);
                self.detail.set_message(progress.message);
            }
        }
    }

    fn finish(&self) {
        if self.silent {
            return;
        }
        self.stage.finish_and_clear();
        self.detail.finish_and_clear();
        let _ = self.multi.clear();
    }
}

fn stage_title(stage: TridBuildStage) -> &'static str {
    match stage {
        TridBuildStage::LoadSource => "Loading source",
        TridBuildStage::ExtractArchive => "Extracting archive",
        TridBuildStage::ParseDefinitions => "Parsing definitions",
        TridBuildStage::ReduceDefinitions => "Reducing definitions",
        TridBuildStage::FinalizePackage => "Finalizing package",
    }
}

fn main() {
    let cli = Cli::parse();
    let paths = BuilderPaths::from_cli(&cli).unwrap_or_else(|error| {
        eprintln!("failed to resolve builder paths: {error}");
        std::process::exit(1);
    });
    let logging = init_logging(LoggingOptions {
        silent: cli.silent,
        verbose: cli.verbose,
        logs_dir: paths.logs_dir.clone(),
    })
    .unwrap_or_else(|error| {
        eprintln!("failed to initialize logging: {error}");
        std::process::exit(1);
    });

    let output = Output::new(cli.silent, logging.log_path.clone());
    let result = run(cli.command, &paths, output);
    if let Err(error) = result {
        error!(error = %error, "builder command failed");
        pause_before_exit(cli.silent);
        std::process::exit(1);
    }
    pause_before_exit(cli.silent);
}

fn run(
    command: Option<Command>,
    paths: &BuilderPaths,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(command) = command else {
        if !output.silent {
            let mut cli = Cli::command();
            cli.print_help()?;
            println!();
        }
        return Ok(());
    };

    match command {
        Command::Pack { output: path } => {
            let path = path.unwrap_or_else(|| paths.default_package_output_path());
            info!(output = %path.display(), "packing bundled runtime definitions");
            let package = load_bundled_package()?;
            let written = write_package(&package, path)?;

            output.section("Bundled Package");
            output.field("Output", written.display());
            output.log_location();
        }
        Command::BuildTridXml {
            input,
            output: path,
        } => {
            let input = input.unwrap_or_else(|| paths.default_trid_input_path());
            let path = path.unwrap_or_else(|| paths.default_package_output_path());
            info!(input = %input.display(), output = %path.display(), "building reduced TrID XML package");
            let ui = BuildUi::new(output.silent);
            let build =
                build_trid_xml_package_with_progress(&input, |progress| ui.update(progress))?;
            ui.finish();
            let written = write_package(&build.package, path)?;

            output.section("Build Complete");
            output.field("Input", input.display());
            output.field("Output", written.display());
            output.log_location();
            output.blank_line();
            print_transform_report(&output, &build.report);
        }
        Command::Inspect { input } => {
            let input = input.unwrap_or_else(|| paths.default_package_output_path());
            info!(input = %input.display(), "inspecting definitions package");
            let summary = inspect_package(input)?;

            output.section("Package Summary");
            output.field("Package Version", &summary.package_version);
            output.field("Tags", summary.tags);
            output.field("Definitions", summary.definition_count);
            output.field("Log", output.log_path.display());
        }
        Command::InspectTridXml { input } => {
            let input = input.unwrap_or_else(|| paths.default_trid_input_path());
            info!(input = %input.display(), "inspecting TrID XML source");
            let ui = BuildUi::new(output.silent);
            let report = {
                let build =
                    build_trid_xml_package_with_progress(&input, |progress| ui.update(progress))?;
                build.report
            };
            ui.finish();

            output.section("Transformation Preview");
            print_transform_report(&output, &report);
            output.log_location();
        }
        Command::Normalize {
            input,
            output: path,
        } => {
            let input = input.unwrap_or_else(|| paths.default_package_output_path());
            let path = path.unwrap_or_else(|| paths.default_package_output_path());
            info!(input = %input.display(), output = %path.display(), "normalizing definitions package");
            let written = normalize_package(input, path)?;

            output.section("Normalized Package");
            output.field("Output", written.display());
            output.log_location();
        }
        Command::Verify { left, right } => {
            info!(left = %left.display(), right = %right.display(), "verifying package equality");
            let matches = packages_match(left, right)?;

            output.section("Verification");
            output.field("Result", if matches { "match" } else { "different" });
            output.log_location();
            if !matches {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn print_transform_report(output: &Output, report: &TridTransformReport) {
    output.field("Total Parsed", report.total_parsed);
    output.field("MIME Corrected", report.mime_corrected);
    output.field("MIME Rejected", report.mime_rejected);
    output.field("Extension Rejected", report.extension_rejected);
    output.field("Signature Rejected", report.signature_rejected);
    output.field("Final Trimmed", report.final_trimmed);
    output.field("Final Kept", report.final_kept);
}

fn pause_before_exit(silent: bool) {
    if silent || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return;
    }

    let _ = write!(io::stdout(), "\nPress any key to continue . . .");
    let _ = io::stdout().flush();

    if enable_raw_mode().is_ok() {
        let _ = wait_for_keypress();
        let _ = disable_raw_mode();
    }

    let _ = writeln!(io::stdout());
}

fn wait_for_keypress() -> io::Result<()> {
    loop {
        match read()? {
            Event::Key(_) => return Ok(()),
            _ => continue,
        }
    }
}

#[derive(Debug, Clone)]
struct BuilderPaths {
    package_dir: PathBuf,
    output_dir: PathBuf,
    logs_dir: PathBuf,
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

    fn default_trid_input_path(&self) -> PathBuf {
        resolve_default_trid_source(&self.package_dir)
    }

    fn default_package_output_path(&self) -> PathBuf {
        self.output_dir.join("filedefs.rpkg")
    }
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

    use super::{BuilderPaths, resolve_default_trid_source};

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
        let cli = super::Cli::parse_from(["builder"]);
        let paths = BuilderPaths::from_cli(&cli).unwrap();
        unsafe {
            std::env::remove_var("RHEO_STORAGE_DEF_BUILDER_BASE_DIR");
        }

        assert_eq!(paths.package_dir, temp.path().join("package"));
        assert_eq!(paths.output_dir, temp.path().join("output"));
        assert_eq!(paths.logs_dir, temp.path().join("logs"));
    }
}
