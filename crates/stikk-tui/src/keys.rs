//! **Key dispatch, from the catalog** (design `TU-05`; RFC 061 decision 2).
//!
//! There is no list of bindings here. A key event resolves to one of RFC 061's [`Key`]s, that key's
//! action comes from [`stikk_core::actions`], and [`crate::app::App::run_action`] says what it does.
//! So a key exists only where the catalog binds it — which is how the phantom `g o` the palette used
//! to print became impossible (RFC 061 F2).
//!
//! # One letter, one action, in either case
//!
//! Outside a text prompt a letter is folded to lower case before the catalog is asked, so Caps Lock
//! on is the same keyboard (RFC 061 decision 2). Nine letters used to mean one thing in lower case
//! and another in upper (F1); now each means one.
//!
//! # The one documented exception to being context-free
//!
//! When a **text-entry** overlay is open (the palette's filter, a commit message, a ref name, a
//! path), printable keys become [`Action::Input`] and Backspace [`Action::Backspace`] — **every
//! character as typed, upper case included** (RFC 061 trap 2: a typed key id must still tell `A`
//! from `a`). The run loop passes [`crate::app::App::wants_text_input`].

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use stikk_core::{Action as CatalogAction, Key, actions};

/// A resolved user intent.
///
/// **[`Self::Run`] carries the catalog entry itself**, so nothing here repeats what an action is
/// called or what key it has. The four that are not catalog actions are the two halves of
/// `app.move`, which the catalog lists as one key, and text entry's own two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No bound action for this key.
    None,
    /// Quit the application unconditionally (Ctrl-C).
    Quit,
    /// Move the selection up — `app.move`'s upper half.
    Up,
    /// Move the selection down — `app.move`'s lower half.
    Down,
    /// Run one catalog action.
    Run(&'static CatalogAction),
    /// A typed character (text-entry overlays only).
    Input(char),
    /// Delete the last typed character (text-entry overlays only).
    Backspace,
}

/// The catalog action a [`Key`] runs, or [`Action::None`] when the catalog binds that key to nothing.
fn run(key: Key) -> Action {
    actions::by_key(key).map_or(Action::None, Action::Run)
}

/// Resolve a key press into an [`Action`]. `text_entry` is true when a text-entry overlay is open,
/// routing printable keys to it.
///
/// **This function translates key events into [`Key`]s and nothing else.** Which action a key runs is
/// the catalog's to say (RFC 061 trap 1). The aliases are the ones [`Key`] itself documents: `←`,
/// `Esc` and `q` are [`Key::Back`]; `↑` `↓` and `k` `j` are [`Key::Move`].
#[must_use]
pub fn dispatch(key: KeyEvent, text_entry: bool) -> Action {
    // Ctrl-C always quits, whatever is open.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    if text_entry {
        return match key.code {
            KeyCode::Esc => run(Key::Back),
            KeyCode::Enter => run(Key::Enter),
            KeyCode::Up => Action::Up,
            KeyCode::Down => Action::Down,
            KeyCode::Backspace => Action::Backspace,
            // **As typed**: the fold is for keys, never for text.
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => Action::Input(c),
            // `←` and `→` do nothing in a prompt, as before RFC 061: there is no cursor inside the
            // typed text to move (it appends and backspaces), so they would have nowhere to go.
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc | KeyCode::Left => run(Key::Back),
        KeyCode::Enter => run(Key::Enter),
        KeyCode::Right => run(Key::Right),
        // **Folded like every other letter** (decision 2): `K` moves up, and `Q` goes back.
        KeyCode::Up | KeyCode::Char('k' | 'K') => Action::Up,
        KeyCode::Down | KeyCode::Char('j' | 'J') => Action::Down,
        KeyCode::Char('?') => run(Key::Help),
        KeyCode::Char(':') => run(Key::Palette),
        KeyCode::Char(' ') => run(Key::Space),
        // **The catalog decides the rest**, folded (decision 2). `q` is `Key::Back`'s letter, so it
        // is asked for as that and not as a letter of its own.
        KeyCode::Char('q' | 'Q') => run(Key::Back),
        KeyCode::Char(c) => actions::by_letter(c).map_or(Action::None, Action::Run),
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests;
