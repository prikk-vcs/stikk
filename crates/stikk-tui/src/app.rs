//! The application state and its transitions (handoff §2/§7 `app.rs`; RFC 006/007; RFC 010).
//!
//! The app owns the view-model stack, the overlay stack, the focused ref, the session refusal history,
//! and the palette colours. It **holds no repository authority** (design INV-8): every view-model is a
//! rendered snapshot of what prikk reported, re-sourced on demand. The only repository facts come from
//! `stikk_core` — the app computes nothing, and every seam error is routed through the one
//! [`stikk_core::present`] mapping (ER-03), never presented ad hoc.
//!
//! **The seam runs off the UI thread (RFC 010).** `App` no longer calls `stikk-core` directly: every
//! method that used to take `prikk: &impl Prikk` now *sends* a [`crate::worker::Request`] to the worker
//! over a channel it owns, and [`App::apply`] is the one entry point for the eventual
//! [`crate::worker::Response`]. A request carries a sequence number; a response for a sequence the app
//! is no longer waiting on is stale — the user has navigated away — and is discarded (§ stale
//! responses, below). This is not optional polish: without it, a slow read whose result lands after the
//! user has moved on would push a screen or overlay they never asked for.
//!
//! Navigation is a stack of [`Screen`]s above the Orientation root; overlays (help/glossary, ref
//! picker, refusal, palette, refusal history, background operations) stack above the active view.
//! [`App::back`] closes the top overlay, else pops a screen (including a pending [`Screen::Loading`] —
//! this is the "stop waiting" semantics RFC 010 decision 4 describes), else quits.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use stikk_core::{
    BlockDetailView, ChangesView, Command, CommitPreviewOutcome, HistoryView, NextTarget,
    OperationContext, Presentation, RefusalHistory, Target, present,
};
use stikk_model::Tier;
use stikk_state::Config;

use crate::overlay::Overlay;
use crate::theme::Palette;
use crate::worker::{Request, RequestKind, Response, ResponseKind};

/// The default focused ref — prikk has no HEAD, so stikk focuses a named ref explicitly (FR-055).
const DEFAULT_REF: &str = "heads/main";

/// The most background operations `App` remembers, for the Background Operations overlay (TU-01).
/// Display-only bookkeeping; bounded so a long session's list does not grow forever.
const OPERATIONS_CAP: usize = 20;

/// The Orientation view's load state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrientationState {
    /// No orientation has been loaded yet — real and observable now that the load runs off-thread
    /// (RFC 010 §5; it existed before but no user could ever see it).
    Loading,
    /// A successfully loaded orientation.
    Loaded(stikk_core::OrientationView),
    /// A load failure, carrying prikk's verbatim message (design NFR-I03).
    Failed(String),
}

/// A screen pushed above the Orientation root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// A screen asked for but not yet arrived (RFC 010 §5). [`App::apply`] replaces it on success or
    /// removes it on error (surfacing the error via the usual overlay/banner); [`App::back`] pops it
    /// directly like any other screen — the "stop waiting" semantics (RFC 010 decision 4).
    Loading {
        /// A short label for what is loading (e.g. `"history"`), for the loading note and the
        /// Background Operations overlay.
        what: &'static str,
        /// The request this placeholder is waiting on; a response for any other `seq` is stale here.
        seq: u64,
    },
    /// A ref's block history, with the selection cursor into `view.blocks`.
    History {
        /// The loaded lineage.
        view: HistoryView,
        /// The selected block index (0 = tip).
        cursor: usize,
        /// The `seq` of an in-flight refresh (`r`/reload), if any. Unlike [`Screen::Loading`], a
        /// refresh does not blank an already-loaded screen while it is in flight (RFC 010 §5) — the
        /// stale view stays visible, distinguished only by the `⟳ n` indicator, until the refresh
        /// resolves or is superseded by a newer one.
        refreshing: Option<u64>,
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
    /// A screen asked for but not yet arrived; `what` names it (e.g. `"history"`).
    Loading(&'static str),
    /// A History screen and its selection cursor.
    History(&'a HistoryView, usize),
    /// A Block-detail screen.
    BlockDetail(&'a BlockDetailView),
    /// The Changes view and whether untracked entries are hidden.
    Changes(&'a ChangesView, bool),
}

/// One background operation the worker has answered or is still working on — display-only bookkeeping
/// for the `⟳ n` status-bar indicator (TU-03) and the Background Operations overlay (TU-01). Never
/// authority (INV-8): nothing about a screen's own state depends on this list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// The request's sequence number.
    pub seq: u64,
    /// A short label naming the kind of work (e.g. `"history"`).
    pub label: &'static str,
    /// Whether it is still running or has finished.
    pub status: OperationStatus,
}

/// Whether a background [`Operation`] is still running or has finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    /// The worker has not yet replied.
    Running,
    /// The worker replied. `ok` is whether the result was a success — never *what* the result was
    /// (INV-8); the overlay is a listing, not a content view.
    Finished {
        /// Whether the operation succeeded.
        ok: bool,
    },
}

/// The running application.
pub struct App {
    repo: PathBuf,
    state: OrientationState,
    /// The `seq` of the orientation request currently awaited, if any — matched in [`App::apply`]
    /// against both the initial load and any later refresh (`reload`), which share this one slot since
    /// there is only ever one "current" orientation.
    orientation_pending: Option<u64>,
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
    next_seq: u64,
    /// The channel to the worker thread (RFC 010). Sending is best-effort: if the worker has already
    /// stopped, the send is dropped and the caller's pending state simply never resolves — the same
    /// "stop waiting" posture as a stale response, not a crash.
    req_tx: mpsc::Sender<Request>,
    operations: Vec<Operation>,
    /// The commit flow's state between overlays (RFC 014 §3) — not carried on [`Overlay::Confirmation`]
    /// itself, since [`stikk_core::PreviewToken`] has no public constructor and no `Clone`/`PartialEq`,
    /// so it cannot live on a type ([`Overlay`]) that derives those. `App` is the one place that holds
    /// it, across exactly the two round trips the flow needs (preview, then confirm+execute).
    pending_commit: Option<PendingCommit>,
}

/// See [`App::pending_commit`].
enum PendingCommit {
    /// The message step finished; a [`RequestKind::CommitPreview`] is in flight for `reff`.
    AwaitingPreview {
        /// The ref this commit targets.
        reff: String,
        /// The message the user typed (`FL-05` step 2), carried through to execution.
        message: String,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` is on the stack awaiting
    /// evidence.
    Confirming {
        /// Feed this into [`stikk_core::commit_confirm_and_execute`] once evidence is supplied.
        token: stikk_core::PreviewToken,
        /// The ref this commit targets (must match the preview's).
        reff: String,
        /// The message the user typed.
        message: String,
    },
}

impl App {
    /// Open `repo` and send the initial orientation request, building the palette from `config`. The
    /// caller (normally [`crate::run`]) owns the paired worker and receiver.
    #[must_use]
    pub(crate) fn open(
        repo: impl Into<PathBuf>,
        config: &Config,
        req_tx: mpsc::Sender<Request>,
    ) -> Self {
        let mut app = Self::new(
            repo,
            OrientationState::Loading,
            Palette::from_theme(config.theme),
            req_tx,
        );
        let seq = app.dispatch(RequestKind::Orient);
        app.orientation_pending = Some(seq);
        app
    }

    /// Construct an app in an explicit state, without sending any request — used by tests so they can
    /// drive [`App::apply`] directly without a real load (handoff §8: "do not write a test that
    /// sleeps").
    #[cfg(test)]
    #[must_use]
    pub(crate) fn from_state(
        repo: impl Into<PathBuf>,
        state: OrientationState,
        palette: Palette,
        req_tx: mpsc::Sender<Request>,
    ) -> Self {
        Self::new(repo, state, palette, req_tx)
    }

    fn new(
        repo: impl Into<PathBuf>,
        state: OrientationState,
        palette: Palette,
        req_tx: mpsc::Sender<Request>,
    ) -> Self {
        Self {
            repo: repo.into(),
            state,
            orientation_pending: None,
            focused_ref: DEFAULT_REF.to_string(),
            screens: Vec::new(),
            overlays: Vec::new(),
            refusals: RefusalHistory::new(),
            banner: None,
            fault: None,
            palette,
            should_quit: false,
            next_seq: 0,
            req_tx,
            operations: Vec::new(),
            pending_commit: None,
        }
    }

    /// Send a request to the worker, recording it as running for the background-operations bookkeeping
    /// (TU-01/TU-03), and return its sequence number so the caller can record where the eventual
    /// response should land (an `orientation_pending`/`refreshing` slot, or a pushed `Screen::Loading`/
    /// `Overlay::Loading`).
    fn dispatch(&mut self, kind: RequestKind) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        let label = kind.label();
        self.operations.push(Operation {
            seq,
            label,
            status: OperationStatus::Running,
        });
        if self.operations.len() > OPERATIONS_CAP {
            self.operations.remove(0);
        }
        // Best-effort: if the worker has already stopped, this is dropped and the slot this seq was
        // meant to fill simply never resolves — the same posture as a stale response, not a crash.
        let _ = self.req_tx.send(Request { seq, kind });
        seq
    }

    fn record_finished(&mut self, seq: u64, ok: bool) {
        if let Some(op) = self.operations.iter_mut().find(|op| op.seq == seq) {
            op.status = OperationStatus::Finished { ok };
        }
    }

    /// Re-source the visible screens from prikk (design FR-106; the `r` key): always re-requests
    /// Orientation, and — if the top screen is History — re-requests that too, in place. The current
    /// view stays visible while either refresh is in flight (RFC 010 §5); only the `⟳ n` indicator
    /// shows anything is happening.
    pub fn reload(&mut self) {
        self.banner = None;
        self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
        if matches!(self.screens.last(), Some(Screen::History { .. })) {
            let reff = self.focused_ref.clone();
            let seq = self.dispatch(RequestKind::History { reff });
            if let Some(Screen::History { refreshing, .. }) = self.screens.last_mut() {
                *refreshing = Some(seq);
            }
        }
    }

    /// Open the History view for the focused ref, pushing a pending placeholder above the current
    /// screen until the response arrives.
    pub fn open_history(&mut self) {
        self.banner = None;
        let reff = self.focused_ref.clone();
        let seq = self.dispatch(RequestKind::History { reff });
        self.screens.push(Screen::Loading {
            what: "history",
            seq,
        });
    }

    /// Open the Changes (worktree-vs-baseline) view for the focused ref (the `w` key). Below prikk
    /// 0.28 the version gate surfaces guidance rather than the broken command (FR-034/UD-03) — that
    /// still arrives as an error response and is surfaced the same way as any other.
    pub fn open_changes(&mut self) {
        self.banner = None;
        let reff = self.focused_ref.clone();
        let seq = self.dispatch(RequestKind::Changes { reff });
        self.screens.push(Screen::Loading {
            what: "changes",
            seq,
        });
    }

    /// Toggle the display-only untracked filter on a focused Changes screen (the `u` key; UD-08).
    pub fn toggle_untracked(&mut self) {
        if let Some(Screen::Changes { hide_untracked, .. }) = self.screens.last_mut() {
            *hide_untracked = !*hide_untracked;
        }
    }

    /// Begin the commit flow (`FL-05` step 1; the `C` key; RFC 014 §3). Checked here, before the
    /// message prompt even opens (`C-T4d`): a session that cannot commit sees why immediately, rather
    /// than typing a message only to be refused at confirmation. This is the same
    /// [`stikk_core::capability_gate`] check the palette's `op.commit` entry uses (RFC 014 §6) — the two
    /// must agree.
    pub fn begin_commit(&mut self) {
        self.banner = None;
        let readiness = self.readiness();
        match stikk_core::capability_gate(stikk_core::COMMIT_OPERATION, Tier::Two, readiness) {
            Ok(()) => {
                self.overlays.push(Overlay::CommitMessage {
                    reff: self.focused_ref.clone(),
                    typed: String::new(),
                });
            }
            Err(stikk_model::StikkError::NotReady { detail }) => self.banner = Some(detail),
            // `capability_gate` only ever returns `NotReady`; kept exhaustive so a future change to
            // that contract fails here loudly rather than silently swallowing a new error shape.
            Err(other) => self.banner = Some(other.to_string()),
        }
    }

    /// `FL-05` step 2 → 3: the message step finished (`Enter` on [`Overlay::CommitMessage`]) — dispatch
    /// the preview, which does the fresh reads and the cross-ref/clean-worktree prevention (RFC 014
    /// §3). A blank message is refused here rather than sent (`UD-01`: required non-empty).
    fn submit_commit_message(&mut self, reff: String, typed: &str) {
        let message = typed.trim().to_string();
        if message.is_empty() {
            return;
        }
        self.overlays.pop();
        let seq = self.dispatch(RequestKind::CommitPreview { reff: reff.clone() });
        self.overlays.push(Overlay::Loading {
            what: "commit preview",
            seq,
        });
        self.pending_commit = Some(PendingCommit::AwaitingPreview { reff, message });
    }

    /// The confirmation step's `Enter`: dispatch confirm+execute with whatever evidence the tier needs.
    /// Tier 2 (commit's own tier) needs only an explicit yes; the typed-name shape stays reachable for
    /// whichever tier-3-typed operation lands next, matching this overlay's general contract (RFC 013
    /// §6) even though nothing at tier 3-typed drives it through `App` yet.
    fn submit_commit_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingCommit::Confirming {
            token,
            reff,
            message,
        }) = self.pending_commit.take()
        else {
            return; // no commit is actually pending — a stray Enter on a Confirmation this app did not open
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::CommitConfirmExecute {
            token,
            readiness,
            evidence,
            reff,
            message,
        });
        self.overlays.push(Overlay::Loading {
            what: "commit",
            seq,
        });
    }

    /// The context-sensitive select/drill-in action (the `Enter` key). Overlays take priority.
    pub fn select(&mut self) {
        match self.overlays.last() {
            Some(Overlay::RefPicker { refs, cursor }) => {
                if let Some(name) = refs.get(*cursor).cloned() {
                    self.overlays.pop();
                    self.focused_ref = name;
                    if matches!(self.screens.last(), Some(Screen::History { .. })) {
                        self.screens.pop();
                    }
                    self.open_history();
                }
            }
            Some(Overlay::Refusal { card, cursor }) => {
                if let Some(step) = card.next_steps.get(*cursor) {
                    let target = step.target;
                    self.overlays.pop();
                    self.activate(target);
                }
            }
            Some(Overlay::Stale {
                next_steps, cursor, ..
            }) => {
                if let Some(step) = next_steps.get(*cursor) {
                    let target = step.target;
                    self.overlays.pop();
                    self.activate(target);
                }
            }
            Some(Overlay::Palette { filter, cursor, .. }) => {
                let hits = stikk_core::palette::matching(filter);
                if let Some(cmd) = hits.get(*cursor).copied() {
                    let readiness = self.readiness();
                    if cmd.available_to(readiness) {
                        self.overlays.pop();
                        self.run_command(cmd);
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
            Some(Overlay::CommitMessage { reff, typed }) => {
                let (reff, typed) = (reff.clone(), typed.clone());
                self.submit_commit_message(reff, &typed);
            }
            // RFC 014 §3: commit is this overlay's first consumer (RFC 013 shipped it unwired).
            Some(Overlay::Confirmation { tier, typed, .. }) => {
                let (tier, typed) = (*tier, typed.clone());
                self.submit_commit_confirmation(tier, &typed);
            }
            // A commit result is dismissed like any other content overlay — no drill-in.
            Some(
                Overlay::Glossary
                | Overlay::Operations { .. }
                | Overlay::Loading { .. }
                | Overlay::CommitResult { .. },
            )
            | None => {
                if self.overlays.is_empty() {
                    self.select_screen();
                }
            }
        }
    }

    /// `Enter` with no overlay open: drill into a block, or open History from the root.
    fn select_screen(&mut self) {
        match self.screens.last() {
            Some(Screen::History { view, cursor, .. }) => {
                let cursor = *cursor;
                if let Some(row) = view.blocks.get(cursor).cloned() {
                    let is_tip = cursor == 0;
                    let reff = self.focused_ref.clone();
                    let seq = self.dispatch(RequestKind::BlockState { reff, row, is_tip });
                    self.screens.push(Screen::Loading {
                        what: "block detail",
                        seq,
                    });
                }
            }
            // Nothing to drill into further, and nothing to do while a load is pending.
            Some(Screen::BlockDetail(_) | Screen::Loading { .. }) => {}
            // Enter on a Changes row would open a per-file content diff — deferred (UD-09); the view
            // states so. No drill-in.
            Some(Screen::Changes { .. }) => {}
            None => self.open_history(),
        }
    }

    /// Open the ref picker overlay, sourcing the ref list through the seam (the `b` key).
    pub fn open_ref_picker(&mut self) {
        self.banner = None;
        let seq = self.dispatch(RequestKind::Refs);
        self.overlays.push(Overlay::Loading { what: "refs", seq });
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
        let readiness = self.readiness();
        self.overlays.push(Overlay::Palette {
            filter: String::new(),
            cursor: 0,
            readiness,
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

    /// Open the Background Operations overlay (the `o` key; TU-01) — a snapshot listing, no cancel
    /// action (RFC 010 decision 6).
    pub fn open_operations(&mut self) {
        if matches!(self.overlays.last(), Some(Overlay::Operations { .. })) {
            self.overlays.pop();
        } else {
            self.overlays.push(Overlay::Operations {
                operations: self.operations.clone(),
            });
        }
    }

    /// Type a character — the palette's filter, a tier-3-typed confirmation's typed name (RFC 013 §6),
    /// or the commit message (RFC 014 §3, reusing this same plumbing).
    pub fn input_char(&mut self, ch: char) {
        match self.overlays.last_mut() {
            Some(Overlay::Palette { filter, cursor, .. }) => {
                filter.push(ch);
                *cursor = 0;
            }
            Some(Overlay::Confirmation { tier, typed, .. }) if *tier == Tier::ThreeTyped => {
                typed.push(ch);
            }
            Some(Overlay::CommitMessage { typed, .. }) => typed.push(ch),
            _ => {}
        }
    }

    /// Delete the last typed character — the palette's filter, a tier-3-typed confirmation's typed
    /// name, or the commit message.
    pub fn backspace(&mut self) {
        match self.overlays.last_mut() {
            Some(Overlay::Palette { filter, cursor, .. }) => {
                filter.pop();
                *cursor = 0;
            }
            Some(Overlay::Confirmation { tier, typed, .. }) if *tier == Tier::ThreeTyped => {
                typed.pop();
            }
            Some(Overlay::CommitMessage { typed, .. }) => {
                typed.pop();
            }
            _ => {}
        }
    }

    /// Whether the top overlay is a text-entry surface (so the key layer routes chars to it). A
    /// confirmation overlay wants text only at [`Tier::ThreeTyped`] (RFC 013 §6) — tiers 2/3 take a
    /// plain yes/no, not typed input. The commit message prompt (RFC 014 §3) always wants text.
    #[must_use]
    pub fn wants_text_input(&self) -> bool {
        matches!(
            self.overlays.last(),
            Some(Overlay::Palette { .. } | Overlay::CommitMessage { .. })
        ) || matches!(
            self.overlays.last(),
            Some(Overlay::Confirmation { tier, .. }) if *tier == Tier::ThreeTyped
        )
    }

    /// Move the selection up (the top overlay's cursor if it has one, else the History cursor).
    pub fn nav_up(&mut self) {
        match self.overlays.last_mut() {
            Some(Overlay::RefPicker { cursor, .. })
            | Some(Overlay::Refusal { cursor, .. })
            | Some(Overlay::Stale { cursor, .. })
            | Some(Overlay::Palette { cursor, .. })
            | Some(Overlay::Refusals { cursor, .. }) => *cursor = cursor.saturating_sub(1),
            // A single prompt or content pane, not a list — no cursor to move (RFC 013 §6/RFC 014 §3).
            Some(
                Overlay::Glossary
                | Overlay::Operations { .. }
                | Overlay::Loading { .. }
                | Overlay::Confirmation { .. }
                | Overlay::CommitMessage { .. }
                | Overlay::CommitResult { .. },
            ) => {}
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
            Some(Overlay::Stale {
                next_steps, cursor, ..
            }) => {
                *cursor = next_index(*cursor, next_steps.len());
            }
            Some(Overlay::Refusals { records, cursor }) => {
                *cursor = next_index(*cursor, records.len());
            }
            Some(Overlay::Palette { filter, cursor, .. }) => {
                let count = stikk_core::palette::matching(filter).len();
                *cursor = next_index(*cursor, count);
            }
            Some(
                Overlay::Glossary
                | Overlay::Operations { .. }
                | Overlay::Loading { .. }
                | Overlay::Confirmation { .. }
                | Overlay::CommitMessage { .. }
                | Overlay::CommitResult { .. },
            ) => {}
            None => {
                if let Some(Screen::History { view, cursor, .. }) = self.screens.last_mut() {
                    *cursor = next_index(*cursor, view.blocks.len());
                }
            }
        }
    }

    /// Go back one step: dismiss a fault or banner, close the top overlay, pop a screen (a pending
    /// [`Screen::Loading`] included — this is the "stop waiting" semantics, RFC 010 decision 4), else
    /// quit.
    pub fn back(&mut self) {
        if self.fault.take().is_some() {
            return;
        }
        if let Some(overlay) = self.overlays.pop() {
            // Cancelling the message step or a confirmation abandons the commit it belonged to (RFC
            // 014 §3) — otherwise a stale `PreviewToken` would sit unused until a later, unrelated
            // commit's response arrived and (harmlessly, but confusingly) found nothing to attach it
            // to. Popping either overlay is exactly "the user is not doing this anymore."
            if matches!(
                overlay,
                Overlay::CommitMessage { .. } | Overlay::Confirmation { .. }
            ) {
                self.pending_commit = None;
            }
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

    /// Apply a response from the worker (RFC 010's one entry point for results). A response whose
    /// `seq` does not match what is currently awaited for its slot is stale — the user has navigated
    /// away since it was requested — and its content is discarded; the background-operations
    /// bookkeeping still records that the operation finished either way (§4/§5).
    pub(crate) fn apply(&mut self, response: Response) {
        let Response { seq, kind } = response;
        let ok = match &kind {
            ResponseKind::Orient(r) => r.is_ok(),
            ResponseKind::History(r) => r.is_ok(),
            ResponseKind::BlockState(r) => r.is_ok(),
            ResponseKind::Refs(r) => r.is_ok(),
            ResponseKind::Changes(r) => r.is_ok(),
            ResponseKind::CommitPreview(r) => r.is_ok(),
            ResponseKind::CommitConfirmExecute(r) => r.is_ok(),
        };
        self.record_finished(seq, ok);
        match kind {
            ResponseKind::Orient(result) => self.apply_orient(seq, result),
            ResponseKind::History(result) => self.apply_history(seq, result),
            ResponseKind::BlockState(result) => self.apply_block_state(seq, result),
            ResponseKind::Refs(result) => self.apply_refs(seq, result),
            ResponseKind::Changes(result) => self.apply_changes(seq, result),
            ResponseKind::CommitPreview(result) => self.apply_commit_preview(seq, result),
            ResponseKind::CommitConfirmExecute(result) => {
                self.apply_commit_confirm_execute(seq, result);
            }
        }
    }

    fn apply_orient(&mut self, seq: u64, result: stikk_model::Result<stikk_core::OrientationView>) {
        if self.orientation_pending != Some(seq) {
            return; // stale: a newer orientation request has since superseded this one
        }
        self.orientation_pending = None;
        self.state = match result {
            Ok(view) => OrientationState::Loaded(view),
            Err(error) => OrientationState::Failed(error.to_string()),
        };
    }

    fn apply_history(&mut self, seq: u64, result: stikk_model::Result<HistoryView>) {
        // Case 1: a reload-refresh of the top History screen — the view stays visible on error.
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::History { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            match result {
                Ok(new_view) => {
                    if let Some(Screen::History {
                        view,
                        cursor,
                        refreshing,
                    }) = self.screens.last_mut()
                    {
                        *cursor = clamp_cursor(*cursor, new_view.blocks.len());
                        *view = new_view;
                        *refreshing = None;
                    }
                }
                Err(error) => {
                    if let Some(Screen::History { refreshing, .. }) = self.screens.last_mut() {
                        *refreshing = None;
                    }
                    self.surface(&error, OperationContext::LoadHistory);
                }
            }
            return;
        }

        // Case 2: a pushed `Screen::Loading` placeholder somewhere in the stack.
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(view) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::History {
                            view,
                            cursor: 0,
                            refreshing: None,
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    self.surface(&error, OperationContext::LoadHistory);
                }
            }
        }
        // Else: no matching slot anywhere — stale, discard.
    }

    fn apply_block_state(&mut self, seq: u64, result: stikk_model::Result<BlockDetailView>) {
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(detail) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::BlockDetail(detail);
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    self.surface(&error, OperationContext::LoadBlockState);
                }
            }
        }
    }

    fn apply_changes(&mut self, seq: u64, result: stikk_model::Result<ChangesView>) {
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(view) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::Changes {
                            view,
                            hide_untracked: false,
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    self.surface(&error, OperationContext::LoadChanges);
                }
            }
        }
    }

    fn apply_refs(&mut self, seq: u64, result: stikk_model::Result<Vec<stikk_prikk::RefEntry>>) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(entries) => {
                    let refs: Vec<String> = entries.into_iter().map(|entry| entry.name).collect();
                    let cursor = refs
                        .iter()
                        .position(|name| name == &self.focused_ref)
                        .unwrap_or(0);
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::RefPicker { refs, cursor };
                    }
                }
                Err(error) => {
                    self.overlays.remove(index);
                    self.surface(&error, OperationContext::ListRefs);
                }
            }
        }
    }

    /// Answers [`RequestKind::CommitPreview`] (RFC 014 §3). Three outcomes, all replacing (or removing)
    /// the `Overlay::Loading` placeholder [`Self::submit_commit_message`] pushed: blocked (`C-T4d`,
    /// stikk's own words — never routed through `present()`, since neither condition is an error),
    /// ready (arms the confirmation), or a genuine seam failure (the usual `present()` routing).
    fn apply_commit_preview(
        &mut self,
        seq: u64,
        result: stikk_model::Result<CommitPreviewOutcome>,
    ) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the preview resolved
        };
        // Consumed regardless of outcome: a blocked or failed preview leaves nothing to confirm, and a
        // ready one hands its pieces to the new `Confirming` state below — never both.
        let pending = self.pending_commit.take();
        match result {
            Ok(CommitPreviewOutcome::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
            }
            Ok(CommitPreviewOutcome::Ready { preview: _, token }) => {
                let Some(PendingCommit::AwaitingPreview { reff, message }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: should not happen, but nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: token.summary().clone(),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_commit = Some(PendingCommit::Confirming {
                    token: *token,
                    reff,
                    message,
                });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Commit);
            }
        }
    }

    /// Answers [`RequestKind::CommitConfirmExecute`] (RFC 014 §3 step 4). On success, shows prikk's
    /// result verbatim and refreshes Orientation so the queue count reflects the new patch; on failure
    /// (`Stale`, `Declined`, the `CrossRef` race, or a genuine refusal), the usual `present()` routing.
    fn apply_commit_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::CommitResult>>,
    ) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the response arrived
        };
        match result {
            Ok(outcome) => {
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::CommitResult {
                        result: outcome.result,
                    };
                }
                self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Commit);
            }
        }
    }

    /// Record that the worker thread itself has stopped (RFC 010 §7/ER-04) — there is no
    /// [`stikk_model::StikkError`] to route through `present()` for "the channel closed", so this is a
    /// direct fault, stated exactly once by the caller (repeating it on every subsequent poll would
    /// make the fault screen impossible to dismiss).
    pub(crate) fn worker_stopped(&mut self) {
        self.fault = Some(
            "the background worker stopped unexpectedly; your session is preserved, but nothing new \
             can be loaded until you restart stikk"
                .to_string(),
        );
    }

    /// Route a seam error through the one presentation mapping (ER-03) and surface it accordingly.
    fn surface(&mut self, error: &stikk_model::StikkError, op: OperationContext) {
        match present(error, op) {
            Presentation::RefusalOverlay(card) => {
                self.refusals.record(card.verbatim.clone(), "refusal", op);
                self.overlays.push(Overlay::Refusal { card, cursor: 0 });
            }
            // Never folded into `RefusalOverlay`/`refusals` (design-review C1): nothing here is
            // prikk's message, so it must not be recorded as one nor rendered under prikk's label.
            Presentation::Stale {
                operation,
                gloss,
                next_steps,
            } => {
                self.overlays.push(Overlay::Stale {
                    operation,
                    gloss,
                    next_steps,
                    cursor: 0,
                });
            }
            Presentation::Banner { message, .. }
            | Presentation::RoutedIntoView { message, .. }
            | Presentation::InConfirmation { message } => self.banner = Some(message),
            Presentation::InlineGuidance { detail, toward } => {
                // RFC 012 F-b: the pointer is target-dependent — Trust & Keys is genuinely the fix for
                // absent signing readiness, but says nothing useful for a prikk-version gate, whose
                // `detail` is already the complete, actionable message on its own.
                self.banner = Some(match toward {
                    Target::TrustKeys => format!("{detail} — see Glossary → Trust & Keys"),
                    _ => detail,
                });
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
    fn activate(&mut self, target: NextTarget) {
        match target {
            NextTarget::OpenView(view) => self.activate_view(view),
            NextTarget::Refresh => self.reload(),
            NextTarget::DismissAndResolveExternally => {}
            _ => {}
        }
    }

    /// Navigate to a view/overlay target (shared by refusal next-steps and the palette).
    fn activate_view(&mut self, target: Target) {
        self.banner = None;
        match target {
            Target::Orientation => self.screens.clear(),
            Target::History => self.open_history(),
            Target::RefPicker => self.open_ref_picker(),
            Target::Changes => self.open_changes(),
            Target::Glossary => self.overlays.push(Overlay::Glossary),
            // Targets whose views land in later increments: no-op for now (the mapping is complete).
            Target::LockInspector | Target::TrustKeys | Target::Verify | Target::Doctor => {}
            _ => {}
        }
    }

    /// Run a palette command (already checked available).
    fn run_command(&mut self, cmd: &Command) {
        if let Some(target) = cmd.opens {
            self.activate_view(target);
            return;
        }
        match cmd.id {
            "view.refresh" => self.reload(),
            "session.refusals" => self.open_refusals(),
            "op.commit" => self.begin_commit(),
            "app.quit" => self.should_quit = true,
            _ => {}
        }
    }

    /// The session's signing readiness (RFC 014 §6) — the tier-aware palette affordance and
    /// [`Self::begin_commit`]'s pre-check both need this, not a derived [`stikk_model::Capability`]
    /// alone, since
    /// only `Readiness` carries whether read-only mode is on. Defaults to
    /// [`stikk_model::Readiness::none`] before orientation has loaded (Viewer-equivalent, matching
    /// every other pre-load default in this file).
    #[must_use]
    fn readiness(&self) -> stikk_model::Readiness {
        match &self.state {
            OrientationState::Loaded(view) => view.readiness,
            _ => stikk_model::Readiness::none(),
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

    /// The count of requests currently awaiting a response, for the `⟳ n` status-bar indicator
    /// (TU-03).
    #[must_use]
    pub fn in_flight_count(&self) -> usize {
        self.operations
            .iter()
            .filter(|op| op.status == OperationStatus::Running)
            .count()
    }

    /// The session's background operations, oldest-first, for the Background Operations overlay
    /// (TU-01).
    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// What the shell should render in the body region.
    #[must_use]
    pub fn focus(&self) -> Focus<'_> {
        match self.screens.last() {
            Some(Screen::Loading { what, .. }) => Focus::Loading(what),
            Some(Screen::History { view, cursor, .. }) => Focus::History(view, *cursor),
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

    /// Push a screen directly, without loading through the seam — for tests.
    #[cfg(test)]
    pub(crate) fn push_screen(&mut self, screen: Screen) {
        self.screens.push(screen);
    }

    /// Push an overlay directly — for tests (RFC 013's confirmation overlay has no live trigger yet).
    #[cfg(test)]
    pub(crate) fn push_overlay(&mut self, overlay: Overlay) {
        self.overlays.push(overlay);
    }

    /// Surface a seam error directly — for tests (drives the ER-03 routing).
    #[cfg(test)]
    pub(crate) fn surface_error(&mut self, error: &stikk_model::StikkError, op: OperationContext) {
        self.surface(error, op);
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
