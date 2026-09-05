//! Global key dispatch (handoff §7; a subset of design `TU-05`).
//!
//! Every key event resolves through [`dispatch`] into an [`Action`] — one seam, so that RFC 002 (the
//! action-id catalog and configurable bindings) can replace the literal bindings here without a
//! rewrite of the run loop. Dispatch names the *intent*; the [`crate::app::App`] resolves what it means
//! for the current screen and overlay stack.
//!
//! One documented exception to being context-free: when a **text-entry** overlay is open (the command
//! palette), printable keys become [`Action::Input`] and Backspace [`Action::Backspace`], so the
//! letters that are bindings elsewhere (`b`, `r`, `?`) type into the filter instead. The run loop
//! passes [`crate::app::App::wants_text_input`].

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A resolved user intent, independent of which key produced it (the seam RFC 002 will feed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No bound action for this key.
    None,
    /// Quit the application unconditionally.
    Quit,
    /// Go back one step: dismiss a fault/banner, close an overlay, pop a screen, or quit at the root.
    Back,
    /// Select / drill in / activate (open History, a block, a ref, a next-step, a palette command).
    Select,
    /// Move the selection up.
    Up,
    /// Move the selection down.
    Down,
    /// Open the ref picker.
    OpenRefPicker,
    /// Open the Changes (worktree-vs-baseline) view.
    OpenChanges,
    /// Toggle the display-only untracked filter (Changes view).
    ToggleUntracked,
    /// Begin the commit flow (`FL-05` step 1; RFC 014).
    Commit,
    /// Open the glossary / help browser.
    OpenGlossary,
    /// Open the command palette.
    OpenPalette,
    /// Open the session refusal history.
    OpenRefusals,
    /// Open the Background Operations overlay (TU-01; RFC 010).
    OpenOperations,
    /// Re-source the current view from prikk.
    Refresh,
    /// A typed character (text-entry overlays only).
    Input(char),
    /// Delete the last typed character (text-entry overlays only).
    Backspace,
}

/// Resolve a key press into an [`Action`]. `text_entry` is true when a text-entry overlay (the palette)
/// is open, routing printable keys to its filter.
#[must_use]
pub fn dispatch(key: KeyEvent, text_entry: bool) -> Action {
    // Ctrl-C always quits, whatever is open.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Action::Quit;
    }
    if text_entry {
        return match key.code {
            KeyCode::Esc => Action::Back,
            KeyCode::Enter => Action::Select,
            KeyCode::Up => Action::Up,
            KeyCode::Down => Action::Down,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => Action::Input(c),
            _ => Action::None,
        };
    }
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::Back,
        KeyCode::Enter => Action::Select,
        KeyCode::Up | KeyCode::Char('k') => Action::Up,
        KeyCode::Down | KeyCode::Char('j') => Action::Down,
        KeyCode::Char('b') => Action::OpenRefPicker,
        KeyCode::Char('w') => Action::OpenChanges,
        KeyCode::Char('u') => Action::ToggleUntracked,
        KeyCode::Char('C') => Action::Commit,
        KeyCode::Char('?') => Action::OpenGlossary,
        KeyCode::Char(':') => Action::OpenPalette,
        KeyCode::Char('R') => Action::OpenRefusals,
        KeyCode::Char('o') => Action::OpenOperations,
        KeyCode::Char('r') => Action::Refresh,
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests;
