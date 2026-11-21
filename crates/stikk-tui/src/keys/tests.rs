//! **Dispatch's own tests: the catalog's keys, and Caps Lock** (RFC 061 decision 2).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use stikk_core::actions;

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// What a key runs, by catalog id — the only way these tests name an action (RFC 061 trap 1).
fn id(code: KeyCode) -> Option<&'static str> {
    match dispatch(key(code), false) {
        Action::Run(action) => Some(action.id),
        _ => None,
    }
}

#[test]
fn back_is_left_esc_and_q() {
    for code in [KeyCode::Char('q'), KeyCode::Esc, KeyCode::Left] {
        assert_eq!(id(code), Some("app.back"), "{code:?}");
    }
}

#[test]
fn enter_asks_what_can_be_done_and_the_right_arrow_opens_the_row() {
    assert_eq!(id(KeyCode::Enter), Some("row.actions"));
    assert_eq!(id(KeyCode::Right), Some("row.open"));
}

#[test]
fn ctrl_c_always_quits_even_in_text_entry() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(dispatch(ctrl_c, false), Action::Quit);
    assert_eq!(dispatch(ctrl_c, true), Action::Quit);
}

#[test]
fn the_arrows_and_their_letters_move() {
    assert_eq!(dispatch(key(KeyCode::Up), false), Action::Up);
    assert_eq!(dispatch(key(KeyCode::Char('k')), false), Action::Up);
    assert_eq!(dispatch(key(KeyCode::Down), false), Action::Down);
    assert_eq!(dispatch(key(KeyCode::Char('j')), false), Action::Down);
    // The catalog lists them as one key, which is what the key line shows.
    assert!(actions::by_id("app.move").is_some_and(|m| m.key == Some(Key::Move)));
}

/// **Every key the catalog binds is one dispatch resolves, and to that same action** (RFC 061 §1's
/// test: no phantom `g o`).
#[test]
fn every_key_the_catalog_binds_is_dispatched() {
    for action in actions::all() {
        let Some(key) = action.key else { continue };
        let codes: Vec<KeyCode> = match key {
            Key::Letter(c) => vec![KeyCode::Char(c), KeyCode::Char(c.to_ascii_uppercase())],
            Key::Enter => vec![KeyCode::Enter],
            Key::Right => vec![KeyCode::Right],
            Key::Back => vec![KeyCode::Esc, KeyCode::Left, KeyCode::Char('q')],
            // `app.move` is the one entry dispatch answers with its own halves, because up and down
            // are two intents under one key.
            Key::Move => {
                for code in [KeyCode::Up, KeyCode::Char('k')] {
                    assert_eq!(dispatch(key2(code), false), Action::Up);
                }
                for code in [KeyCode::Down, KeyCode::Char('j')] {
                    assert_eq!(dispatch(key2(code), false), Action::Down);
                }
                continue;
            }
            Key::Help => vec![KeyCode::Char('?')],
            Key::Palette => vec![KeyCode::Char(':')],
            Key::Space => vec![KeyCode::Char(' ')],
        };
        for code in codes {
            assert_eq!(id(code), Some(action.id), "{:?} -> {}", code, action.id);
        }
    }
}

/// `key`, under another name, for use inside a match arm.
fn key2(code: KeyCode) -> KeyEvent {
    key(code)
}

/// **Caps Lock is the same keyboard** (RFC 061 decision 2, F1): for *every* letter, upper and lower
/// case resolve to one action — so no capital can reach a card its lower case would not.
#[test]
fn caps_lock_changes_nothing_for_any_letter() {
    for c in 'a'..='z' {
        let lower = dispatch(key(KeyCode::Char(c)), false);
        let upper = dispatch(key(KeyCode::Char(c.to_ascii_uppercase())), false);
        assert_eq!(lower, upper, "{c:?} and its capital differ");
    }
    // And the nine pairs F1 named are each one action now.
    for c in ['q', 's', 'u', 'a', 'i', 'm', 'v', 'r', 't'] {
        assert_eq!(
            id(KeyCode::Char(c)),
            id(KeyCode::Char(c.to_ascii_uppercase())),
            "{c:?}"
        );
    }
}

/// **Decision 2's table, exactly** — read off the catalog, so the table and the code cannot drift.
#[test]
fn decision_2s_table_is_what_dispatch_does() {
    let table = [
        ('b', "ref.pick"),
        ('w', "view.changes"),
        ('p', "view.queue"),
        ('v', "view.verify"),
        ('t', "view.trust"),
        ('o', "bundle.open"),
        ('c', "op.commit"),
        ('s', "op.seal"),
        ('r', "view.refresh"),
        ('d', "op.compare"),
        ('m', "block.mark"),
        ('f', "view.raw"),
        ('u', "changes.untracked"),
    ];
    for (letter, expected) in table {
        assert_eq!(id(KeyCode::Char(letter)), Some(expected), "{letter:?}");
    }
    assert_eq!(id(KeyCode::Char('?')), Some("view.glossary"));
    assert_eq!(id(KeyCode::Char(':')), Some("view.palette"));
    assert_eq!(id(KeyCode::Char(' ')), Some("seal.consent"));
    // **The letters the fourteen gave up are free** (decision 2): nothing answers them.
    for letter in ['n', 'x', 'e', 'g', 'h', 'l', 'y', 'z'] {
        assert_eq!(id(KeyCode::Char(letter)), None, "{letter:?} is bound");
    }
}

/// **The fourteen that lost their letter are still reachable**, by name, in the palette.
#[test]
fn the_fourteen_moved_actions_are_in_the_palette() {
    let moved = [
        "ref.switch",
        "ref.merge-plan",
        "ref.close",
        "ref.branch",
        "ref.tag",
        "bundle.import",
        "ref.export",
        "trust.adopt",
        "trust.remove",
        "block.undo",
        "file.save",
        "file.save-all",
        "session.refusals",
        "view.operations",
    ];
    assert_eq!(moved.len(), 14);
    let palette: Vec<&str> = actions::matching("")
        .into_iter()
        .map(|action| action.id)
        .collect();
    for id in moved {
        assert!(palette.contains(&id), "{id} is not in the palette");
        // And it has no key of its own any more.
        assert!(
            actions::by_id(id).expect(id).key.is_none(),
            "{id} still has a key"
        );
    }
}

#[test]
fn text_entry_routes_printables_to_input() {
    // With a text-entry overlay open, letters that are keys elsewhere type into the filter.
    assert_eq!(dispatch(key(KeyCode::Char('b')), true), Action::Input('b'));
    assert_eq!(dispatch(key(KeyCode::Char('r')), true), Action::Input('r'));
    assert_eq!(dispatch(key(KeyCode::Char('?')), true), Action::Input('?'));
    assert_eq!(dispatch(key(KeyCode::Backspace), true), Action::Backspace);
    // Esc/Enter/arrows still steer the overlay.
    assert!(matches!(dispatch(key(KeyCode::Esc), true), Action::Run(a) if a.id == "app.back"));
    assert!(matches!(dispatch(key(KeyCode::Enter), true), Action::Run(a) if a.id == "row.actions"));
    assert_eq!(dispatch(key(KeyCode::Up), true), Action::Up);
    // `←` and `→` do nothing in a prompt, as before RFC 061.
    assert_eq!(dispatch(key(KeyCode::Left), true), Action::None);
    assert_eq!(dispatch(key(KeyCode::Right), true), Action::None);
}

/// **A typed key id still tells `A` from `a`** (RFC 061 trap 2): the fold is for keys only, and a
/// prompt takes every character as typed.
#[test]
fn a_prompt_takes_every_character_as_typed() {
    for c in ['Q', 'S', 'x', 'A', 'a', 'U', ' '] {
        assert_eq!(
            dispatch(key(KeyCode::Char(c)), true),
            Action::Input(c),
            "{c:?}"
        );
    }
    // The whole of a key id, as `prikk key status` prints one.
    let typed: String = "SSH-ED25519-AbC"
        .chars()
        .map(|c| match dispatch(key(KeyCode::Char(c)), true) {
            Action::Input(c) => c,
            other => panic!("{c:?} became {other:?}"),
        })
        .collect();
    assert_eq!(typed, "SSH-ED25519-AbC");
}

#[test]
fn unbound_key_is_none() {
    assert_eq!(dispatch(key(KeyCode::Char('z')), false), Action::None);
    assert_eq!(dispatch(key(KeyCode::F(5)), false), Action::None);
}
