//! The worker thread and the request/response protocol that connects it to [`crate::app::App`]
//! (design CC-01; RFC 010). These types are frontend↔worker plumbing, not `stikk-core` operations —
//! the operations themselves stay synchronous and unchanged (RFC 010 decision 3); this module is
//! simply the thread that calls them and the messages that cross the channel between it and the UI
//! thread. Crate-internal only: nothing outside `stikk-tui` needs to construct a [`Request`] or
//! [`Response`], since external callers only ever use [`crate::run`].

use std::path::Path;
use std::sync::mpsc;

use stikk_core::{
    BlockDetailView, ChangesView, CommitPreviewOutcome, Evidence, HistoryView, OrientationView,
    Outcome, PreviewToken, block_detail, changes_view, commit_confirm_and_execute, commit_preview,
    history_view, list_refs, orient,
};
use stikk_model::{Readiness, Result};
use stikk_prikk::{BlockRow, CommitResult, Prikk, RefEntry};

/// How many blocks the History view requests at a time (design FR-011 caps the listing).
pub(crate) const HISTORY_LIMIT: usize = 200;

/// One unit of work for the worker thread. Carries the sequence number its [`Response`] must echo, so
/// [`crate::app::App::apply`] can discard a reply for work the user has navigated away from since it
/// was sent (RFC 010 §4 — the correctness risk this increment exists to close).
#[derive(Debug)]
pub(crate) struct Request {
    pub(crate) seq: u64,
    pub(crate) kind: RequestKind,
}

/// The operation a [`Request`] asks the worker to run — one per `stikk-core` read this frontend drives.
/// Deliberately mirrors the design's request-category vocabulary at the granularity the TUI needs it,
/// not the full `CT-03` set (only the reads this frontend currently issues have a variant).
#[derive(Debug)]
pub(crate) enum RequestKind {
    /// Read the repository orientation.
    Orient,
    /// Read a ref's block lineage.
    History {
        /// The ref to list.
        reff: String,
    },
    /// Read a block's replayed state (populated only when `is_tip`; prikk replays only to the tip).
    BlockState {
        /// The ref `row` belongs to.
        reff: String,
        /// The block whose detail is being assembled.
        row: BlockRow,
        /// Whether `row` is the ref's tip.
        is_tip: bool,
    },
    /// List every ref pointer, for the ref picker.
    Refs,
    /// Read worktree-vs-baseline status for a ref.
    Changes {
        /// The ref to compare the worktree against.
        reff: String,
    },
    /// Build the commit preview for a ref (design `FL-05` step 3; RFC 014).
    CommitPreview {
        /// The ref to commit to.
        reff: String,
    },
    /// Confirm and execute a commit in one round trip (RFC 014 §3 step 4) — there is no user action
    /// between confirmation succeeding and execution starting, so this is one worker request, not two.
    CommitConfirmExecute {
        /// The token [`RequestKind::CommitPreview`] minted.
        token: PreviewToken,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        readiness: Readiness,
        /// The user's confirmation evidence (an explicit yes, at commit's tier 2).
        evidence: Evidence,
        /// The ref to commit to (must match the preview's).
        reff: String,
        /// The commit message, typed in the message step before this request was ever built.
        message: String,
    },
}

/// The worker's answer to one [`Request`], echoing its `seq`.
#[derive(Debug)]
pub(crate) struct Response {
    pub(crate) seq: u64,
    pub(crate) kind: ResponseKind,
}

/// The result of running the [`RequestKind`] a [`Response`] echoes.
#[derive(Debug)]
pub(crate) enum ResponseKind {
    /// Answers [`RequestKind::Orient`].
    Orient(Result<OrientationView>),
    /// Answers [`RequestKind::History`].
    History(Result<HistoryView>),
    /// Answers [`RequestKind::BlockState`].
    BlockState(Result<BlockDetailView>),
    /// Answers [`RequestKind::Refs`].
    Refs(Result<Vec<RefEntry>>),
    /// Answers [`RequestKind::Changes`].
    Changes(Result<ChangesView>),
    /// Answers [`RequestKind::CommitPreview`].
    CommitPreview(Result<CommitPreviewOutcome>),
    /// Answers [`RequestKind::CommitConfirmExecute`].
    CommitConfirmExecute(Result<Outcome<CommitResult>>),
}

/// A short, display-only label for the kind of work a [`RequestKind`] represents — used for the
/// pending-screen/overlay note and the Background Operations listing (TU-01/TU-03). Never repository
/// content; purely stikk's own naming of its own request.
impl RequestKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Orient => "orientation",
            Self::History { .. } => "history",
            Self::BlockState { .. } => "block detail",
            Self::Refs => "refs",
            Self::Changes { .. } => "changes",
            Self::CommitPreview { .. } => "commit preview",
            Self::CommitConfirmExecute { .. } => "commit",
        }
    }
}

/// The worker loop: receive a request, run the matching (synchronous, unchanged — RFC 010 decision 3)
/// `stikk-core` operation, send the response. Ends when `req_rx` disconnects, which happens when the UI
/// thread drops its sender on quit — no separate shutdown signal is needed.
///
/// Takes the channel halves **by value**, not by reference: neither `Receiver` nor `Sender` is `Sync`,
/// so a shared reference to either cannot cross into a spawned thread (only an owned value, or a
/// `Sender`'s own `Clone`, can) — this is `std::thread::scope`'s own sketch in the handoff, not an
/// incidental choice.
pub(crate) fn run(
    prikk: &impl Prikk,
    repo: &Path,
    req_rx: mpsc::Receiver<Request>,
    res_tx: mpsc::Sender<Response>,
) {
    while let Ok(Request { seq, kind }) = req_rx.recv() {
        let kind = match kind {
            RequestKind::Orient => ResponseKind::Orient(orient(prikk, repo)),
            RequestKind::History { reff } => {
                ResponseKind::History(history_view(prikk, repo, &reff, HISTORY_LIMIT))
            }
            RequestKind::BlockState { reff, row, is_tip } => {
                ResponseKind::BlockState(block_detail(prikk, repo, &reff, row, is_tip))
            }
            RequestKind::Refs => ResponseKind::Refs(list_refs(prikk, repo)),
            RequestKind::Changes { reff } => {
                ResponseKind::Changes(changes_view(prikk, repo, &reff))
            }
            RequestKind::CommitPreview { reff } => {
                ResponseKind::CommitPreview(commit_preview(prikk, repo, &reff))
            }
            RequestKind::CommitConfirmExecute {
                token,
                readiness,
                evidence,
                reff,
                message,
            } => ResponseKind::CommitConfirmExecute(commit_confirm_and_execute(
                prikk, repo, token, readiness, evidence, &reff, &message,
            )),
        };
        // The UI thread has quit and dropped its receiver; nothing left to deliver to.
        if res_tx.send(Response { seq, kind }).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
