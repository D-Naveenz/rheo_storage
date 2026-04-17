use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::capabilities::rheo_storage::register_rheo_storage_capability;
use crate::command::{CommandRegistry, ToolContext};
use crate::shell::{can_launch, run_shell};

pub fn run() -> Result<()> {
    let cli = parse_root_args(env::args().skip(1).collect())?;
    let requested_repo_root = cli.repo_root.clone();
    let repo_root = normalize_repo_root(cli.repo_root).with_context(|| {
        format!(
            "failed to canonicalize repo root '{}'",
            requested_repo_root.display()
        )
    })?;

    let mut registry = CommandRegistry::new();
    register_rheo_storage_capability(&mut registry);

    if cli.show_version {
        println!("{}", crate::version());
        return Ok(());
    }

    if cli.show_help {
        print!("{}", help_text(&registry));
        return Ok(());
    }

    let context = ToolContext {
        repo_root,
        silent: cli.silent,
        verbose: cli.verbose,
        package_dir: cli.package_dir,
        output_dir: cli.output_dir,
        logs_dir: cli.logs_dir,
    };

    if cli.command.is_empty() {
        if can_launch() {
            run_shell(&registry, &context)?;
            return Ok(());
        }

        print!("{}", help_text(&registry));
        return Ok(());
    }

    let result = registry.execute(&context, &cli.command)?;
    result.print(context.silent);
    if result.exit_code != 0 {
        std::process::exit(result.exit_code);
    }
    Ok(())
}

fn normalize_repo_root(path: PathBuf) -> Result<PathBuf> {
    let canonical = path.canonicalize()?;

    #[cfg(windows)]
    {
        const VERBATIM_PREFIX: &str = r"\\?\";
        let canonical_text = canonical.to_string_lossy();
        if let Some(stripped) = canonical_text.strip_prefix(VERBATIM_PREFIX) {
            return Ok(PathBuf::from(stripped));
        }
    }

    Ok(canonical)
}

#[derive(Debug, Clone)]
struct RootArgs {
    repo_root: PathBuf,
    silent: bool,
    verbose: u8,
    package_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    logs_dir: Option<PathBuf>,
    show_help: bool,
    show_version: bool,
    command: Vec<String>,
}

fn parse_root_args(args: Vec<String>) -> Result<RootArgs> {
    let mut parsed = RootArgs {
        repo_root: PathBuf::from("."),
        silent: false,
        verbose: 0,
        package_dir: None,
        output_dir: None,
        logs_dir: None,
        show_help: false,
        show_version: false,
        command: Vec::new(),
    };

    let mut index = 0;
    while index < args.len() {
        let token = &args[index];
        match token.as_str() {
            "-h" | "--help" => {
                parsed.show_help = true;
                index += 1;
            }
            "--version" => {
                parsed.show_version = true;
                index += 1;
            }
            "-s" | "--silent" => {
                parsed.silent = true;
                index += 1;
            }
            "-v" | "--verbose" => {
                parsed.verbose += 1;
                index += 1;
            }
            "--repo-root" => {
                parsed.repo_root = PathBuf::from(next_value(&args, index, "--repo-root")?);
                index += 2;
            }
            "--package-dir" => {
                parsed.package_dir =
                    Some(PathBuf::from(next_value(&args, index, "--package-dir")?));
                index += 2;
            }
            "--output-dir" => {
                parsed.output_dir = Some(PathBuf::from(next_value(&args, index, "--output-dir")?));
                index += 2;
            }
            "--logs-dir" => {
                parsed.logs_dir = Some(PathBuf::from(next_value(&args, index, "--logs-dir")?));
                index += 2;
            }
            _ if token.starts_with("--repo-root=") => {
                parsed.repo_root = PathBuf::from(token.trim_start_matches("--repo-root="));
                index += 1;
            }
            _ if token.starts_with("--package-dir=") => {
                parsed.package_dir =
                    Some(PathBuf::from(token.trim_start_matches("--package-dir=")));
                index += 1;
            }
            _ if token.starts_with("--output-dir=") => {
                parsed.output_dir = Some(PathBuf::from(token.trim_start_matches("--output-dir=")));
                index += 1;
            }
            _ if token.starts_with("--logs-dir=") => {
                parsed.logs_dir = Some(PathBuf::from(token.trim_start_matches("--logs-dir=")));
                index += 1;
            }
            _ if token.starts_with('-') => bail!("unknown global option: {token}"),
            _ => {
                parsed.command.extend(args[index..].iter().cloned());
                break;
            }
        }
    }

    Ok(parsed)
}

fn next_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str> {
    args.get(index + 1)
        .map(String::as_str)
        .with_context(|| format!("{option} requires a value"))
}

fn help_text(registry: &CommandRegistry) -> String {
    format!(
        "Usage: rheo_tool [global-options] <command>\n\nGlobal options:\n  --repo-root <path>\n  --package-dir <path>\n  --output-dir <path>\n  --logs-dir <path>\n  -s, --silent\n  -v, --verbose\n  -h, --help\n  --version\n\n{}",
        registry.help_text()
    )
}
