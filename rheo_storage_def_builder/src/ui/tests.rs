use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

use crate::BuilderPaths;

use super::app::ShellApp;
use super::input::{is_actionable_key, is_exit_key};
use super::model::{FormState, MENU_ITEMS, MenuAction, Screen};

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
fn key_release_events_are_ignored() {
    let key = KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: KeyEventState::NONE,
    };
    assert!(!is_actionable_key(key));
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
