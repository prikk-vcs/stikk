//! Tests for key dispatch (handoff §7/§10).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn q_and_esc_map_to_back() {
    assert_eq!(dispatch(key(KeyCode::Char('q')), false), Action::Back);
    assert_eq!(dispatch(key(KeyCode::Esc), false), Action::Back);
}

#[test]
fn ctrl_c_always_quits_even_in_text_entry() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(dispatch(ctrl_c, false), Action::Quit);
    assert_eq!(dispatch(ctrl_c, true), Action::Quit);
}

#[test]
fn navigation_and_surface_keys_are_bound() {
    assert_eq!(dispatch(key(KeyCode::Enter), false), Action::Select);
    assert_eq!(dispatch(key(KeyCode::Up), false), Action::Up);
    assert_eq!(dispatch(key(KeyCode::Char('k')), false), Action::Up);
    assert_eq!(dispatch(key(KeyCode::Down), false), Action::Down);
    assert_eq!(dispatch(key(KeyCode::Char('j')), false), Action::Down);
    assert_eq!(
        dispatch(key(KeyCode::Char('b')), false),
        Action::OpenRefPicker
    );
    assert_eq!(
        dispatch(key(KeyCode::Char('?')), false),
        Action::OpenGlossary
    );
    assert_eq!(
        dispatch(key(KeyCode::Char(':')), false),
        Action::OpenPalette
    );
    assert_eq!(
        dispatch(key(KeyCode::Char('R')), false),
        Action::OpenRefusals
    );
    assert_eq!(dispatch(key(KeyCode::Char('r')), false), Action::Refresh);
}

#[test]
fn text_entry_routes_printables_to_input() {
    // With a text-entry overlay open, letters that are bindings elsewhere type into the filter.
    assert_eq!(dispatch(key(KeyCode::Char('b')), true), Action::Input('b'));
    assert_eq!(dispatch(key(KeyCode::Char('r')), true), Action::Input('r'));
    assert_eq!(dispatch(key(KeyCode::Char('?')), true), Action::Input('?'));
    assert_eq!(dispatch(key(KeyCode::Backspace), true), Action::Backspace);
    // Esc/Enter/arrows still steer the overlay.
    assert_eq!(dispatch(key(KeyCode::Esc), true), Action::Back);
    assert_eq!(dispatch(key(KeyCode::Enter), true), Action::Select);
    assert_eq!(dispatch(key(KeyCode::Up), true), Action::Up);
}

#[test]
fn unbound_key_is_none() {
    assert_eq!(dispatch(key(KeyCode::Char('z')), false), Action::None);
}
