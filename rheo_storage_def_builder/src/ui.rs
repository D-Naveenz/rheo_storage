use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use crate::BuilderPaths;
use crate::builder::{TridBuildProgress, TridBuildStage};
use crate::runner::{BuilderAction, CommandReport, ReportStatus, execute_action};

const MENU_ITEMS: [MenuAction; 8] = [
    MenuAction::Pack,
    MenuAction::BuildTridXml,
    MenuAction::Inspect,
    MenuAction::InspectTridXml,
    MenuAction::Normalize,
    MenuAction::Verify,
    MenuAction::ViewLogs,
    MenuAction::Exit,
];
const TICK_RATE: Duration = Duration::from_millis(200);
const LOGO: &[&str] = &[
    "██████╗ ██╗  ██╗███████╗ ██████╗ ",
    "██╔══██╗██║  ██║██╔════╝██╔═══██╗",
    "██████╔╝███████║█████╗  ██║   ██║",
    "██╔══██╗██╔══██║██╔══╝  ██║   ██║",
    "██║  ██║██║  ██║███████╗╚██████╔╝",
    "╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝ ╚═════╝ ",
];

pub(crate) fn run_shell(paths: BuilderPaths, log_path: PathBuf) -> io::Result<()> {
    let mut app = ShellApp::new(paths, log_path);
    ratatui::run(|terminal| app.run(terminal))
}

#[derive(Debug)]
struct ShellApp {
    paths: BuilderPaths,
    log_path: PathBuf,
    screen: Screen,
    menu_index: usize,
    show_logs: bool,
    log_lines: Vec<String>,
    tick: usize,
    should_exit: bool,
}

impl ShellApp {
    fn new(paths: BuilderPaths, log_path: PathBuf) -> Self {
        Self {
            paths,
            log_path,
            screen: Screen::Menu,
            menu_index: 0,
            show_logs: false,
            log_lines: Vec::new(),
            tick: 0,
            should_exit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(TICK_RATE)?
                && let Event::Key(key) = event::read()?
            {
                self.handle_key(key);
            }
            self.tick();
        }

        Ok(())
    }

    fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if self.show_logs {
            self.refresh_logs();
        }

        if let Screen::Running(state) = &mut self.screen {
            while let Ok(event) = state.receiver.try_recv() {
                match event {
                    RunnerEvent::Progress(progress) => state.progress = Some(progress),
                    RunnerEvent::Done(result) => {
                        self.screen = match result {
                            Ok(report) => Screen::Result(ResultScreen::from_report(report)),
                            Err(error) => Screen::Result(ResultScreen::from_error(error)),
                        };
                        break;
                    }
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if is_exit_key(key) {
            self.should_exit = true;
            return;
        }

        match &mut self.screen {
            Screen::Menu => self.handle_menu_key(key),
            Screen::Form(_) => self.handle_form_key(key),
            Screen::Running(_) => {
                if matches!(key.code, KeyCode::Char('l') | KeyCode::Char('L')) {
                    self.show_logs = !self.show_logs;
                }
            }
            Screen::Result(_) => self.handle_result_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.menu_index = self.menu_index.saturating_sub(1);
            }
            KeyCode::Down => {
                self.menu_index = (self.menu_index + 1).min(MENU_ITEMS.len().saturating_sub(1));
            }
            KeyCode::Enter => self.activate_menu_item(),
            KeyCode::Esc => self.should_exit = true,
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.show_logs = !self.show_logs;
            }
            _ => {}
        }
    }

    fn handle_form_key(&mut self, key: KeyEvent) {
        let Some(form) = (match &mut self.screen {
            Screen::Form(form) => Some(form),
            _ => None,
        }) else {
            return;
        };

        if form.editing {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => form.editing = false,
                KeyCode::Backspace => {
                    form.fields[form.selected].value.pop();
                }
                KeyCode::Char(character)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    form.fields[form.selected].value.push(character);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Up => {
                form.selected = form.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                form.selected = (form.selected + 1).min(form.fields.len().saturating_sub(1));
            }
            KeyCode::Enter => form.editing = true,
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5) => match form.to_action() {
                Ok(action) => self.start_action(action),
                Err(error) => {
                    self.screen = Screen::Result(ResultScreen::from_error(error));
                }
            },
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.show_logs = !self.show_logs;
            }
            KeyCode::Esc => self.screen = Screen::Menu,
            _ => {}
        }
    }

    fn handle_result_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => self.screen = Screen::Menu,
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.show_logs = !self.show_logs;
            }
            _ => {}
        }
    }

    fn activate_menu_item(&mut self) {
        match MENU_ITEMS[self.menu_index] {
            MenuAction::ViewLogs => {
                self.show_logs = !self.show_logs;
            }
            MenuAction::Exit => self.should_exit = true,
            action => {
                self.screen = Screen::Form(FormState::new(action, &self.paths));
            }
        }
    }

    fn start_action(&mut self, action: BuilderAction) {
        let log_path = self.log_path.clone();
        let action_title = action.title().to_string();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = execute_action(action, &log_path, |progress| {
                let _ = sender.send(RunnerEvent::Progress(progress));
            })
            .map_err(|error| error.to_string());
            let _ = sender.send(RunnerEvent::Done(result));
        });

        self.screen = Screen::Running(RunningScreen {
            title: action_title,
            progress: None,
            receiver,
        });
    }

    fn refresh_logs(&mut self) {
        self.log_lines = tail_log_lines(&self.log_path, 14);
    }

    fn draw(&self, frame: &mut Frame<'_>) {
        let root = frame.area();
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(3),
                Constraint::Min(12),
                Constraint::Length(2),
            ])
            .split(root);

        self.render_logo(frame, vertical[0]);
        self.render_status(frame, vertical[1]);

        let main_chunks = if self.show_logs {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
                .split(vertical[2])
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(vertical[2])
        };

        self.render_main(frame, main_chunks[0]);
        if self.show_logs {
            self.render_logs(frame, main_chunks[1]);
        }
        self.render_help(frame, vertical[3]);
    }

    fn render_logo(&self, frame: &mut Frame<'_>, area: Rect) {
        let text = LOGO
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    *line,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            })
            .collect::<Vec<_>>();

        let block = Block::default()
            .title("Rheo Definitions Builder")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        frame.render_widget(
            Paragraph::new(Text::from(text))
                .block(block)
                .alignment(Alignment::Center),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame<'_>, area: Rect) {
        let lines = vec![
            Line::from(vec![
                Span::styled("Package ", Style::default().fg(Color::Yellow)),
                Span::raw(self.paths.package_dir.display().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Output ", Style::default().fg(Color::Yellow)),
                Span::raw(
                    self.paths
                        .default_package_output_path()
                        .display()
                        .to_string(),
                ),
                Span::raw("   "),
                Span::styled("Log ", Style::default().fg(Color::Yellow)),
                Span::raw(self.log_path.display().to_string()),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title(format!("Mode: {}", self.screen.mode_name()))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn render_main(&self, frame: &mut Frame<'_>, area: Rect) {
        match &self.screen {
            Screen::Menu => self.render_menu(frame, area),
            Screen::Form(form) => self.render_form(frame, area, form),
            Screen::Running(state) => self.render_running(frame, area, state),
            Screen::Result(state) => self.render_result(frame, area, state),
        }
    }

    fn render_menu(&self, frame: &mut Frame<'_>, area: Rect) {
        let items = MENU_ITEMS
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let style = if index == self.menu_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>2}. ", index + 1),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(item.title(), style),
                    Span::raw("  "),
                    Span::styled(
                        item.subtitle(self.show_logs),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            List::new(items).block(
                Block::default()
                    .title("Main Menu")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            ),
            area,
        );
    }

    fn render_form(&self, frame: &mut Frame<'_>, area: Rect, form: &FormState) {
        let lines = form
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let prefix = if index == form.selected {
                    if form.editing { "✎" } else { "▶" }
                } else {
                    " "
                };
                let value_style = if index == form.selected {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(vec![
                    Span::styled(
                        format!("{prefix} {:<14}", field.label),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(field.value.as_str(), value_style),
                    Span::raw("  "),
                    Span::styled(field.help, Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title(form.action.form_title())
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Magenta)),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_running(&self, frame: &mut Frame<'_>, area: Rect, state: &RunningScreen) {
        let block = Block::default()
            .title(state.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));
        frame.render_widget(block.clone(), area);
        let inner = area.inner(Margin {
            vertical: 2,
            horizontal: 2,
        });

        if let Some(progress) = &state.progress {
            match progress.total {
                Some(total) if total > 0 => {
                    let ratio = progress.current as f64 / total as f64;
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2),
                            Constraint::Length(3),
                            Constraint::Min(1),
                        ])
                        .split(inner);
                    frame.render_widget(
                        Paragraph::new(progress.message.as_str()).alignment(Alignment::Center),
                        chunks[0],
                    );
                    frame.render_widget(
                        Gauge::default()
                            .gauge_style(
                                Style::default()
                                    .fg(Color::Cyan)
                                    .bg(Color::Black)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .label(format!("{}/{}", progress.current, total))
                            .ratio(ratio),
                        chunks[1],
                    );
                    frame.render_widget(
                        Paragraph::new(format!("Stage: {}", stage_name(progress.stage)))
                            .alignment(Alignment::Center),
                        chunks[2],
                    );
                }
                _ => {
                    let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠧", "⠇"];
                    let glyph = spinner[self.tick % spinner.len()];
                    frame.render_widget(
                        Paragraph::new(format!("{glyph} {}", progress.message))
                            .alignment(Alignment::Center),
                        inner,
                    );
                }
            }
        } else {
            frame.render_widget(
                Paragraph::new("Preparing command...")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow)),
                inner,
            );
        }
    }

    fn render_result(&self, frame: &mut Frame<'_>, area: Rect, state: &ResultScreen) {
        frame.render_widget(Clear, area);
        let popup = centered_rect(78, 68, area);
        let border_color = match state.status {
            ReportStatus::Success => Color::Green,
            ReportStatus::Warning => Color::Yellow,
        };
        let block = Block::default()
            .title(state.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let lines = state
            .lines()
            .into_iter()
            .map(Line::from)
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true }),
            inner,
        );
    }

    fn render_logs(&self, frame: &mut Frame<'_>, area: Rect) {
        let lines = if self.log_lines.is_empty() {
            vec![Line::from("No log entries yet.")]
        } else {
            self.log_lines
                .iter()
                .map(|line| Line::from(line.as_str()))
                .collect::<Vec<_>>()
        };

        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .title("Logs")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_help(&self, frame: &mut Frame<'_>, area: Rect) {
        let help = match &self.screen {
            Screen::Menu => "Up/Down select  Enter open  L toggle logs  Esc exit  Ctrl+C exit",
            Screen::Form(form) if form.editing => {
                "Type to edit  Backspace delete  Enter stop editing  Esc stop editing"
            }
            Screen::Form(_) => {
                "Up/Down select field  Enter edit  R/F5 run  L toggle logs  Esc back"
            }
            Screen::Running(_) => "L toggle logs  Ctrl+C exit",
            Screen::Result(_) => "Enter return to menu  L toggle logs  Ctrl+C exit",
        };

        frame.render_widget(
            Paragraph::new(help)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }
}

#[derive(Debug)]
enum Screen {
    Menu,
    Form(FormState),
    Running(RunningScreen),
    Result(ResultScreen),
}

impl Screen {
    fn mode_name(&self) -> &'static str {
        match self {
            Self::Menu => "Shell",
            Self::Form(_) => "Configure",
            Self::Running(_) => "Working",
            Self::Result(_) => "Results",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    Pack,
    BuildTridXml,
    Inspect,
    InspectTridXml,
    Normalize,
    Verify,
    ViewLogs,
    Exit,
}

impl MenuAction {
    fn title(self) -> &'static str {
        match self {
            Self::Pack => "Pack bundled runtime package",
            Self::BuildTridXml => "Build reduced package from TrID XML",
            Self::Inspect => "Inspect package",
            Self::InspectTridXml => "Inspect TrID XML source",
            Self::Normalize => "Normalize package",
            Self::Verify => "Verify package equality",
            Self::ViewLogs => "View logs",
            Self::Exit => "Exit",
        }
    }

    fn subtitle(self, logs_visible: bool) -> &'static str {
        match self {
            Self::Pack => "Write bundled runtime package to output",
            Self::BuildTridXml => "Use package/ as default source and output/filedefs.rpkg",
            Self::Inspect => "Inspect the current package output",
            Self::InspectTridXml => "Preview reductions without writing output",
            Self::Normalize => "Decode and re-encode an existing package",
            Self::Verify => "Compare two packages semantically",
            Self::ViewLogs if logs_visible => "Hide the live log pane",
            Self::ViewLogs => "Show the live log pane",
            Self::Exit => "Leave the Rheo shell",
        }
    }

    fn form_title(self) -> &'static str {
        match self {
            Self::Pack => "Pack bundled runtime package",
            Self::BuildTridXml => "Build reduced package",
            Self::Inspect => "Inspect package",
            Self::InspectTridXml => "Inspect TrID XML source",
            Self::Normalize => "Normalize package",
            Self::Verify => "Verify package equality",
            Self::ViewLogs => "View logs",
            Self::Exit => "Exit",
        }
    }
}

#[derive(Debug)]
struct FormState {
    action: MenuAction,
    fields: Vec<FormField>,
    selected: usize,
    editing: bool,
}

impl FormState {
    fn new(action: MenuAction, paths: &BuilderPaths) -> Self {
        let fields = match action {
            MenuAction::Pack => vec![FormField::new(
                "Output",
                paths.default_package_output_path().display().to_string(),
                true,
                "Destination package file",
            )],
            MenuAction::BuildTridXml => vec![
                FormField::new(
                    "Input",
                    paths.default_trid_input_path().display().to_string(),
                    true,
                    "Source .7z, XML, or directory",
                ),
                FormField::new(
                    "Output",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "Destination package file",
                ),
            ],
            MenuAction::Inspect => vec![FormField::new(
                "Input",
                paths.default_package_output_path().display().to_string(),
                true,
                "Package file to inspect",
            )],
            MenuAction::InspectTridXml => vec![FormField::new(
                "Input",
                paths.default_trid_input_path().display().to_string(),
                true,
                "Source .7z, XML, or directory",
            )],
            MenuAction::Normalize => vec![
                FormField::new(
                    "Input",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "Package file to normalize",
                ),
                FormField::new(
                    "Output",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "Destination package file",
                ),
            ],
            MenuAction::Verify => vec![
                FormField::new(
                    "Left",
                    paths.default_package_output_path().display().to_string(),
                    true,
                    "First package file",
                ),
                FormField::new("Right", String::new(), true, "Second package file"),
            ],
            MenuAction::ViewLogs | MenuAction::Exit => Vec::new(),
        };

        Self {
            action,
            fields,
            selected: 0,
            editing: false,
        }
    }

    fn to_action(&self) -> Result<BuilderAction, String> {
        for field in &self.fields {
            if field.required && field.value.trim().is_empty() {
                return Err(format!("{} is required.", field.label));
            }
        }

        Ok(match self.action {
            MenuAction::Pack => BuilderAction::Pack {
                output: PathBuf::from(self.fields[0].value.trim()),
            },
            MenuAction::BuildTridXml => BuilderAction::BuildTridXml {
                input: PathBuf::from(self.fields[0].value.trim()),
                output: PathBuf::from(self.fields[1].value.trim()),
            },
            MenuAction::Inspect => BuilderAction::Inspect {
                input: PathBuf::from(self.fields[0].value.trim()),
            },
            MenuAction::InspectTridXml => BuilderAction::InspectTridXml {
                input: PathBuf::from(self.fields[0].value.trim()),
            },
            MenuAction::Normalize => BuilderAction::Normalize {
                input: PathBuf::from(self.fields[0].value.trim()),
                output: PathBuf::from(self.fields[1].value.trim()),
            },
            MenuAction::Verify => BuilderAction::Verify {
                left: PathBuf::from(self.fields[0].value.trim()),
                right: PathBuf::from(self.fields[1].value.trim()),
            },
            MenuAction::ViewLogs | MenuAction::Exit => {
                return Err("This menu item does not launch a command.".to_string());
            }
        })
    }
}

#[derive(Debug)]
struct FormField {
    label: &'static str,
    value: String,
    required: bool,
    help: &'static str,
}

impl FormField {
    fn new(label: &'static str, value: String, required: bool, help: &'static str) -> Self {
        Self {
            label,
            value,
            required,
            help,
        }
    }
}

#[derive(Debug)]
struct RunningScreen {
    title: String,
    progress: Option<TridBuildProgress>,
    receiver: Receiver<RunnerEvent>,
}

#[derive(Debug)]
enum RunnerEvent {
    Progress(TridBuildProgress),
    Done(Result<CommandReport, String>),
}

#[derive(Debug)]
struct ResultScreen {
    title: String,
    status: ReportStatus,
    lines: Vec<String>,
}

impl ResultScreen {
    fn from_report(report: CommandReport) -> Self {
        let mut lines = Vec::with_capacity(report.fields.len() + 2);
        lines.push(String::new());
        for field in &report.fields {
            lines.push(format!("{:<20} {}", field.label, field.value));
        }
        lines.push(String::new());
        lines.push("Press Enter to return to the main menu.".to_string());
        Self {
            title: report.title,
            status: report.status,
            lines,
        }
    }

    fn from_error(error: String) -> Self {
        Self {
            title: "Command Failed".to_string(),
            status: ReportStatus::Warning,
            lines: vec![
                String::new(),
                error,
                String::new(),
                "Press Enter to return to the main menu.".to_string(),
            ],
        }
    }

    fn lines(&self) -> Vec<String> {
        self.lines.clone()
    }
}

fn stage_name(stage: TridBuildStage) -> &'static str {
    match stage {
        TridBuildStage::LoadSource => "Loading source",
        TridBuildStage::ExtractArchive => "Extracting archive",
        TridBuildStage::ParseDefinitions => "Parsing definitions",
        TridBuildStage::ReduceDefinitions => "Reducing definitions",
        TridBuildStage::FinalizePackage => "Finalizing package",
    }
}

fn centered_rect(horizontal: u16, vertical: u16, area: Rect) -> Rect {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical) / 2),
            Constraint::Percentage(vertical),
            Constraint::Percentage((100 - vertical) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal) / 2),
            Constraint::Percentage(horizontal),
            Constraint::Percentage((100 - horizontal) / 2),
        ])
        .split(vertical_chunks[1])[1]
}

fn is_exit_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn tail_log_lines(path: &Path, max_lines: usize) -> Vec<String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut lines = contents.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.len() > max_lines {
        let split = lines.len() - max_lines;
        lines.drain(..split);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{FormState, MENU_ITEMS, MenuAction, Screen, ShellApp, is_exit_key};
    use crate::BuilderPaths;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    fn sample_paths() -> BuilderPaths {
        BuilderPaths {
            package_dir: PathBuf::from("package"),
            output_dir: PathBuf::from("output"),
            logs_dir: PathBuf::from("logs"),
        }
    }

    #[test]
    fn menu_contains_all_primary_actions() {
        assert_eq!(MENU_ITEMS.len(), 8);
        assert_eq!(MENU_ITEMS[0], MenuAction::Pack);
        assert_eq!(MENU_ITEMS[1], MenuAction::BuildTridXml);
        assert_eq!(MENU_ITEMS[7], MenuAction::Exit);
    }

    #[test]
    fn build_form_uses_default_paths() {
        let form = FormState::new(MenuAction::BuildTridXml, &sample_paths());
        assert_eq!(form.fields[0].value, "package\\triddefs_xml.7z");
        assert_eq!(form.fields[1].value, "output\\filedefs.rpkg");
    }

    #[test]
    fn ctrl_c_is_always_an_exit_key() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_exit_key(key));
    }

    #[test]
    fn log_toggle_keeps_shell_on_menu() {
        let mut app = ShellApp::new(
            sample_paths(),
            PathBuf::from("logs\\2026-04-10_def_builder.log"),
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert!(app.show_logs);
        assert!(matches!(app.screen, Screen::Menu));
    }
}
