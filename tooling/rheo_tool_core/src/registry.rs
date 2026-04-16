use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::{CommandHandler, CommandResult, ToolContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpec {
    pub name: &'static str,
    pub prompt: &'static str,
    pub summary: &'static str,
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub id: &'static str,
    pub path: &'static [&'static str],
    pub summary: &'static str,
    pub args_summary: &'static str,
    pub section: &'static str,
    pub handler: CommandHandler,
}

pub trait ToolCapability {
    fn register(&self, registry: &mut CommandRegistry);
}

#[derive(Default)]
pub struct CommandRegistry {
    sections: BTreeMap<&'static str, SectionSpec>,
    commands: Vec<CommandSpec>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_section(&mut self, section: SectionSpec) {
        self.sections.insert(section.name, section);
    }

    pub fn add_command(&mut self, command: CommandSpec) {
        self.commands.push(command);
    }

    pub fn sections(&self) -> impl Iterator<Item = &SectionSpec> {
        self.sections.values()
    }

    pub fn commands(&self) -> impl Iterator<Item = &CommandSpec> {
        self.commands.iter()
    }

    pub fn commands_for_section<'a>(
        &'a self,
        section: &'static str,
    ) -> impl Iterator<Item = &'a CommandSpec> + 'a {
        self.commands
            .iter()
            .filter(move |command| command.section == section)
    }

    pub fn resolve<'a>(&'a self, args: &'a [String]) -> Option<(&'a CommandSpec, &'a [String])> {
        self.commands
            .iter()
            .filter(|command| args.len() >= command.path.len())
            .filter(|command| {
                command
                    .path
                    .iter()
                    .zip(args.iter())
                    .all(|(expected, actual)| expected == actual)
            })
            .max_by_key(|command| command.path.len())
            .map(|command| (command, &args[command.path.len()..]))
    }

    pub fn execute(&self, context: &ToolContext, args: &[String]) -> Result<CommandResult> {
        let Some((command, rest)) = self.resolve(args) else {
            bail!("unknown command path: {}", args.join(" "));
        };
        (command.handler)(context, rest)
    }

    pub fn help_text(&self) -> String {
        let mut output = String::from("Rheo tool commands:\n");
        for section in self.sections.values() {
            output.push_str(&format!("\n{}:\n", section.name));
            for command in self.commands_for_section(section.name) {
                let path = command.path.join(" ");
                if command.args_summary.is_empty() {
                    output.push_str(&format!("  {path:<28} {}\n", command.summary));
                } else {
                    output.push_str(&format!(
                        "  {:<28} {}\n",
                        format!("{path} {}", command.args_summary),
                        command.summary
                    ));
                }
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{StructuredReport, ToolContext};

    fn noop(_: &ToolContext, _: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::success())
    }

    fn report_handler(_: &ToolContext, args: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::with_report(StructuredReport {
            title: "dispatch".to_owned(),
            fields: vec![crate::ReportField {
                label: "args".to_owned(),
                value: args.join(" "),
            }],
        }))
    }

    fn context() -> ToolContext {
        ToolContext {
            repo_root: PathBuf::from("."),
            silent: false,
            verbose: 0,
            package_dir: None,
            output_dir: None,
            logs_dir: None,
        }
    }

    #[test]
    fn resolves_longest_matching_path() {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "config",
            prompt: "cfg> ",
            summary: "Config commands",
        });
        registry.add_command(CommandSpec {
            id: "config",
            path: &["config"],
            summary: "Config root",
            args_summary: "",
            section: "config",
            handler: noop,
        });
        registry.add_command(CommandSpec {
            id: "config.show",
            path: &["config", "show"],
            summary: "Show config",
            args_summary: "",
            section: "config",
            handler: noop,
        });

        let args = vec!["config".to_owned(), "show".to_owned(), "--x".to_owned()];
        let (command, rest) = registry.resolve(&args).expect("command should resolve");
        assert_eq!(command.id, "config.show");
        assert_eq!(rest, &["--x".to_owned()]);
    }

    #[test]
    fn execute_dispatches_to_registered_handler() {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "verify",
            prompt: "verify> ",
            summary: "Verification commands",
        });
        registry.add_command(CommandSpec {
            id: "verify.package",
            path: &["verify", "package"],
            summary: "Verify package",
            args_summary: "[--configuration <name>]",
            section: "verify",
            handler: report_handler,
        });

        let result = registry
            .execute(
                &context(),
                &[
                    "verify".to_owned(),
                    "package".to_owned(),
                    "--configuration".to_owned(),
                    "Release".to_owned(),
                ],
            )
            .expect("command should execute");

        assert_eq!(result.exit_code, 0);
        let report = result.report.expect("report should be returned");
        assert_eq!(report.title, "dispatch");
        assert_eq!(report.fields[0].value, "--configuration Release");
    }

    #[test]
    fn help_text_groups_commands_by_section() {
        let mut registry = CommandRegistry::new();
        registry.add_section(SectionSpec {
            name: "config",
            prompt: "cfg> ",
            summary: "Config commands",
        });
        registry.add_command(CommandSpec {
            id: "config.show",
            path: &["config", "show"],
            summary: "Show config",
            args_summary: "",
            section: "config",
            handler: noop,
        });

        let help = registry.help_text();
        assert!(help.contains("Rheo tool commands:"));
        assert!(help.contains("config:"));
        assert!(help.contains("config show"));
        assert!(help.contains("Show config"));
    }
}
