//! Tests for key dispatch (handoff §7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn q_and_esc_map_to_back() {
    // Back is resolved by the app: close an overlay, pop a screen, or quit at the root.
    assert_eq!(dispatch(key(KeyCode::Char('q'))), Action::Back);
    assert_eq!(dispatch(key(KeyCode::Esc)), Action::Back);
}

#[test]
fn ctrl_c_always_quits() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(dispatch(ctrl_c), Action::Quit);
}

#[test]
fn navigation_keys_are_bound() {
    assert_eq!(dispatch(key(KeyCode::Enter)), Action::Select);
    assert_eq!(dispatch(key(KeyCode::Up)), Action::Up);
    assert_eq!(dispatch(key(KeyCode::Char('k'))), Action::Up);
    assert_eq!(dispatch(key(KeyCode::Down)), Action::Down);
    assert_eq!(dispatch(key(KeyCode::Char('j'))), Action::Down);
    assert_eq!(dispatch(key(KeyCode::Char('b'))), Action::OpenRefPicker);
}

#[test]
fn help_and_refresh_are_bound() {
    assert_eq!(dispatch(key(KeyCode::Char('?'))), Action::ToggleHelp);
    assert_eq!(dispatch(key(KeyCode::Char('r'))), Action::Refresh);
}

#[test]
fn unbound_key_is_none() {
    assert_eq!(dispatch(key(KeyCode::Char('z'))), Action::None);
}
