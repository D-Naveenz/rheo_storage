use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use rheo_repo_tool::{
    VersionChannel, VersionPart, bump_version, init_env, load_config, set_version, show, sync,
};

use rheo_tool::defs::{DefsCommand, DefsPaths, execute as execute_defs, print_defs_help};
use rheo_tool::package::{PackageOptions, pack as package_pack, publish as package_publish};
use rheo_tool::shell::{can_launch, run_shell};
use rheo_tool::verify::{verify_ci, verify_docs, verify_package, verify_release_config};

#[derive(Debug, Parser)]
#[command(name = "rheo_tool")]
#[command(
    about = "Unified Rheo operator CLI for definitions, verification, packaging, and releases."
)]
#[command(version)]
#[command(next_line_help = true)]
struct Cli {
    #[arg(long, default_value = ".")]
    repo_root: PathBuf,

    #[arg(short = 's', long, global = true)]
    silent: bool,

    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    verbose: u8,

    #[arg(long, global = true)]
    package_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    output_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    logs_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Version {
        #[command(subcommand)]
        command: VersionCommand,
    },
    Verify {
        #[command(subcommand)]
        command: VerifyCommand,
    },
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    Defs {
        #[command(subcommand)]
        command: DefsSubcommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Show,
    Sync,
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EnvCommand {
    Init,
}

#[derive(Debug, Subcommand)]
enum VersionCommand {
    Set {
        #[arg(long)]
        channel: ChannelArg,
        version: String,
    },
    Bump {
        #[arg(long)]
        channel: ChannelArg,
        #[arg(long)]
        part: PartArg,
    },
}

#[derive(Debug, Subcommand)]
enum VerifyCommand {
    ReleaseConfig,
    Ci,
    Docs,
    Package {
        #[command(flatten)]
        options: PackageArgs,
    },
}

#[derive(Debug, Subcommand)]
enum PackageCommand {
    Pack {
        #[command(flatten)]
        options: PackageArgs,
    },
    Publish {
        #[command(flatten)]
        options: PublishArgs,
    },
}

#[derive(Debug, Subcommand)]
enum ReleaseCommand {
    Publish {
        #[command(flatten)]
        options: PublishArgs,
    },
}

#[derive(Debug, Subcommand)]
enum DefsSubcommand {
    Pack {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    BuildTridXml {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Inspect {
        #[arg(short, long)]
        input: Option<PathBuf>,
    },
    InspectTridXml {
        #[arg(short, long)]
        input: Option<PathBuf>,
    },
    Normalize {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Verify {
        #[arg(long)]
        left: PathBuf,
        #[arg(long)]
        right: PathBuf,
    },
    SyncEmbedded {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, clap::Args)]
struct PackageArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long)]
    version: Option<String>,
}

#[derive(Debug, clap::Args)]
struct PublishArgs {
    #[arg(long, default_value = "Release")]
    configuration: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    api_key_env: Option<String>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    execute: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ChannelArg {
    Rust,
    Nuget,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PartArg {
    Major,
    Minor,
    Patch,
}

impl From<ChannelArg> for VersionChannel {
    fn from(value: ChannelArg) -> Self {
        match value {
            ChannelArg::Rust => VersionChannel::Rust,
            ChannelArg::Nuget => VersionChannel::NuGet,
        }
    }
}

impl From<PartArg> for VersionPart {
    fn from(value: PartArg) -> Self {
        match value {
            PartArg::Major => VersionPart::Major,
            PartArg::Minor => VersionPart::Minor,
            PartArg::Patch => VersionPart::Patch,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = cli.repo_root.canonicalize()?;

    if cli.command.is_none() {
        if can_launch() {
            run_shell(
                &repo_root,
                cli.silent,
                cli.verbose,
                cli.package_dir.clone(),
                cli.output_dir.clone(),
                cli.logs_dir.clone(),
                dispatch_shell_args,
            )?;
            return Ok(());
        }

        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(());
    }

    let exit_code = execute(cli, repo_root)?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn dispatch_shell_args(
    args: Vec<String>,
    repo_root: &Path,
    silent: bool,
    verbose: u8,
    package_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    logs_dir: Option<PathBuf>,
) -> Result<i32> {
    if args.first().is_some_and(|value| value == "defs") && args.len() == 1 {
        print_defs_help();
        return Ok(0);
    }

    let mut all_args = vec![
        "rheo_tool".to_owned(),
        format!("--repo-root={}", repo_root.display()),
    ];
    if silent {
        all_args.push("--silent".to_owned());
    }
    for _ in 0..verbose {
        all_args.push("--verbose".to_owned());
    }
    if let Some(package_dir) = package_dir {
        all_args.push(format!("--package-dir={}", package_dir.display()));
    }
    if let Some(output_dir) = output_dir {
        all_args.push(format!("--output-dir={}", output_dir.display()));
    }
    if let Some(logs_dir) = logs_dir {
        all_args.push(format!("--logs-dir={}", logs_dir.display()));
    }
    all_args.extend(args);

    let cli = Cli::try_parse_from(all_args).map_err(|error| anyhow::anyhow!(error.to_string()))?;
    execute(cli, repo_root.to_path_buf())
}

fn execute(cli: Cli, repo_root: PathBuf) -> Result<i32> {
    let config = load_config(&repo_root)?;
    match cli
        .command
        .expect("command should exist after shell handling")
    {
        Command::Config { command } => match command {
            ConfigCommand::Show => {
                print!("{}", show(&repo_root)?);
                Ok(0)
            }
            ConfigCommand::Sync => {
                sync(&repo_root)?;
                println!("Synchronized repository configuration.");
                Ok(0)
            }
            ConfigCommand::Env {
                command: EnvCommand::Init,
            } => {
                let created = init_env(&repo_root)?;
                if created {
                    println!("Created .env.local from .env.example.");
                } else {
                    println!(".env.local already exists.");
                }
                Ok(0)
            }
        },
        Command::Version { command } => match command {
            VersionCommand::Set { channel, version } => {
                set_version(&repo_root, channel.into(), &version)?;
                println!("Updated version to {version}.");
                Ok(0)
            }
            VersionCommand::Bump { channel, part } => {
                let next = bump_version(&repo_root, channel.into(), part.into())?;
                println!("{next}");
                Ok(0)
            }
        },
        Command::Verify { command } => {
            match command {
                VerifyCommand::ReleaseConfig => verify_release_config(&repo_root)?,
                VerifyCommand::Ci => verify_ci(&repo_root, &config)?,
                VerifyCommand::Docs => verify_docs(&repo_root)?,
                VerifyCommand::Package { options } => verify_package(
                    &repo_root,
                    &config,
                    &package_options(options, cli.output_dir.clone(), None, false),
                )?,
            }
            Ok(0)
        }
        Command::Package { command } => match command {
            PackageCommand::Pack { options } => {
                let package = package_pack(
                    &repo_root,
                    &config,
                    &package_options(options, cli.output_dir.clone(), None, false),
                )?;
                println!("Packed {}", package.display());
                Ok(0)
            }
            PackageCommand::Publish { options } => {
                let package_options =
                    package_options_from_publish(options, cli.output_dir.clone())?;
                package_publish(&repo_root, &config, &package_options)?;
                Ok(0)
            }
        },
        Command::Release { command } => match command {
            ReleaseCommand::Publish { options } => {
                let package_options =
                    package_options_from_publish(options, cli.output_dir.clone())?;
                package_publish(&repo_root, &config, &package_options)?;
                Ok(0)
            }
        },
        Command::Defs { command } => {
            let paths = DefsPaths::from_repo_root(
                &repo_root,
                cli.package_dir.clone(),
                cli.output_dir.clone(),
                cli.logs_dir.clone(),
            );
            let defs_command = match command {
                DefsSubcommand::Pack { output } => DefsCommand::Pack { output },
                DefsSubcommand::BuildTridXml { input, output } => {
                    DefsCommand::BuildTridXml { input, output }
                }
                DefsSubcommand::Inspect { input } => DefsCommand::Inspect { input },
                DefsSubcommand::InspectTridXml { input } => DefsCommand::InspectTridXml { input },
                DefsSubcommand::Normalize { input, output } => {
                    DefsCommand::Normalize { input, output }
                }
                DefsSubcommand::Verify { left, right } => DefsCommand::Verify { left, right },
                DefsSubcommand::SyncEmbedded {
                    input,
                    output,
                    check,
                } => DefsCommand::SyncEmbedded {
                    input,
                    output,
                    check,
                },
            };
            execute_defs(defs_command, &paths, cli.silent, cli.verbose, false)
        }
    }
}

fn package_options(
    options: PackageArgs,
    output_dir: Option<PathBuf>,
    source_override: Option<String>,
    execute_publish: bool,
) -> PackageOptions {
    PackageOptions {
        configuration: options.configuration,
        version_override: options.version,
        source_override,
        api_key_env_override: None,
        output_dir,
        execute_publish,
    }
}

fn package_options_from_publish(
    options: PublishArgs,
    output_dir: Option<PathBuf>,
) -> Result<PackageOptions> {
    if options.dry_run && options.execute {
        bail!("--dry-run and --execute cannot be used together");
    }
    Ok(PackageOptions {
        configuration: options.configuration,
        version_override: options.version,
        source_override: options.source,
        api_key_env_override: options.api_key_env,
        output_dir,
        execute_publish: options.execute && !options.dry_run,
    })
}
