//! The worker thread and the request/response protocol that connects it to [`crate::app::App`]
//! (design CC-01; RFC 010). These types are frontend↔worker plumbing, not `stikk-core` operations —
//! the operations themselves stay synchronous and unchanged (RFC 010 decision 3); this module is
//! simply the thread that calls them and the messages that cross the channel between it and the UI
//! thread. Crate-internal only: nothing outside `stikk-tui` needs to construct a [`Request`] or
//! [`Response`], since external callers only ever use [`crate::run`].

use std::path::Path;
use std::sync::mpsc;

use stikk_core::{
    BlockDetailView, ChangesRead, CommitPreviewOutcome, CommitToken, Evidence, HistoryView,
    OrientationView, Outcome, PreviewToken, SealPreviewOutcome, block_detail, change_token,
    changes_view, commit_confirm_and_execute, commit_preview, history_view, list_refs, orient,
    seal_confirm_and_execute, seal_preview,
};
use stikk_core::{QueueView, queue_view};
use stikk_model::{ChangeToken, Readiness, Result};
use stikk_prikk::{BlockRow, CommitResult, Prikk, RefEntry, SealResult};

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
    /// Read the change token, then the repository orientation (RFC 031 §3), answered together.
    Orient,
    /// Read the change token alone: the silent check (RFC 031 §4). `App` sends it without recording an
    /// operation, so it never appears in the Operations list or `⟳ n`.
    ChangeCheck,
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
    /// Read the active queue, for the Queue view (RFC 028).
    Queue,
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
        /// The token [`RequestKind::CommitPreview`] minted. It owns the previewed ref and view (RFC 030
        /// decision 3), so the commit is authored onto the previewed ref and no other. Boxed for the same
        /// reason `App` boxes it: it carries the whole previewed `ChangesView`.
        token: Box<CommitToken>,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        readiness: Readiness,
        /// The user's confirmation evidence (an explicit yes, at commit's tier 2).
        evidence: Evidence,
        /// The commit message, typed in the message step before this request was ever built.
        message: String,
    },
    /// Build the seal preview for a ref (design `FL-06`; RFC 016 §5/§6).
    SealPreview {
        /// The ref to seal.
        reff: String,
    },
    /// Confirm and execute a seal in one round trip (RFC 016 §5) — the no-audit consent step (§8) has
    /// already happened client-side, as its own act, before this request is ever built.
    SealConfirmExecute {
        /// The token [`RequestKind::SealPreview`] minted.
        token: PreviewToken,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        readiness: Readiness,
        /// The user's confirmation evidence (an explicit yes, at seal's tier 3 — untyped, RFC 013 Q3).
        evidence: Evidence,
        /// The ref to seal (must match the preview's).
        reff: String,
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
    Orient(OrientRead),
    /// Answers [`RequestKind::ChangeCheck`].
    ChangeCheck(Result<ChangeToken>),
    /// Answers [`RequestKind::History`].
    History(Result<HistoryView>),
    /// Answers [`RequestKind::BlockState`], echoing the ref it was read for, so the screen it fills names
    /// that ref and a later refresh re-reads it (RFC 031 review v1 §2.2).
    BlockState {
        reff: String,
        result: Result<BlockDetailView>,
    },
    /// Answers [`RequestKind::Refs`].
    Refs(Result<Vec<RefEntry>>),
    /// Answers [`RequestKind::Queue`].
    Queue(Result<QueueView>),
    /// Answers [`RequestKind::Changes`].
    Changes(Result<ChangesRead>),
    /// Answers [`RequestKind::CommitPreview`].
    CommitPreview(Result<CommitPreviewOutcome>),
    /// Answers [`RequestKind::CommitConfirmExecute`].
    CommitConfirmExecute(Result<Outcome<CommitResult>>),
    /// Answers [`RequestKind::SealPreview`].
    SealPreview(Result<SealPreviewOutcome>),
    /// Answers [`RequestKind::SealConfirmExecute`].
    SealConfirmExecute(Result<Outcome<SealResult>>),
}

/// What one [`RequestKind::Orient`] read found (RFC 031 §3): the change token, **read first**, and the
/// Orientation it is stamped with. The two results are independent: a failed token read leaves the
/// previous stamp in place whatever Orientation's own result was, and the other way round.
///
/// **Why the token comes first:** a change landing between the two reads is then in Orientation but not
/// in the token, so the next check differs and refreshes, which is harmless. Read the other way round,
/// the change would be in the token and not on screen, and no check would ever notice it.
#[derive(Debug)]
pub(crate) struct OrientRead {
    pub(crate) token: Result<ChangeToken>,
    pub(crate) view: Result<OrientationView>,
}

#[cfg(test)]
impl OrientRead {
    /// A read whose token is `NullBackend::supported()`'s default, for tests about something else.
    pub(crate) fn stamped(view: Result<OrientationView>) -> Self {
        Self {
            token: Ok(ChangeToken::compose(
                [("heads/main", "0".repeat(64).as_str())],
                0,
                None,
                &stikk_model::CurrentBranch::NotReported,
            )),
            view,
        }
    }
}

/// A short, display-only label for the kind of work a [`RequestKind`] represents — used for the
/// pending-screen/overlay note and the Background Operations listing (TU-01/TU-03). Never repository
/// content; purely stikk's own naming of its own request.
impl RequestKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Orient => "orientation",
            Self::ChangeCheck => "change check",
            Self::History { .. } => "history",
            Self::BlockState { .. } => "block detail",
            Self::Refs => "refs",
            Self::Queue => "queue",
            Self::Changes { .. } => "changes",
            Self::CommitPreview { .. } => "commit preview",
            Self::CommitConfirmExecute { .. } => "commit",
            Self::SealPreview { .. } => "seal preview",
            Self::SealConfirmExecute { .. } => "seal",
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
            RequestKind::Orient => {
                let token = change_token(prikk, repo);
                ResponseKind::Orient(OrientRead {
                    token,
                    view: orient(prikk, repo),
                })
            }
            RequestKind::ChangeCheck => ResponseKind::ChangeCheck(change_token(prikk, repo)),
            RequestKind::History { reff } => {
                ResponseKind::History(history_view(prikk, repo, &reff, HISTORY_LIMIT))
            }
            RequestKind::BlockState { reff, row, is_tip } => {
                let result = block_detail(prikk, repo, &reff, row, is_tip);
                ResponseKind::BlockState { reff, result }
            }
            RequestKind::Refs => ResponseKind::Refs(list_refs(prikk, repo)),
            RequestKind::Queue => ResponseKind::Queue(queue_view(prikk, repo)),
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
                message,
            } => ResponseKind::CommitConfirmExecute(commit_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &message,
            )),
            RequestKind::SealPreview { reff } => {
                ResponseKind::SealPreview(seal_preview(prikk, repo, &reff))
            }
            RequestKind::SealConfirmExecute {
                token,
                readiness,
                evidence,
                reff,
            } => ResponseKind::SealConfirmExecute(seal_confirm_and_execute(
                prikk, repo, token, readiness, evidence, &reff,
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
