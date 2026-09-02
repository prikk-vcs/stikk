//! Global key dispatch (handoff §2 `keys.rs`, a subset of design `TU-05`).
//!
//! Every key event resolves through [`dispatch`] into an [`Action`] — one seam, so that RFC 002 (the
//! action-id catalog and configurable bindings) can replace the literal bindings here without a
//! rewrite of the run loop. This increment ships a fixed set: quit, toggle help, refresh, and
//! "close the top overlay before quitting".

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A resolved user intent, independent of which key produced it (the seam RFC 002 will feed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No bound action for this key.
    None,
    /// Quit the application.
    Quit,
    /// Toggle the Help overlay.
    ToggleHelp,
    /// Re-source the current view from prikk.
    Refresh,
    /// Close the top overlay (used when one is open, before quitting).
    CloseOverlay,
}

/// Resolve a key press into an [`Action`]. `has_overlay` is true when an overlay is open, so that
/// `q`/`Esc` close the overlay first rather than quitting (handoff §7).
#[must_use]
pub fn dispatch(key: KeyEvent, has_overlay: bool) -> Action {
    // Ctrl-C always quits, overlay or not.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            if has_overlay {
                Action::CloseOverlay
            } else {
                Action::Quit
            }
        }
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests;
