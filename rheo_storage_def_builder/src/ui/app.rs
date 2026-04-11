use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::BuilderPaths;
use crate::runner::{BuilderAction, execute_action};

use super::input::{is_actionable_key, is_exit_key};
use super::model::{
    FormState, MENU_ITEMS, MenuAction, ResultScreen, RunnerEvent, RunningScreen, Screen,
};

pub(super) const TICK_RATE: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub(crate) struct ShellApp {
    pub(super) paths: BuilderPaths,
    pub(super) log_path: PathBuf,
    pub(super) screen: Screen,
    pub(super) menu_index: usize,
    pub(super) show_logs: bool,
    pub(super) log_lines: Vec<String>,
    pub(super) tick: usize,
    pub(super) should_exit: bool,
}

impl ShellApp {
    pub(crate) fn new(paths: BuilderPaths, log_path: PathBuf) -> Self {
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

    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(TICK_RATE)?
                && let Event::Key(key) = event::read()?
                && is_actionable_key(key)
            {
                self.handle_key(key);
            }
            self.tick();
        }

        Ok(())
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
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
