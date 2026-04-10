use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rheo_storage_def_builder::{
    build_trid_xml_package_with_report, inspect_package, inspect_trid_xml_source,
    load_bundled_package, normalize_package, packages_match, write_package,
};

#[derive(Debug, Parser)]
#[command(name = "rheo_storage_def_builder")]
#[command(about = "Inspect, build, normalize, and emit Rheo definitions packages.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write the bundled runtime package to an output path.
    Pack {
        #[arg(short, long, default_value = "Output/filedefs.rpkg")]
        output: PathBuf,
    },
    /// Build a package from TrID XML definitions in a file, directory, or .7z archive.
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
    /// Print summary information about a TrID XML source.
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Pack { output } => {
            let package = load_bundled_package()?;
            let path = write_package(&package, output)?;
            println!("wrote package: {}", path.display());
        }
        Command::BuildTridXml { input, output } => {
            let build = build_trid_xml_package_with_report(&input)?;
            let path = write_package(&build.package, output)?;
            println!("built package from TrID XML: {}", path.display());
            println!("total_parsed={}", build.report.total_parsed);
            println!("mime_corrected={}", build.report.mime_corrected);
            println!("mime_rejected={}", build.report.mime_rejected);
            println!("extension_rejected={}", build.report.extension_rejected);
            println!("signature_rejected={}", build.report.signature_rejected);
            println!("final_trimmed={}", build.report.final_trimmed);
            println!("final_kept={}", build.report.final_kept);
        }
        Command::Inspect { input } => {
            let summary = inspect_package(input)?;
            println!("package_version={}", summary.package_version);
            println!("tags={}", summary.tags);
            println!("definition_count={}", summary.definition_count);
        }
        Command::InspectTridXml { input } => {
            let report = inspect_trid_xml_source(input)?;
            println!("total_parsed={}", report.total_parsed);
            println!("mime_corrected={}", report.mime_corrected);
            println!("mime_rejected={}", report.mime_rejected);
            println!("extension_rejected={}", report.extension_rejected);
            println!("signature_rejected={}", report.signature_rejected);
            println!("final_trimmed={}", report.final_trimmed);
            println!("final_kept={}", report.final_kept);
        }
        Command::Normalize { input, output } => {
            let path = normalize_package(input, output)?;
            println!("normalized package: {}", path.display());
        }
        Command::Verify { left, right } => {
            let matches = packages_match(left, right)?;
            println!("{}", if matches { "match" } else { "different" });
            if !matches {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
