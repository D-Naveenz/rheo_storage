use std::sync::Arc;

use rheo_tool_rheo_storage::{
    CommandResult as RepoCommandResult, ReportField as RepoReportField, RheoStorageCapability,
    StructuredReport as RepoStructuredReport, ToolContext as RepoToolContext,
};

use crate::command::{
    CommandRegistry, CommandResult, CommandSpec, ReportField, SectionSpec, StructuredReport,
    ToolContext,
};

pub fn register_rheo_storage_capability(registry: &mut CommandRegistry) {
    let capability = RheoStorageCapability;

    for section in capability.sections() {
        registry.add_section(SectionSpec {
            name: section.name,
            prompt: section.prompt,
            summary: section.summary,
        });
    }

    for command in capability.commands() {
        let handler = command.handler;
        registry.add_command(CommandSpec {
            id: command.id,
            path: command.path,
            summary: command.summary,
            args_summary: command.args_summary,
            section: command.section,
            handler: Arc::new(move |context, args| {
                let repo_context = convert_context(context);
                let result = handler(&repo_context, args)?;
                Ok(convert_result(result))
            }),
        });
    }
}

fn convert_context(context: &ToolContext) -> RepoToolContext {
    RepoToolContext {
        repo_root: context.repo_root.clone(),
        silent: context.silent,
        verbose: context.verbose,
        package_dir: context.package_dir.clone(),
        output_dir: context.output_dir.clone(),
        logs_dir: context.logs_dir.clone(),
    }
}

fn convert_result(result: RepoCommandResult) -> CommandResult {
    CommandResult {
        exit_code: result.exit_code,
        message: result.message,
        report: result.report.map(convert_report),
    }
}

fn convert_report(report: RepoStructuredReport) -> StructuredReport {
    StructuredReport {
        title: report.title,
        fields: report.fields.into_iter().map(convert_field).collect(),
    }
}

fn convert_field(field: RepoReportField) -> ReportField {
    ReportField {
        label: field.label,
        value: field.value,
    }
}

#[cfg(test)]
mod tests {
    use super::register_rheo_storage_capability;
    use crate::command::CommandRegistry;

    #[test]
    fn registration_adds_expected_sections_and_commands() {
        let mut registry = CommandRegistry::new();
        register_rheo_storage_capability(&mut registry);

        let sections = registry
            .sections()
            .map(|section| section.name)
            .collect::<Vec<_>>();
        assert_eq!(
            sections,
            vec!["config", "defs", "package", "release", "verify", "version"]
        );

        let commands = registry
            .commands()
            .map(|command| command.id)
            .collect::<Vec<_>>();
        assert!(commands.contains(&"config.show"));
        assert!(commands.contains(&"defs.inspect-trid-xml"));
        assert!(commands.contains(&"verify.package"));
        assert!(commands.contains(&"release.publish"));
    }
}
