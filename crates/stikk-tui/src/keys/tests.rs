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
    // RFC 035 decision 5: the raw-view toggle, in `u`'s pattern — resolved here, meaningful only on
    // the screen it acts on.
    assert_eq!(dispatch(key(KeyCode::Char('v')), false), Action::ToggleRaw);
    assert_eq!(
        dispatch(key(KeyCode::Char('o')), false),
        Action::OpenOperations
    );
    assert_eq!(dispatch(key(KeyCode::Char('C')), false), Action::Commit);
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

#[test]
fn capital_q_opens_the_queue_and_lowercase_q_still_goes_back() {
    // RFC 028 Handoff A §6: `Q` opens the Queue view; `q` is still Back.
    assert_eq!(dispatch(key(KeyCode::Char('Q')), false), Action::OpenQueue);
    assert_eq!(dispatch(key(KeyCode::Char('q')), false), Action::Back);
    // In a text-entry overlay it types, like every other letter.
    assert_eq!(dispatch(key(KeyCode::Char('Q')), true), Action::Input('Q'));
}

/// RFC 058 decision 5: **`U` undoes a block, and `u` is still the untracked filter** — the one
/// reason the undo took a capital letter.
#[test]
fn capital_u_undoes_a_block_and_lowercase_u_is_still_the_untracked_filter() {
    assert_eq!(dispatch(key(KeyCode::Char('U')), false), Action::UndoBlock);
    assert_eq!(
        dispatch(key(KeyCode::Char('u')), false),
        Action::ToggleUntracked
    );
    // And in a text-entry overlay it types, like every other letter (the message prompt).
    assert_eq!(dispatch(key(KeyCode::Char('U')), true), Action::Input('U'));
}
