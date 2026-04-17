use std::io::{self, IsTerminal, Write};

use anyhow::{Context, Result};

use crate::command::{CommandRegistry, CommandResult, ToolContext};

pub fn can_launch() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn run_shell(registry: &CommandRegistry, context: &ToolContext) -> Result<()> {
    println!("Rheo shell");
    println!("Type 'help' for commands, 'use <section>' to change section, and 'exit' to quit.");

    let mut section = "root".to_owned();
    loop {
        print!("{}", prompt_for(registry, &section));
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
            println!("{}", registry.help_text());
            continue;
        }
        if line == "back" {
            section = "root".to_owned();
            continue;
        }
        if let Some(target) = line.strip_prefix("use ").map(str::trim) {
            if target == "root" || registry.sections().any(|section| section.name == target) {
                section = target.to_owned();
                continue;
            }
            eprintln!("unknown section '{target}'");
            continue;
        }

        let mut args = shlex::split(line).context("failed to parse shell command")?;
        if section != "root" {
            args.insert(0, section.clone());
        }

        match registry.execute(context, &args) {
            Ok(result) => print_result(context, result),
            Err(error) => eprintln!("{error:#}"),
        }
    }
    Ok(())
}

fn print_result(context: &ToolContext, result: CommandResult) {
    result.print(context.silent);
    if result.exit_code != 0 {
        println!("Command exited with status {}.", result.exit_code);
    }
}

fn prompt_for(registry: &CommandRegistry, current_section: &str) -> String {
    if current_section == "root" {
        return "rheo> ".to_owned();
    }

    registry
        .sections()
        .find(|section| section.name == current_section)
        .map(|section| section.prompt.to_owned())
        .unwrap_or_else(|| format!("rheo:{current_section}> "))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;

    use crate::command::{CommandRegistry, CommandSpec, SectionSpec, ToolContext};

    use super::prompt_for;

    fn noop(_: &ToolContext, _: &[String]) -> Result<crate::command::CommandResult> {
        Ok(crate::command::CommandResult::success())
    }

    #[test]
    fn prompt_uses_section_prompt_when_registered() {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "verify",
            prompt: "rheo:verify> ",
            summary: "Verification",
        });
        registry.add_command(CommandSpec {
            id: "verify.ci",
            path: &["verify", "ci"],
            summary: "Run CI checks",
            args_summary: "",
            section: "verify",
            handler: Arc::new(noop),
        });

        assert_eq!(prompt_for(&registry, "verify"), "rheo:verify> ");
    }

    #[test]
    fn prompt_falls_back_for_unknown_sections() {
        let registry = CommandRegistry::new();
        assert_eq!(prompt_for(&registry, "defs"), "rheo:defs> ");
    }

    #[test]
    fn prompt_for_root_uses_root_prompt() {
        let registry = CommandRegistry::new();
        assert_eq!(prompt_for(&registry, "root"), "rheo> ");
    }
}
