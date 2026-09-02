//! The application state and its transitions (handoff §2 `app.rs`, §3 render model).
//!
//! The app owns the current view-model stack, the overlay stack, the focused ref, and the palette. It
//! **holds no repository authority** (design INV-8): every view-model is a rendered snapshot of what
//! prikk reported, re-sourced on demand. The only repository facts come from `stikk_core` — the app
//! computes nothing.
//!
//! Navigation is a stack of [`Screen`]s above the Orientation root: opening History pushes a screen,
//! drilling into a block pushes another, and [`App::back`] pops one (closing the top overlay first, or
//! quitting when nothing is left). This is the shape the Compare/Changes views plug into next.

use std::path::{Path, PathBuf};

use stikk_core::{BlockDetailView, HistoryView, block_detail, history_view, list_refs, orient};
use stikk_prikk::Prikk;
use stikk_state::Config;

use crate::overlay::Overlay;
use crate::theme::Palette;

/// How many blocks the History view requests at a time (design FR-011 caps the listing).
const HISTORY_LIMIT: usize = 200;

/// The default focused ref — prikk has no HEAD, so stikk focuses a named ref explicitly (FR-055).
const DEFAULT_REF: &str = "heads/main";

/// The Orientation view's load state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrientationState {
    /// The initial state before the first load completes.
    Loading,
    /// A successfully loaded orientation.
    Loaded(stikk_core::OrientationView),
    /// A load failure, carrying prikk's verbatim message (design NFR-I03).
    Failed(String),
}

/// A screen pushed above the Orientation root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// A ref's block history, with the selection cursor into `view.blocks`.
    History {
        /// The loaded lineage.
        view: HistoryView,
        /// The selected block index (0 = tip).
        cursor: usize,
    },
    /// A single block's detail.
    BlockDetail(BlockDetailView),
}

/// What the shell should render in the body region — the top screen, or the Orientation root.
#[derive(Debug, Clone, Copy)]
pub enum Focus<'a> {
    /// The Orientation root and its load state.
    Orientation(&'a OrientationState),
    /// A History screen and its selection cursor.
    History(&'a HistoryView, usize),
    /// A Block-detail screen.
    BlockDetail(&'a BlockDetailView),
}

/// The running application.
pub struct App {
    repo: PathBuf,
    state: OrientationState,
    focused_ref: String,
    screens: Vec<Screen>,
    overlays: Vec<Overlay>,
    palette: Palette,
    should_quit: bool,
}

impl App {
    /// Open `repo` and load its orientation through the seam, building the palette from `config`.
    #[must_use]
    pub fn open(repo: impl Into<PathBuf>, prikk: &impl Prikk, config: &Config) -> Self {
        let repo = repo.into();
        let state = load(prikk, &repo);
        Self {
            repo,
            state,
            focused_ref: DEFAULT_REF.to_string(),
            screens: Vec::new(),
            overlays: Vec::new(),
            palette: Palette::from_theme(config.theme),
            should_quit: false,
        }
    }

    /// Construct an app in an explicit state, without loading — used by tests and the runnable example
    /// so they need no prikk binary or repository.
    #[must_use]
    pub fn from_state(repo: impl Into<PathBuf>, state: OrientationState, palette: Palette) -> Self {
        Self {
            repo: repo.into(),
            state,
            focused_ref: DEFAULT_REF.to_string(),
            screens: Vec::new(),
            overlays: Vec::new(),
            palette,
            should_quit: false,
        }
    }

    /// Re-source the visible screens from prikk (design FR-106; the `r` key). The Orientation root is
    /// always refreshed; a History screen on top is reloaded too, its cursor clamped to the new length.
    pub fn reload(&mut self, prikk: &impl Prikk) {
        self.state = load(prikk, &self.repo);
        let Some(Screen::History { cursor, .. }) = self.screens.last() else {
            return;
        };
        let cursor = *cursor;
        match history_view(prikk, &self.repo, &self.focused_ref, HISTORY_LIMIT) {
            Ok(view) => {
                let cursor = clamp_cursor(cursor, view.blocks.len());
                if let Some(top) = self.screens.last_mut() {
                    *top = Screen::History { view, cursor };
                }
            }
            Err(error) => self.notice(error.to_string()),
        }
    }

    /// Open the History view for the focused ref, pushing it above the current screen (the `Enter` key
    /// on Orientation, or after picking a ref).
    pub fn open_history(&mut self, prikk: &impl Prikk) {
        match history_view(prikk, &self.repo, &self.focused_ref, HISTORY_LIMIT) {
            Ok(view) => self.screens.push(Screen::History { view, cursor: 0 }),
            Err(error) => self.notice(error.to_string()),
        }
    }

    /// The context-sensitive select/drill-in action (the `Enter` key).
    ///
    /// - Ref picker open → adopt the highlighted ref and (re)open its History.
    /// - History on top → open the selected block's detail.
    /// - Orientation root → open the focused ref's History.
    pub fn select(&mut self, prikk: &impl Prikk) {
        if let Some(Overlay::RefPicker { refs, cursor }) = self.overlays.last() {
            if let Some(name) = refs.get(*cursor).cloned() {
                self.overlays.pop();
                self.focused_ref = name;
                // Replace a History already on top rather than stacking a second one.
                if matches!(self.screens.last(), Some(Screen::History { .. })) {
                    self.screens.pop();
                }
                self.open_history(prikk);
            }
            return;
        }
        match self.screens.last() {
            Some(Screen::History { view, cursor }) => {
                let cursor = *cursor;
                if let Some(row) = view.blocks.get(cursor).cloned() {
                    let is_tip = cursor == 0;
                    let reff = self.focused_ref.clone();
                    match block_detail(prikk, &self.repo, &reff, row, is_tip) {
                        Ok(detail) => self.screens.push(Screen::BlockDetail(detail)),
                        Err(error) => self.notice(error.to_string()),
                    }
                }
            }
            Some(Screen::BlockDetail(_)) => {}
            None => self.open_history(prikk),
        }
    }

    /// Open the ref picker overlay, sourcing the ref list through the seam (the `b` key).
    pub fn open_ref_picker(&mut self, prikk: &impl Prikk) {
        match list_refs(prikk, &self.repo) {
            Ok(entries) => {
                let refs: Vec<String> = entries.into_iter().map(|entry| entry.name).collect();
                let cursor = refs
                    .iter()
                    .position(|name| name == &self.focused_ref)
                    .unwrap_or(0);
                self.overlays.push(Overlay::RefPicker { refs, cursor });
            }
            Err(error) => self.notice(error.to_string()),
        }
    }

    /// Move the selection up (the ref picker's cursor if open, else the History cursor).
    pub fn nav_up(&mut self) {
        if let Some(Overlay::RefPicker { cursor, .. }) = self.overlays.last_mut() {
            *cursor = cursor.saturating_sub(1);
        } else if let Some(Screen::History { cursor, .. }) = self.screens.last_mut() {
            *cursor = cursor.saturating_sub(1);
        }
    }

    /// Move the selection down, clamped to the list length.
    pub fn nav_down(&mut self) {
        if let Some(Overlay::RefPicker { refs, cursor }) = self.overlays.last_mut() {
            *cursor = next_index(*cursor, refs.len());
        } else if let Some(Screen::History { view, cursor }) = self.screens.last_mut() {
            *cursor = next_index(*cursor, view.blocks.len());
        }
    }

    /// Go back one step: close the top overlay, else pop the top screen, else quit.
    pub fn back(&mut self) {
        if self.overlays.pop().is_some() {
            return;
        }
        if self.screens.pop().is_some() {
            return;
        }
        self.should_quit = true;
    }

    /// The repository root this app opened.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// The ref the session is focused on (never a HEAD — prikk has none, design FR-055).
    #[must_use]
    pub fn focused_ref(&self) -> &str {
        &self.focused_ref
    }

    /// The current orientation load state (the root screen's data).
    #[must_use]
    pub fn state(&self) -> &OrientationState {
        &self.state
    }

    /// What the shell should render in the body region.
    #[must_use]
    pub fn focus(&self) -> Focus<'_> {
        match self.screens.last() {
            Some(Screen::History { view, cursor }) => Focus::History(view, *cursor),
            Some(Screen::BlockDetail(detail)) => Focus::BlockDetail(detail),
            None => Focus::Orientation(&self.state),
        }
    }

    /// The active colour palette.
    #[must_use]
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// The top overlay, if any (the one that renders and receives navigation keys).
    #[must_use]
    pub fn top_overlay(&self) -> Option<&Overlay> {
        self.overlays.last()
    }

    /// Whether any overlay is open.
    #[must_use]
    pub fn has_overlay(&self) -> bool {
        !self.overlays.is_empty()
    }

    /// Whether the run loop should exit.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Request exit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Toggle the Help overlay: open it if the top overlay is not Help, else close it.
    pub fn toggle_help(&mut self) {
        if matches!(self.overlays.last(), Some(Overlay::Help)) {
            self.overlays.pop();
        } else {
            self.overlays.push(Overlay::Help);
        }
    }

    /// Close the top overlay, if any.
    pub fn close_overlay(&mut self) {
        self.overlays.pop();
    }

    /// Push a transient notice overlay carrying prikk's verbatim message (design NFR-I03). The full
    /// refusal overlay with next-steps is increment 4; this preserves the message meanwhile.
    fn notice(&mut self, message: String) {
        self.overlays.push(Overlay::Notice(message));
    }

    /// Push a screen directly, without loading through the seam — used by tests and the runnable
    /// example so they need no prikk binary (mirrors [`App::from_state`]).
    pub fn push_screen(&mut self, screen: Screen) {
        self.screens.push(screen);
    }

    /// Set the focused ref without a reload — for tests and the example.
    pub fn set_focused_ref(&mut self, reff: impl Into<String>) {
        self.focused_ref = reff.into();
    }

    /// Push an overlay directly — for tests and the example.
    pub fn push_overlay(&mut self, overlay: Overlay) {
        self.overlays.push(overlay);
    }
}

/// Load orientation through the seam, mapping any error to its verbatim message.
fn load(prikk: &impl Prikk, repo: &Path) -> OrientationState {
    match orient(prikk, repo) {
        Ok(view) => OrientationState::Loaded(view),
        Err(error) => OrientationState::Failed(error.to_string()),
    }
}

/// The next cursor index, clamped so it never runs off the end of a `len`-item list.
fn next_index(cursor: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (cursor + 1).min(len - 1)
    }
}

/// Clamp an existing cursor to a (possibly shrunk) list length.
fn clamp_cursor(cursor: usize, len: usize) -> usize {
    if len == 0 { 0 } else { cursor.min(len - 1) }
}

#[cfg(test)]
mod tests;
