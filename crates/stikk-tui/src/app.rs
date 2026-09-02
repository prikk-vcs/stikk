//! The application state and its transitions (handoff §2/§7 `app.rs`; RFC 006/007).
//!
//! The app owns the view-model stack, the overlay stack, the focused ref, the session refusal history,
//! and the palette colours. It **holds no repository authority** (design INV-8): every view-model is a
//! rendered snapshot of what prikk reported, re-sourced on demand. The only repository facts come from
//! `stikk_core` — the app computes nothing, and every seam error is routed through the one
//! [`stikk_core::present`] mapping (ER-03), never presented ad hoc.
//!
//! Navigation is a stack of [`Screen`]s above the Orientation root; overlays (help/glossary, ref
//! picker, refusal, palette, refusal history) stack above the active view. [`App::back`] closes the
//! top overlay, else pops a screen, else quits.

use std::path::{Path, PathBuf};

use stikk_core::{
    BlockDetailView, ChangesView, Command, HistoryView, NextTarget, OperationContext, Presentation,
    RefusalHistory, Target, block_detail, changes_view, history_view, list_refs, orient, present,
};
use stikk_model::Capability;
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
    /// The worktree-vs-baseline Changes view, with the UD-08 display-only untracked filter.
    Changes {
        /// The loaded status.
        view: ChangesView,
        /// Whether untracked entries are hidden (display only — a commit still captures them).
        hide_untracked: bool,
    },
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
    /// The Changes view and whether untracked entries are hidden.
    Changes(&'a ChangesView, bool),
}

/// The running application.
pub struct App {
    repo: PathBuf,
    state: OrientationState,
    focused_ref: String,
    screens: Vec<Screen>,
    overlays: Vec<Overlay>,
    refusals: RefusalHistory,
    /// A transient one-line banner for non-overlay presentations (lock-conflict, guidance, plain
    /// statements). Cleared on the next navigation (design OP-03 banner class).
    banner: Option<String>,
    /// A stikk-internal fault: the repository was untouched; the user may continue read-only (ER-04).
    fault: Option<String>,
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
            refusals: RefusalHistory::new(),
            banner: None,
            fault: None,
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
            refusals: RefusalHistory::new(),
            banner: None,
            fault: None,
            palette,
            should_quit: false,
        }
    }

    /// Re-source the visible screens from prikk (design FR-106; the `r` key). The Orientation root is
    /// always refreshed; a History screen on top is reloaded too, its cursor clamped to the new length.
    pub fn reload(&mut self, prikk: &impl Prikk) {
        self.banner = None;
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
            Err(error) => self.surface(&error, OperationContext::LoadHistory),
        }
    }

    /// Open the History view for the focused ref, pushing it above the current screen.
    pub fn open_history(&mut self, prikk: &impl Prikk) {
        self.banner = None;
        match history_view(prikk, &self.repo, &self.focused_ref, HISTORY_LIMIT) {
            Ok(view) => self.screens.push(Screen::History { view, cursor: 0 }),
            Err(error) => self.surface(&error, OperationContext::LoadHistory),
        }
    }

    /// Open the Changes (worktree-vs-baseline) view for the focused ref (the `w` key). Below prikk
    /// 0.28 the version gate surfaces guidance rather than the broken command (FR-034/UD-03).
    pub fn open_changes(&mut self, prikk: &impl Prikk) {
        self.banner = None;
        match changes_view(prikk, &self.repo, &self.focused_ref) {
            Ok(view) => self.screens.push(Screen::Changes {
                view,
                hide_untracked: false,
            }),
            Err(error) => self.surface(&error, OperationContext::LoadChanges),
        }
    }

    /// Toggle the display-only untracked filter on a focused Changes screen (the `u` key; UD-08).
    pub fn toggle_untracked(&mut self) {
        if let Some(Screen::Changes { hide_untracked, .. }) = self.screens.last_mut() {
            *hide_untracked = !*hide_untracked;
        }
    }

    /// The context-sensitive select/drill-in action (the `Enter` key). Overlays take priority.
    pub fn select(&mut self, prikk: &impl Prikk) {
        match self.overlays.last() {
            Some(Overlay::RefPicker { refs, cursor }) => {
                if let Some(name) = refs.get(*cursor).cloned() {
                    self.overlays.pop();
                    self.focused_ref = name;
                    if matches!(self.screens.last(), Some(Screen::History { .. })) {
                        self.screens.pop();
                    }
                    self.open_history(prikk);
                }
            }
            Some(Overlay::Refusal { card, cursor }) => {
                if let Some(step) = card.next_steps.get(*cursor) {
                    let target = step.target;
                    self.overlays.pop();
                    self.activate(target, prikk);
                }
            }
            Some(Overlay::Palette { filter, cursor, .. }) => {
                let hits = stikk_core::palette::matching(filter);
                if let Some(cmd) = hits.get(*cursor).copied() {
                    let capability = self.capability();
                    if cmd.available_to(capability) {
                        self.overlays.pop();
                        self.run_command(cmd, prikk);
                    }
                }
            }
            Some(Overlay::Refusals { records, cursor }) => {
                if let Some(record) = records.get(*cursor).cloned() {
                    self.overlays.pop();
                    // Rebuild the card from the remembered refusal (history holds refusals only).
                    let err = stikk_model::StikkError::Refusal {
                        message: record.verbatim.clone(),
                    };
                    if let Presentation::RefusalOverlay(card) = present(&err, record.operation) {
                        self.overlays.push(Overlay::Refusal { card, cursor: 0 });
                    }
                }
            }
            Some(Overlay::Glossary) => {}
            None => self.select_screen(prikk),
        }
    }

    /// `Enter` with no overlay open: drill into a block, or open History from the root.
    fn select_screen(&mut self, prikk: &impl Prikk) {
        match self.screens.last() {
            Some(Screen::History { view, cursor }) => {
                let cursor = *cursor;
                if let Some(row) = view.blocks.get(cursor).cloned() {
                    let is_tip = cursor == 0;
                    let reff = self.focused_ref.clone();
                    match block_detail(prikk, &self.repo, &reff, row, is_tip) {
                        Ok(detail) => self.screens.push(Screen::BlockDetail(detail)),
                        Err(error) => self.surface(&error, OperationContext::LoadBlockState),
                    }
                }
            }
            Some(Screen::BlockDetail(_)) => {}
            // Enter on a Changes row would open a per-file content diff — deferred (UD-09); the view
            // states so. No drill-in.
            Some(Screen::Changes { .. }) => {}
            None => self.open_history(prikk),
        }
    }

    /// Open the ref picker overlay, sourcing the ref list through the seam (the `b` key).
    pub fn open_ref_picker(&mut self, prikk: &impl Prikk) {
        self.banner = None;
        match list_refs(prikk, &self.repo) {
            Ok(entries) => {
                let refs: Vec<String> = entries.into_iter().map(|entry| entry.name).collect();
                let cursor = refs
                    .iter()
                    .position(|name| name == &self.focused_ref)
                    .unwrap_or(0);
                self.overlays.push(Overlay::RefPicker { refs, cursor });
            }
            Err(error) => self.surface(&error, OperationContext::ListRefs),
        }
    }

    /// Open the glossary / help browser (the `?` key).
    pub fn open_glossary(&mut self) {
        if matches!(self.overlays.last(), Some(Overlay::Glossary)) {
            self.overlays.pop();
        } else {
            self.overlays.push(Overlay::Glossary);
        }
    }

    /// Open the command palette (the `:` key).
    pub fn open_palette(&mut self) {
        let capability = self.capability();
        self.overlays.push(Overlay::Palette {
            filter: String::new(),
            cursor: 0,
            capability,
        });
    }

    /// Open the session refusal history (the `R` key).
    pub fn open_refusals(&mut self) {
        let records = self
            .refusals
            .recent()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        self.overlays.push(Overlay::Refusals { records, cursor: 0 });
    }

    /// Type a character (only meaningful while the palette is open — text entry).
    pub fn input_char(&mut self, ch: char) {
        if let Some(Overlay::Palette { filter, cursor, .. }) = self.overlays.last_mut() {
            filter.push(ch);
            *cursor = 0;
        }
    }

    /// Delete the last filter character (palette text entry).
    pub fn backspace(&mut self) {
        if let Some(Overlay::Palette { filter, cursor, .. }) = self.overlays.last_mut() {
            filter.pop();
            *cursor = 0;
        }
    }

    /// Whether the top overlay is a text-entry surface (so the key layer routes chars to it).
    #[must_use]
    pub fn wants_text_input(&self) -> bool {
        matches!(self.overlays.last(), Some(Overlay::Palette { .. }))
    }

    /// Move the selection up (the top overlay's cursor if it has one, else the History cursor).
    pub fn nav_up(&mut self) {
        match self.overlays.last_mut() {
            Some(Overlay::RefPicker { cursor, .. })
            | Some(Overlay::Refusal { cursor, .. })
            | Some(Overlay::Palette { cursor, .. })
            | Some(Overlay::Refusals { cursor, .. }) => *cursor = cursor.saturating_sub(1),
            Some(Overlay::Glossary) => {}
            None => {
                if let Some(Screen::History { cursor, .. }) = self.screens.last_mut() {
                    *cursor = cursor.saturating_sub(1);
                }
            }
        }
    }

    /// Move the selection down, clamped to the active list length.
    pub fn nav_down(&mut self) {
        match self.overlays.last_mut() {
            Some(Overlay::RefPicker { refs, cursor }) => *cursor = next_index(*cursor, refs.len()),
            Some(Overlay::Refusal { card, cursor }) => {
                *cursor = next_index(*cursor, card.next_steps.len());
            }
            Some(Overlay::Refusals { records, cursor }) => {
                *cursor = next_index(*cursor, records.len());
            }
            Some(Overlay::Palette { filter, cursor, .. }) => {
                let count = stikk_core::palette::matching(filter).len();
                *cursor = next_index(*cursor, count);
            }
            Some(Overlay::Glossary) => {}
            None => {
                if let Some(Screen::History { view, cursor }) = self.screens.last_mut() {
                    *cursor = next_index(*cursor, view.blocks.len());
                }
            }
        }
    }

    /// Go back one step: dismiss a fault or banner, close the top overlay, pop a screen, else quit.
    pub fn back(&mut self) {
        if self.fault.take().is_some() {
            return;
        }
        if self.overlays.pop().is_some() {
            return;
        }
        if self.banner.take().is_some() {
            return;
        }
        if self.screens.pop().is_some() {
            return;
        }
        self.should_quit = true;
    }

    /// Route a seam error through the one presentation mapping (ER-03) and surface it accordingly.
    fn surface(&mut self, error: &stikk_model::StikkError, op: OperationContext) {
        match present(error, op) {
            Presentation::RefusalOverlay(card) => {
                self.refusals.record(card.verbatim.clone(), "refusal", op);
                self.overlays.push(Overlay::Refusal { card, cursor: 0 });
            }
            Presentation::Banner { message, .. }
            | Presentation::RoutedIntoView { message, .. }
            | Presentation::InConfirmation { message } => self.banner = Some(message),
            Presentation::InlineGuidance { detail, .. } => {
                self.banner = Some(format!("{detail} — see Glossary → Trust & Keys"));
            }
            Presentation::PlainStatement { detail, original } => {
                self.banner = Some(match original {
                    Some(cause) => format!("{detail}: {cause}"),
                    None => detail,
                });
            }
            Presentation::FaultScreen { detail } => self.fault = Some(detail),
            // `Presentation` is `#[non_exhaustive]`: a class added later degrades to an honest banner
            // carrying prikk's own text, never a panic (RR-5 discipline).
            _ => self.banner = Some(error.to_string()),
        }
    }

    /// Activate a refusal card's next-step target (navigational only — NFR-S04).
    fn activate(&mut self, target: NextTarget, prikk: &impl Prikk) {
        match target {
            NextTarget::OpenView(view) => self.activate_view(view, prikk),
            NextTarget::Refresh => self.reload(prikk),
            NextTarget::DismissAndResolveExternally => {}
            _ => {}
        }
    }

    /// Navigate to a view/overlay target (shared by refusal next-steps and the palette).
    fn activate_view(&mut self, target: Target, prikk: &impl Prikk) {
        self.banner = None;
        match target {
            Target::Orientation => self.screens.clear(),
            Target::History => self.open_history(prikk),
            Target::RefPicker => self.open_ref_picker(prikk),
            Target::Changes => self.open_changes(prikk),
            Target::Glossary => self.overlays.push(Overlay::Glossary),
            // Targets whose views land in later increments: no-op for now (the mapping is complete).
            Target::LockInspector | Target::TrustKeys | Target::Verify | Target::Doctor => {}
            _ => {}
        }
    }

    /// Run a palette command (already checked available).
    fn run_command(&mut self, cmd: &Command, prikk: &impl Prikk) {
        if let Some(target) = cmd.opens {
            self.activate_view(target, prikk);
            return;
        }
        match cmd.id {
            "view.refresh" => self.reload(prikk),
            "session.refusals" => self.open_refusals(),
            "app.quit" => self.should_quit = true,
            _ => {}
        }
    }

    /// The session's derived capability (Viewer when no orientation is loaded).
    #[must_use]
    fn capability(&self) -> Capability {
        match &self.state {
            OrientationState::Loaded(view) => view.capability,
            _ => Capability::Viewer,
        }
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

    /// The transient banner, if any (design OP-03 banner/inline classes).
    #[must_use]
    pub fn banner(&self) -> Option<&str> {
        self.banner.as_deref()
    }

    /// The active fault message, if a stikk-internal fault occurred (ER-04).
    #[must_use]
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    /// The session refusal history (FR-112), for the recent-refusals overlay and tests.
    #[must_use]
    pub fn refusals(&self) -> &RefusalHistory {
        &self.refusals
    }

    /// What the shell should render in the body region.
    #[must_use]
    pub fn focus(&self) -> Focus<'_> {
        match self.screens.last() {
            Some(Screen::History { view, cursor }) => Focus::History(view, *cursor),
            Some(Screen::BlockDetail(detail)) => Focus::BlockDetail(detail),
            Some(Screen::Changes {
                view,
                hide_untracked,
            }) => Focus::Changes(view, *hide_untracked),
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

    /// Close the top overlay, if any.
    pub fn close_overlay(&mut self) {
        self.overlays.pop();
    }

    /// Push a screen directly, without loading through the seam — for tests and the example.
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

    /// Surface a seam error directly — for tests and the example (drives the ER-03 routing).
    pub fn surface_error(&mut self, error: &stikk_model::StikkError, op: OperationContext) {
        self.surface(error, op);
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
