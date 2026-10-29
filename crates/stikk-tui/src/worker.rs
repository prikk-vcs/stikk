//! The worker thread and the request/response protocol that connects it to [`crate::app::App`]
//! (design CC-01; RFC 010). These types are frontend↔worker plumbing, not `stikk-core` operations —
//! the operations themselves stay synchronous and unchanged (RFC 010 decision 3); this module is
//! simply the thread that calls them and the messages that cross the channel between it and the UI
//! thread. Crate-internal only: nothing outside `stikk-tui` needs to construct a [`Request`] or
//! [`Response`], since external callers only ever use [`crate::run`].

use std::path::Path;
use std::sync::mpsc;

use crate::app::{CompareSource, RefWriteOp};
use crate::overlay::RefWriteKind;
use stikk_core::{
    BlockDetailView, ChangesRead, CommitPreviewOutcome, CommitToken, Evidence, HistoryView,
    OrientationView, Outcome, PreviewToken, SealPreviewOutcome, block_detail, change_token,
    changes_view, commit_confirm_and_execute, commit_preview, history_view, list_refs, orient,
    seal_confirm_and_execute, seal_preview,
};
use stikk_core::{PatchDetailView, PatchMessageState, QueueView, patch_detail, queue_view};
use stikk_model::{ChangeToken, Readiness, Result};
use stikk_prikk::{BlockRow, CommitResult, Prikk, SealResult};

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
    /// Read one **queued** patch's content, for Patch detail (`FR-030`; RFC 035 decision 6) — one
    /// `show <patch-id>` read. A *sealed* patch never comes through here: Block detail has already
    /// read its block's report and Patch detail renders one patch from it, spawning nothing.
    PatchDetail {
        /// The patch to show, by **patch** id: a queued patch has no block to ask about (RFC 035 F2).
        patch_id: String,
        /// The message the Queue already read, in its three states — prikk reports a queued patch's
        /// message in `status`, not in `show`, so re-reading it here would be a second read for a fact
        /// stikk already holds.
        message: PatchMessageState,
    },
    /// Compare two points, or the worktree with what `commit` would author against (`FR-033`;
    /// RFC 038). **The comparison is decided on the UI thread**, where the marked block, the
    /// selected one and the loaded lineage all live; the worker only runs it.
    Compare {
        /// What to compare.
        comparison: stikk_prikk::Comparison,
        /// How to head the screen, in stikk's own words — *the previous block in this ref's
        /// history*, *the marked block*, *the worktree*. The seam reports the two points; only
        /// this layer knows which entry point named them.
        opened: CompareSource,
    },
    /// Run `prikk verify` and shape its report (`FR-100`; RFC 040).
    ///
    /// **Only ever sent because the reader asked** — `V`, the palette, or `r` on the screen it
    /// opened. It scans the whole repository, so nothing schedules it: not the change check, not a
    /// refresh of any other screen, and not startup (RFC 040 decision 4).
    Verify,
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
        /// The token [`RequestKind::SealPreview`] minted. Boxed, as commit's and RFC 044's are: a
        /// token is far larger than anything else that crosses this channel, and an unboxed one
        /// sets the size of every request the worker ever receives.
        token: Box<PreviewToken>,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        readiness: Readiness,
        /// The user's confirmation evidence (an explicit yes, at seal's tier 3 — untyped, RFC 013 Q3).
        evidence: Evidence,
        /// The ref to seal (must match the preview's).
        reff: String,
    },
    /// Build the preview for one of RFC 044's three ref writes.
    RefWritePreview {
        /// What would be written. It names the operation **and** every argument prikk will be
        /// handed, so the preview restates the call that will actually run.
        op: RefWriteOp,
    },
    /// Confirm and execute a ref write in one round trip (RFC 044) — as commit does, and for the
    /// same reason: nothing happens between the confirmation succeeding and the write starting.
    RefWriteConfirmExecute {
        /// The token [`RequestKind::RefWritePreview`] minted. Boxed as the commit flow boxes its
        /// own, to keep this variant from dwarfing the rest of the enum.
        token: Box<PreviewToken>,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        readiness: Readiness,
        /// The user's confirmation evidence (an explicit yes, at Publication's tier 3).
        evidence: Evidence,
        /// The previewed call, executed unchanged.
        op: RefWriteOp,
    },
    /// Build the preview for a branch switch — **including its file plan**, which is a `diff`
    /// read inside the preview, not here (RFC 045 decision 5).
    SwitchPreview {
        /// The branch to switch to.
        target: String,
    },
    /// Confirm and execute a branch switch in one round trip (RFC 045).
    SwitchConfirmExecute {
        /// The token [`RequestKind::SwitchPreview`] minted.
        token: Box<PreviewToken>,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        /// **Not a key**: `Materialization` needs read-write and nothing more (decision 2).
        readiness: Readiness,
        /// The user's confirmation evidence (an explicit yes, at Tier Three).
        evidence: Evidence,
        /// The branch to switch to (must match the preview's).
        target: String,
    },
    /// Build the merge preview (RFC 047 decision 6): the guards, the baseline and prikk's plan,
    /// all read again, so what is confirmed is current.
    MergePreview {
        /// The three names the merge would run with.
        target: stikk_core::MergeTarget,
        /// The baseline the Merge plan view showed, short — the preview blocks if it moved.
        shown_baseline: String,
    },
    /// Confirm and execute a merge in one round trip, as commit and seal do.
    MergeConfirmExecute {
        /// The token [`RequestKind::MergePreview`] minted.
        token: Box<PreviewToken>,
        /// The session's signing readiness, read on the UI thread immediately before dispatch.
        readiness: Readiness,
        /// The user's confirmation evidence (a typed name, at Tier Three).
        evidence: Evidence,
        /// The three names (must match the preview's).
        target: stikk_core::MergeTarget,
    },
    /// Read prikk's merge plan for two branches (RFC 047 decision 4). **Read-only**: the baseline
    /// is found from two `log` reads and the plan from one `merge-plan`, and nothing is written.
    MergePlan {
        /// The branch that would be merged into -- the selected one, `--into`.
        into: String,
        /// The branch whose work would come in -- the marked one, `--from`.
        from: String,
    },
    /// Preview importing a bundle (RFC 050 Handoff B).
    ImportPreview {
        /// The file and the received branch the card will name.
        target: stikk_core::ImportTarget,
    },
    /// Confirm and run the import this token was previewed for.
    ImportConfirmExecute {
        /// The armed token.
        token: Box<stikk_core::PreviewToken>,
        /// The session's signing readiness, read on the UI thread as every other write does.
        readiness: stikk_model::Readiness,
        /// The reader's answer.
        evidence: stikk_core::Evidence,
        /// The file and the received branch (must match the preview's).
        target: stikk_core::ImportTarget,
    },
    /// Preview exporting a branch to a file (RFC 050 Handoff B).
    ExportPreview {
        /// The branch and the resolved destination.
        target: stikk_core::ExportTarget,
    },
    /// Read the maintainer keys this repository trusts (RFC 054).
    TrustRead,
    /// Preview adopting a maintainer key (RFC 056 decision 2).
    TrustAdoptPreview {
        /// The session's signing readiness, read on the UI thread as every other write does.
        readiness: stikk_model::Readiness,
        /// The id typed, already checked for shape.
        key_id: String,
        /// The public key typed, already checked for shape.
        public_key: String,
    },
    /// Confirm and run the adopt this token was previewed for.
    TrustAdoptConfirmExecute {
        /// The armed token.
        token: Box<stikk_core::PreviewToken>,
        /// The session's signing readiness.
        readiness: stikk_model::Readiness,
        /// The reader's answer -- the typed key id.
        evidence: stikk_core::Evidence,
        /// Must match the preview's.
        key_id: String,
        /// Must match the preview's.
        public_key: String,
    },
    /// Preview removing a maintainer key (RFC 056 decision 3).
    TrustRemovePreview {
        /// The session's signing readiness.
        readiness: stikk_model::Readiness,
        /// The id of the selected row.
        key_id: String,
    },
    /// Confirm and run the removal this token was previewed for.
    TrustRemoveConfirmExecute {
        /// The armed token.
        token: Box<stikk_core::PreviewToken>,
        /// The session's signing readiness.
        readiness: stikk_model::Readiness,
        /// The reader's answer -- the typed key id.
        evidence: stikk_core::Evidence,
        /// Must match the preview's.
        key_id: String,
    },
    /// Preview saving one file as it was at a block (RFC 053).
    SaveFilePreview {
        /// The block, the repository path and the resolved destination.
        target: stikk_core::SaveFileTarget,
    },
    /// Confirm and run the file save this token was previewed for.
    SaveFileConfirmExecute {
        /// The armed token.
        token: Box<stikk_core::PreviewToken>,
        /// The session's signing readiness, read on the UI thread as every other write does.
        readiness: stikk_model::Readiness,
        /// The reader's answer.
        evidence: stikk_core::Evidence,
        /// Must match the preview's.
        target: stikk_core::SaveFileTarget,
    },
    /// Preview saving every file at a block (RFC 053).
    SaveAllPreview {
        /// The block and the resolved destination directory.
        target: stikk_core::SaveAllTarget,
    },
    /// Confirm and run the whole-block save this token was previewed for.
    SaveAllConfirmExecute {
        /// The armed token.
        token: Box<stikk_core::PreviewToken>,
        /// The session's signing readiness.
        readiness: stikk_model::Readiness,
        /// The reader's answer.
        evidence: stikk_core::Evidence,
        /// Must match the preview's.
        target: stikk_core::SaveAllTarget,
    },
    /// Confirm and run the export this token was previewed for.
    ExportConfirmExecute {
        /// The armed token.
        token: Box<stikk_core::PreviewToken>,
        /// The session's signing readiness, read on the UI thread as every other write does.
        readiness: stikk_model::Readiness,
        /// The reader's answer.
        evidence: stikk_core::Evidence,
        /// The branch and the destination (must match the preview's).
        target: stikk_core::ExportTarget,
    },
    /// Open a bundle file: verify it, and preview it against the local branch of its own name
    /// (RFC 050 Handoff A).
    OpenBundle {
        /// The file, **already resolved** against the directory stikk was started in. Resolved
        /// on the UI thread, where the start directory is known, so the worker runs and the view
        /// shows the same string.
        path: std::path::PathBuf,
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
    /// Answers [`RequestKind::Compare`], echoing how it was opened so the screen can say it.
    Compare {
        /// How the comparison was asked for.
        opened: CompareSource,
        /// The comparison, or the band's refusal to make one.
        result: Result<stikk_core::CompareView>,
    },
    /// Answers [`RequestKind::Verify`].
    Verify(Result<stikk_core::VerifyView>),
    /// Answers [`RequestKind::BlockState`], echoing the ref it was read for, so the screen it fills names
    /// that ref and a later refresh re-reads it (RFC 031 review v1 §2.2).
    BlockState {
        reff: String,
        result: Result<BlockDetailView>,
    },
    /// Answers [`RequestKind::Refs`] — the merged pointers **and** the unpublished current
    /// branch beside them (RFC 037 decision 3).
    Refs(Result<stikk_core::RefList>),
    /// Answers [`RequestKind::Queue`].
    Queue(Result<QueueView>),
    /// Answers [`RequestKind::PatchDetail`].
    PatchDetail(Result<PatchDetailView>),
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
    /// Answers [`RequestKind::RefWritePreview`].
    RefWritePreview(Result<stikk_core::RefWritePreview>),
    /// Answers [`RequestKind::SwitchPreview`].
    SwitchPreview(Result<stikk_core::SwitchPreview>),
    /// Answers [`RequestKind::SwitchConfirmExecute`].
    SwitchConfirmExecute(Result<Outcome<stikk_prikk::SwitchResult>>),
    /// Answers [`RequestKind::MergePlan`].
    MergePlan(Result<stikk_core::MergePlanRead>),
    /// Answers [`RequestKind::OpenBundle`].
    OpenBundle(Result<stikk_core::BundleOpened>),
    /// Answers [`RequestKind::ImportPreview`].
    ImportPreview(Result<stikk_core::ImportPreview>),
    /// Answers [`RequestKind::ImportConfirmExecute`].
    ImportConfirmExecute(Result<stikk_core::Outcome<stikk_prikk::BundleImport>>),
    /// Answers [`RequestKind::TrustRead`].
    TrustRead(Result<stikk_core::TrustView>),
    /// Answers [`RequestKind::TrustAdoptPreview`].
    TrustAdoptPreview(Result<stikk_core::AdoptPreview>),
    /// Answers [`RequestKind::TrustRemovePreview`].
    TrustRemovePreview(Result<stikk_core::RemovePreview>),
    /// Answers either trust write.
    TrustWrite(Result<stikk_core::Outcome<stikk_core::TrustWriteResult>>),
    /// Answers [`RequestKind::SaveFilePreview`].
    SaveFilePreview(Result<stikk_core::SaveFilePreview>),
    /// Answers [`RequestKind::SaveFileConfirmExecute`].
    SaveFileConfirmExecute(Result<stikk_core::Outcome<stikk_prikk::FileSaved>>),
    /// Answers [`RequestKind::SaveAllPreview`].
    SaveAllPreview(Result<stikk_core::SaveAllPreview>),
    /// Answers [`RequestKind::SaveAllConfirmExecute`].
    SaveAllConfirmExecute(Result<stikk_core::Outcome<stikk_prikk::Extracted>>),
    /// Answers [`RequestKind::ExportPreview`].
    ExportPreview(Result<stikk_core::ExportPreview>),
    /// Answers [`RequestKind::ExportConfirmExecute`].
    ExportConfirmExecute(Result<stikk_core::Outcome<stikk_prikk::BundleExport>>),
    /// Answers [`RequestKind::MergePreview`].
    MergePreview(Result<stikk_core::MergePreview>),
    /// Answers [`RequestKind::MergeConfirmExecute`].
    MergeConfirmExecute(Result<Outcome<stikk_prikk::MergeResult>>),
    /// Answers [`RequestKind::RefWriteConfirmExecute`].
    RefWriteConfirmExecute {
        /// Echoed from the request, so the result card can title itself without the UI thread
        /// keeping a second copy of what it already sent.
        kind: RefWriteKind,
        /// prikk's lines, or whatever stopped the write.
        result: Result<Outcome<stikk_prikk::RefWrite>>,
    },
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
            Self::PatchDetail { .. } => "patch detail",
            Self::Compare { .. } => "compare",
            Self::Verify => "verify",
            Self::Changes { .. } => "changes",
            Self::CommitPreview { .. } => "commit preview",
            Self::CommitConfirmExecute { .. } => "commit",
            Self::SealPreview { .. } => "seal preview",
            Self::SealConfirmExecute { .. } => "seal",
            Self::RefWritePreview { .. } => "ref write preview",
            Self::RefWriteConfirmExecute { .. } => "ref write",
            Self::SwitchPreview { .. } => "switch preview",
            Self::MergePlan { .. } => "merge plan",
            Self::OpenBundle { .. } => "bundle",
            Self::ImportPreview { .. } => "import preview",
            Self::ImportConfirmExecute { .. } => "import",
            Self::TrustRead => "trust",
            Self::TrustAdoptPreview { .. } | Self::TrustRemovePreview { .. } => "trust preview",
            Self::TrustAdoptConfirmExecute { .. } | Self::TrustRemoveConfirmExecute { .. } => {
                "trust write"
            }
            Self::SaveFilePreview { .. } => "save preview",
            Self::SaveFileConfirmExecute { .. } => "save",
            Self::SaveAllPreview { .. } => "save preview",
            Self::SaveAllConfirmExecute { .. } => "save",
            Self::ExportPreview { .. } => "export preview",
            Self::ExportConfirmExecute { .. } => "export",
            Self::MergePreview { .. } => "merge preview",
            Self::MergeConfirmExecute { .. } => "merge",
            Self::SwitchConfirmExecute { .. } => "switch",
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
            RequestKind::Compare { comparison, opened } => ResponseKind::Compare {
                opened,
                result: stikk_core::compare(prikk, repo, &comparison),
            },
            RequestKind::Verify => ResponseKind::Verify(stikk_core::verify(prikk, repo)),
            RequestKind::Refs => ResponseKind::Refs(list_refs(prikk, repo)),
            RequestKind::Queue => ResponseKind::Queue(queue_view(prikk, repo)),
            RequestKind::PatchDetail { patch_id, message } => {
                ResponseKind::PatchDetail(patch_detail(prikk, repo, &patch_id, &message))
            }
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
                prikk, repo, *token, readiness, evidence, &reff,
            )),
            RequestKind::RefWritePreview { op } => ResponseKind::RefWritePreview(match &op {
                RefWriteOp::CreateBranch { name, from_ref } => {
                    stikk_core::create_branch_preview(prikk, repo, name, from_ref)
                }
                RefWriteOp::CreateTag {
                    name,
                    target,
                    message,
                } => stikk_core::create_tag_preview(
                    prikk,
                    repo,
                    stikk_core::NewTag {
                        name,
                        target,
                        message,
                    },
                ),
                RefWriteOp::CloseBranch { name } => {
                    stikk_core::close_branch_preview(prikk, repo, name)
                }
            }),
            RequestKind::RefWriteConfirmExecute {
                token,
                readiness,
                evidence,
                op,
            } => ResponseKind::RefWriteConfirmExecute {
                kind: op.kind(),
                result: match &op {
                    RefWriteOp::CreateBranch { name, from_ref } => {
                        stikk_core::create_branch_confirm_and_execute(
                            prikk, repo, *token, readiness, evidence, name, from_ref,
                        )
                    }
                    RefWriteOp::CreateTag {
                        name,
                        target,
                        message,
                    } => stikk_core::create_tag_confirm_and_execute(
                        prikk,
                        repo,
                        *token,
                        readiness,
                        evidence,
                        stikk_core::NewTag {
                            name,
                            target,
                            message,
                        },
                    ),
                    RefWriteOp::CloseBranch { name } => {
                        stikk_core::close_branch_confirm_and_execute(
                            prikk, repo, *token, readiness, evidence, name,
                        )
                    }
                },
            },
            RequestKind::SwitchPreview { target } => {
                ResponseKind::SwitchPreview(stikk_core::switch_preview(prikk, repo, &target))
            }
            RequestKind::MergePreview {
                target,
                shown_baseline,
            } => ResponseKind::MergePreview(stikk_core::merge_preview(
                prikk,
                repo,
                &target.into,
                &target.from,
                &shown_baseline,
                stikk_core::MERGE_LINEAGE_LIMIT,
            )),
            RequestKind::MergeConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            } => ResponseKind::MergeConfirmExecute(stikk_core::merge_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &target,
            )),
            RequestKind::MergePlan { into, from } => {
                ResponseKind::MergePlan(stikk_core::merge_plan_view(
                    prikk,
                    repo,
                    &into,
                    &from,
                    stikk_core::MERGE_LINEAGE_LIMIT,
                ))
            }
            RequestKind::OpenBundle { path } => {
                ResponseKind::OpenBundle(stikk_core::open_bundle(prikk, repo, &path))
            }
            RequestKind::ImportPreview { target } => {
                ResponseKind::ImportPreview(stikk_core::import_preview(
                    prikk,
                    repo,
                    &target.path,
                    stikk_core::BUNDLE_LINEAGE_LIMIT,
                ))
            }
            RequestKind::TrustRead => ResponseKind::TrustRead(stikk_core::trust::read(prikk, repo)),
            RequestKind::TrustAdoptPreview {
                readiness,
                key_id,
                public_key,
            } => ResponseKind::TrustAdoptPreview(stikk_core::adopt_preview(
                prikk,
                repo,
                readiness,
                &key_id,
                &public_key,
            )),
            RequestKind::TrustAdoptConfirmExecute {
                token,
                readiness,
                evidence,
                key_id,
                public_key,
            } => ResponseKind::TrustWrite(stikk_core::adopt_confirm_and_execute(
                prikk,
                repo,
                *token,
                readiness,
                evidence,
                &key_id,
                &public_key,
            )),
            RequestKind::TrustRemovePreview { readiness, key_id } => {
                ResponseKind::TrustRemovePreview(stikk_core::remove_preview(
                    prikk, repo, readiness, &key_id,
                ))
            }
            RequestKind::TrustRemoveConfirmExecute {
                token,
                readiness,
                evidence,
                key_id,
            } => ResponseKind::TrustWrite(stikk_core::remove_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &key_id,
            )),
            RequestKind::SaveFilePreview { target } => {
                ResponseKind::SaveFilePreview(stikk_core::save_file_preview(
                    prikk,
                    repo,
                    &target.block,
                    &target.path,
                    &target.output,
                ))
            }
            RequestKind::SaveFileConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            } => ResponseKind::SaveFileConfirmExecute(stikk_core::save_file_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &target,
            )),
            RequestKind::SaveAllPreview { target } => ResponseKind::SaveAllPreview(
                stikk_core::save_all_preview(prikk, repo, &target.block, &target.dir),
            ),
            RequestKind::SaveAllConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            } => ResponseKind::SaveAllConfirmExecute(stikk_core::save_all_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &target,
            )),
            RequestKind::ExportPreview { target } => ResponseKind::ExportPreview(
                stikk_core::export_preview(prikk, repo, &target.reff, &target.output),
            ),
            RequestKind::ExportConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            } => ResponseKind::ExportConfirmExecute(stikk_core::export_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &target,
            )),
            RequestKind::ImportConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            } => ResponseKind::ImportConfirmExecute(stikk_core::import_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &target,
            )),
            RequestKind::SwitchConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            } => ResponseKind::SwitchConfirmExecute(stikk_core::switch_confirm_and_execute(
                prikk, repo, *token, readiness, evidence, &target,
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
