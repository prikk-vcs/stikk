//! The application state and its transitions (handoff §2 `app.rs`, §3 render model).
//!
//! The app owns the current view-model, the overlay stack, and the palette. It **holds no repository
//! authority** (design INV-8): the [`OrientationState`] is a rendered snapshot of what prikk reported,
//! re-sourced on [`App::reload`]. The only repository facts come from [`stikk_core::orient`] — the app
//! computes nothing.

use std::path::{Path, PathBuf};

use stikk_core::{OrientationView, orient};
use stikk_prikk::Prikk;
use stikk_state::Config;

use crate::overlay::Overlay;
use crate::theme::Palette;

/// The Orientation view's load state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrientationState {
    /// The initial state before the first load completes.
    Loading,
    /// A successfully loaded orientation.
    Loaded(OrientationView),
    /// A load failure, carrying prikk's verbatim message (design NFR-I03).
    Failed(String),
}

/// The running application.
pub struct App {
    repo: PathBuf,
    state: OrientationState,
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
            overlays: Vec::new(),
            palette,
            should_quit: false,
        }
    }

    /// Re-source the orientation from prikk (design FR-106; the `r` key).
    pub fn reload(&mut self, prikk: &impl Prikk) {
        self.state = load(prikk, &self.repo);
    }

    /// The repository root this app opened.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// The current orientation load state.
    #[must_use]
    pub fn state(&self) -> &OrientationState {
        &self.state
    }

    /// The active colour palette.
    #[must_use]
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// The top overlay, if any (the one that renders and receives the close key).
    #[must_use]
    pub fn top_overlay(&self) -> Option<Overlay> {
        self.overlays.last().copied()
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
        if self.overlays.last() == Some(&Overlay::Help) {
            self.overlays.pop();
        } else {
            self.overlays.push(Overlay::Help);
        }
    }

    /// Close the top overlay, if any.
    pub fn close_overlay(&mut self) {
        self.overlays.pop();
    }
}

/// Load orientation through the seam, mapping any error to its verbatim message.
fn load(prikk: &impl Prikk, repo: &Path) -> OrientationState {
    match orient(prikk, repo) {
        Ok(view) => OrientationState::Loaded(view),
        Err(error) => OrientationState::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
