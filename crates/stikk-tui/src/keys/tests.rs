//! Tests for key dispatch (handoff §7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn q_and_esc_quit_when_no_overlay() {
    assert_eq!(dispatch(key(KeyCode::Char('q')), false), Action::Quit);
    assert_eq!(dispatch(key(KeyCode::Esc), false), Action::Quit);
}

#[test]
fn q_and_esc_close_overlay_before_quitting() {
    assert_eq!(
        dispatch(key(KeyCode::Char('q')), true),
        Action::CloseOverlay
    );
    assert_eq!(dispatch(key(KeyCode::Esc), true), Action::CloseOverlay);
}

#[test]
fn ctrl_c_always_quits() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(dispatch(ctrl_c, false), Action::Quit);
    assert_eq!(dispatch(ctrl_c, true), Action::Quit);
}

#[test]
fn help_and_refresh_are_bound() {
    assert_eq!(dispatch(key(KeyCode::Char('?')), false), Action::ToggleHelp);
    assert_eq!(dispatch(key(KeyCode::Char('r')), false), Action::Refresh);
}

#[test]
fn unbound_key_is_none() {
    assert_eq!(dispatch(key(KeyCode::Char('z')), false), Action::None);
}
