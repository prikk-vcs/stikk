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
    /// Open the Queue view (RFC 028).
    OpenQueue,
    /// Open the Verify view (`FR-100`; RFC 040). **Capital `V`**, like `Q` and `R`, because it
    /// opens a repository-wide view; lowercase `v` stays the raw toggle it is everywhere else.
    OpenVerify,
    /// Toggle the display-only untracked filter (Changes view).
    ToggleUntracked,
    /// Mark or unmark the selected block, for a comparison (History; RFC 038 decision 3).
    MarkBlock,
    /// Compare (`FR-033`; RFC 038 decision 3). What is compared depends on the screen: the marked
    /// block against the selected one, a block against the previous one in this ref's history, or
    /// the worktree against what `commit` would author against.
    Compare,
    /// Toggle Patch detail's raw view — prikk's own fields (RFC 035 decision 5).
    ToggleRaw,
    /// Begin the commit flow (`FL-05` step 1; RFC 014).
    Commit,
    /// Begin the seal flow (`FL-06` step 1; RFC 016).
    Seal,
    /// Create a branch at the selected ref's tip (the ref picker; RFC 044 decision 4).
    CreateBranch,
    /// Create a tag: at the selected ref's tip in the ref picker, or at the selected block in
    /// History and Block detail (RFC 044 decision 4).
    CreateTag,
    /// Close the selected branch (the ref picker; RFC 044 decision 4). **Never the branch the
    /// repository is on** — that one is refused, in stikk's own words.
    CloseBranch,
    /// Switch the repository to the selected branch (the ref picker; RFC 045 decision 7).
    /// **`Enter` is not this**: `Enter` focuses stikk on a ref and touches nothing.
    SwitchBranch,
    /// Open the Merge plan for *the marked branch into the selected one* (the ref picker;
    /// RFC 047 decision 4). **Reads only**: it shows prikk's plan and merges nothing.
    MergePlan,
    /// Open a bundle file: prikk's verification and what importing it would change (RFC 050
    /// decision 3). **Reads only**: nothing is imported and nothing is written.
    OpenBundle,
    /// Import the open bundle as a received branch (the Bundle view; RFC 050 decision 4).
    /// **Offered only where the view read prikk's verification and its tip line.**
    Import,
    /// Toggle the seal consent acknowledgement (RFC 016 §8) — meaningful only while
    /// [`crate::overlay::Overlay::SealConsent`] is on top; a no-op everywhere else.
    ToggleSealConsent,
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
        KeyCode::Char('Q') => Action::OpenQueue,
        KeyCode::Char('V') => Action::OpenVerify,
        KeyCode::Char('u') => Action::ToggleUntracked,
        KeyCode::Char('m') => Action::MarkBlock,
        KeyCode::Char('d') => Action::Compare,
        KeyCode::Char('v') => Action::ToggleRaw,
        KeyCode::Char('C') => Action::Commit,
        KeyCode::Char('S') => Action::Seal,
        KeyCode::Char('n') => Action::CreateBranch,
        KeyCode::Char('t') => Action::CreateTag,
        KeyCode::Char('x') => Action::CloseBranch,
        KeyCode::Char('s') => Action::SwitchBranch,
        KeyCode::Char('M') => Action::MergePlan,
        KeyCode::Char('i') => Action::OpenBundle,
        KeyCode::Char('I') => Action::Import,
        KeyCode::Char(' ') => Action::ToggleSealConsent,
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
