//! **From an iced key event to the program's intent** (RFC 069 rules 6 and 7, RFC 070 decision 4).
//!
//! **The same table as the terminal's** `stikk_tui::keys::dispatch`, because the desktop app binds
//! exactly the terminal's keys, from `stikk_core::actions` (rule 7). There is no list of bindings
//! here either: a key event resolves to one of RFC 061's [`Key`]s, that key's action comes from the
//! catalog, and the program says what it does. So a key exists only where the catalog binds it.
//!
//! # What is the desktop's own, and why
//!
//! - **`Named(Process)` is no action** (rule 6). RFC 065's W3 measured Windows sending `Process`
//!   keys while an input method composes. Treated as a letter it would act, which is how a
//!   Japanese user would find a card opening under their composition.
//! - **A letter never acts while a text field has focus** (rule 6). **Focus decides, not the
//!   card**, because in a GUI a field can have focus outside a text card. Until Handoff C there is
//!   no text field in the app, so the only answer is the program's own `wants_text_input`; the
//!   parameter is already shaped for the second source rather than discovered later.
//! - **Snora's `Shortcuts` and `Escape` routing, F6 and Tab, come first** (rule 6), handled by
//!   `snora::keyboard::traverse` before this function is asked. stikk adds no key model beside it.

use iced::keyboard::{self, Key as IcedKey, Modifiers, key::Named};
use stikk_app::Intent;
use stikk_core::{Key, actions};

/// The catalog action a [`Key`] runs, or [`Intent::None`] when the catalog binds that key to
/// nothing.
fn run(key: Key) -> Intent {
    actions::by_key(key).map_or(Intent::None, Intent::Run)
}

/// **Resolve a key press into an [`Intent`].**
///
/// `text_focus` is true when the keyboard belongs to a text field — the program's own
/// `wants_text_input`, and later a focused field outside a text card (rule 6).
///
/// Returns [`Intent::None`] for a key that means nothing here, which the program takes as no
/// action. A `None` is never a silent drop: the state log records the key beside the state it did
/// not change.
#[must_use]
pub fn dispatch(key: &IcedKey, modifiers: Modifiers, text_focus: bool) -> Intent {
    // Ctrl-C always quits, whatever is open — the terminal's own first line.
    if modifiers.control() && matches!(key.as_ref(), keyboard::Key::Character("c")) {
        return Intent::Quit;
    }
    // **Rule 6: an input method's own key is no action.** Before anything else, so no later arm
    // can see it as a character.
    if matches!(key.as_ref(), keyboard::Key::Named(Named::Process)) {
        return Intent::None;
    }
    if text_focus {
        return match key.as_ref() {
            keyboard::Key::Named(Named::Escape) => run(Key::Back),
            keyboard::Key::Named(Named::Enter) => run(Key::Enter),
            keyboard::Key::Named(Named::ArrowUp) => Intent::Up,
            keyboard::Key::Named(Named::ArrowDown) => Intent::Down,
            keyboard::Key::Named(Named::Backspace) => Intent::Backspace,
            // **As typed**: the fold is for keys, never for text. A typed key id must still tell
            // `A` from `a` (RFC 061 trap 2). Text itself arrives as text, through the field, so
            // this arm is only for the keys a field does not take.
            _ => Intent::None,
        };
    }
    match key.as_ref() {
        // **The left arrow goes back and never quits** (RFC 061 review v1 C2).
        keyboard::Key::Named(Named::ArrowLeft) => Intent::BackWithoutQuit,
        keyboard::Key::Named(Named::Escape) => run(Key::Back),
        keyboard::Key::Named(Named::Enter) => run(Key::Enter),
        keyboard::Key::Named(Named::ArrowRight) => run(Key::Right),
        keyboard::Key::Named(Named::ArrowUp) => Intent::Up,
        keyboard::Key::Named(Named::ArrowDown) => Intent::Down,
        keyboard::Key::Named(Named::Space) => run(Key::Space),
        keyboard::Key::Character(s) => character(s),
        _ => Intent::None,
    }
}

/// One typed character, outside a text field: the terminal's own arms, in its order.
fn character(s: &str) -> Intent {
    match s {
        "?" => run(Key::Help),
        ":" => run(Key::Palette),
        " " => run(Key::Space),
        // **Folded like every other letter** (RFC 061 decision 2): `K` moves up, `Q` goes back.
        "k" | "K" => Intent::Up,
        "j" | "J" => Intent::Down,
        // `q` is `Key::Back`'s letter, so it is asked for as that and not as a letter of its own.
        "q" | "Q" => run(Key::Back),
        // **The catalog decides the rest**, folded.
        other => other
            .chars()
            .next()
            .and_then(actions::by_letter)
            .map_or(Intent::None, Intent::Run),
    }
}

#[cfg(test)]
mod tests;
