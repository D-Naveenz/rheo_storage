use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap};

use crate::builder::TridBuildProgress;

use super::app::ShellApp;
use super::model::{
    APP_VERSION, FormState, LOGO, MENU_ITEMS, ResultScreen, RunningScreen, Screen, stage_name,
};

impl ShellApp {
    pub(super) fn draw(&self, frame: &mut Frame<'_>) {
        let root = frame.area();
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),
                Constraint::Length(4),
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
        let mut text = LOGO
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
        text.push(Line::default());
        text.push(Line::from(Span::styled(
            format!("Definition Builder v{APP_VERSION}"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));

        frame.render_widget(
            Paragraph::new(Text::from(text))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            area,
        );
    }

    fn render_status(&self, frame: &mut Frame<'_>, area: Rect) {
        let lines = vec![
            Line::from(vec![
                Span::styled("Screen ", Style::default().fg(Color::Yellow)),
                Span::raw(self.screen.title()),
                Span::raw("   "),
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
                        .title("Session")
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
                    let percent = (ratio * 100.0).clamp(0.0, 100.0);
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2),
                            Constraint::Length(3),
                            Constraint::Length(2),
                            Constraint::Length(5),
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
                            .label(format!("{percent:.1}%  ({}/{})", progress.current, total))
                            .ratio(ratio),
                        chunks[1],
                    );
                    frame.render_widget(
                        Paragraph::new(current_item_text(progress)).alignment(Alignment::Center),
                        chunks[2],
                    );
                    frame.render_widget(
                        Paragraph::new(progress_stats_lines(progress))
                            .alignment(Alignment::Left)
                            .wrap(Wrap { trim: true }),
                        chunks[3],
                    );
                    frame.render_widget(
                        Paragraph::new(format!("Stage: {}", stage_name(progress.stage)))
                            .alignment(Alignment::Center),
                        chunks[4],
                    );
                }
                _ => {
                    let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠧", "⠇"];
                    let glyph = spinner[self.tick % spinner.len()];
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2),
                            Constraint::Length(2),
                            Constraint::Min(1),
                        ])
                        .split(inner);
                    frame.render_widget(
                        Paragraph::new(format!("{glyph} {}", progress.message))
                            .alignment(Alignment::Center),
                        chunks[0],
                    );
                    frame.render_widget(
                        Paragraph::new(current_item_text(progress)).alignment(Alignment::Center),
                        chunks[1],
                    );
                    frame.render_widget(
                        Paragraph::new(progress_stats_lines(progress))
                            .alignment(Alignment::Left)
                            .wrap(Wrap { trim: true }),
                        chunks[2],
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
            crate::runner::ReportStatus::Success => Color::Green,
            crate::runner::ReportStatus::Warning => Color::Yellow,
        };
        let block = Block::default()
            .title(state.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let lines = state
            .lines()
            .iter()
            .cloned()
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

fn current_item_text(progress: &TridBuildProgress) -> String {
    progress
        .current_item
        .as_ref()
        .map(|item| format!("Current: {item}"))
        .unwrap_or_else(|| "Current: waiting for details".to_string())
}

fn progress_stats_lines(progress: &TridBuildProgress) -> Vec<Line<'static>> {
    let stats = &progress.stats;
    vec![
        Line::from(vec![
            stat_label("Parsed"),
            stat_value(stats.parsed_count),
            Span::raw("   "),
            stat_label("Accepted"),
            stat_value(stats.accepted_count),
            Span::raw("   "),
            stat_label("MIME fixed"),
            stat_value(stats.mime_corrected),
        ]),
        Line::from(vec![
            stat_label("MIME rejected"),
            stat_value(stats.mime_rejected),
            Span::raw("   "),
            stat_label("Ext rejected"),
            stat_value(stats.extension_rejected),
            Span::raw("   "),
            stat_label("Sig rejected"),
            stat_value(stats.signature_rejected),
        ]),
        Line::from(vec![
            stat_label("Trimmed"),
            stat_value(stats.final_trimmed),
            Span::raw("   "),
            stat_label("Processed"),
            stat_value(progress.current),
            Span::raw("   "),
            stat_label("Stage total"),
            progress
                .total
                .map(stat_value)
                .unwrap_or_else(|| Span::styled("n/a", Style::default().fg(Color::DarkGray))),
        ]),
    ]
}

fn stat_label(label: &'static str) -> Span<'static> {
    Span::styled(format!("{label} "), Style::default().fg(Color::Yellow))
}

fn stat_value(value: usize) -> Span<'static> {
    Span::styled(value.to_string(), Style::default().fg(Color::White))
}
