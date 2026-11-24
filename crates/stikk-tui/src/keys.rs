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
//! path), printable keys become [`Intent::Input`] and Backspace [`Intent::Backspace`] — **every
//! character as typed, upper case included** (RFC 061 trap 2: a typed key id must still tell `A`
//! from `a`). The run loop passes [`crate::app::App::wants_text_input`].

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use stikk_core::{Action as CatalogAction, Key, actions};

use crate::app::{App, Effect};

/// A resolved user intent.
///
/// **[`Self::Run`] carries the catalog entry itself**, so nothing here repeats what an action is
/// called or what key it has. The rest are the two halves of `app.move`, which the catalog lists as
/// one key, the gentler half of `app.back`, and text entry's own two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// No bound action for this key.
    None,
    /// Quit the application unconditionally (Ctrl-C).
    Quit,
    /// Back, **and never quit** — the left arrow, `app.back`'s gentler half (RFC 061 review v1 C2).
    ///
    /// The catalog binds one action to `←`, `Esc` and `q`; this is the one of the three that stops at
    /// the root instead of ending the session, because a reader moving around presses an arrow key
    /// constantly.
    BackWithoutQuit,
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

/// The catalog action a [`Key`] runs, or [`Intent::None`] when the catalog binds that key to nothing.
fn run(key: Key) -> Intent {
    actions::by_key(key).map_or(Intent::None, Intent::Run)
}

/// Resolve a key press into an [`Intent`]. `text_entry` is true when a text-entry overlay is open,
/// routing printable keys to it.
///
/// **This function translates key events into [`Key`]s and nothing else.** Which action a key runs is
/// the catalog's to say (RFC 061 trap 1). The aliases are the ones [`Key`] itself documents: `←`,
/// `Esc` and `q` are [`Key::Back`]; `↑` `↓` and `k` `j` are [`Key::Move`].
#[must_use]
pub fn dispatch(key: KeyEvent, text_entry: bool) -> Intent {
    // Ctrl-C always quits, whatever is open.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Intent::Quit;
    }
    if text_entry {
        return match key.code {
            KeyCode::Esc => run(Key::Back),
            KeyCode::Enter => run(Key::Enter),
            KeyCode::Up => Intent::Up,
            KeyCode::Down => Intent::Down,
            KeyCode::Backspace => Intent::Backspace,
            // **As typed**: the fold is for keys, never for text.
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => Intent::Input(c),
            // `←` and `→` do nothing in a prompt, as before RFC 061: there is no cursor inside the
            // typed text to move (it appends and backspaces), so they would have nowhere to go.
            _ => Intent::None,
        };
    }
    match key.code {
        // **The left arrow goes back and never quits** (RFC 061 review v1 C2): one action, and the
        // one of its three keys that must not end the session at the root.
        KeyCode::Left => Intent::BackWithoutQuit,
        KeyCode::Esc => run(Key::Back),
        KeyCode::Enter => run(Key::Enter),
        KeyCode::Right => run(Key::Right),
        // **Folded like every other letter** (decision 2): `K` moves up, and `Q` goes back.
        KeyCode::Up | KeyCode::Char('k' | 'K') => Intent::Up,
        KeyCode::Down | KeyCode::Char('j' | 'J') => Intent::Down,
        KeyCode::Char('?') => run(Key::Help),
        KeyCode::Char(':') => run(Key::Palette),
        KeyCode::Char(' ') => run(Key::Space),
        // **The catalog decides the rest**, folded (decision 2). `q` is `Key::Back`'s letter, so it
        // is asked for as that and not as a letter of its own.
        KeyCode::Char('q' | 'Q') => run(Key::Back),
        KeyCode::Char(c) => actions::by_letter(c).map_or(Intent::None, Intent::Run),
        _ => Intent::None,
    }
}

/// **What a key press does** -- the one function from a crossterm `KeyEvent` to the program (RFC 067
/// decision 1).
///
/// A free function, and not a method, because **the program takes intents, not key events**: `App`
/// has no method that accepts a [`KeyEvent`] any more, so a front end cannot reach it with one, and
/// the next front end is not obliged to invent crossterm events to drive the program.
///
/// The run loop calls this and nothing else, and **tests press keys through it**. That is the point:
/// every scrolling test used to call a handler directly (`nav_down`, `select`), so no test could see
/// that `↓` reached nothing at all in the help overlay. A test that calls a handler proves the
/// handler works; only a test that presses a key proves the key is wired to it.
pub fn handle_key(app: &mut App, key: KeyEvent) -> Effect {
    app.handle(dispatch(key, app.wants_text_input()))
}

#[cfg(test)]
mod tests;
