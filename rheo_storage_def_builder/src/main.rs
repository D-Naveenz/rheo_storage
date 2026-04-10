mod logging;

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write the bundled runtime package to an output path.
    Pack {
        #[arg(short, long, default_value = "Output/filedefs.rpkg")]
        output: PathBuf,
    },
    /// Build a reduced package from TrID XML definitions in a file, directory, or .7z archive.
    BuildTridXml {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value = "Output/filedefs.rpkg")]
        output: PathBuf,
    },
    /// Print summary information about a package.
    Inspect {
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Print transformation diagnostics for a TrID XML source.
    InspectTridXml {
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Normalize an existing package by decoding and re-encoding it.
    Normalize {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long, default_value = "Output/filedefs.rpkg")]
        output: PathBuf,
    },
    /// Compare two package files for semantic equality.
    Verify {
        #[arg(long)]
        left: PathBuf,
        #[arg(long)]
        right: PathBuf,
    },
}

#[derive(Debug, Clone, Copy)]
struct Output {
    silent: bool,
}

impl Output {
    fn new(silent: bool) -> Self {
        Self { silent }
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
    init_logging(LoggingOptions {
        silent: cli.silent,
        verbose: cli.verbose,
    });

    let output = Output::new(cli.silent);
    let result = run(cli.command, output);
    if let Err(error) = result {
        error!(error = %error, "builder command failed");
        pause_before_exit(cli.silent);
        std::process::exit(1);
    }
    pause_before_exit(cli.silent);
}

fn run(command: Option<Command>, output: Output) -> Result<(), Box<dyn std::error::Error>> {
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
            info!(output = %path.display(), "packing bundled runtime definitions");
            let package = load_bundled_package()?;
            let written = write_package(&package, path)?;

            output.section("Bundled Package");
            output.field("Output", written.display());
        }
        Command::BuildTridXml {
            input,
            output: path,
        } => {
            info!(input = %input.display(), output = %path.display(), "building reduced TrID XML package");
            let ui = BuildUi::new(output.silent);
            let build =
                build_trid_xml_package_with_progress(&input, |progress| ui.update(progress))?;
            ui.finish();
            let written = write_package(&build.package, path)?;

            output.section("Build Complete");
            output.field("Input", input.display());
            output.field("Output", written.display());
            output.blank_line();
            print_transform_report(output, &build.report);
        }
        Command::Inspect { input } => {
            info!(input = %input.display(), "inspecting definitions package");
            let summary = inspect_package(input)?;

            output.section("Package Summary");
            output.field("Package Version", &summary.package_version);
            output.field("Tags", summary.tags);
            output.field("Definitions", summary.definition_count);
        }
        Command::InspectTridXml { input } => {
            info!(input = %input.display(), "inspecting TrID XML source");
            let ui = BuildUi::new(output.silent);
            let report = {
                let build =
                    build_trid_xml_package_with_progress(&input, |progress| ui.update(progress))?;
                build.report
            };
            ui.finish();

            output.section("Transformation Preview");
            print_transform_report(output, &report);
        }
        Command::Normalize {
            input,
            output: path,
        } => {
            info!(input = %input.display(), output = %path.display(), "normalizing definitions package");
            let written = normalize_package(input, path)?;

            output.section("Normalized Package");
            output.field("Output", written.display());
        }
        Command::Verify { left, right } => {
            info!(left = %left.display(), right = %right.display(), "verifying package equality");
            let matches = packages_match(left, right)?;

            output.section("Verification");
            output.field("Result", if matches { "match" } else { "different" });
            if !matches {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn print_transform_report(output: Output, report: &TridTransformReport) {
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
