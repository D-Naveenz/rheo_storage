use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rheo_storage_def_builder::{
    build_trid_xml_package, inspect_package, inspect_trid_xml_source, load_bundled_package,
    normalize_package, packages_match, write_package,
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
            let package = build_trid_xml_package(&input)?;
            let path = write_package(&package, output)?;
            println!("built package from TrID XML: {}", path.display());
        }
        Command::Inspect { input } => {
            let summary = inspect_package(input)?;
            println!("package_version={}", summary.package_version);
            println!("tags={}", summary.tags);
            println!("definition_count={}", summary.definition_count);
        }
        Command::InspectTridXml { input } => {
            let summary = inspect_trid_xml_source(input)?;
            println!("package_version={}", summary.package_version);
            println!("tags={}", summary.tags);
            println!("definition_count={}", summary.definition_count);
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
