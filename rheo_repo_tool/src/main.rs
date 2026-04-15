use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use rheo_repo_tool::{
    VersionChannel, VersionPart, bump_version, init_env, set_version, show, sync, verify_release,
};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[arg(long, default_value = ".")]
    repo_root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Show,
    Sync,
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
    Version {
        #[command(subcommand)]
        command: VersionCommand,
    },
    Verify {
        #[command(subcommand)]
        command: VerifyCommand,
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
    Release,
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

    match cli.command {
        Command::Show => {
            print!("{}", show(&repo_root)?);
        }
        Command::Sync => {
            sync(&repo_root)?;
            println!("Synchronized repository configuration.");
        }
        Command::Env {
            command: EnvCommand::Init,
        } => {
            let created = init_env(&repo_root)?;
            if created {
                println!("Created .env.local from .env.example.");
            } else {
                println!(".env.local already exists.");
            }
        }
        Command::Version { command } => match command {
            VersionCommand::Set { channel, version } => {
                set_version(&repo_root, channel.into(), &version)?;
                println!("Updated version to {version}.");
            }
            VersionCommand::Bump { channel, part } => {
                let next = bump_version(&repo_root, channel.into(), part.into())?;
                println!("{next}");
            }
        },
        Command::Verify {
            command: VerifyCommand::Release,
        } => {
            verify_release(&repo_root)?;
            println!("Release configuration is valid.");
        }
    }

    Ok(())
}
