//! Global key dispatch (handoff §2 `keys.rs`, a subset of design `TU-05`).
//!
//! Every key event resolves through [`dispatch`] into an [`Action`] — one seam, so that RFC 002 (the
//! action-id catalog and configurable bindings) can replace the literal bindings here without a
//! rewrite of the run loop. Dispatch is context-free: it names the *intent* (`Select`, `Back`, `Up`),
//! and the [`crate::app::App`] resolves what that means for the current screen and overlay stack.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A resolved user intent, independent of which key produced it (the seam RFC 002 will feed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No bound action for this key.
    None,
    /// Quit the application unconditionally.
    Quit,
    /// Go back one step: close an overlay, pop a screen, or quit at the root.
    Back,
    /// Select / drill in (open History, open a block, pick a ref).
    Select,
    /// Move the selection up.
    Up,
    /// Move the selection down.
    Down,
    /// Open the ref picker.
    OpenRefPicker,
    /// Toggle the Help overlay.
    ToggleHelp,
    /// Re-source the current view from prikk.
    Refresh,
}

/// Resolve a key press into an [`Action`]. Dispatch is context-free; `Back` folds in the old
/// "close overlay before quitting" behaviour, resolved by the app against its stacks (handoff §7).
#[must_use]
pub fn dispatch(key: KeyEvent) -> Action {
    // Ctrl-C always quits, whatever is open.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::Back,
        KeyCode::Enter => Action::Select,
        KeyCode::Up | KeyCode::Char('k') => Action::Up,
        KeyCode::Down | KeyCode::Char('j') => Action::Down,
        KeyCode::Char('b') => Action::OpenRefPicker,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests;
