use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellSection {
    Root,
    Defs,
    Verify,
    Package,
    Release,
    Config,
    Version,
}

impl ShellSection {
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "defs" => Some(Self::Defs),
            "verify" => Some(Self::Verify),
            "package" => Some(Self::Package),
            "release" => Some(Self::Release),
            "config" => Some(Self::Config),
            "version" => Some(Self::Version),
            "root" => Some(Self::Root),
            _ => None,
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Self::Root => "rheo> ",
            Self::Defs => "rheo:defs> ",
            Self::Verify => "rheo:verify> ",
            Self::Package => "rheo:package> ",
            Self::Release => "rheo:release> ",
            Self::Config => "rheo:config> ",
            Self::Version => "rheo:version> ",
        }
    }
}

pub fn can_launch() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn run_shell<F>(
    repo_root: &Path,
    silent: bool,
    verbose: u8,
    package_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    logs_dir: Option<PathBuf>,
    mut dispatcher: F,
) -> Result<()>
where
    F: FnMut(
        Vec<String>,
        &Path,
        bool,
        u8,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ) -> Result<i32>,
{
    println!("Rheo shell");
    println!("Type 'help' for commands, 'use <section>' to change section, and 'exit' to quit.");

    let mut section = ShellSection::Root;
    loop {
        print!("{}", section.prompt());
        io::stdout().flush().context("failed to flush stdout")?;

        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .context("failed to read shell input")?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, "exit" | "quit") {
            break;
        }
        if line == "help" {
            print_shell_help();
            continue;
        }
        if line == "back" {
            section = ShellSection::Root;
            continue;
        }
        if let Some(target) = line
            .strip_prefix("use ")
            .and_then(|value| ShellSection::from_label(value.trim()))
        {
            section = target;
            continue;
        }

        let mut args = shlex::split(line).context("failed to parse shell command")?;
        if section != ShellSection::Root {
            match section {
                ShellSection::Defs => args.insert(0, "defs".to_owned()),
                ShellSection::Verify => args.insert(0, "verify".to_owned()),
                ShellSection::Package => args.insert(0, "package".to_owned()),
                ShellSection::Release => {
                    args.insert(0, "publish".to_owned());
                    args.insert(0, "release".to_owned());
                }
                ShellSection::Config => args.insert(0, "config".to_owned()),
                ShellSection::Version => args.insert(0, "version".to_owned()),
                ShellSection::Root => {}
            }
        }

        match dispatcher(
            args,
            repo_root,
            silent,
            verbose,
            package_dir.clone(),
            output_dir.clone(),
            logs_dir.clone(),
        ) {
            Ok(code) if code != 0 => println!("Command exited with status {code}."),
            Ok(_) => {}
            Err(error) => eprintln!("{error:#}"),
        }
    }

    Ok(())
}

fn print_shell_help() {
    println!("Sections: defs, verify, package, release, config, version");
    println!("Examples:");
    println!("  use verify");
    println!("  ci");
    println!("  use defs");
    println!("  inspect-trid-xml");
    println!("  use package");
    println!("  pack --version 2.0.0");
}
