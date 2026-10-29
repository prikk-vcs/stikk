//! The application state and its transitions (handoff §2/§7 `app.rs`; RFC 006/007; RFC 010).
//!
//! The app owns the view-model stack, the overlay stack, the focused ref, the session refusal history,
//! and the palette colours. It **holds no repository authority** (design INV-8): every view-model is a
//! rendered snapshot of what prikk reported, re-sourced on demand. The only repository facts come from
//! `stikk_core` — the app computes nothing, and every seam error is routed through the one
//! [`fn@stikk_core::present`] mapping (ER-03), never presented ad hoc.
//!
//! **The seam runs off the UI thread (RFC 010).** `App` no longer calls `stikk-core` directly: every
//! method that used to take `prikk: &impl Prikk` now *sends* a `Request` (`crate::worker`) to the
//! worker over a channel it owns, and `App::apply` (crate-private — the worker's `Response` never
//! crosses this crate's boundary) is the one entry point for the eventual answer. A request carries a
//! sequence number; a response for a sequence the app
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
use std::time::{Duration, Instant};

use stikk_core::{
    BlockDetailView, COMMIT_OPERATION, ChangesRead, ChangesView, Command, CommitPreviewOutcome,
    HistoryView, NextTarget, OperationContext, Presentation, RefusalHistory, SEAL_OPERATION,
    SealPreviewOutcome, Target, present, staleness_notice,
};
use stikk_core::{PatchDetailView, PatchMessageState, QueueView, RefHistory};
use stikk_model::{ChangeToken, CurrentBranch, StaleCause, StikkError, Tier};
use stikk_state::Config;

use crate::overlay::{Overlay, RefNameKind, RefWriteKind, SavePathKind};
use crate::theme::Palette;
use crate::worker::{OrientRead, Request, RequestKind, Response, ResponseKind};

/// The fallback ref (RFC 029 Handoff B §2, §4): focused on open **only** when prikk names no current
/// branch (below 0.42, or an unresolved pointer) and `heads/main` is published, and the one row an empty
/// ref picker offers. When prikk names a branch, stikk opens on that instead. prikk has no HEAD; its
/// current branch is "a default, never an authority", and stikk's focus stays its own (FR-055).
const DEFAULT_REF: &str = "heads/main";

/// The most background operations `App` remembers, for the Background Operations overlay (TU-01).
/// Display-only bookkeeping; bounded so a long session's list does not grow forever.
const OPERATIONS_CAP: usize = 20;

/// How long stikk waits, while idle, between silent change checks (RFC 031 Q1, ruled (b)). A check is one
/// change-token read, three prikk spawns measured at 1.0 ms (20 files) to 3.0 ms (301 files, 20 blocks)
/// at prikk 0.42.0 (RFC 031 F3), so every 5 seconds is well under a tenth of a percent of one core. It
/// covers terminals that report no focus; where focus is reported, [`App::focus_gained`] checks at once.
/// **A constant, not a setting.**
const CHANGE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

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

/// stikk's focused ref (FR-055), in the three states RFC 029 Handoff B §2 names. Client-side only:
/// changing it never runs `prikk branch switch` and never touches the worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefFocus {
    /// Nothing is focused yet, and no Orientation read has succeeded. The first successful one resolves
    /// focus, unless the user picks a ref first.
    Pending,
    /// The first read arrived and no ref qualified: prikk names no current branch and `heads/main` is
    /// not published. The ref picker was opened.
    Unfocused,
    /// A ref name, which the user can change with the picker at any time.
    Ref(String),
}

/// A screen pushed above the Orientation root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// A screen asked for but not yet arrived (RFC 010 §5). `App::apply` (crate-private) replaces it
    /// on success or removes it on error (surfacing the error via the usual overlay/banner);
    /// [`App::back`] pops it directly like any other screen — the "stop waiting" semantics (RFC 010
    /// decision 4).
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
    BlockDetail {
        /// The ref this block's detail was read for. A refresh re-reads this ref, never the focused one,
        /// which the ref picker can have moved since (RFC 031 review v1 §2.2).
        reff: String,
        /// The loaded detail.
        view: BlockDetailView,
        /// The selected patch in the block's Patches section (RFC 035 §4.1), `0` where there are none.
        /// `Enter` opens Patch detail for it, **from the report already read**.
        cursor: usize,
        /// The `seq` of an in-flight refresh, if any (RFC 031 §7). Only the tip's detail is ever
        /// refreshed: an older block's detail cannot change.
        refreshing: Option<u64>,
    },
    /// The worktree-vs-baseline Changes view, with the UD-08 display-only untracked filter.
    Changes {
        /// The loaded status.
        view: ChangesView,
        /// Whether untracked entries are hidden (display only — a commit still captures them).
        hide_untracked: bool,
        /// Whether the ref has published history (RFC 032 decision 5), read **beside** the view and kept
        /// beside it here; a refresh updates both.
        history: RefHistory,
        /// The `seq` of an in-flight refresh (`r`, or a detected change), if any — the view stays
        /// visible while it runs, as History's does (RFC 010 §5; RFC 031 §7).
        refreshing: Option<u64>,
    },
    /// The Queue view (RFC 028): the active queue, repository-wide.
    Queue {
        /// The loaded queue.
        view: QueueView,
        /// The `seq` of an in-flight refresh (`r`), if any — the view stays visible while it runs, as
        /// History's does (RFC 010 §5).
        refreshing: Option<u64>,
        /// First content row drawn, counting from zero after wrapping (RFC 028 A review v1 §2.1).
        /// `↑`/`↓` now move [`Self::Queue::cursor`] and the renderer **follows** it, scrolling only as
        /// far as it must to keep the selected patch on screen; it still clamps the value and writes it
        /// back, the Glossary's idiom, because only the renderer knows the wrapped height. A refresh
        /// keeps both, and the next render clamps them to the new content.
        offset: std::cell::Cell<u16>,
        /// The selected queued patch (RFC 035 §4.2), `0` for an empty queue. `Enter` opens Patch detail
        /// for it, with one `show <patch-id>` read.
        cursor: usize,
    },
    /// A comparison: what changed between two points (`FR-033`; RFC 038).
    Compare {
        /// The comparison, or the band's reason there is none.
        view: stikk_core::CompareView,
        /// How it was opened, which is what the screen says about *which side is which*.
        opened: CompareSource,
        /// The selected entry — the one `v` shows prikk's own fields for.
        cursor: usize,
        /// Whether the raw view is showing for the selected entry (`v`), Patch detail's toggle.
        raw: bool,
        /// The `seq` of an in-flight re-read, if any — the comparison stays visible while it runs.
        /// Only a worktree comparison is ever re-read; two sealed blocks cannot change.
        refreshing: Option<u64>,
        /// First content row drawn, clamped and written back by the renderer (Patch detail's
        /// idiom): one entry's hunks can be a whole file.
        offset: std::cell::Cell<u16>,
    },
    /// prikk's plan for merging one branch into another (`FR-080`; RFC 047 decision 4).
    ///
    /// **Inert.** It shows what prikk printed and nothing acts on it; the merge itself is RFC
    /// 047's Handoff B.
    MergePlan {
        /// The two branches, the baseline stikk found, and prikk's plan verbatim.
        view: Box<stikk_core::MergePlanView>,
        /// First content row drawn, clamped and written back by the renderer: a plan can run
        /// past a short terminal, and `FR-081` asks for all of it.
        offset: std::cell::Cell<u16>,
    },
    /// What one bundle file is, and what it would change (`FR-091`; RFC 050 decision 3).
    ///
    /// **Inert.** It shows prikk's verification and prikk's preview, and nothing on it acts;
    /// importing is RFC 050's Handoff B, and until it lands the view advertises no key that
    /// would do nothing.
    Bundle {
        /// The resolved path, prikk's two answers and stikk's sentence -- or prikk's refusal.
        view: Box<stikk_core::BundleOpened>,
        /// First content row drawn, clamped and written back by the renderer: a verification
        /// and a long effect list run past a short terminal.
        offset: std::cell::Cell<u16>,
    },
    /// The maintainer keys this repository trusts (RFC 054 decision 2). **Read-only**: there is
    /// nothing on this screen to confirm, and no key to press but `Esc`.
    Trust {
        /// The list, or why there is none.
        view: stikk_core::TrustView,
        /// First content row drawn, clamped and written back by the renderer — Verify's idiom:
        /// a repository can adopt more keys than a terminal has rows.
        offset: std::cell::Cell<u16>,
    },
    /// prikk's verdict on whether the repository is sound (`FR-100`; RFC 040).
    Verify {
        /// The report, or the band's reason there is none.
        view: stikk_core::VerifyView,
        /// Whether the raw view is showing (`v`): prikk's own JSON instead of stikk's rendering.
        raw: bool,
        /// The `seq` of an in-flight re-run (`r`), if any — the report stays visible while it
        /// runs, with the heading saying so. **A verdict is never cached across runs** (RFC 040
        /// decision 4): what is on screen is what this run said.
        refreshing: Option<u64>,
        /// First content row drawn, clamped and written back by the renderer (Patch detail's
        /// idiom): 17 stages plus the raw report is taller than a terminal.
        offset: std::cell::Cell<u16>,
    },
    /// One patch's detail: what it changed, in prikk's own terms (`FR-030`; RFC 035).
    PatchDetail {
        /// The loaded detail.
        view: PatchDetailView,
        /// Whether the raw view is showing: prikk's own fields for each operation, instead of stikk's
        /// rendering of them (`v`; RFC 035 decision 5). A toggle, never a second read.
        raw: bool,
        /// How this screen was opened, which decides what `r` re-reads (RFC 035 §4.3).
        source: PatchSource,
        /// The `seq` of an in-flight re-read, if any — the view stays visible while it runs.
        refreshing: Option<u64>,
        /// First content row drawn, clamped and written back by the renderer (the Queue's idiom): a
        /// patch's operations, and a bounded preimage among them, run past one screen.
        offset: std::cell::Cell<u16>,
    },
}

/// How a [`Screen::Compare`] was opened — **stikk's own words for which side is which** (RFC 038
/// decision 3).
///
/// prikk's report names two points and says nothing about why they were chosen. Only this layer
/// knows, and the difference matters to a reader: *the previous block in this ref's history* is a
/// claim about the lineage they are browsing, not about parentage — **a merge block has two
/// parents, and neither of them need be the row above it** (measured at 0.51: a merge block sits
/// at `update_seq` 3 above a `Normal` block, with `parent_count` 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareSource {
    /// Two blocks the user marked and selected, ordered older → newer by the lineage.
    MarkedPair,
    /// A block against the next older row in the loaded lineage.
    PreviousInHistory,
    /// The worktree against what `commit` would author against.
    WorktreeAgainstBaseline,
    /// Two refs chosen in the ref picker: the marked one against the selected one (RFC 041
    /// decision 2).
    ///
    /// **No lineage order is invented.** Two blocks in one ref's history have an order -- prikk's
    /// `update_seq` -- and [`Self::MarkedPair`] uses it. Two refs do not: neither is older in any
    /// sense stikk can read off the pointers. So the order is the one the reader chose, marked
    /// first, and the heading says which is which rather than implying a direction.
    ///
    /// **It carries the two names** so `r` can ask again. A ref moves; the comparison must follow
    /// it (RFC 041 decision 3), and a screen that only remembered the blocks prikk resolved last
    /// time would quietly re-run the old comparison for ever.
    RefPair {
        /// The marked ref.
        from: String,
        /// The selected ref.
        to: String,
    },
}

impl CompareSource {
    /// One line saying what was compared, in stikk's own voice.
    #[must_use]
    pub fn describe(&self) -> &'static str {
        match self {
            Self::MarkedPair => "the marked block, against the selected one \u{2014} older first",
            Self::PreviousInHistory => {
                "this block, against the previous block in this ref's history"
            }
            Self::WorktreeAgainstBaseline => {
                "the worktree, against what `commit` would author against"
            }
            Self::RefPair { .. } => {
                "the marked ref, against the selected one \u{2014} your order, not a lineage's"
            }
        }
    }
}

/// How a [`Screen::PatchDetail`] was opened — and therefore what `r` can honestly re-read (RFC 035
/// §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchSource {
    /// Opened from the Queue, by patch id. **`r` re-reads that same patch id**, and renders whatever
    /// prikk now reports: a patch sealed while the screen was open comes back `queued: false` with its
    /// node-addressed paths **unresolved**, because a bare sealed patch id resolves nothing (RFC 035 A1
    /// F7, row 2). The earlier read's path is **not** carried over. The resulting screen is worse than
    /// the one before it and tells the truth, which is the trade this project makes every time.
    Queued {
        /// The patch id to re-read.
        patch_id: String,
        /// The message the Queue read, carried for the re-render.
        message: PatchMessageState,
    },
    /// Opened from Block detail, out of the block's own report. **`r` re-reads nothing**: a sealed
    /// block's patches are fixed, the same reason an older block's detail is never refreshed (RFC 031
    /// §7). `r` still re-reads Orientation, as it does on every screen.
    InBlock,
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
    /// A Block-detail screen and its selected patch.
    BlockDetail(&'a BlockDetailView, usize),
    /// The Trust view (RFC 054 decision 2): the keys this repository trusts, and the scroll
    /// offset the renderer clamps and writes back.
    Trust(&'a stikk_core::TrustView, &'a std::cell::Cell<u16>),
    /// The Changes view, whether untracked entries are hidden, and whether the ref has published history.
    Changes(&'a ChangesView, bool, &'a RefHistory),
    /// The Queue view, its scroll offset and its selected patch.
    Queue(&'a QueueView, &'a std::cell::Cell<u16>, usize),
    /// One patch's detail, whether the raw view is showing, and its scroll offset.
    PatchDetail(&'a PatchDetailView, bool, &'a std::cell::Cell<u16>),
    /// A comparison and the state the screen holds for it.
    Compare(CompareFocus<'a>),
    /// prikk's merge plan for two branches, and the screen's scroll offset.
    MergePlan(&'a stikk_core::MergePlanView, &'a std::cell::Cell<u16>),
    /// What one bundle file is and what it would change, and the screen's scroll offset.
    Bundle(&'a stikk_core::BundleOpened, &'a std::cell::Cell<u16>),
    /// The Verify report, whether the raw view is showing, whether a re-run is in flight, and the
    /// scroll offset.
    Verify(
        &'a stikk_core::VerifyView,
        bool,
        bool,
        &'a std::cell::Cell<u16>,
    ),
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

/// Everything [`Screen::Compare`] hands its renderer. One struct rather than five positional
/// arguments: the renderer takes a palette, a frame and an area besides, and five more alongside
/// them is where a caller swaps two of the same type without the compiler noticing.
#[derive(Debug, Clone, Copy)]
pub struct CompareFocus<'a> {
    /// The comparison, or the band's reason there is none.
    pub view: &'a stikk_core::CompareView,
    /// How it was opened — stikk's own words for which side is which.
    pub opened: &'a CompareSource,
    /// The selected entry.
    pub cursor: usize,
    /// Whether the raw view is showing for that entry.
    pub raw: bool,
    /// The scroll offset, clamped and written back by the renderer.
    pub offset: &'a std::cell::Cell<u16>,
}

/// A block marked for comparison (`m`), with the lineage it was marked in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedBlock {
    /// The ref whose History the mark was made in.
    pub reff: String,
    /// The marked block's id.
    pub block_id: String,
    /// Its `update_seq`, which is what orders the pair **older → newer** (RFC 038 §7 trap 5) —
    /// prikk's own monotonic publication sequence, never the row index, which is a display order
    /// over however much of the lineage happens to be loaded.
    pub update_seq: u64,
}

/// The running application.
pub struct App {
    repo: PathBuf,
    state: OrientationState,
    /// The `seq` of the orientation request currently awaited, if any — matched in [`App::apply`]
    /// against both the initial load and any later refresh (`reload`), which share this one slot since
    /// there is only ever one "current" orientation.
    orientation_pending: Option<u64>,
    /// Resolved once, by [`App::resolve_focus`] on the first successful Orientation read; the user's
    /// from then on. A later read never moves it (RFC 029 Handoff B §2).
    ref_focus: RefFocus,
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
    /// The block marked for a comparison (`m`; RFC 038 decision 3), by id and by the ref whose
    /// lineage it was marked in.
    ///
    /// **On `App`, not on the History screen**, because a mark has to survive what a reader does
    /// between the two halves of a comparison: drilling into a block and coming back pops and
    /// pushes that screen. The ref is carried with it so a mark made in one lineage cannot be
    /// compared against a block the user then selected in another — the ref picker can move
    /// between the two presses.
    marked_block: Option<MarkedBlock>,
    /// The ref marked in the picker for a comparison (RFC 041 decision 1).
    ///
    /// **A different thing from [`Self::marked_block`], deliberately kept apart.** `m` and `d`
    /// mean "block" in History and "ref" in the picker, and one mark must never be read as the
    /// other: comparing the block you marked yesterday against a ref you selected today is a pair
    /// nobody asked for. Each key reads only its own mark.
    ///
    /// **Lifetime: it survives closing and reopening the picker, and is cleared by a ref change**
    /// (decision 1's *"until the next ref change"*). Surviving a reopen is what makes the mark
    /// usable at all -- the picker is where both halves of the pair are chosen, and a mark that
    /// died on `Esc` could never be paired with anything. A ref change clears it because the
    /// reader has moved on, and a stale pair is worse than an empty one.
    marked_ref: Option<String>,
    pending_commit: Option<PendingCommit>,
    /// The ref pointers the last [`RequestKind::Refs`] read, kept beside the picker's own list of
    /// names (RFC 044).
    ///
    /// **The picker offers names; three of its keys need facts about them** — whether a branch is
    /// already closed, and whether a row is a published ref at all. Those facts are read with the
    /// list and belong with it, not re-read per key press, and never guessed from the name. Looked
    /// up **by name**, so this cannot drift out of step with the picker's row order.
    ///
    /// The unpublished current branch is deliberately absent: it is not a published pointer, so a
    /// lookup for it finds nothing and the three keys decline it in stikk's own words (decision 7).
    ref_rows: Vec<stikk_prikk::RefEntry>,
    /// The seal flow's state between overlays (RFC 016 §5/§8) — the same reason [`Self::pending_commit`]
    /// exists, across the **three** round trips seal's own extra consent step needs (preview, then the
    /// consent act, then confirm+execute).
    pending_seal: Option<PendingSeal>,
    /// The ref-write flow's state between overlays (RFC 044) — the same reason
    /// [`Self::pending_commit`] exists, across the two round trips the flow needs (preview, then
    /// confirm+execute). The prompts that precede the preview carry their own text on the overlay;
    /// only what prikk will be handed travels here.
    pending_ref_write: Option<PendingRefWrite>,
    /// The switch flow's state between overlays (RFC 045) — the same reason
    /// [`Self::pending_ref_write`] exists, across the two round trips the flow needs.
    pending_switch: Option<PendingSwitch>,
    /// The merge being previewed, confirmed or run (RFC 047 decision 6). Its own slot, not a
    /// shared one: a merge is the only flow that carries three names through a confirmation,
    /// and folding it into another pending would make both ask *which kind is this*.
    pending_merge: Option<PendingMerge>,
    /// The import in flight, if any (RFC 050 decision 4). Its own slot, not `pending_merge`'s:
    /// the two carry different targets and neither can be in flight while the other is.
    pending_import: Option<PendingImport>,
    /// The export in flight, if any (RFC 050 decision 5).
    pending_export: Option<PendingExport>,
    /// The file or whole-block save in flight, if any (RFC 053).
    pending_save: Option<PendingSave>,
    /// The change token read with the last Orientation read whose token read succeeded (RFC 031 §3). A
    /// check compares against it; `None` until the first stamp, and no check is sent before it.
    stamp: Option<ChangeToken>,
    /// The `seq` of the one silent change check in flight, if any (RFC 031 §4).
    check_pending: Option<u64>,
    /// Where this repository is remembered, or `None` for a launch that remembers nothing
    /// (RFC 046 decision 6, and a host with no resolvable state directory).
    session: Option<crate::Session>,
    /// What was remembered, read once at open. **Never shown** — it is handed to
    /// [`stikk_core::restore`] once prikk's own reads are in, and nothing of it reaches the
    /// screen before that (`T-T4`; §5 trap 2).
    stored: Option<stikk_state::SessionRecord>,
    /// The refs read sent at open **only when there is a record to validate against it**
    /// (decision 5). `None` once it has landed, or when nothing was remembered.
    restore_refs_pending: Option<u64>,
    /// The ref list that read returned, held until Orientation lands too — focus is resolved
    /// from both or from neither.
    restore_refs: Option<Vec<stikk_prikk::RefEntry>>,
    /// The record last written, so a transition that changes nothing writes nothing.
    saved: Option<stikk_state::SessionRecord>,
    /// The untracked filter a newly opened Changes view starts with — restored from the session
    /// record (RFC 046), `false` without one, and set by the `u` toggle so the next Changes view
    /// opens the way the last one was left.
    hide_untracked_default: bool,
    /// Set once a save is refused or fails: the banner is shown once and the store is not tried
    /// again this session (decision 3).
    session_write_failed: bool,
    /// When [`CHANGE_CHECK_INTERVAL`] started counting: the last check sent, or the first
    /// [`App::tick`] after a stamp. A stamp clears it, since `apply` is not handed the time; ticks come
    /// every `POLL`, so the clock starts at most that late.
    interval_from: Option<Instant>,
}

/// See [`App::pending_commit`].
enum PendingCommit {
    /// The message step finished; a [`RequestKind::CommitPreview`] is in flight for `reff`.
    AwaitingPreview {
        /// The message the user typed (`FL-05` step 2), carried through to execution. The ref travels in
        /// the preview request, and from there inside the [`stikk_core::CommitToken`] (RFC 030 decision
        /// 3), so it is not kept here a second time.
        message: String,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` is on the stack awaiting
    /// evidence.
    Confirming {
        /// Feed this into [`stikk_core::commit_confirm_and_execute`] once evidence is supplied. It owns
        /// the previewed ref and view (RFC 030 decision 3), so no separate ref travels beside it. Boxed
        /// because it carries the whole previewed `ChangesView`, and `clippy::large_enum_variant` is right
        /// that this variant would otherwise dwarf `AwaitingPreview`.
        token: Box<stikk_core::CommitToken>,
        /// The message the user typed.
        message: String,
    },
}

/// See [`App::pending_seal`].
enum PendingSeal {
    /// `Overlay::Loading` is on the stack; a [`RequestKind::SealPreview`] is in flight for `reff`.
    AwaitingPreview {
        /// The ref this seal targets.
        reff: String,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` is on the stack awaiting the
    /// ceremony's first act (an explicit yes, tier 3 — RFC 013 Q3 keeps seal untyped).
    Confirming {
        /// Feed this into [`stikk_core::seal_confirm_and_execute`] once both acts are supplied.
        token: stikk_core::PreviewToken,
        /// The ref this seal targets (must match the preview's).
        reff: String,
    },
    /// The first act is done; `Overlay::SealConsent` is on the stack awaiting the second, distinct act
    /// (RFC 016 §8) before [`RequestKind::SealConfirmExecute`] is ever dispatched.
    AwaitingConsent {
        /// Carried through from `Confirming` unchanged.
        token: stikk_core::PreviewToken,
        /// The ref this seal targets.
        reff: String,
    },
}

/// What one ref write will hand prikk (RFC 044) — built when the user finishes the prompt, carried
/// unchanged through the preview and the confirmation, so the call that runs is the call that was
/// previewed and nothing re-derives it in between.
///
/// **The names are already prefixed.** The prompt shows `heads/`/`tags/` fixed and joins it to what
/// was typed, so by the time one of these exists there is one name, passed to prikk as **one
/// argument** whatever is in it (trap 3: a space is legal to prikk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RefWriteOp {
    /// `branch create <name> --from <from_ref>`. **`--from` is always passed** (trap 1): the user
    /// chose a ref, and prikk's default source is a different question.
    CreateBranch {
        /// The new branch's full name.
        name: String,
        /// The ref whose tip it starts at.
        from_ref: String,
    },
    /// `tag create <name> --target <target>`, plus `-m <message>` when the message is non-empty.
    CreateTag {
        /// The new tag's full name.
        name: String,
        /// A ref name or a block id (F2: `--target` takes either).
        target: String,
        /// The message, possibly empty.
        message: String,
    },
    /// `branch close <name>`.
    CloseBranch {
        /// The branch to close.
        name: String,
    },
}

impl RefWriteOp {
    /// Which write this is, for the result card's own words.
    pub(crate) fn kind(&self) -> RefWriteKind {
        match self {
            Self::CreateBranch { .. } => RefWriteKind::CreateBranch,
            Self::CreateTag { .. } => RefWriteKind::CreateTag,
            Self::CloseBranch { .. } => RefWriteKind::CloseBranch,
        }
    }

    /// The prompt to reopen when this write fails, so **a failure keeps the user's typed name**
    /// (RFC 044 §3) and the correction is one edit rather than a retype. A close has no prompt to
    /// reopen — its name was a row the user selected, not something they wrote.
    fn prompt(&self) -> Option<Overlay> {
        let (kind, source, name) = match self {
            Self::CreateBranch { name, from_ref } => (RefNameKind::Branch, from_ref, name),
            Self::CreateTag { name, target, .. } => (RefNameKind::Tag, target, name),
            Self::CloseBranch { .. } => return None,
        };
        Some(Overlay::RefName {
            kind,
            source: source.clone(),
            // What the user typed is the name less the prefix the prompt showed them. It is
            // always there (the prompt is what joined it on), and `unwrap_or` keeps the whole
            // name rather than losing it if that ever stops being true.
            typed: name.strip_prefix(kind.prefix()).unwrap_or(name).to_string(),
        })
    }
}

/// See [`App::pending_switch`].
enum PendingSwitch {
    /// `Overlay::Loading` is on the stack; a [`RequestKind::SwitchPreview`] is in flight.
    AwaitingPreview {
        /// The branch to switch to.
        target: String,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` awaits an explicit yes.
    Confirming {
        /// Feed this into [`stikk_core::switch_confirm_and_execute`]. Boxed as every other
        /// armed token here is: a token dwarfs the name beside it.
        token: Box<stikk_core::PreviewToken>,
        /// The branch to switch to (must match the preview's).
        target: String,
    },
    /// Confirm+execute is in flight. **The target is kept** because success moves focus to it
    /// (decision 8), and by then the request that carried it is gone.
    Executing {
        /// The branch being switched to.
        target: String,
    },
}

/// See [`App::pending_import`] (RFC 050 decision 4).
///
/// **The same three states a merge keeps**, and for the same reasons: the target is needed
/// again after the request that carried it is gone, because success re-reads the ref lists and
/// opens the received branch's History.
enum PendingImport {
    /// `Overlay::Loading` is on the stack; a [`RequestKind::ImportPreview`] is in flight.
    AwaitingPreview {
        /// The file and the received branch.
        target: stikk_core::ImportTarget,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` awaits a yes.
    Confirming {
        /// Feed this into [`stikk_core::import_confirm_and_execute`].
        token: Box<stikk_core::PreviewToken>,
        /// The file and the received branch (must match the preview's).
        target: stikk_core::ImportTarget,
    },
    /// Confirm+execute is in flight.
    Executing {
        /// The file and the received branch the import ran with.
        target: stikk_core::ImportTarget,
    },
}

/// See [`App::pending_export`] (RFC 050 decision 5).
enum PendingExport {
    /// `Overlay::Loading` is on the stack; a [`RequestKind::ExportPreview`] is in flight.
    AwaitingPreview {
        /// The branch and the destination.
        target: stikk_core::ExportTarget,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` awaits a yes.
    Confirming {
        /// Feed this into [`stikk_core::export_confirm_and_execute`].
        token: Box<stikk_core::PreviewToken>,
        /// The branch and the destination (must match the preview's).
        target: stikk_core::ExportTarget,
    },
    /// Confirm+execute is in flight.
    Executing,
}

/// See [`App::pending_save`] (RFC 053).
///
/// **One slot for both keys.** `F` and `A` are the same ceremony over a different target, and
/// only one can be in flight — the prompt is modal. Two slots would be two things to keep in
/// step for no behaviour anyone can reach.
enum PendingSave {
    /// `Overlay::Loading` is on the stack; a preview is in flight.
    AwaitingPreview(SaveTarget),
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` awaits a yes.
    Confirming {
        /// Feed this into the matching `*_confirm_and_execute`.
        token: Box<stikk_core::PreviewToken>,
        /// Must match the preview's.
        target: SaveTarget,
    },
    /// Confirm+execute is in flight; the target is kept for the result sentence.
    Executing(SaveTarget),
}

/// Which of the two saves is pending.
#[derive(Clone)]
enum SaveTarget {
    /// One file, from `F`.
    File(stikk_core::SaveFileTarget),
    /// Every file at the block, from `A`.
    All(stikk_core::SaveAllTarget),
}

/// See [`App::pending_merge`] (RFC 047 decision 6).
enum PendingMerge {
    /// `Overlay::Loading` is on the stack; a [`RequestKind::MergePreview`] is in flight.
    AwaitingPreview {
        /// The three names the merge will run with.
        target: stikk_core::MergeTarget,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` awaits a typed name.
    Confirming {
        /// Feed this into [`stikk_core::merge_confirm_and_execute`]. Boxed as every other armed
        /// token here is.
        token: Box<stikk_core::PreviewToken>,
        /// The three names (must match the preview's).
        target: stikk_core::MergeTarget,
    },
    /// Confirm+execute is in flight. **The names are kept** because success re-reads the branch
    /// merged into, and by then the request that carried them is gone.
    Executing {
        /// The three names the merge ran with.
        target: stikk_core::MergeTarget,
    },
}

/// See [`App::pending_ref_write`].
enum PendingRefWrite {
    /// `Overlay::Loading` is on the stack; a [`RequestKind::RefWritePreview`] is in flight.
    AwaitingPreview {
        /// What the preview is for.
        op: RefWriteOp,
    },
    /// The preview succeeded; `token` is armed and `Overlay::Confirmation` awaits evidence.
    Confirming {
        /// Feed this into the matching `stikk_core` execution once evidence is supplied.
        ///
        /// Boxed for the reason the commit flow boxes its own: a token is far larger than the
        /// operation beside it, and `clippy::large_enum_variant` is right about the imbalance.
        token: Box<stikk_core::PreviewToken>,
        /// Carried through from `AwaitingPreview` unchanged — **the previewed call, executed**.
        op: RefWriteOp,
    },
}

impl App {
    /// Open `repo` and send the initial orientation request, building the palette from `config`. The
    /// caller (normally [`crate::run`]) owns the paired worker and receiver.
    #[must_use]
    pub(crate) fn open(
        repo: impl Into<PathBuf>,
        config: &Config,
        session: Option<crate::Session>,
        req_tx: mpsc::Sender<Request>,
    ) -> Self {
        let mut app = Self::new(
            repo,
            OrientationState::Loading,
            Palette::from_theme(config.theme),
            req_tx,
        );
        // Read before the first dispatch, and **shown nowhere** until prikk has confirmed it
        // (RFC 046 decision 5). `load` answers `None` for every failure, so a corrupt or foreign
        // record is simply a launch that starts fresh.
        app.stored = session
            .as_ref()
            .and_then(|s| stikk_state::session::load(&s.state_dir, &s.canonical_root));
        app.session = session;
        let seq = app.dispatch(RequestKind::Orient);
        app.orientation_pending = Some(seq);
        // **Only when there is something to validate.** Without a record, focus resolves from
        // Orientation alone, exactly as it did before sessions existed — no extra read, no
        // extra wait.
        if app.stored.is_some() {
            app.restore_refs_pending = Some(app.dispatch(RequestKind::Refs));
        }
        app
    }

    /// Construct an app in an explicit state, without sending any request — used by tests so they can
    /// drive [`App::apply`] directly without a real load (handoff §8: "do not write a test that
    /// sleeps").
    ///
    /// **A `Loaded` state starts focused on `heads/main`**, as if its first read had resolved there, so a
    /// test about something else needs no resolution step. Any other state starts `Pending`, and a test
    /// about resolution (RFC 029 Handoff B) starts there and drives `apply`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn from_state(
        repo: impl Into<PathBuf>,
        state: OrientationState,
        palette: Palette,
        req_tx: mpsc::Sender<Request>,
    ) -> Self {
        let mut app = Self::new(repo, state, palette, req_tx);
        if matches!(app.state, OrientationState::Loaded(_)) {
            app.ref_focus = RefFocus::Ref(DEFAULT_REF.to_string());
        }
        app
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
            ref_focus: RefFocus::Pending,
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
            marked_block: None,
            marked_ref: None,
            pending_commit: None,
            pending_seal: None,
            pending_ref_write: None,
            pending_switch: None,
            pending_merge: None,
            pending_import: None,
            pending_export: None,
            pending_save: None,
            ref_rows: Vec::new(),
            session: None,
            stored: None,
            restore_refs_pending: None,
            restore_refs: None,
            saved: None,
            hide_untracked_default: false,
            session_write_failed: false,
            stamp: None,
            check_pending: None,
            interval_from: None,
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

    /// Re-source the visible screens from prikk (design FR-106; the `r` key, and a detected change, RFC
    /// 031 §6): always re-requests Orientation, and the top screen too, in place, when it is History, the
    /// Queue, Changes, or the tip's Block detail. The current view stays visible while either refresh is
    /// in flight (RFC 010 §5); only the `⟳ n` indicator shows anything is happening.
    pub fn reload(&mut self) {
        self.banner = None;
        self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
        // The top screen refreshes in place, its view visible while the read runs (RFC 010 §5). Every
        // in-place refresh re-reads the ref its screen shows, never the focused ref: the ref picker can
        // push another ref's History above this screen and leave focus there (RFC 031 review v1 §2.2).
        let request = match self.screens.last() {
            Some(Screen::History { view, .. }) => Some(RequestKind::History {
                reff: view.reff.clone(),
            }),
            Some(Screen::Queue { .. }) => Some(RequestKind::Queue),
            // RFC 035 §4.3: a queued patch can be sealed while its screen is open, so `r` re-reads the
            // same patch id and renders what prikk now says. A patch opened from a block's report is
            // not re-read at all — a sealed block's patches are fixed, the same reason an older
            // block's detail is never refreshed (RFC 031 §7).
            Some(Screen::PatchDetail {
                source: PatchSource::Queued { patch_id, message },
                ..
            }) => Some(RequestKind::PatchDetail {
                patch_id: patch_id.clone(),
                message: message.clone(),
            }),
            Some(Screen::PatchDetail {
                source: PatchSource::InBlock,
                ..
            }) => None,
            // A comparison between two **blocks** cannot change, for the same reason a sealed
            // block's detail cannot. One against the **worktree** can change under the reader at
            // any moment, and that is the comparison `r` exists for.
            Some(Screen::Compare { opened, .. }) => match opened {
                CompareSource::WorktreeAgainstBaseline => Some(RequestKind::Compare {
                    comparison: stikk_prikk::Comparison::WorktreeAgainstBaseline,
                    opened: CompareSource::WorktreeAgainstBaseline,
                }),
                // **A pair of refs is re-asked by name** (RFC 041 decision 3): a ref moves, so
                // `r` compares wherever they point *now*, and the heading's block ids change with
                // it. Re-sending the blocks prikk resolved last time would re-run yesterday's
                // comparison under today's heading.
                CompareSource::RefPair { from, to } => Some(RequestKind::Compare {
                    comparison: stikk_prikk::Comparison::Points {
                        from: from.clone(),
                        to: to.clone(),
                    },
                    opened: CompareSource::RefPair {
                        from: from.clone(),
                        to: to.clone(),
                    },
                }),
                // Two sealed blocks cannot change, for the same reason a sealed block's detail
                // cannot.
                CompareSource::MarkedPair | CompareSource::PreviousInHistory => None,
            },
            // **`r` re-runs the verification** (RFC 040 decision 3). A verdict is about the
            // moment it was taken and is never cached across runs, so this is the only way to a
            // newer one — and it only ever happens because the reader asked.
            Some(Screen::Verify { .. }) => Some(RequestKind::Verify),
            // RFC 054: `r` re-reads the list. Adopting a key happens in prikk, in another
            // terminal, so this is the only way to a newer answer.
            Some(Screen::Trust { .. }) => Some(RequestKind::TrustRead),
            // RFC 031 §7: Changes re-reads the ref it shows.
            Some(Screen::Changes { view, .. }) => Some(RequestKind::Changes {
                reff: view.reff.clone(),
            }),
            // RFC 031 §7: only the tip's detail can change.
            Some(Screen::BlockDetail { reff, view, .. }) if view.is_tip => {
                Some(RequestKind::BlockState {
                    reff: reff.clone(),
                    row: view.row.clone(),
                    is_tip: true,
                })
            }
            // **The Merge plan is not re-read in place** (RFC 047): it holds no `refreshing`
            // slot, and a plan is about the two tips as they stood when it was read. `q` and `M`
            // ask for a current one, which is the same two reads without a half-updated screen.
            // **The Bundle view is not re-read in place either** (RFC 050): it holds no
            // `refreshing` slot, and what a file is does not change under the reader. `i` asks
            // again, with the path still in hand.
            Some(
                Screen::BlockDetail { .. }
                | Screen::MergePlan { .. }
                | Screen::Bundle { .. }
                | Screen::Loading { .. },
            )
            | None => None,
        };
        let Some(kind) = request else {
            return;
        };
        let seq = self.dispatch(kind);
        if let Some(
            Screen::History { refreshing, .. }
            | Screen::Verify { refreshing, .. }
            | Screen::Queue { refreshing, .. }
            | Screen::Changes { refreshing, .. }
            | Screen::PatchDetail { refreshing, .. }
            | Screen::BlockDetail { refreshing, .. },
        ) = self.screens.last_mut()
        {
            *refreshing = Some(seq);
        }
    }

    /// Called once per UI-loop iteration with the current time (RFC 031 §5). Sends a silent change check
    /// when 5 seconds have passed since the last check was sent or the last token was stamped, and the
    /// check's conditions hold: a token stamped, no check pending, no request running, no Orientation
    /// read pending. Does no I/O otherwise.
    pub fn tick(&mut self, now: Instant) {
        let Some(from) = self.interval_from else {
            if self.stamp.is_some() {
                self.interval_from = Some(now);
            }
            return;
        };
        if now.saturating_duration_since(from) >= CHANGE_CHECK_INTERVAL {
            self.send_check(now);
        }
    }

    /// The terminal reported focus returning (RFC 031 §5): check at once, when the check's conditions
    /// hold. Counts as a check sent for the interval.
    pub fn focus_gained(&mut self, now: Instant) {
        self.send_check(now);
    }

    /// Send the silent change check (RFC 031 §4) **only** when a token has been stamped, no check is
    /// pending, no request the user caused is running, and no Orientation read is pending. The last two
    /// are what keep stikk from ever taking its own commit or seal for an outside change: Enter
    /// dispatches confirm-and-execute, and its success dispatches Orientation, which stamps a token taken
    /// after the write. **Nothing else suppresses it** — an open confirmation especially, since §6 needs
    /// to check while one waits on the user.
    ///
    /// Sent without [`Self::dispatch`], so it adds no [`Operation`]: a check is never in the Operations
    /// list or `⟳ n`.
    fn send_check(&mut self, now: Instant) {
        if self.stamp.is_none()
            || self.check_pending.is_some()
            || self.in_flight_count() != 0
            || self.orientation_pending.is_some()
        {
            return;
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        let _ = self.req_tx.send(Request {
            seq,
            kind: RequestKind::ChangeCheck,
        });
        self.check_pending = Some(seq);
        self.interval_from = Some(now);
    }

    /// Answers [`RequestKind::ChangeCheck`] (RFC 031 §4, §6). A failure is silent. An equal token does
    /// nothing. A differing one re-stamps, makes an armed confirmation stale, refreshes what is on screen
    /// through [`Self::reload`], and then — since `reload` clears the banner — says so in `OP-04`'s words.
    fn apply_change_check(&mut self, seq: u64, result: stikk_model::Result<ChangeToken>) {
        if self.check_pending != Some(seq) {
            return;
        }
        self.check_pending = None;
        let (Ok(current), Some(previous)) = (result, self.stamp) else {
            return;
        };
        let Some(Presentation::Banner { message, .. }) = staleness_notice(&previous, &current)
        else {
            return;
        };
        self.stamp = Some(current);
        self.interval_from = None;
        self.stale_armed_confirmation();
        self.reload();
        self.banner = Some(message);
    }

    /// RFC 031 Q2 (b): an armed confirmation open when a repository change is detected is replaced at once
    /// by the stale overlay, cause *Repository*, built through `present()` so the words are RFC 030's.
    ///
    /// **Searched through the whole overlay stack, not just its top** (review v1 §2.1): over a tier-2 or
    /// untyped tier-3 card, `?`, `:`, `o` and `R` open their overlays on top of it, and that card would
    /// otherwise stay armed one Esc away. The armed overlay and **every overlay above it** are removed, since
    /// they were opened over a flow that no longer exists. The pending flows are cleared as [`Self::back`]
    /// clears them, so nothing stays armed. A `CommitMessage` is not armed (no preview exists yet) and is
    /// left open.
    fn stale_armed_confirmation(&mut self) {
        let (commit, seal) = (self.pending_commit.is_some(), self.pending_seal.is_some());
        let armed = |overlay: &Overlay| match overlay {
            Overlay::Confirmation { .. } => commit || seal,
            Overlay::SealConsent { .. } => true,
            _ => false,
        };
        let Some(index) = self.overlays.iter().rposition(armed) else {
            return;
        };
        let (operation, context) = if commit {
            (COMMIT_OPERATION, OperationContext::Commit)
        } else {
            (SEAL_OPERATION, OperationContext::Other)
        };
        self.overlays.truncate(index);
        self.pending_commit = None;
        self.pending_seal = None;
        let stale = StikkError::Stale {
            operation: operation.to_string(),
            cause: StaleCause::Repository,
        };
        self.surface(&stale, context);
    }

    /// Open the History view for the focused ref, pushing a pending placeholder above the current
    /// screen until the response arrives.
    pub fn open_history(&mut self) {
        self.banner = None;
        let Some(reff) = self.require_focus() else {
            return;
        };
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
        let Some(reff) = self.require_focus() else {
            return;
        };
        let seq = self.dispatch(RequestKind::Changes { reff });
        self.screens.push(Screen::Loading {
            what: "changes",
            seq,
        });
    }

    /// Open the Queue view (the `Q` key, or the palette; RFC 028 Handoff A §6). **Needs no focused ref**:
    /// the queue is repository-wide.
    pub fn open_queue(&mut self) {
        self.banner = None;
        let seq = self.dispatch(RequestKind::Queue);
        self.screens.push(Screen::Loading { what: "queue", seq });
    }

    /// Open the Verify view (the `V` key, or the palette; `FR-100`, RFC 040 decision 3). **Needs
    /// no focused ref**: verification is repository-wide.
    ///
    /// **Only ever on demand.** `verify` scans the whole repository, so nothing else starts it:
    /// not startup, not the change check, not another screen's refresh. `V` is capital for the
    /// same reason `Q` and `R` are — it opens a repository-wide view — and lowercase `v` stays the
    /// raw toggle it already is everywhere else.
    pub fn open_verify(&mut self) {
        self.banner = None;
        let seq = self.dispatch(RequestKind::Verify);
        self.screens.push(Screen::Loading {
            what: "verify",
            seq,
        });
    }

    /// `T` anywhere a repository is open (RFC 054 decision 2): the keys this repository trusts.
    pub fn open_trust(&mut self) {
        self.banner = None;
        let seq = self.dispatch(RequestKind::TrustRead);
        self.screens.push(Screen::Loading { what: "trust", seq });
    }

    /// Answers [`RequestKind::TrustRead`]. **Nothing here can fail into a banner**: below the
    /// band and a refused list are both things the view itself says, so the screen opens either
    /// way and says what it knows.
    fn apply_trust_read(&mut self, seq: u64, result: stikk_model::Result<stikk_core::TrustView>) {
        let Some(index) = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: t, .. } if *t == seq))
        else {
            return; // stale
        };
        match result {
            Ok(view) => {
                if let Some(slot) = self.screens.get_mut(index) {
                    *slot = Screen::Trust {
                        view,
                        offset: std::cell::Cell::new(0),
                    };
                }
            }
            Err(error) => {
                self.screens.remove(index);
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Toggle the display-only untracked filter on a focused Changes screen (the `u` key; UD-08).
    pub fn toggle_untracked(&mut self) {
        if let Some(Screen::Changes { hide_untracked, .. }) = self.screens.last_mut() {
            *hide_untracked = !*hide_untracked;
            self.hide_untracked_default = *hide_untracked;
            self.flush_session();
        }
    }

    /// Toggle Patch detail's raw view (the `v` key; RFC 035 decision 5) — prikk's own fields for each
    /// operation instead of stikk's rendering of them. **A toggle, never a second read**: both sides
    /// come out of the one report already in hand, which is also the only place an `unresolved_node_id`
    /// is legible rather than apologised for (F6). A no-op on every other screen, as `u` is.
    pub fn toggle_raw(&mut self) {
        match self.screens.last_mut() {
            Some(Screen::PatchDetail { raw, .. })
            | Some(Screen::Compare { raw, .. })
            | Some(Screen::Verify { raw, .. }) => {
                *raw = !*raw;
            }
            _ => {}
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
        match stikk_core::capability_gate(
            stikk_core::COMMIT_OPERATION,
            stikk_model::RequestCategory::QueueMutation,
            readiness,
        ) {
            Ok(()) => {
                // After the capability check, as the palette orders its reasons (`Command::
                // unavailable_reason`): the two must agree.
                let Some(reff) = self.require_focus() else {
                    return;
                };
                self.overlays.push(Overlay::CommitMessage {
                    reff,
                    typed: String::new(),
                    messages_persist: self.messages_persist(),
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
        let seq = self.dispatch(RequestKind::CommitPreview { reff });
        self.overlays.push(Overlay::Loading {
            what: "commit preview",
            seq,
        });
        self.pending_commit = Some(PendingCommit::AwaitingPreview { message });
    }

    /// The confirmation step's `Enter`: dispatch confirm+execute with whatever evidence the tier needs.
    /// Tier 2 (commit's own tier) needs only an explicit yes; the typed-name shape stays reachable for
    /// whichever tier-3-typed operation lands next, matching this overlay's general contract (RFC 013
    /// §6) even though nothing at tier 3-typed drives it through `App` yet.
    fn submit_commit_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingCommit::Confirming { token, message }) = self.pending_commit.take() else {
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
            message,
        });
        self.overlays.push(Overlay::Loading {
            what: "commit",
            seq,
        });
    }

    /// Begin the seal flow (`FL-06` step 1; the `S` key; RFC 016 §5). Checked here, before anything is
    /// dispatched (`C-T4d`), the same [`stikk_core::capability_gate`] check the palette's `op.seal`
    /// entry uses — the two must agree, matching `begin_commit`'s own precedent.
    pub fn begin_seal(&mut self) {
        self.banner = None;
        let readiness = self.readiness();
        match stikk_core::capability_gate(
            SEAL_OPERATION,
            stikk_model::RequestCategory::Publication,
            readiness,
        ) {
            Ok(()) => {
                let Some(reff) = self.require_focus() else {
                    return;
                };
                let seq = self.dispatch(RequestKind::SealPreview { reff: reff.clone() });
                self.overlays.push(Overlay::Loading {
                    what: "seal preview",
                    seq,
                });
                self.pending_seal = Some(PendingSeal::AwaitingPreview { reff });
            }
            Err(stikk_model::StikkError::NotReady { detail }) => self.banner = Some(detail),
            // `capability_gate` only ever returns `NotReady`; kept exhaustive so a future change to
            // that contract fails here loudly rather than silently swallowing a new error shape.
            Err(other) => self.banner = Some(other.to_string()),
        }
    }

    /// The confirmation step's `Enter` for a pending seal (RFC 016 §5's first act). **Does not**
    /// dispatch anything yet — unlike commit, seal has a second, distinct act still to come (the
    /// no-audit consent, RFC 016 §8), so this only records the evidence and opens
    /// [`Overlay::SealConsent`]. Seal stays untyped (RFC 013 Q3), so `tier`/`typed` are accepted for
    /// shape parity with [`Self::submit_commit_confirmation`] but only `Evidence::ExplicitYes` is ever
    /// produced in practice.
    fn submit_seal_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingSeal::Confirming { token, reff }) = self.pending_seal.take() else {
            return; // no seal is actually pending — a stray Enter on a Confirmation this app did not open
        };
        let _ = (tier, typed); // seal's confirm() evidence is always ExplicitYes (RFC 013 Q3)
        self.overlays.pop();
        self.overlays.push(Overlay::SealConsent {
            reff: reff.clone(),
            acknowledged: false,
        });
        self.pending_seal = Some(PendingSeal::AwaitingConsent { token, reff });
    }

    /// Toggle the seal consent acknowledgement (RFC 016 §8; the Space key) — the only way
    /// `Overlay::SealConsent.acknowledged` ever changes. Reaching this screen implies nothing; only
    /// this explicit act does.
    pub fn toggle_seal_consent(&mut self) {
        if let Some(Overlay::SealConsent { acknowledged, .. }) = self.overlays.last_mut() {
            *acknowledged = !*acknowledged;
        }
    }

    /// The consent step's `Enter` (RFC 016 §8's second act): dispatches confirm+execute **only when
    /// acknowledged is true** — an unacknowledged `Enter` does nothing, which is the whole point of
    /// this step existing (cannot be defaulted).
    fn submit_seal_consent(&mut self) {
        let Some(Overlay::SealConsent { acknowledged, .. }) = self.overlays.last() else {
            return;
        };
        if !*acknowledged {
            return;
        }
        let Some(PendingSeal::AwaitingConsent { token, reff }) = self.pending_seal.take() else {
            return; // defensive: should not happen if SealConsent is on the stack at all
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::SealConfirmExecute {
            token: Box::new(token),
            readiness,
            evidence: stikk_core::Evidence::ExplicitYes,
            reff,
        });
        self.overlays.push(Overlay::Loading { what: "seal", seq });
    }

    /// **stikk's own words for a row with no tip** (RFC 044 decision 7, following RFC 041
    /// decision 4's precedent). The branch you are on before its first seal names nothing prikk
    /// published, so there is nothing to branch from, tag or close -- and prikk's own answer for
    /// it is *"ref heads/main does not exist in this repository"*, which is false and alarming.
    const UNPUBLISHED_HAS_NO_TIP: &'static str = "this branch has no published history yet \u{2014} there is no tip to branch from, to tag, or \
         to close. Commit and seal on it first.";

    /// **stikk's own words for `x` on a row that is already closed** (RFC 044 §3), read from the
    /// list's own `closed` flag, so prikk is never asked a question stikk already has the answer
    /// to -- and the reader is never shown prikk's *"is already closed"* for a state the picker
    /// was showing them at the moment they pressed the key.
    const ALREADY_CLOSED: &'static str = "this branch is already closed.";

    /// **stikk's own words for `x` on a tag** (RFC 044 decision 4): closing is a branch
    /// operation, and prikk has no delete for either.
    const TAGS_ARE_NOT_CLOSED: &'static str =
        "only a branch can be closed; a tag has no open or closed state, and prikk has no delete.";

    /// Begin creating a branch at the selected ref's tip (the `n` key in the ref picker; RFC 044
    /// decision 4). Opens the name prompt; **nothing is dispatched until a name exists**.
    pub fn begin_create_branch(&mut self) {
        let Some(from_ref) = self.ref_write_source() else {
            return;
        };
        self.overlays.pop(); // the picker closes on its way, as `d` does
        self.overlays.push(Overlay::RefName {
            kind: RefNameKind::Branch,
            source: from_ref,
            typed: String::new(),
        });
    }

    /// Begin creating a tag (the `t` key; RFC 044 decision 4): **at the selected ref's tip** in
    /// the picker, and **at the selected block, by block id**, in History and Block detail. Those
    /// are the two things prikk's `--target` takes (F2), and the key means the one the reader is
    /// looking at.
    pub fn begin_create_tag(&mut self) {
        let target = if matches!(self.overlays.last(), Some(Overlay::RefPicker { .. })) {
            let Some(reff) = self.ref_write_source() else {
                return;
            };
            self.overlays.pop();
            reff
        } else {
            if !self.overlays.is_empty() || !self.ref_writes_offered() {
                return;
            }
            match self.screens.last() {
                Some(Screen::History { view, cursor, .. }) => {
                    let Some(row) = view.blocks.get(*cursor) else {
                        return;
                    };
                    row.block_id.clone()
                }
                Some(Screen::BlockDetail { view, .. }) => view.row.block_id.clone(),
                _ => return,
            }
        };
        self.overlays.push(Overlay::RefName {
            kind: RefNameKind::Tag,
            source: target,
            typed: String::new(),
        });
    }

    /// Begin closing the selected branch (the `x` key in the ref picker; RFC 044 decision 4).
    ///
    /// **Three answers come from what stikk already knows**, before any preview: a tag cannot be
    /// closed, an already-closed branch says so from the list's own flag, and a row with no
    /// published tip has nothing to close. **The fourth -- closing the branch you are on -- is
    /// core's** ([`stikk_core::CLOSING_THE_CURRENT_BRANCH`]), because only the preview's own read
    /// knows which branch that is at the moment the key is pressed.
    pub fn begin_close_branch(&mut self) {
        let Some(name) = self.ref_write_source() else {
            return;
        };
        let flags = self
            .ref_rows
            .iter()
            .find(|entry| entry.name == name)
            .map_or((false, false), |entry| (entry.is_tag(), entry.closed));
        match flags {
            (true, _) => {
                self.banner = Some(Self::TAGS_ARE_NOT_CLOSED.to_string());
                return;
            }
            (_, true) => {
                self.banner = Some(Self::ALREADY_CLOSED.to_string());
                return;
            }
            _ => {}
        }
        self.overlays.pop(); // the picker closes on its way, as `d` does
        self.start_ref_write(RefWriteOp::CloseBranch { name });
    }

    /// The selected published ref the three keys act on, or `None` with stikk's own reason already
    /// in the banner.
    ///
    /// **Everything the three keys share** -- the band, the capability, and the rows that name no
    /// published tip -- so none of them can answer one of those differently from the others.
    fn ref_write_source(&mut self) -> Option<String> {
        self.banner = None;
        let Some(Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        }) = self.overlays.last()
        else {
            return None;
        };
        let (cursor, unpublished_main, first_is_unpublished) =
            (*cursor, *unpublished_main, *unpublished_current);
        let name = refs.get(cursor).cloned();
        if !self.ref_writes_offered() {
            return None;
        }
        // The guessed row of an empty picker (RFC 029 Handoff B §4) and the unpublished current
        // branch (RFC 037 decision 3) are the same case here: neither names a published tip.
        if unpublished_main || (first_is_unpublished && cursor == 0) {
            self.banner = Some(Self::UNPUBLISHED_HAS_NO_TIP.to_string());
            return None;
        }
        name
    }

    /// Whether the three ref writes are offered at all: the band first, then the capability.
    ///
    /// **The band comes first** because below it the operations do not exist -- telling a reader
    /// they need a maintainer key for something their prikk cannot do either way would send them
    /// to fix the wrong thing. Above it, [`stikk_core::capability_gate`] is the same check the
    /// palette's own entries use, in stikk's words and **before prikk runs** (decision 2), so
    /// prikk's signature refusal is the fallback and never the first thing a user meets.
    fn ref_writes_offered(&mut self) -> bool {
        self.banner = None;
        if let Some(minor) = self.prikk_minor()
            && minor < stikk_core::REF_WRITES_FROM_MINOR
        {
            self.banner = Some(stikk_core::ref_writes_unavailable(minor));
            return false;
        }
        let readiness = self.readiness();
        match stikk_core::capability_gate(
            stikk_core::CREATE_BRANCH_OPERATION,
            stikk_model::RequestCategory::Publication,
            readiness,
        ) {
            Ok(()) => true,
            Err(stikk_model::StikkError::NotReady { detail }) => {
                self.banner = Some(detail);
                false
            }
            // `capability_gate` only ever returns `NotReady`; kept exhaustive so a future change to
            // that contract fails here loudly rather than silently swallowing a new error shape.
            Err(other) => {
                self.banner = Some(other.to_string());
                false
            }
        }
    }

    /// **stikk's own words for `s` on a tag** (RFC 045 decision 7): a switch moves the current
    /// branch, and a tag is not one.
    const TAGS_ARE_NOT_SWITCHED_TO: &'static str =
        "only a branch can be switched to; a tag names a point in history, not a place to work.";

    /// **stikk's own words for `s` on the branch already current** — the same fact
    /// [`stikk_core::ALREADY_THE_CURRENT_BRANCH`] states, said here so prikk is never asked.
    const ALREADY_HERE: &'static str = stikk_core::ALREADY_THE_CURRENT_BRANCH;

    /// **stikk's own words for `s` on a closed branch**, from the list's own flag — prikk refuses
    /// it too, and being told by the list you are looking at beats being told by a spawn.
    const CLOSED_CANNOT_BE_SWITCHED_TO: &'static str =
        "this branch is closed; switch to an open one.";

    /// Begin switching to the selected branch (the `s` key in the ref picker; RFC 045 decision 7).
    ///
    /// **Four answers come from what stikk already knows**, before any preview: a tag cannot be
    /// switched to, a closed branch says so from the list's own flag, the branch already current
    /// says so from orientation, and a row with no published tip has nothing to switch to. The
    /// band is checked first, as RFC 044's keys check theirs.
    pub fn begin_switch_branch(&mut self) {
        let Some(target) = self.switch_target() else {
            return;
        };
        self.overlays.pop(); // the picker closes on its way, as `d` and `n` do
        let seq = self.dispatch(RequestKind::SwitchPreview {
            target: target.clone(),
        });
        self.overlays.push(Overlay::Loading {
            what: "switch preview",
            seq,
        });
        self.pending_switch = Some(PendingSwitch::AwaitingPreview { target });
    }

    /// The branch `s` acts on, or `None` with stikk's own reason already in the banner.
    fn switch_target(&mut self) -> Option<String> {
        self.banner = None;
        let Some(Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        }) = self.overlays.last()
        else {
            return None;
        };
        let (cursor, unpublished_main, first_is_unpublished) =
            (*cursor, *unpublished_main, *unpublished_current);
        let name = refs.get(cursor).cloned();
        if let Some(minor) = self.prikk_minor()
            && minor < stikk_core::SWITCH_FROM_MINOR
        {
            self.banner = Some(stikk_core::switch_unavailable(minor));
            return None;
        }
        // A switch needs read-write and no key (RFC 045 decision 2) — the gate reads that from
        // `Materialization` itself, so read-only is the only session this refuses.
        let readiness = self.readiness();
        if let Err(error) = stikk_core::capability_gate(
            stikk_core::SWITCH_OPERATION,
            stikk_model::RequestCategory::Materialization,
            readiness,
        ) {
            self.banner = Some(error.to_string());
            return None;
        }
        if unpublished_main || (first_is_unpublished && cursor == 0) {
            self.banner = Some(Self::UNPUBLISHED_HAS_NO_TIP.to_string());
            return None;
        }
        let name = name?;
        if stikk_model::CurrentBranch::Branch(stikk_model::RefName::parse(&name).ok()?)
            == self.current_branch()
        {
            self.banner = Some(Self::ALREADY_HERE.to_string());
            return None;
        }
        match self
            .ref_rows
            .iter()
            .find(|entry| entry.name == name)
            .map_or((false, false), |entry| (entry.is_tag(), entry.closed))
        {
            (true, _) => {
                self.banner = Some(Self::TAGS_ARE_NOT_SWITCHED_TO.to_string());
                None
            }
            (_, true) => {
                self.banner = Some(Self::CLOSED_CANNOT_BE_SWITCHED_TO.to_string());
                None
            }
            _ => Some(name),
        }
    }

    /// **The five sentences `M` declines with live in `stikk-core`** (RFC 047 Handoff B §2).
    ///
    /// The preview re-checks every one of these against freshly read state, because the current
    /// branch or the queue can change between the view opening and the confirmation. Two copies
    /// of the same sentence are two things that can drift, and the one that drifts is the one
    /// the reader sees less often -- so there is one copy, and this key and that preview answer
    /// from it.
    /// **stikk's own words for `M` with nothing marked** -- `d`'s sentence, for `d`'s reason.
    const NOTHING_IS_MARKED: &'static str = "mark a branch with m first";

    /// Open the Merge plan for *the marked branch into the selected one* (the `M` key; RFC 047
    /// decision 4).
    ///
    /// **Every guard answers from what stikk already holds** (decision 5): the band from
    /// orientation, the current branch and the queue from the same read, closed and tag from the
    /// ref list the picker is drawn from. None of them spawns prikk, so a merge stikk will not
    /// show costs nothing to be told about.
    pub fn begin_merge_plan(&mut self) {
        let Some((into, from)) = self.merge_pair() else {
            return;
        };
        self.overlays.pop(); // the picker closes on its way, as `d` and `s` do
        let seq = self.dispatch(RequestKind::MergePlan { into, from });
        self.screens.push(Screen::Loading {
            what: "merge plan",
            seq,
        });
    }

    /// Open the path prompt for a bundle file (the `i` key; RFC 050 decision 3).
    ///
    /// **Wherever a repository is open**, not only in the ref picker: a bundle arrives from
    /// outside the repository and belongs to no ref, so there is no row for it to hang off.
    ///
    /// **The band answers before the prompt**, from orientation and without spawning anything:
    /// asking a reader to type a path and then telling them this prikk cannot read it would
    /// waste the typing.
    pub fn begin_open_bundle(&mut self) {
        self.banner = None;
        if !self.overlays.is_empty() {
            return;
        }
        if let Some(minor) = self.prikk_minor()
            && minor < stikk_core::BUNDLE_FROM_MINOR
        {
            self.banner = Some(stikk_core::bundle_unavailable(minor));
            return;
        }
        self.overlays.push(Overlay::BundlePath {
            typed: String::new(),
        });
    }

    /// The bundle path prompt's `Enter` (RFC 050 decision 3). **An empty path is not sent** --
    /// the same rule the name prompt applies, for the same reason: there is nothing to ask prikk.
    ///
    /// **The path is resolved here, on the UI thread**, against the directory stikk was started
    /// in -- the process's own working directory, which stikk never changes. Resolving here and
    /// passing the result means the worker runs, and the view shows, one string.
    fn submit_bundle_path(&mut self, typed: &str) {
        let typed = typed.trim();
        if typed.is_empty() {
            return;
        }
        self.overlays.pop();
        // `std::path::absolute` joins and tidies **without touching the filesystem**:
        // `canonicalize` would fail on a path that does not exist, and *this file is not there*
        // is prikk's answer to give, in prikk's words, not a shape stikk invents before asking.
        let path = std::path::Path::new(typed);
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let seq = self.dispatch(RequestKind::OpenBundle { path });
        self.screens.push(Screen::Loading {
            what: "bundle",
            seq,
        });
    }

    /// The pair `M` means -- **the marked branch into the selected one** -- or `None` with
    /// stikk's own reason already in the banner.
    fn merge_pair(&mut self) -> Option<(String, String)> {
        self.banner = None;
        let Some(Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        }) = self.overlays.last()
        else {
            return None;
        };
        let (cursor, unpublished_main, first_is_unpublished) =
            (*cursor, *unpublished_main, *unpublished_current);
        let into = refs.get(cursor).cloned();
        // **The band first**, as `s` and RFC 044's keys check theirs: below it the operation does
        // not exist, and a sentence about marking a branch would send the reader to fix the
        // wrong thing.
        if let Some(minor) = self.prikk_minor()
            && minor < stikk_core::MERGE_FROM_MINOR
        {
            self.banner = Some(stikk_core::merge_unavailable(minor));
            return None;
        }
        // The guessed row of an empty picker and the unpublished current branch name no
        // published tip, so neither side can be one (RFC 037 B).
        if unpublished_main || (first_is_unpublished && cursor == 0) {
            self.banner = Some(Self::UNPUBLISHED_HAS_NO_TIP.to_string());
            return None;
        }
        let into = into?;
        let Some(from) = self.marked_ref.clone() else {
            self.banner = Some(Self::NOTHING_IS_MARKED.to_string());
            return None;
        };
        if from == into {
            self.banner = Some(stikk_core::THE_SAME_BRANCH_TWICE.to_string());
            return None;
        }
        // **Both sides**, against the list the picker is drawn from: a tag or a closed branch is
        // no more mergeable for being the marked one.
        for name in [&into, &from] {
            match self
                .ref_rows
                .iter()
                .find(|entry| &entry.name == name)
                .map_or((false, false), |entry| (entry.is_tag(), entry.closed))
            {
                (true, _) => {
                    self.banner = Some(stikk_core::TAGS_ARE_NOT_MERGED.to_string());
                    return None;
                }
                (_, true) => {
                    self.banner = Some(stikk_core::CLOSED_IS_NOT_MERGED.to_string());
                    return None;
                }
                _ => {}
            }
        }
        // **F3 and F4, and only about `into`.** Both are facts about the branch being merged
        // *into*; the branch merged *from* is read, not moved, and neither applies to it.
        if stikk_model::RefName::parse(&into)
            .ok()
            .map(stikk_model::CurrentBranch::Branch)
            == Some(self.current_branch())
        {
            self.banner = Some(stikk_core::INTO_THE_CURRENT_BRANCH.to_string());
            return None;
        }
        if let OrientationState::Loaded(view) = &self.state
            && view.queued_patches > 0
            && view.queued_target.as_deref() == Some(into.as_str())
        {
            self.banner = Some(stikk_core::WORK_IS_QUEUED_FOR_INTO.to_string());
            return None;
        }
        Some((into, from))
    }

    /// Begin the merge the open Merge plan describes (`Enter`; RFC 047 decision 6).
    ///
    /// **Only from a `Confluent` plan.** prikk merges nothing else, so a confirmation for any
    /// other outcome would be a question whose only answer is a refusal; the banner says which
    /// word prikk used instead.
    pub fn begin_merge(&mut self) {
        self.banner = None;
        let Some(Screen::MergePlan { view, .. }) = self.screens.last() else {
            return;
        };
        if !view.outcome.is_confluent() {
            self.banner = Some(stikk_core::not_confluent(view.outcome));
            return;
        }
        let shown_baseline = view.baseline_short();
        let target = stikk_core::MergeTarget {
            baseline: view.baseline.clone(),
            into: view.into.clone(),
            from: view.from.clone(),
        };
        let seq = self.dispatch(RequestKind::MergePreview {
            target: target.clone(),
            shown_baseline,
        });
        self.overlays.push(Overlay::Loading {
            what: "merge preview",
            seq,
        });
        self.pending_merge = Some(PendingMerge::AwaitingPreview { target });
    }

    /// Begin the import the open Bundle view describes (the `I` key; RFC 050 decision 4).
    ///
    /// **Only where the view has prikk's verification and its tip line** (§3b): without the tip
    /// there is no older-bundle guard, and an import with no guard is the one F2 measured. The
    /// view does not advertise `I` in that state either, so this is the same answer twice --
    /// here because a key must not act on a screen that does not offer it.
    pub fn begin_import(&mut self) {
        self.banner = None;
        let Some(Screen::Bundle { view, .. }) = self.screens.last() else {
            return;
        };
        let stikk_core::BundleOpened::Read(view) = view.as_ref() else {
            return;
        };
        let Some(received) = view.received_branch() else {
            return;
        };
        let target = stikk_core::ImportTarget {
            path: std::path::PathBuf::from(&view.path),
            received,
        };
        let seq = self.dispatch(RequestKind::ImportPreview {
            target: target.clone(),
        });
        self.overlays.push(Overlay::Loading {
            what: "import preview",
            seq,
        });
        self.pending_import = Some(PendingImport::AwaitingPreview { target });
    }

    /// The confirmation step's `Enter` for a pending import: Tier Two, and no key.
    fn submit_import_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingImport::Confirming { token, target }) = self.pending_import.take() else {
            return; // no import is actually pending
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::ImportConfirmExecute {
            token,
            readiness,
            evidence,
            target: target.clone(),
        });
        self.overlays.push(Overlay::Loading {
            what: "import",
            seq,
        });
        self.pending_import = Some(PendingImport::Executing { target });
    }

    /// Answers [`RequestKind::ImportPreview`]. A block is a banner; the Bundle view stays open
    /// underneath it, which is where the reader can see prikk's own words.
    fn apply_import_preview(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::ImportPreview>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale: the reader navigated away before the preview resolved
        };
        let pending = self.pending_import.take();
        match result {
            Ok(stikk_core::ImportPreview::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
            }
            Ok(stikk_core::ImportPreview::Ready { token }) => {
                let Some(PendingImport::AwaitingPreview { target }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_import = Some(PendingImport::Confirming { token, target });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// Answers [`RequestKind::ImportConfirmExecute`].
    ///
    /// On success: prikk's lines verbatim on the result card, the ref lists re-read, and
    /// **focus moves to the received branch with its History open** (`FR-091`). Unlike a merge,
    /// this is a move the reader asked for: they pointed stikk at a file and said import it,
    /// and what arrived is a branch they have never seen. Leaving them on the Bundle view
    /// would make them go and find it.
    fn apply_import_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::BundleImport>>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        let target = match self.pending_import.take() {
            Some(
                PendingImport::Executing { target } | PendingImport::Confirming { target, .. },
            ) => Some(target),
            Some(PendingImport::AwaitingPreview { .. }) | None => None,
        };
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::BundleImport::Done(printed) => {
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::ImportResult { printed };
                    }
                    if let Some(target) = target {
                        self.clear_ref_mark_on_ref_change(&target.received);
                        self.ref_focus = RefFocus::Ref(target.received.clone());
                        let seq = self.dispatch(RequestKind::History {
                            reff: target.received,
                        });
                        self.screens.push(Screen::Loading {
                            what: "history",
                            seq,
                        });
                    }
                    self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
                    self.dispatch(RequestKind::Refs);
                }
                // The band is checked before the key offers anything, so this is only reachable
                // if prikk changed under the session.
                stikk_prikk::BundleImport::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::bundle_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// Begin exporting the selected branch (the `E` key in the ref picker; RFC 050 decision 5).
    ///
    /// **Four rows are declined from what stikk already holds**, before any preview: a tag, a
    /// received ref, a closed branch and the unpublished row. The preview re-asks them all from
    /// freshly read refs, because a branch can be closed between this key and the confirmation
    /// -- so these are the picker's copy of one rule, not a second rule.
    pub fn begin_export(&mut self) {
        self.banner = None;
        if let Some(minor) = self.prikk_minor()
            && minor < stikk_core::BUNDLE_FROM_MINOR
        {
            self.banner = Some(stikk_core::export_unavailable(minor));
            return;
        }
        let Some(Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        }) = self.overlays.last()
        else {
            return;
        };
        // **The guessed row of an empty picker and the unpublished current branch are one
        // case**, as they are for the ref writes: neither names a published tip, and a closure
        // of nothing is not a bundle.
        if *unpublished_main || (*unpublished_current && *cursor == 0) {
            self.banner = Some(stikk_core::UNPUBLISHED_IS_NOT_EXPORTED.to_string());
            return;
        }
        let Some(reff) = refs.get(*cursor).cloned() else {
            return;
        };
        let Some(entry) = self.ref_rows.iter().find(|entry| entry.name == reff) else {
            // In the picker but not in the rows is the unpublished case again.
            self.banner = Some(stikk_core::UNPUBLISHED_IS_NOT_EXPORTED.to_string());
            return;
        };
        if entry.is_tag() {
            self.banner = Some(stikk_core::TAGS_ARE_NOT_EXPORTED.to_string());
            return;
        }
        if entry.received {
            self.banner = Some(stikk_core::RECEIVED_IS_NOT_EXPORTED.to_string());
            return;
        }
        if entry.closed {
            self.banner = Some(stikk_core::CLOSED_IS_NOT_EXPORTED.to_string());
            return;
        }
        // **The suggestion uses the ref pointer the picker already holds**, so no read is spent
        // on a prompt the reader may cancel. It is a display id either way: the card reads the
        // tip again from `history`, which is the value prikk would name.
        let suggested = stikk_core::suggested_destination(&self.repo, &reff, &entry.id);
        self.overlays.pop(); // the picker closes on its way, as `n`, `t` and `M` do
        self.overlays.push(Overlay::ExportPath {
            reff,
            typed: suggested.display().to_string(),
        });
    }

    /// The export prompt's `Enter` (RFC 050 decision 5). **An empty path is not sent**, and the
    /// path is resolved here, as the open prompt's is.
    fn submit_export_path(&mut self, reff: String, typed: &str) {
        let typed = typed.trim();
        if typed.is_empty() {
            return;
        }
        self.overlays.pop();
        let output = std::path::Path::new(typed);
        let output = std::path::absolute(output).unwrap_or_else(|_| output.to_path_buf());
        let target = stikk_core::ExportTarget { reff, output };
        let seq = self.dispatch(RequestKind::ExportPreview {
            target: target.clone(),
        });
        self.overlays.push(Overlay::Loading {
            what: "export preview",
            seq,
        });
        self.pending_export = Some(PendingExport::AwaitingPreview { target });
    }

    /// The confirmation step's `Enter` for a pending export: Tier Two, and no key.
    fn submit_export_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingExport::Confirming { token, target }) = self.pending_export.take() else {
            return; // no export is actually pending
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::ExportConfirmExecute {
            token,
            readiness,
            evidence,
            target,
        });
        self.overlays.push(Overlay::Loading {
            what: "export",
            seq,
        });
        self.pending_export = Some(PendingExport::Executing);
    }

    /// `F` in Block detail (RFC 053 decision 3): save the file the cursor is on.
    ///
    /// **Nothing is spawned by any of the answers below** — the band, the missing list and the
    /// cursor-on-a-patch case are all decided from what Block detail already read.
    pub fn begin_save_file(&mut self) {
        let Some(Screen::BlockDetail { view, cursor, .. }) = self.screens.last() else {
            return;
        };
        if let Some(reason) = self.files_unavailable_reason() {
            self.banner = Some(reason);
            return;
        }
        if let Some(reason) = view.state.unreported_reason() {
            self.banner = Some(stikk_core::no_files_to_save(&reason));
            return;
        }
        let Some(path) = stikk_core::block_cursor_file(view, *cursor) else {
            // **The cursor is on a patch** (or past the end): `F` saves a file, and saying which
            // is better than saving whatever happens to be first.
            self.banner = Some(stikk_core::F_SAVES_A_FILE.to_string());
            return;
        };
        let (path, block) = (path.to_string(), view.row.block_id.clone());
        let suggested = stikk_core::suggested_file_destination(&self.repo, &path, &block);
        self.overlays.push(Overlay::SavePath {
            what: SavePathKind::File {
                block,
                path: path.clone(),
            },
            heading: path,
            typed: suggested.display().to_string(),
        });
    }

    /// `A` in Block detail (RFC 053 decision 3): save every file at this block.
    pub fn begin_save_all(&mut self) {
        let Some(Screen::BlockDetail { view, .. }) = self.screens.last() else {
            return;
        };
        if let Some(reason) = self.files_unavailable_reason() {
            self.banner = Some(reason);
            return;
        }
        if let Some(reason) = view.state.unreported_reason() {
            self.banner = Some(stikk_core::no_files_to_save(&reason));
            return;
        }
        let block = view.row.block_id.clone();
        let suggested = stikk_core::suggested_all_destination(&self.repo, &block);
        self.overlays.push(Overlay::SavePath {
            heading: stikk_core::block_short(&block),
            what: SavePathKind::All { block },
            typed: suggested.display().to_string(),
        });
    }

    /// stikk's own sentence below the band, or `None` in it — read from the Orientation this
    /// session already holds, so neither key handshakes (`C-T4d`).
    fn files_unavailable_reason(&self) -> Option<String> {
        let minor = self.prikk_minor()?;
        (minor < stikk_core::FILES_FROM_MINOR).then(|| stikk_core::files_unavailable(minor))
    }

    /// The save prompt's `Enter`. **An empty path is not sent**, and the path is resolved here,
    /// as the export prompt's is — the seam takes absolute paths only.
    fn submit_save_path(&mut self, what: SavePathKind, typed: &str) {
        let typed = typed.trim();
        if typed.is_empty() {
            return;
        }
        self.overlays.pop();
        let resolved = std::path::Path::new(typed);
        let resolved = std::path::absolute(resolved).unwrap_or_else(|_| resolved.to_path_buf());
        let (target, kind) = match what {
            SavePathKind::File { block, path } => {
                let target = stikk_core::SaveFileTarget {
                    block,
                    path,
                    output: resolved,
                };
                (
                    SaveTarget::File(target.clone()),
                    RequestKind::SaveFilePreview { target },
                )
            }
            SavePathKind::All { block } => {
                let target = stikk_core::SaveAllTarget {
                    block,
                    dir: resolved,
                };
                (
                    SaveTarget::All(target.clone()),
                    RequestKind::SaveAllPreview { target },
                )
            }
        };
        let seq = self.dispatch(kind);
        self.overlays.push(Overlay::Loading {
            what: "save preview",
            seq,
        });
        self.pending_save = Some(PendingSave::AwaitingPreview(target));
    }

    /// The confirmation step's `Enter` for a pending save: Tier Two, and no key.
    fn submit_save_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingSave::Confirming { token, target }) = self.pending_save.take() else {
            return; // no save is actually pending
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let kind = match target.clone() {
            SaveTarget::File(target) => RequestKind::SaveFileConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            },
            SaveTarget::All(target) => RequestKind::SaveAllConfirmExecute {
                token,
                readiness,
                evidence,
                target,
            },
        };
        let seq = self.dispatch(kind);
        self.overlays.push(Overlay::Loading { what: "save", seq });
        self.pending_save = Some(PendingSave::Executing(target));
    }

    /// Answers either save preview: the card, or stikk's own reason.
    fn apply_save_preview(
        &mut self,
        seq: u64,
        blocked: Option<String>,
        token: Option<Box<stikk_core::PreviewToken>>,
        error: Option<stikk_model::StikkError>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        let pending = self.pending_save.take();
        if let Some(error) = error {
            self.overlays.remove(index);
            self.surface(&error, OperationContext::Bundle);
            return;
        }
        if let Some(reason) = blocked {
            self.overlays.remove(index);
            self.banner = Some(reason);
            return;
        }
        let (Some(token), Some(PendingSave::AwaitingPreview(target))) = (token, pending) else {
            self.overlays.remove(index);
            return; // defensive
        };
        if let Some(slot) = self.overlays.get_mut(index) {
            *slot = Overlay::Confirmation {
                summary: Box::new(token.summary().clone()),
                tier: token.tier(),
                typed: String::new(),
                error: None,
            };
        }
        self.pending_save = Some(PendingSave::Confirming { token, target });
    }

    /// Answers [`RequestKind::SaveFileConfirmExecute`]. **Nothing is re-read**: nothing in the
    /// repository changed (RFC 053 decision 6).
    fn apply_save_file_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::FileSaved>>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        let pending = self.pending_save.take();
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::FileSaved::Done => {
                    let Some(PendingSave::Executing(SaveTarget::File(target))) = pending else {
                        self.overlays.remove(index);
                        return; // defensive
                    };
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::SaveResult {
                            lines: vec![stikk_core::saved_file_line(&target)],
                        };
                    }
                }
                stikk_prikk::FileSaved::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::files_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// Answers [`RequestKind::SaveAllConfirmExecute`].
    fn apply_save_all_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::Extracted>>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        let pending = self.pending_save.take();
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::Extracted::Read(report) => {
                    let Some(PendingSave::Executing(SaveTarget::All(target))) = pending else {
                        self.overlays.remove(index);
                        return; // defensive
                    };
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::SaveResult {
                            lines: stikk_core::saved_all_lines(&target, &report),
                        };
                    }
                }
                stikk_prikk::Extracted::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::files_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// Answers [`RequestKind::ExportPreview`]. A block is a banner: the reader is back where
    /// they were, with stikk's reason and the path still theirs to change.
    fn apply_export_preview(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::ExportPreview>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        let pending = self.pending_export.take();
        match result {
            Ok(stikk_core::ExportPreview::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
            }
            Ok(stikk_core::ExportPreview::Ready { token }) => {
                let Some(PendingExport::AwaitingPreview { target }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_export = Some(PendingExport::Confirming { token, target });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// Answers [`RequestKind::ExportConfirmExecute`]. prikk's lines verbatim, and **nothing in
    /// the repository is re-read**: an export changes no ref, no block and no file of the
    /// repository's -- it writes one file outside it.
    fn apply_export_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::BundleExport>>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        self.pending_export = None;
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::BundleExport::Done(printed) => {
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::ExportResult { printed };
                    }
                }
                stikk_prikk::BundleExport::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::export_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// The confirmation step's `Enter` for a pending merge: Tier Three, as seal's is.
    fn submit_merge_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingMerge::Confirming { token, target }) = self.pending_merge.take() else {
            return; // no merge is actually pending
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::MergeConfirmExecute {
            token,
            readiness,
            evidence,
            target: target.clone(),
        });
        self.overlays.push(Overlay::Loading { what: "merge", seq });
        self.pending_merge = Some(PendingMerge::Executing { target });
    }

    /// Answers [`RequestKind::MergePreview`]. A block is a banner; the plan view stays open
    /// underneath it, which is where the reader can see prikk's own reasoning.
    fn apply_merge_preview(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::MergePreview>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale: the reader navigated away before the preview resolved
        };
        let pending = self.pending_merge.take();
        match result {
            Ok(stikk_core::MergePreview::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
            }
            Ok(stikk_core::MergePreview::Ready { token }) => {
                let Some(PendingMerge::AwaitingPreview { target }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_merge = Some(PendingMerge::Confirming { token, target });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::MergePlan);
            }
        }
    }

    /// Answers [`RequestKind::MergeConfirmExecute`].
    ///
    /// On success: prikk's lines verbatim on the result card, then **the ref lists and any open
    /// History are re-read**. **Focus does not move** — unlike a switch, a merge changes no file
    /// and the reader is still working wherever they were; the card's own line says how to go to
    /// the merged branch if they want to.
    fn apply_merge_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::MergeResult>>,
    ) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq))
        else {
            return; // stale
        };
        let target = match self.pending_merge.take() {
            Some(PendingMerge::Executing { target } | PendingMerge::Confirming { target, .. }) => {
                Some(target)
            }
            Some(PendingMerge::AwaitingPreview { .. }) | None => None,
        };
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::MergeResult::Done(printed) => {
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::MergeResult { printed };
                    }
                    // **The branch merged into gained a block**, so a History of it is now
                    // short by one. Every other screen shows something this did not touch.
                    if let Some(target) = &target
                        && let Some(Screen::History { view, .. }) = self.screens.last()
                        && view.reff == target.into
                    {
                        let seq = self.dispatch(RequestKind::History {
                            reff: target.into.clone(),
                        });
                        if let Some(Screen::History { refreshing, .. }) = self.screens.last_mut() {
                            *refreshing = Some(seq);
                        }
                    }
                    self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
                    self.dispatch(RequestKind::Refs);
                }
                // The band is checked before the key offers anything, so this is only reachable
                // if prikk changed under the session.
                stikk_prikk::MergeResult::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::merge_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::MergePlan);
            }
        }
    }

    /// prikk's current branch, as the last orientation read reported it.
    fn current_branch(&self) -> stikk_model::CurrentBranch {
        match &self.state {
            OrientationState::Loaded(view) => view.current_branch.clone(),
            OrientationState::Loading | OrientationState::Failed(_) => {
                stikk_model::CurrentBranch::NotReported
            }
        }
    }

    /// The confirmation step's `Enter` for a pending switch (RFC 045 decision 3): Tier Three, so
    /// an explicit yes and no typed name.
    fn submit_switch_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingSwitch::Confirming { token, target }) = self.pending_switch.take() else {
            return; // no switch is actually pending
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::SwitchConfirmExecute {
            token,
            readiness,
            evidence,
            target: target.clone(),
        });
        self.overlays.push(Overlay::Loading {
            what: "switch",
            seq,
        });
        self.pending_switch = Some(PendingSwitch::Executing { target });
    }

    /// The name prompt's `Enter` (RFC 044 decision 5). **An empty name is not sent** -- the one
    /// rule stikk applies to a name -- and a tag goes on to its message prompt before anything is
    /// previewed, because `-m` is part of the call the preview restates.
    fn submit_ref_name(&mut self, kind: RefNameKind, source: String, typed: &str) {
        let typed = typed.trim();
        if typed.is_empty() {
            return;
        }
        let name = format!("{}{typed}", kind.prefix());
        self.overlays.pop();
        match kind {
            RefNameKind::Branch => self.start_ref_write(RefWriteOp::CreateBranch {
                name,
                from_ref: source,
            }),
            RefNameKind::Tag => self.overlays.push(Overlay::TagMessage {
                name,
                target: source,
                typed: String::new(),
            }),
        }
    }

    /// The tag message prompt's `Enter` (RFC 044 decision 5). **Empty is a valid answer**: it
    /// sends no `-m`, which is what prikk does for a tag without a message.
    fn submit_tag_message(&mut self, name: String, target: String, typed: &str) {
        self.overlays.pop();
        self.start_ref_write(RefWriteOp::CreateTag {
            name,
            target,
            message: typed.trim().to_string(),
        });
    }

    /// Dispatch the preview for `op` and park the flow on it.
    fn start_ref_write(&mut self, op: RefWriteOp) {
        let seq = self.dispatch(RequestKind::RefWritePreview { op: op.clone() });
        self.overlays.push(Overlay::Loading {
            what: "ref write preview",
            seq,
        });
        self.pending_ref_write = Some(PendingRefWrite::AwaitingPreview { op });
    }

    /// The confirmation step's `Enter` for a pending ref write (RFC 044; RFC 013 §6). Tier Three,
    /// as seal is, so the evidence is an explicit yes; the typed shape is accepted for parity with
    /// the other two flows and never produced here.
    fn submit_ref_write_confirmation(&mut self, tier: Tier, typed: &str) {
        let Some(PendingRefWrite::Confirming { token, op }) = self.pending_ref_write.take() else {
            return; // no ref write is actually pending -- a stray Enter on a Confirmation this app did not open
        };
        let evidence = match tier {
            Tier::ThreeTyped => stikk_core::Evidence::TypedName(typed.to_string()),
            _ => stikk_core::Evidence::ExplicitYes,
        };
        self.overlays.pop();
        let readiness = self.readiness();
        let seq = self.dispatch(RequestKind::RefWriteConfirmExecute {
            token,
            readiness,
            evidence,
            op,
        });
        self.overlays.push(Overlay::Loading {
            what: "ref write",
            seq,
        });
    }

    /// The context-sensitive select/drill-in action (the `Enter` key). Overlays take priority.
    pub fn select(&mut self) {
        match self.overlays.last() {
            Some(Overlay::RefPicker {
                refs,
                cursor,
                unpublished_main,
                ..
            }) => {
                // **The unpublished current branch needs no case of its own**: RFC 037 Handoff B
                // §3 says selecting it focuses that ref like any other, and it is in `refs`.
                let picked = if *unpublished_main {
                    // RFC 029 Handoff B §4: the one row an empty picker offers. It acts like any other
                    // pick: prikk reports an unpublished ref's history as empty (exit 0, at 0.28 and
                    // 0.42), and the History view says there are no sealed blocks yet.
                    Some(DEFAULT_REF.to_string())
                } else {
                    refs.get(*cursor).cloned()
                };
                if let Some(name) = picked {
                    self.overlays.pop();
                    // **The next ref change ends the mark's life** (RFC 041 decision 1). It
                    // survives closing and reopening the picker -- that is where both halves of a
                    // pair are chosen, so a mark that died on `Esc` could never be paired -- but
                    // choosing a different ref means the reader has moved on.
                    self.clear_ref_mark_on_ref_change(&name);
                    self.ref_focus = RefFocus::Ref(name);
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
                    if cmd.available(readiness, self.focused_ref().is_some()) {
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
                    if let Presentation::RefusalOverlay(card) =
                        present(&err, record.operation, self.prikk_minor())
                    {
                        self.overlays.push(Overlay::Refusal { card, cursor: 0 });
                    }
                }
            }
            Some(Overlay::CommitMessage { reff, typed, .. }) => {
                let (reff, typed) = (reff.clone(), typed.clone());
                self.submit_commit_message(reff, &typed);
            }
            Some(Overlay::RefName {
                kind,
                source,
                typed,
            }) => {
                let (kind, source, typed) = (*kind, source.clone(), typed.clone());
                self.submit_ref_name(kind, source, &typed);
            }
            Some(Overlay::BundlePath { typed }) => {
                let typed = typed.clone();
                self.submit_bundle_path(&typed);
            }
            Some(Overlay::ExportPath { reff, typed }) => {
                let (reff, typed) = (reff.clone(), typed.clone());
                self.submit_export_path(reff, &typed);
            }
            Some(Overlay::SavePath { what, typed, .. }) => {
                let (what, typed) = (what.clone(), typed.clone());
                self.submit_save_path(what, &typed);
            }
            Some(Overlay::TagMessage {
                name,
                target,
                typed,
            }) => {
                let (name, target, typed) = (name.clone(), target.clone(), typed.clone());
                self.submit_tag_message(name, target, &typed);
            }
            // Shared by commit (RFC 014 §3, this overlay's first consumer) and seal (RFC 016 §5) —
            // routed by whichever pending flow is actually waiting, never by the overlay's own content
            // (both use tier/typed identically; only what happens next differs). Neither should ever be
            // `Some` at once (each flow pops its own message/consent step before pushing this one), so
            // checking commit first is a tie-break that cannot actually occur, not a real ambiguity.
            Some(Overlay::Confirmation { tier, typed, .. }) => {
                let (tier, typed) = (*tier, typed.clone());
                if self.pending_commit.is_some() {
                    self.submit_commit_confirmation(tier, &typed);
                } else if self.pending_seal.is_some() {
                    self.submit_seal_confirmation(tier, &typed);
                } else if self.pending_ref_write.is_some() {
                    self.submit_ref_write_confirmation(tier, &typed);
                } else if self.pending_switch.is_some() {
                    self.submit_switch_confirmation(tier, &typed);
                } else if self.pending_merge.is_some() {
                    self.submit_merge_confirmation(tier, &typed);
                } else if self.pending_import.is_some() {
                    self.submit_import_confirmation(tier, &typed);
                } else if self.pending_export.is_some() {
                    self.submit_export_confirmation(tier, &typed);
                } else if self.pending_save.is_some() {
                    self.submit_save_confirmation(tier, &typed);
                }
            }
            Some(Overlay::SealConsent { .. }) => self.submit_seal_consent(),
            // A commit/seal result is dismissed like any other content overlay — no drill-in.
            // Deliberately unclamped here: only the renderer knows the viewport and the wrapped
            // content height, so it clamps and writes the clamped value back through the `Cell`
            // (see `Overlay::Glossary`). Holding ↓ past the bottom therefore parks at the bottom
            // instead of banking presses that ↑ has to spend one at a time undoing.
            Some(Overlay::Glossary { offset }) => offset.set(offset.get().saturating_add(1)),
            Some(
                Overlay::Operations { .. }
                | Overlay::Loading { .. }
                | Overlay::CommitWouldRefuse { .. }
                | Overlay::CommitWouldRefuseDeclarations { .. }
                | Overlay::CommitResult { .. }
                | Overlay::SealResult { .. }
                | Overlay::RefWriteResult { .. }
                | Overlay::SwitchResult { .. }
                | Overlay::MergeResult { .. }
                | Overlay::ImportResult { .. }
                | Overlay::ExportResult { .. }
                | Overlay::SaveResult { .. },
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
                    // The ref this History shows, which the focused ref need not be (§ `reload`).
                    let reff = view.reff.clone();
                    let is_tip = cursor == 0;
                    let seq = self.dispatch(RequestKind::BlockState { reff, row, is_tip });
                    self.screens.push(Screen::Loading {
                        what: "block detail",
                        seq,
                    });
                }
            }
            // RFC 035 §4.1: one patch of this block, rendered **from the report Block detail already
            // read**. No second read, and the label decision 3a may add comes from that same report's
            // patch count — never from inspecting a later patch for a rename (§5 trap 3).
            Some(Screen::BlockDetail { view, cursor, .. }) => {
                let rows = view.patches.rows();
                if let Some(row) = rows.get(*cursor) {
                    // The minor the Orientation read already carries — Patch detail spawns
                    // nothing here, so it must not handshake for it (RFC 043 decision 2).
                    let detail = stikk_core::patch_detail_in_block(
                        &row.patch,
                        &view.row.block_id,
                        &row.message,
                        rows.len(),
                        self.prikk_minor(),
                    );
                    self.screens.push(Screen::PatchDetail {
                        view: detail,
                        raw: false,
                        source: PatchSource::InBlock,
                        refreshing: None,
                        offset: std::cell::Cell::new(0),
                    });
                }
            }
            // Nothing to do while a load is pending.
            Some(Screen::Loading { .. }) => {}
            // **`d` opens the per-file content diff** (RFC 038 decision 3) — `Enter` on a Changes
            // row still has nothing to drill into, since the diff is of the whole worktree and not
            // of the selected row.
            Some(Screen::Changes { .. }) => {}
            // RFC 035 §4.2: the selected queued patch, with one `show <patch-id>` read. A queued patch
            // has no block to ask about (F2), so the patch id is the id that resolves the most.
            Some(Screen::Queue { view, cursor, .. }) => {
                if let Some(patch) = view.patches.get(*cursor) {
                    let patch_id = patch.id.clone();
                    let message = patch.detail_message.clone();
                    let seq = self.dispatch(RequestKind::PatchDetail { patch_id, message });
                    self.screens.push(Screen::Loading {
                        what: "patch detail",
                        seq,
                    });
                }
            }
            // Patch detail is the leaf: `v` toggles prikk's own fields, and there is nothing below it.
            Some(Screen::PatchDetail { .. }) => {}
            // Compare is a leaf too: `v` toggles prikk's own fields for the selected entry, and
            // `↑`/`↓` move between entries. There is nothing below an entry to open.
            Some(Screen::Compare { .. }) => {}
            // Verify is a leaf: `v` toggles prikk's own JSON and `r` re-runs it. There is nothing
            // below a verdict to open — and **nothing here acts on it** (RFC 040 decision 4):
            // Verify repairs nothing, clears no lock, and changes nothing at all.
            Some(Screen::Verify { .. }) => {}
            // **The Trust view is a leaf, and read-only** (RFC 054 decision 2): there is
            // nothing on it to open and nothing to confirm.
            Some(Screen::Trust { .. }) => {}
            // **`Enter` is the merge** (RFC 047 decision 6) — offered only from a plan prikk
            // judged `Confluent`, because that is the only outcome prikk will merge from.
            Some(Screen::MergePlan { .. }) => self.begin_merge(),
            // **The Bundle view is a leaf, and nothing on it acts** (RFC 050 Handoff A §4c):
            // `I` arrives with Handoff B, and until it does `Enter` here must do nothing rather
            // than find something to do.
            Some(Screen::Bundle { .. }) => {}
            None => self.open_history(),
        }
    }

    /// Mark or unmark the selected block, for a comparison (the `m` key; RFC 038 decision 3).
    ///
    /// Only History has a lineage to mark in. Pressing `m` on the already-marked block clears the
    /// mark, which is the only way to take one back without making a comparison.
    pub fn mark_block(&mut self) {
        // **The picker's `m` marks a ref, not a block** (RFC 041 decision 1). An overlay is what
        // the reader is looking at, so it answers the key first -- and the two marks are stored
        // apart, so neither key can ever read the other's.
        if matches!(self.overlays.last(), Some(Overlay::RefPicker { .. })) {
            self.mark_ref();
            return;
        }
        let Some(Screen::History { view, cursor, .. }) = self.screens.last() else {
            return;
        };
        let Some(row) = view.blocks.get(*cursor) else {
            return;
        };
        let mark = MarkedBlock {
            reff: view.reff.clone(),
            block_id: row.block_id.clone(),
            update_seq: row.update_seq,
        };
        if self.marked_block.as_ref() == Some(&mark) {
            self.marked_block = None;
            self.banner = Some("mark cleared".to_string());
        } else {
            self.marked_block = Some(mark);
            self.banner = Some("marked \u{2014} d compares it with the selected block".to_string());
        }
    }

    /// The block currently marked for a comparison, if any — what History draws its mark from.
    #[must_use]
    pub fn marked_block(&self) -> Option<&MarkedBlock> {
        self.marked_block.as_ref()
    }

    /// **stikk's own words for the row that cannot be a side** (RFC 041 decision 4). The branch
    /// you are on before its first seal has no block, so prikk has nothing to compare -- measured
    /// at 0.51, where naming it is `exit 1`, *"ref heads/main does not exist in this
    /// repository"*, the same words a ref that was never created gets. Passing that through would
    /// tell a reader their branch does not exist, which is false and alarming; stikk says what is
    /// actually true and points at the comparison that branch *does* have.
    const UNPUBLISHED_CANNOT_COMPARE: &'static str = "this branch has no published history to compare yet \u{2014} w, then d, compares your \
         worktree with what commit would author against";

    /// Mark or unmark the selected **ref**, for a comparison (the `m` key in the picker; RFC 041
    /// decision 1).
    fn mark_ref(&mut self) {
        let Some(Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        }) = self.overlays.last()
        else {
            return;
        };
        // The guessed row of an empty picker names nothing prikk published either (RFC 029
        // Handoff B §4), so it cannot be a side any more than the unpublished current branch can.
        if *unpublished_main {
            self.banner = Some(Self::UNPUBLISHED_CANNOT_COMPARE.to_string());
            return;
        }
        let cursor = *cursor;
        let first_is_unpublished = *unpublished_current;
        let Some(name) = refs.get(cursor).cloned() else {
            return;
        };
        if first_is_unpublished && cursor == 0 {
            self.banner = Some(Self::UNPUBLISHED_CANNOT_COMPARE.to_string());
            return;
        }
        // **The banner never names the ref.** It is repository text, and a banner is one line of
        // stikk's own voice; the mark itself is visible on the row, under the identifier rule.
        if self.marked_ref.as_deref() == Some(name.as_str()) {
            self.marked_ref = None;
            self.banner = Some("mark cleared".to_string());
        } else {
            self.marked_ref = Some(name);
            self.banner = Some("marked \u{2014} d compares it with the selected ref".to_string());
        }
    }

    /// The ref currently marked for a comparison, if any -- what the picker draws its mark from.
    #[must_use]
    pub fn marked_ref(&self) -> Option<&str> {
        self.marked_ref.as_deref()
    }

    /// Forget the ref mark, because the reader moved to another ref (RFC 041 decision 1's
    /// *"until the next ref change"*). **A stale pair is worse than an empty one.**
    fn clear_ref_mark_on_ref_change(&mut self, new_ref: &str) {
        if !matches!(&self.ref_focus, RefFocus::Ref(current) if current == new_ref) {
            self.marked_ref = None;
        }
    }

    /// Compare (the `d` key; RFC 038 decision 3). What is compared depends on the screen.
    pub fn compare(&mut self) {
        // **The picker's `d` compares two refs** (RFC 041 decision 1), and it closes the picker
        // on its way, so the Compare screen is not drawn under an overlay.
        let pair = if matches!(self.overlays.last(), Some(Overlay::RefPicker { .. })) {
            match self.ref_pair_comparison() {
                Some(pair) => {
                    self.overlays.pop();
                    Some(pair)
                }
                // The reason is already in the banner, and the picker stays open so the reader
                // can act on it.
                None => return,
            }
        } else {
            self.comparison()
        };
        let Some((comparison, opened)) = pair else {
            return;
        };
        let seq = self.dispatch(RequestKind::Compare { comparison, opened });
        self.screens.push(Screen::Loading {
            what: "compare",
            seq,
        });
    }

    /// The pair the picker's `d` means: **the marked ref against the selected one** (RFC 041
    /// decision 2), or `None` with the reason said in the banner.
    fn ref_pair_comparison(&mut self) -> Option<(stikk_prikk::Comparison, CompareSource)> {
        let Some(Overlay::RefPicker {
            refs,
            cursor,
            unpublished_main,
            unpublished_current,
        }) = self.overlays.last()
        else {
            return None;
        };
        if *unpublished_main {
            self.banner = Some(Self::UNPUBLISHED_CANNOT_COMPARE.to_string());
            return None;
        }
        let cursor = *cursor;
        let first_is_unpublished = *unpublished_current;
        let to = refs.get(cursor).cloned()?;
        // **Neither side may be the unpublished current branch** (decision 4). `m` already
        // refuses it, so it can only arrive here as the *selected* side.
        if first_is_unpublished && cursor == 0 {
            self.banner = Some(Self::UNPUBLISHED_CANNOT_COMPARE.to_string());
            return None;
        }
        let Some(from) = self.marked_ref.clone() else {
            self.banner = Some("mark a ref with m first".to_string());
            return None;
        };
        // **No order is invented** (decision 2): marked is `from`, selected is `to`, even when
        // they are the same ref -- which prikk answers as "no differences" like any other pair,
        // and is not special-cased here.
        Some((
            stikk_prikk::Comparison::Points {
                from: from.clone(),
                to: to.clone(),
            },
            CompareSource::RefPair { from, to },
        ))
    }

    /// Which comparison `d` means here, or `None` with the reason said — the three entry points of
    /// RFC 038 decision 3, and nothing else.
    fn comparison(&mut self) -> Option<(stikk_prikk::Comparison, CompareSource)> {
        match self.screens.last() {
            // **The worktree against what `commit` would author against**: the bare `diff`.
            Some(Screen::Changes { .. }) => Some((
                stikk_prikk::Comparison::WorktreeAgainstBaseline,
                CompareSource::WorktreeAgainstBaseline,
            )),
            Some(Screen::History { view, cursor, .. }) => {
                // Cloned out of the screen before `block_comparison` takes `&mut self` to set a
                // banner; the lineage is read-only to it either way.
                let selected = view.blocks.get(*cursor)?.clone();
                let reff = view.reff.clone();
                let lineage: Vec<stikk_prikk::BlockRow> = view.blocks.clone();
                self.block_comparison(&reff, &selected, Some(&lineage))
            }
            Some(Screen::BlockDetail { reff, view, .. }) => {
                let reff = reff.clone();
                let row = view.row.clone();
                self.block_comparison(&reff, &row, None)
            }
            _ => None,
        }
    }

    /// A block's comparison: against the marked block when there is one in this lineage, else
    /// against the previous block in this ref's history.
    fn block_comparison(
        &mut self,
        reff: &str,
        selected: &stikk_prikk::BlockRow,
        lineage: Option<&[stikk_prikk::BlockRow]>,
    ) -> Option<(stikk_prikk::Comparison, CompareSource)> {
        // **A mark made in another lineage is not a pair.** The ref picker can move between the
        // two presses, and comparing across refs is not what `m` offered.
        if let Some(mark) = self.marked_block.clone().filter(|m| m.reff == reff) {
            // **Older → newer, by prikk's own `update_seq`** (§7 trap 5), whichever was marked
            // first. Marking a row and selecting the same row is a comparison of a block with
            // itself, which prikk answers as "no differences" like any other — not special-cased.
            let (from, to) = if mark.update_seq <= selected.update_seq {
                (mark.block_id, selected.block_id.clone())
            } else {
                (selected.block_id.clone(), mark.block_id)
            };
            return Some((
                stikk_prikk::Comparison::Points { from, to },
                CompareSource::MarkedPair,
            ));
        }
        // **The previous block in this ref's history**, which is the next older row in the loaded
        // lineage — named as exactly that, and never as "the parent": a merge block has two
        // parents and neither need be the row above it (measured at 0.51).
        let previous = lineage.and_then(|blocks| {
            let at = blocks
                .iter()
                .position(|b| b.block_id == selected.block_id)?;
            blocks.get(at + 1)
        });
        match previous {
            Some(previous) => Some((
                stikk_prikk::Comparison::Points {
                    from: previous.block_id.clone(),
                    to: selected.block_id.clone(),
                },
                CompareSource::PreviousInHistory,
            )),
            None => {
                // **stikk's own reason, and no fetch** (Handoff A §3). More history might hold a
                // previous block, but reading it behind a keypress the user meant as "compare"
                // would be stikk doing something else than it was asked.
                self.banner = Some(
                    "the previous block is not in the loaded history \u{2014} nothing to compare \
                     this block with"
                        .to_string(),
                );
                None
            }
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
        if matches!(self.overlays.last(), Some(Overlay::Glossary { .. })) {
            self.overlays.pop();
        } else {
            self.overlays.push(Overlay::Glossary {
                offset: std::cell::Cell::new(0),
            });
        }
    }

    /// Open the command palette (the `:` key).
    pub fn open_palette(&mut self) {
        let readiness = self.readiness();
        let ref_focused = self.focused_ref().is_some();
        self.overlays.push(Overlay::Palette {
            filter: String::new(),
            cursor: 0,
            readiness,
            ref_focused,
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
            Some(
                Overlay::CommitMessage { typed, .. }
                | Overlay::RefName { typed, .. }
                | Overlay::BundlePath { typed }
                | Overlay::ExportPath { typed, .. }
                | Overlay::SavePath { typed, .. }
                | Overlay::TagMessage { typed, .. },
            ) => typed.push(ch),
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
            Some(
                Overlay::CommitMessage { typed, .. }
                | Overlay::RefName { typed, .. }
                | Overlay::BundlePath { typed }
                | Overlay::ExportPath { typed, .. }
                | Overlay::SavePath { typed, .. }
                | Overlay::TagMessage { typed, .. },
            ) => {
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
            Some(
                Overlay::Palette { .. }
                    | Overlay::CommitMessage { .. }
                    | Overlay::RefName { .. }
                    | Overlay::BundlePath { .. }
                    | Overlay::ExportPath { .. }
                    | Overlay::SavePath { .. }
                    | Overlay::SaveResult { .. }
                    | Overlay::TagMessage { .. }
            )
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
            // The Glossary has no selection — it scrolls (RFC 023 F2). Same keys as every other
            // overlay's cursor rather than a second scrolling idiom, which is why the Keys section of
            // this very panel can describe them in one line (`NFR-A03`).
            Some(Overlay::Glossary { offset }) => offset.set(offset.get().saturating_sub(1)),
            // A single prompt or content pane, not a list — no cursor to move (RFC 013 §6/RFC 014 §3).
            Some(
                Overlay::Operations { .. }
                | Overlay::Loading { .. }
                | Overlay::CommitWouldRefuse { .. }
                | Overlay::CommitWouldRefuseDeclarations { .. }
                | Overlay::Confirmation { .. }
                | Overlay::CommitMessage { .. }
                | Overlay::CommitResult { .. }
                | Overlay::SealConsent { .. }
                | Overlay::SealResult { .. }
                | Overlay::RefName { .. }
                | Overlay::BundlePath { .. }
                | Overlay::ExportPath { .. }
                | Overlay::SavePath { .. }
                | Overlay::SaveResult { .. }
                | Overlay::TagMessage { .. }
                | Overlay::RefWriteResult { .. }
                | Overlay::SwitchResult { .. }
                | Overlay::MergeResult { .. }
                | Overlay::ImportResult { .. }
                | Overlay::ExportResult { .. },
            ) => {}
            None => match self.screens.last_mut() {
                Some(Screen::History { cursor, .. }) => *cursor = cursor.saturating_sub(1),
                // RFC 035 §4.2: the Queue now has a selection, and the renderer scrolls to follow it.
                Some(Screen::Queue { cursor, .. })
                | Some(Screen::BlockDetail { cursor, .. })
                | Some(Screen::Compare { cursor, .. }) => {
                    *cursor = cursor.saturating_sub(1);
                }
                // Patch detail has no selection — it scrolls, the Glossary's idiom. The Merge
                // plan is prikk's prose, so it scrolls the same way and for the same reason, and
                // **so is Verify's report**: its renderer has always clamped an offset and its
                // title has always offered `↑/↓` once the report outgrew the panel, but no arm
                // here moved it, so the keys did nothing and a long report's tail could not be
                // reached at all.
                Some(
                    Screen::PatchDetail { offset, .. }
                    | Screen::MergePlan { offset, .. }
                    | Screen::Bundle { offset, .. }
                    | Screen::Verify { offset, .. }
                    | Screen::Trust { offset, .. },
                ) => {
                    offset.set(offset.get().saturating_sub(1));
                }
                _ => {}
            },
        }
    }

    /// Move the selection down, clamped to the active list length.
    pub fn nav_down(&mut self) {
        match self.overlays.last_mut() {
            Some(Overlay::RefPicker {
                refs,
                cursor,
                unpublished_main,
                ..
            }) => {
                let len = if *unpublished_main { 1 } else { refs.len() };
                *cursor = next_index(*cursor, len);
            }
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
                Overlay::Glossary { .. }
                | Overlay::Operations { .. }
                | Overlay::Loading { .. }
                | Overlay::CommitWouldRefuse { .. }
                | Overlay::CommitWouldRefuseDeclarations { .. }
                | Overlay::Confirmation { .. }
                | Overlay::CommitMessage { .. }
                | Overlay::CommitResult { .. }
                | Overlay::SealConsent { .. }
                | Overlay::SealResult { .. }
                | Overlay::RefName { .. }
                | Overlay::BundlePath { .. }
                | Overlay::ExportPath { .. }
                | Overlay::SavePath { .. }
                | Overlay::SaveResult { .. }
                | Overlay::TagMessage { .. }
                | Overlay::RefWriteResult { .. }
                | Overlay::SwitchResult { .. }
                | Overlay::MergeResult { .. }
                | Overlay::ImportResult { .. }
                | Overlay::ExportResult { .. },
            ) => {}
            None => match self.screens.last_mut() {
                Some(Screen::History { view, cursor, .. }) => {
                    *cursor = next_index(*cursor, view.blocks.len());
                }
                Some(Screen::Queue { view, cursor, .. }) => {
                    *cursor = next_index(*cursor, view.patches.len());
                }
                // **Patches, then files** (RFC 053 decision 3): one cursor over both sections,
                // so `↓` reaches a file without a second selection to keep in step.
                Some(Screen::BlockDetail { view, cursor, .. }) => {
                    *cursor = next_index(*cursor, stikk_core::block_cursor_len(view));
                }
                // Deliberately unclamped here: only the renderer knows the wrapped height, so it clamps
                // and writes the clamped value back, as the Glossary does.
                Some(
                    Screen::PatchDetail { offset, .. }
                    | Screen::MergePlan { offset, .. }
                    | Screen::Bundle { offset, .. }
                    | Screen::Verify { offset, .. }
                    | Screen::Trust { offset, .. },
                ) => {
                    offset.set(offset.get().saturating_add(1));
                }
                // **The cursor moves between entries, and the renderer follows it** — the Queue's
                // idiom. `↑`/`↓` on a view whose sections are arbitrarily tall must select, not
                // scroll by one row, or reaching the third entry of a long diff is a hundred
                // presses.
                Some(Screen::Compare { view, cursor, .. }) => {
                    let entries = view.listed().map_or(0, |c| c.entries.len());
                    *cursor = next_index(*cursor, entries);
                }
                _ => {}
            },
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
            // to. Popping either overlay is exactly "the user is not doing this anymore." `Confirmation`
            // is shared with seal (RFC 016 §5), so clear both pending flows unconditionally — only the
            // one actually in flight is ever `Some`.
            if matches!(
                overlay,
                Overlay::CommitMessage { .. } | Overlay::Confirmation { .. }
            ) {
                self.pending_commit = None;
                self.pending_seal = None;
            }
            // The same posture for RFC 044's prompts and its share of `Confirmation`: cancelling
            // any step of the flow abandons the write it belonged to. The prompts come *before*
            // anything is dispatched, so there is usually nothing pending to clear — clearing it
            // anyway is what keeps a token from a flow the user walked away from out of the next
            // one's path.
            if matches!(
                overlay,
                Overlay::RefName { .. } | Overlay::TagMessage { .. } | Overlay::Confirmation { .. }
            ) {
                self.pending_ref_write = None;
            }
            // RFC 045: the same posture for a switch, which shares `Confirmation` with the rest.
            // RFC 050: and for an import, which shares it too.
            if matches!(overlay, Overlay::Confirmation { .. }) {
                self.pending_switch = None;
                self.pending_import = None;
                self.pending_export = None;
            }
            // Cancelling the consent step abandons the seal it belonged to (RFC 016 §8) — the same
            // posture, one step later in seal's own flow.
            if matches!(overlay, Overlay::SealConsent { .. }) {
                self.pending_seal = None;
            }
            return;
        }
        if self.banner.take().is_some() {
            return;
        }
        if self.screens.pop().is_some() {
            // Popping back to another top-level view is a transition like any other.
            self.flush_session();
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
            ResponseKind::ChangeCheck(r) => r.is_ok(),
            ResponseKind::Orient(r) => r.view.is_ok(),
            ResponseKind::History(r) => r.is_ok(),
            ResponseKind::BlockState { result, .. } => result.is_ok(),
            ResponseKind::Refs(r) => r.is_ok(),
            ResponseKind::Changes(r) => r.is_ok(),
            ResponseKind::Queue(r) => r.is_ok(),
            ResponseKind::PatchDetail(r) => r.is_ok(),
            ResponseKind::Compare { result, .. } => result.is_ok(),
            // **Whether the *operation* succeeded, never whether the repository is sound.** A
            // damaged repository is a successful read of a bad verdict (RFC 040 F2), and the
            // background-operations overlay is a listing, not a content view (INV-8).
            ResponseKind::Verify(r) => r.is_ok(),
            ResponseKind::CommitPreview(r) => r.is_ok(),
            ResponseKind::CommitConfirmExecute(r) => r.is_ok(),
            ResponseKind::SealPreview(r) => r.is_ok(),
            ResponseKind::SealConfirmExecute(r) => r.is_ok(),
            ResponseKind::RefWritePreview(r) => r.is_ok(),
            ResponseKind::RefWriteConfirmExecute { result, .. } => result.is_ok(),
            ResponseKind::SwitchPreview(r) => r.is_ok(),
            ResponseKind::SwitchConfirmExecute(r) => r.is_ok(),
            ResponseKind::MergePlan(r) => r.is_ok(),
            ResponseKind::MergePreview(r) => r.is_ok(),
            ResponseKind::MergeConfirmExecute(r) => r.is_ok(),
            // **Whether the *read* succeeded, never whether the bundle is sound.** A file prikk
            // refuses is a successful read of a refusal, the same posture Verify takes.
            ResponseKind::OpenBundle(r) => r.is_ok(),
            ResponseKind::ImportPreview(r) => r.is_ok(),
            ResponseKind::ImportConfirmExecute(r) => r.is_ok(),
            ResponseKind::ExportPreview(r) => r.is_ok(),
            ResponseKind::ExportConfirmExecute(r) => r.is_ok(),
            ResponseKind::TrustRead(r) => r.is_ok(),
            ResponseKind::SaveFilePreview(r) => r.is_ok(),
            ResponseKind::SaveFileConfirmExecute(r) => r.is_ok(),
            ResponseKind::SaveAllPreview(r) => r.is_ok(),
            ResponseKind::SaveAllConfirmExecute(r) => r.is_ok(),
        };
        // A check was never recorded as an operation (RFC 031 §4), so this finds nothing for it.
        self.record_finished(seq, ok);
        match kind {
            ResponseKind::ChangeCheck(result) => self.apply_change_check(seq, result),
            ResponseKind::Orient(read) => self.apply_orient(seq, read),
            ResponseKind::History(result) => self.apply_history(seq, result),
            ResponseKind::BlockState { reff, result } => self.apply_block_state(seq, reff, result),
            ResponseKind::Refs(result) => self.apply_refs(seq, result),
            ResponseKind::Changes(result) => self.apply_changes(seq, result),
            ResponseKind::Queue(result) => self.apply_queue(seq, result),
            ResponseKind::PatchDetail(result) => self.apply_patch_detail(seq, result),
            ResponseKind::Verify(result) => self.apply_verify(seq, result),
            ResponseKind::Compare { opened, result } => self.apply_compare(seq, opened, result),
            ResponseKind::CommitPreview(result) => self.apply_commit_preview(seq, result),
            ResponseKind::CommitConfirmExecute(result) => {
                self.apply_commit_confirm_execute(seq, result);
            }
            ResponseKind::SealPreview(result) => self.apply_seal_preview(seq, result),
            ResponseKind::SealConfirmExecute(result) => {
                self.apply_seal_confirm_execute(seq, result);
            }
            ResponseKind::RefWritePreview(result) => self.apply_ref_write_preview(seq, result),
            ResponseKind::RefWriteConfirmExecute { kind, result } => {
                self.apply_ref_write_confirm_execute(seq, kind, result);
            }
            ResponseKind::TrustRead(result) => self.apply_trust_read(seq, result),
            ResponseKind::SaveFilePreview(result) => match result {
                Ok(stikk_core::SaveFilePreview::Blocked(reason)) => {
                    self.apply_save_preview(seq, Some(reason), None, None);
                }
                Ok(stikk_core::SaveFilePreview::Ready { token }) => {
                    self.apply_save_preview(seq, None, Some(token), None);
                }
                Err(error) => self.apply_save_preview(seq, None, None, Some(error)),
            },
            ResponseKind::SaveAllPreview(result) => match result {
                Ok(stikk_core::SaveAllPreview::Blocked(reason)) => {
                    self.apply_save_preview(seq, Some(reason), None, None);
                }
                Ok(stikk_core::SaveAllPreview::Ready { token }) => {
                    self.apply_save_preview(seq, None, Some(token), None);
                }
                Err(error) => self.apply_save_preview(seq, None, None, Some(error)),
            },
            ResponseKind::SaveFileConfirmExecute(result) => {
                self.apply_save_file_confirm_execute(seq, result);
            }
            ResponseKind::SaveAllConfirmExecute(result) => {
                self.apply_save_all_confirm_execute(seq, result);
            }
            ResponseKind::SwitchPreview(result) => self.apply_switch_preview(seq, result),
            ResponseKind::SwitchConfirmExecute(result) => {
                self.apply_switch_confirm_execute(seq, result);
            }
            ResponseKind::MergePlan(result) => self.apply_merge_plan(seq, result),
            ResponseKind::MergePreview(result) => self.apply_merge_preview(seq, result),
            ResponseKind::MergeConfirmExecute(result) => {
                self.apply_merge_confirm_execute(seq, result);
            }
            ResponseKind::OpenBundle(result) => self.apply_open_bundle(seq, result),
            ResponseKind::ImportPreview(result) => self.apply_import_preview(seq, result),
            ResponseKind::ImportConfirmExecute(result) => {
                self.apply_import_confirm_execute(seq, result);
            }
            ResponseKind::ExportPreview(result) => self.apply_export_preview(seq, result),
            ResponseKind::ExportConfirmExecute(result) => {
                self.apply_export_confirm_execute(seq, result);
            }
        }
        // **One flush point for every transition a response completes** (RFC 046 decision 4):
        // the view that just landed, the focus a switch just moved, the picker's pick once its
        // History arrives. Not a timer — a response only exists because something happened — and
        // `flush_session` writes nothing when nothing in the record changed, so the silent
        // change check costs no write.
        self.flush_session();
    }

    fn apply_orient(&mut self, seq: u64, read: OrientRead) {
        if self.orientation_pending != Some(seq) {
            return; // stale: a newer orientation request has since superseded this one
        }
        self.orientation_pending = None;
        // RFC 031 §3: stamped whenever the token read succeeded, whatever Orientation's own result; a
        // failed token read keeps the previous stamp. Stamping says nothing, the first time or any other.
        if let Ok(token) = read.token {
            self.stamp = Some(token);
            self.interval_from = None;
        }
        self.state = match read.view {
            Ok(view) => {
                // Once, and only while nothing is focused yet: a later read — `r`, the read after a
                // commit or seal — never moves focus, and a pick made while pending has already won.
                //
                // **With a record in hand, focus waits for the ref list too** (RFC 046 decision 5):
                // the remembered ref is used only if prikk still lists it, and until that read
                // lands there is nothing stikk may honestly show. Until then the screen is what a
                // loading screen was before sessions existed.
                if self.ref_focus == RefFocus::Pending
                    && (self.stored.is_none() || self.restore_refs.is_some())
                {
                    self.resolve_session(&view);
                }
                OrientationState::Loaded(view)
            }
            // A failed read leaves focus pending, so the first *successful* read still resolves it.
            Err(error) => OrientationState::Failed(error.to_string()),
        };
    }

    /// Which of RFC 046 F2's four top-level views is open, for the record.
    ///
    /// **Read from the top down, skipping what a record may not hold.** Block detail, Patch
    /// detail and Compare are pushed on the view they were opened from, so skipping them finds
    /// that view — which is what decision 2 asks for, without this needing to know *how* each
    /// was opened. Verify is skipped for `NFR-P05`: reopening it would run a full verification
    /// at launch.
    fn current_view(&self) -> stikk_state::LastView {
        for screen in self.screens.iter().rev() {
            match screen {
                Screen::History { .. } => return stikk_state::LastView::History,
                Screen::Changes { .. } => return stikk_state::LastView::Changes,
                Screen::Queue { .. } => return stikk_state::LastView::Queue,
                Screen::BlockDetail { .. }
                | Screen::PatchDetail { .. }
                | Screen::Compare { .. }
                | Screen::MergePlan { .. }
                | Screen::Bundle { .. }
                | Screen::Verify { .. }
                | Screen::Trust { .. }
                | Screen::Loading { .. } => {}
            }
        }
        stikk_state::LastView::Orientation
    }

    /// Write the record, if there is a store, if anything changed, and if the store has not
    /// already refused once (RFC 046 decisions 3 and 4).
    ///
    /// **Called on transitions, never on a timer.** A refused or failed write says so once, in a
    /// banner, and is not tried again this session: a reader who cannot be remembered should
    /// hear it once, not at every key press.
    fn flush_session(&mut self) {
        if self.session_write_failed {
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        let record = stikk_state::SessionRecord {
            root: session.canonical_root.clone(),
            focus: self.focused_ref().map(ToString::to_string),
            view: self.current_view(),
            hide_untracked: self.hide_untracked_default,
        };
        if self.saved.as_ref() == Some(&record) {
            return;
        }
        match stikk_state::session::save(&session.state_dir, &session.canonical_root, &record) {
            Ok(()) => self.saved = Some(record),
            Err(error) => {
                self.session_write_failed = true;
                self.banner = Some(format!("this session will not be remembered: {error}"));
            }
        }
    }

    /// Resolve focus, the view and the filter — from the record where prikk confirms it, and
    /// from today's rule where it does not (RFC 046 decision 5).
    ///
    /// **Everything this opens is re-read from prikk**, which is what makes restoring a *view*
    /// safe: the record says which of four, never what is in it.
    fn resolve_session(&mut self, view: &stikk_core::OrientationView) {
        let Some(stored) = self.stored.take() else {
            self.resolve_focus(view);
            return;
        };
        let refs = self.restore_refs.take().unwrap_or_default();
        let restored = stikk_core::restore(
            Some(&stored),
            &refs,
            &view.current_branch,
            view.main_ref_state.is_some(),
        );
        self.ref_focus = match restored.focus {
            Some(name) => RefFocus::Ref(name),
            None => RefFocus::Unfocused,
        };
        self.hide_untracked_default = restored.hide_untracked;
        match restored.view {
            stikk_state::LastView::Orientation => {}
            stikk_state::LastView::History if self.focused_ref().is_some() => self.open_history(),
            stikk_state::LastView::Changes if self.focused_ref().is_some() => self.open_changes(),
            stikk_state::LastView::Queue => self.open_queue(),
            // History and Changes both need a ref; with none focused there is nothing to open
            // them on, and Orientation is where a session with no focus belongs anyway.
            stikk_state::LastView::History | stikk_state::LastView::Changes => {}
        }
        if self.ref_focus == RefFocus::Unfocused {
            self.open_ref_picker();
        }
        // **Set last.** Every `open_*` above clears the banner on its way, as each does for an
        // ordinary key press; a note set before them would be gone before it was drawn.
        if let Some(note) = restored.note {
            self.banner = Some(note);
        }
        // The record is now what is on screen, so the first transition does not rewrite it.
        self.saved = Some(stored);
    }

    /// Where stikk opens (RFC 029 Q1, ruled (b); Handoff B §2). **stikk follows where prikk would start,
    /// and never moves prikk**: it reads the current branch from the `status` report Orientation already
    /// makes, and never `.prikk/current-branch` itself.
    ///
    /// A branch prikk names is followed even when it is unpublished — a fresh 0.42 repository names
    /// `heads/main` before anything is sealed there, and prikk would author a first commit on it. The
    /// fallback governs only when prikk names no branch.
    fn resolve_focus(&mut self, view: &stikk_core::OrientationView) {
        match (&view.current_branch, &view.main_ref_state) {
            (CurrentBranch::Branch(branch), _) => {
                self.ref_focus = RefFocus::Ref(branch.as_str().to_string());
            }
            (CurrentBranch::NotReported | CurrentBranch::Unresolved(_), Some(_)) => {
                self.ref_focus = RefFocus::Ref(DEFAULT_REF.to_string());
            }
            (CurrentBranch::NotReported | CurrentBranch::Unresolved(_), None) => {
                self.ref_focus = RefFocus::Unfocused;
                self.open_ref_picker();
            }
        }
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

    fn apply_block_state(
        &mut self,
        seq: u64,
        reff: String,
        result: stikk_model::Result<BlockDetailView>,
    ) {
        // RFC 031 §7: an in-place refresh of the tip's detail, which stays visible on error.
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::BlockDetail { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            if let Some(Screen::BlockDetail { refreshing, .. }) = self.screens.last_mut() {
                *refreshing = None;
            }
            match result {
                Ok(detail) => {
                    if let Some(Screen::BlockDetail { view, .. }) = self.screens.last_mut() {
                        *view = detail;
                    }
                }
                Err(error) => self.surface(&error, OperationContext::LoadBlockState),
            }
            return;
        }
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(detail) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::BlockDetail {
                            reff,
                            view: detail,
                            cursor: 0,
                            refreshing: None,
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    self.surface(&error, OperationContext::LoadBlockState);
                }
            }
        }
    }

    /// Answers [`RequestKind::Queue`] (RFC 028). The same two cases as [`Self::apply_history`]: an
    /// in-place refresh of the top Queue screen, which stays visible on error, or a pushed
    /// `Screen::Loading` placeholder.
    fn apply_queue(&mut self, seq: u64, result: stikk_model::Result<QueueView>) {
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::Queue { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            match result {
                Ok(new_view) => {
                    // The offset stays: the next render clamps it to the new content.
                    if let Some(Screen::Queue {
                        view, refreshing, ..
                    }) = self.screens.last_mut()
                    {
                        *view = new_view;
                        *refreshing = None;
                    }
                }
                Err(error) => {
                    if let Some(Screen::Queue { refreshing, .. }) = self.screens.last_mut() {
                        *refreshing = None;
                    }
                    self.surface(&error, OperationContext::LoadQueue);
                }
            }
            return;
        }
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(view) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::Queue {
                            view,
                            refreshing: None,
                            offset: std::cell::Cell::new(0),
                            cursor: 0,
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    self.surface(&error, OperationContext::LoadQueue);
                }
            }
        }
    }

    /// Answers [`RequestKind::PatchDetail`] (RFC 035 §4.2/§4.3): the Queue's drill-in, or its own
    /// `r` re-read.
    ///
    /// **A re-read replaces the view wholesale.** Nothing is merged forward from the earlier read —
    /// not a path, not prikk's `queued` word — because a patch sealed since then reports both
    /// differently, and carrying either over would show a state prikk no longer reports (RFC 035 A1 F7,
    /// row 2). The raw toggle is the user's own and stays as they left it.
    /// Answers [`RequestKind::Compare`]. A comparison against the worktree can be refreshed in
    /// place; one between two blocks is never re-read, so only the first branch below ever fires
    /// for it.
    fn apply_compare(
        &mut self,
        seq: u64,
        opened: CompareSource,
        result: stikk_model::Result<stikk_core::CompareView>,
    ) {
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::Compare { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            if let Some(Screen::Compare { refreshing, .. }) = self.screens.last_mut() {
                *refreshing = None;
            }
            match result {
                Ok(compared) => {
                    if let Some(Screen::Compare {
                        view,
                        cursor,
                        offset,
                        ..
                    }) = self.screens.last_mut()
                    {
                        *view = compared;
                        // The content changed shape; the next render clamps both to it.
                        *cursor = 0;
                        offset.set(0);
                    }
                }
                Err(error) => self.surface(&error, OperationContext::Compare),
            }
            return;
        }
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(view) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::Compare {
                            view,
                            opened,
                            cursor: 0,
                            raw: false,
                            refreshing: None,
                            offset: std::cell::Cell::new(0),
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    // **A refusal from `diff` goes through the usual path, verbatim** (RFC 038
                    // decision 6). Unlike Block detail's file list there is nothing left to show
                    // beside it: this screen *is* the comparison.
                    self.surface(&error, OperationContext::Compare);
                }
            }
        }
    }

    /// Answers [`RequestKind::MergePlan`].
    ///
    /// **A decline is a banner, not a screen.** stikk's reason for naming no baseline is one
    /// sentence, and the reader is still in the picker's world -- the same shape `d`'s own
    /// declines take, rather than a screen whose only content is why there is no content.
    fn apply_merge_plan(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::MergePlanRead>,
    ) {
        let Some(index) = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq))
        else {
            return;
        };
        match result {
            Ok(stikk_core::MergePlanRead::Read(view)) => {
                if let Some(slot) = self.screens.get_mut(index) {
                    *slot = Screen::MergePlan {
                        view,
                        offset: std::cell::Cell::new(0),
                    };
                }
            }
            Ok(stikk_core::MergePlanRead::Declined(reason)) => {
                self.screens.remove(index);
                self.banner = Some(reason);
            }
            Err(error) => {
                self.screens.remove(index);
                self.surface(&error, OperationContext::MergePlan);
            }
        }
    }

    /// Answers [`RequestKind::OpenBundle`].
    ///
    /// **A refusal and a band are screens here, not banners** — unlike the Merge plan's decline.
    /// The reader asked about one named file, and prikk's reason for not reading it is the whole
    /// answer to that question: a banner over the view they came from would put prikk's words
    /// where the path is not, and the path is what they would want to check.
    fn apply_open_bundle(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::BundleOpened>,
    ) {
        let Some(index) = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq))
        else {
            return;
        };
        match result {
            Ok(opened) => {
                if let Some(slot) = self.screens.get_mut(index) {
                    *slot = Screen::Bundle {
                        view: Box::new(opened),
                        offset: std::cell::Cell::new(0),
                    };
                }
            }
            Err(error) => {
                self.screens.remove(index);
                self.surface(&error, OperationContext::Bundle);
            }
        }
    }

    /// Answers [`RequestKind::Verify`]. Re-runnable in place with `r`, like History and the
    /// Queue: the report stays visible while the new run works.
    fn apply_verify(&mut self, seq: u64, result: stikk_model::Result<stikk_core::VerifyView>) {
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::Verify { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            if let Some(Screen::Verify { refreshing, .. }) = self.screens.last_mut() {
                *refreshing = None;
            }
            match result {
                Ok(fresh) => {
                    if let Some(Screen::Verify { view, offset, .. }) = self.screens.last_mut() {
                        // **Replaced, never merged.** A verdict is about the moment it was taken
                        // (RFC 040 decision 4), so the previous one is gone the instant this one
                        // lands -- there is no state in which the screen shows two runs at once.
                        *view = fresh;
                        offset.set(0);
                    }
                }
                Err(error) => self.surface(&error, OperationContext::Verify),
            }
            return;
        }
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(view) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::Verify {
                            view,
                            raw: false,
                            refreshing: None,
                            offset: std::cell::Cell::new(0),
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    // **A refusal goes through the usual path, verbatim.** This screen *is* the
                    // report, so there is nothing left to show beside the failure -- and a
                    // damaged repository never arrives here, because the seam already read its
                    // exit 1 as the report it carried (RFC 040 decision 1).
                    self.surface(&error, OperationContext::Verify);
                }
            }
        }
    }

    fn apply_patch_detail(&mut self, seq: u64, result: stikk_model::Result<PatchDetailView>) {
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::PatchDetail { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            if let Some(Screen::PatchDetail { refreshing, .. }) = self.screens.last_mut() {
                *refreshing = None;
            }
            match result {
                Ok(detail) => {
                    if let Some(Screen::PatchDetail { view, offset, .. }) = self.screens.last_mut()
                    {
                        *view = detail;
                        // The content changed shape; the next render clamps this to it.
                        offset.set(0);
                    }
                }
                Err(error) => self.surface(&error, OperationContext::LoadPatch),
            }
            return;
        }
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(view) => {
                    // Only a Queue drill-in reaches here, so the source is that patch id (§4.3).
                    let source = PatchSource::Queued {
                        patch_id: view.patch_id.clone(),
                        message: self.queued_message(&view.patch_id),
                    };
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::PatchDetail {
                            view,
                            raw: false,
                            source,
                            refreshing: None,
                            offset: std::cell::Cell::new(0),
                        };
                    }
                }
                Err(error) => {
                    self.screens.remove(index);
                    self.surface(&error, OperationContext::LoadPatch);
                }
            }
        }
    }

    /// The message the Queue screen below holds for `patch_id`, for a later re-read to render with.
    /// Falls back to *not reported* when the Queue has since been popped or no longer lists the patch —
    /// which is absence, not an empty message (`C-T2c′`).
    fn queued_message(&self, patch_id: &str) -> PatchMessageState {
        self.screens
            .iter()
            .rev()
            .find_map(|screen| match screen {
                Screen::Queue { view, .. } => view
                    .patches
                    .iter()
                    .find(|patch| patch.id == patch_id)
                    .map(|patch| patch.detail_message.clone()),
                _ => None,
            })
            .unwrap_or(PatchMessageState::NotReported)
    }

    fn apply_changes(&mut self, seq: u64, result: stikk_model::Result<ChangesRead>) {
        // RFC 031 §7: an in-place refresh, as the Queue's; `hide_untracked` stays as the user set it, and
        // the view stays visible on error.
        let is_top_refresh = matches!(
            self.screens.last(),
            Some(Screen::Changes { refreshing: Some(s), .. }) if *s == seq
        );
        if is_top_refresh {
            if let Some(Screen::Changes { refreshing, .. }) = self.screens.last_mut() {
                *refreshing = None;
            }
            match result {
                Ok(read) => {
                    if let Some(Screen::Changes { view, history, .. }) = self.screens.last_mut() {
                        *view = read.view;
                        *history = read.history;
                    }
                }
                Err(error) => self.surface(&error, OperationContext::LoadChanges),
            }
            return;
        }
        let index = self
            .screens
            .iter()
            .position(|s| matches!(s, Screen::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(read) => {
                    if let Some(slot) = self.screens.get_mut(index) {
                        *slot = Screen::Changes {
                            view: read.view,
                            hide_untracked: self.hide_untracked_default,
                            history: read.history,
                            refreshing: None,
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

    fn apply_refs(&mut self, seq: u64, result: stikk_model::Result<stikk_core::RefList>) {
        // **The restore read** (RFC 046 decision 5), which opened no overlay: it exists only so
        // the remembered ref can be checked against what prikk lists before anything is shown.
        // A failed read is an empty list, which makes every remembered ref "gone" — the safe
        // direction, and the fallback is today's rule.
        if self.restore_refs_pending == Some(seq) {
            self.restore_refs_pending = None;
            let refs = result.map(|list| list.refs).unwrap_or_default();
            self.ref_rows.clone_from(&refs);
            self.restore_refs = Some(refs);
            if self.ref_focus == RefFocus::Pending
                && let OrientationState::Loaded(view) = &self.state
            {
                let view = view.clone();
                self.resolve_session(&view);
            }
            return;
        }
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        if let Some(index) = index {
            match result {
                Ok(list) => {
                    // **The rows, kept beside the names** (RFC 044): `n`, `t` and `x` need to
                    // know whether a row is a tag and whether a branch is already closed, and
                    // those facts arrive with this read and nowhere else.
                    self.ref_rows.clone_from(&list.refs);
                    let named = list.unpublished_current.name().map(ToString::to_string);
                    // **The branch prikk named goes first** (RFC 037 decision 4). It is the branch
                    // the user is standing on, and in a fresh repository it is the only row — so
                    // the cursor's default position lands on the row they almost certainly want.
                    let mut refs: Vec<String> = named.iter().cloned().collect();
                    refs.extend(list.refs.into_iter().map(|entry| entry.name));
                    let unpublished_current = named.is_some();
                    let cursor = refs
                        .iter()
                        .position(|name| Some(name.as_str()) == self.focused_ref())
                        .unwrap_or(0);
                    // RFC 029 Handoff B §4: only an empty list offers the unpublished `heads/main`. A
                    // repository with other refs does not, because that is roadmap item 8's confusion.
                    //
                    // **And only where prikk cannot say** (RFC 037 decision 3). That row is a guess
                    // stikk made because prikk would not answer; from prikk 0.47 prikk answers, and
                    // a guess beside an answer is the worse of the two — including when the answer
                    // is `null`, where the guess would assert a branch prikk just denied.
                    let unpublished_main = refs.is_empty()
                        && matches!(
                            list.unpublished_current,
                            stikk_prikk::UnpublishedBranch::NotReported
                        );
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::RefPicker {
                            refs,
                            cursor,
                            unpublished_main,
                            unpublished_current,
                        };
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
            // RFC 027 decision 5: prikk says commit would refuse. In place of the loading placeholder,
            // as an overlay — never the one-line banner above, which cannot hold a path and prikk's
            // reason for each — and never through `present()`: nothing was attempted.
            Ok(CommitPreviewOutcome::WouldRefuse(paths)) => {
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::CommitWouldRefuse { paths };
                }
            }
            // RFC 034 §5: the same posture for a declaration prikk would refuse — an overlay, because
            // prikk's refusal is two sentences and a banner would clip it.
            Ok(CommitPreviewOutcome::WouldRefuseDeclarations(declarations)) => {
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::CommitWouldRefuseDeclarations { declarations };
                }
            }
            Ok(CommitPreviewOutcome::Ready { preview: _, token }) => {
                let Some(PendingCommit::AwaitingPreview { message }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: should not happen, but nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_commit = Some(PendingCommit::Confirming { token, message });
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

    /// Answers [`RequestKind::SealPreview`] (RFC 016 §5/§6). Mirrors
    /// [`Self::apply_commit_preview`]'s three-outcome shape: blocked (`C-T4d`, an empty queue or a
    /// cross-ref target, stikk's own words, never routed through `present()`), ready (opens the
    /// confirmation — seal's first act), or a genuine seam failure (the usual `present()` routing).
    fn apply_seal_preview(&mut self, seq: u64, result: stikk_model::Result<SealPreviewOutcome>) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the preview resolved
        };
        // Consumed regardless of outcome: a blocked or failed preview leaves nothing to confirm, and a
        // ready one hands its pieces to the new `Confirming` state below — never both.
        let pending = self.pending_seal.take();
        match result {
            Ok(SealPreviewOutcome::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
            }
            Ok(SealPreviewOutcome::Ready { token }) => {
                let Some(PendingSeal::AwaitingPreview { reff }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: should not happen, but nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_seal = Some(PendingSeal::Confirming {
                    token: *token,
                    reff,
                });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Answers [`RequestKind::SealConfirmExecute`] (RFC 016 §5, the ceremony's tail — both acts already
    /// happened client-side before this was ever dispatched). On success, shows prikk's result verbatim
    /// and refreshes Orientation so the queue count reflects the seal; on failure (`Stale`, `Declined`,
    /// the `CrossRef` race, a trust refusal, or a genuine refusal), the usual `present()` routing.
    fn apply_seal_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::SealResult>>,
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
                    *slot = Overlay::SealResult {
                        result: outcome.result,
                    };
                }
                self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Answers [`RequestKind::RefWritePreview`] (RFC 044). The same three-outcome shape commit's
    /// and seal's previews have: blocked (stikk's own words, never routed through `present()` --
    /// nothing was attempted and nothing refused), ready (arms the confirmation), or a genuine
    /// seam failure.
    fn apply_ref_write_preview(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::RefWritePreview>,
    ) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the preview resolved
        };
        // Consumed regardless of outcome, as commit's is: a blocked or failed preview leaves
        // nothing to confirm, and a ready one hands its pieces to `Confirming` below.
        let pending = self.pending_ref_write.take();
        match result {
            Ok(stikk_core::RefWritePreview::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
                // **The typed name survives a block too** (RFC 044 §3). A blocked close has no
                // prompt to come back to; a blocked create does, and retyping it would be the
                // reader's second attempt at a name stikk never sent.
                if let Some(PendingRefWrite::AwaitingPreview { op }) = pending
                    && let Some(prompt) = op.prompt()
                {
                    self.overlays.push(prompt);
                }
            }
            Ok(stikk_core::RefWritePreview::Ready { token }) => {
                let Some(PendingRefWrite::AwaitingPreview { op }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: should not happen, but nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_ref_write = Some(PendingRefWrite::Confirming { token, op });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.restore_prompt(pending);
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Answers [`RequestKind::RefWriteConfirmExecute`] (RFC 044 decision 8).
    ///
    /// On success: prikk's lines verbatim, and **the ref lists are re-read**, so the new branch or
    /// tag is in the picker the next time it opens and a closed branch has moved to the closed
    /// rows. Nothing is parsed out of the lines themselves (trap 2) -- the lists are the state.
    ///
    /// On failure: the usual `present()` routing, with **the name prompt put back underneath it**
    /// carrying what the user typed, so dismissing prikk's refusal lands on the name again.
    fn apply_ref_write_confirm_execute(
        &mut self,
        seq: u64,
        kind: RefWriteKind,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::RefWrite>>,
    ) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the response arrived
        };
        // Taken here and not at dispatch: this is the one place that knows whether the flow is
        // over (it succeeded) or the user is being handed their name back (it did not).
        let pending = self.pending_ref_write.take();
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::RefWrite::Done(lines) => {
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::RefWriteResult { kind, lines };
                    }
                    self.dispatch(RequestKind::Refs);
                }
                // The band is checked before the keys offer anything, so this is only reachable
                // if prikk changed under the session. stikk's own words, never prikk's silence.
                stikk_prikk::RefWrite::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::ref_writes_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                self.restore_prompt(pending);
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Put a create's name prompt back, carrying what the user typed (RFC 044 §3). A close has no
    /// prompt, and a flow with nothing pending has nothing to restore.
    fn restore_prompt(&mut self, pending: Option<PendingRefWrite>) {
        let op = match pending {
            Some(
                PendingRefWrite::AwaitingPreview { op } | PendingRefWrite::Confirming { op, .. },
            ) => op,
            None => return,
        };
        if let Some(prompt) = op.prompt() {
            self.overlays.push(prompt);
        }
    }

    /// Answers [`RequestKind::SwitchPreview`] (RFC 045). The three-outcome shape commit, seal and
    /// the ref writes already have: blocked (stikk's own words, never through `present()` —
    /// nothing was attempted), ready (arms the confirmation), or a genuine seam failure.
    fn apply_switch_preview(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::SwitchPreview>,
    ) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the preview resolved
        };
        let pending = self.pending_switch.take();
        match result {
            Ok(stikk_core::SwitchPreview::Blocked(reason)) => {
                self.overlays.remove(index);
                self.banner = Some(reason);
            }
            Ok(stikk_core::SwitchPreview::Ready { token }) => {
                let Some(PendingSwitch::AwaitingPreview { target }) = pending else {
                    self.overlays.remove(index);
                    return; // defensive: nothing to confirm without this
                };
                if let Some(slot) = self.overlays.get_mut(index) {
                    *slot = Overlay::Confirmation {
                        summary: Box::new(token.summary().clone()),
                        tier: token.tier(),
                        typed: String::new(),
                        error: None,
                    };
                }
                self.pending_switch = Some(PendingSwitch::Confirming { token, target });
            }
            Err(error) => {
                self.overlays.remove(index);
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Answers [`RequestKind::SwitchConfirmExecute`] (RFC 045 decision 8).
    ///
    /// On success: prikk's two streams verbatim, then **orientation, the ref lists and Changes
    /// are re-read**, and **focus moves to the branch switched to**. `FR-055` says a later *read*
    /// never moves focus; this is the user's own action, and leaving History on the old branch
    /// would show a worktree that is no longer its files.
    ///
    /// On failure: the usual `present()` routing. Every refusal here wrote nothing (F2).
    fn apply_switch_confirm_execute(
        &mut self,
        seq: u64,
        result: stikk_model::Result<stikk_core::Outcome<stikk_prikk::SwitchResult>>,
    ) {
        let index = self
            .overlays
            .iter()
            .position(|o| matches!(o, Overlay::Loading { seq: s, .. } if *s == seq));
        let Some(index) = index else {
            return; // stale: the user navigated away before the response arrived
        };
        let target = match self.pending_switch.take() {
            Some(
                PendingSwitch::Executing { target } | PendingSwitch::Confirming { target, .. },
            ) => Some(target),
            Some(PendingSwitch::AwaitingPreview { .. }) | None => None,
        };
        match result {
            Ok(outcome) => match outcome.result {
                stikk_prikk::SwitchResult::Done { printed, noted } => {
                    if let Some(slot) = self.overlays.get_mut(index) {
                        *slot = Overlay::SwitchResult { printed, noted };
                    }
                    if let Some(target) = target {
                        self.clear_ref_mark_on_ref_change(&target);
                        self.ref_focus = RefFocus::Ref(target.clone());
                        // **Changes is the one screen a switch makes wrong** (§3e). It compares
                        // the worktree against a baseline, and the worktree is now another
                        // branch's files -- so it is re-read against the branch switched to, not
                        // refreshed against the one that was left. History below it is not
                        // wrong: a screen shows its own ref, never the focused one (RFC 031
                        // review v1 §2.2), and that branch's lineage is unchanged.
                        if matches!(self.screens.last(), Some(Screen::Changes { .. })) {
                            let seq = self.dispatch(RequestKind::Changes {
                                reff: target.clone(),
                            });
                            if let Some(Screen::Changes {
                                view, refreshing, ..
                            }) = self.screens.last_mut()
                            {
                                view.reff = target;
                                *refreshing = Some(seq);
                            }
                        }
                    }
                    self.orientation_pending = Some(self.dispatch(RequestKind::Orient));
                    self.dispatch(RequestKind::Refs);
                }
                // The band is checked before the key offers anything, so this is only reachable
                // if prikk changed under the session.
                stikk_prikk::SwitchResult::Unavailable => {
                    self.overlays.remove(index);
                    self.banner = Some(stikk_core::switch_unavailable(
                        self.prikk_minor().unwrap_or_default(),
                    ));
                }
            },
            Err(error) => {
                self.overlays.remove(index);
                // **Back to the picker, under prikk's refusal** (§3e). A dirty worktree or a
                // file in the way is something the reader fixes and tries again, and the picker
                // is where the second attempt starts; the list is re-read on the way, because
                // the refusal may be about a ref that moved.
                self.reopen_ref_picker();
                self.surface(&error, OperationContext::Other);
            }
        }
    }

    /// Re-read the ref list and leave the picker where the reader left it.
    ///
    /// Pushed **under** whatever is surfaced next, so dismissing a refusal lands on the picker
    /// rather than on the screen behind it.
    fn reopen_ref_picker(&mut self) {
        let seq = self.dispatch(RequestKind::Refs);
        self.overlays.push(Overlay::Loading { what: "refs", seq });
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

    /// prikk's minor version, when orientation has loaded.
    ///
    /// `None` before the first load, and on a load failure — which `present` reads as "cannot exclude
    /// prikk 0.28", the direction that keeps an explanation rather than hiding it (RFC 026 C §2).
    fn prikk_minor(&self) -> Option<u32> {
        match &self.state {
            OrientationState::Loaded(view) => Some(view.prikk_minor),
            OrientationState::Loading | OrientationState::Failed(_) => None,
        }
    }

    /// Route a seam error through the one presentation mapping (ER-03) and surface it accordingly.
    fn surface(&mut self, error: &stikk_model::StikkError, op: OperationContext) {
        match present(error, op, self.prikk_minor()) {
            Presentation::RefusalOverlay(card) => {
                self.refusals.record(card.verbatim.clone(), "refusal", op);
                self.overlays.push(Overlay::Refusal { card, cursor: 0 });
            }
            // Never folded into `RefusalOverlay`/`refusals` (design-review C1): nothing here is
            // prikk's message, so it must not be recorded as one nor rendered under prikk's label.
            Presentation::Stale {
                operation,
                cause,
                headline,
                gloss,
                next_steps,
            } => {
                self.overlays.push(Overlay::Stale {
                    operation,
                    cause,
                    headline,
                    gloss,
                    next_steps,
                    cursor: 0,
                });
            }
            Presentation::Banner { message, .. }
            | Presentation::RoutedIntoView { message, .. }
            | Presentation::InConfirmation { message } => self.banner = Some(message),
            Presentation::InlineGuidance { detail, toward: _ } => {
                // RFC 018 F2: this arm used to send `Target::TrustKeys` toward "see Glossary → Trust &
                // Keys" — there is no such section (`Keys` names the keybinding table; a user following
                // the old pointer after a signing-readiness refusal landed on what `j`/`k` do). A
                // pointer to a place that does not exist costs a navigation to discover that; no pointer
                // is the honest answer until `FR-104`'s Trust & Keys view exists to point at. `detail`
                // is already the complete, actionable message on its own for every `toward` value today.
                // The trust-refusal shape no longer reaches this arm at all (review v2, C1): its
                // explanation needs room to wrap, which this one-line banner cannot give it, so
                // `present()` routes it through `RefusalOverlay` above instead.
                //
                // `toward` stays load-bearing even though nothing here reads it: `present()`'s own
                // classification (RFC 012 F-b) still distinguishes `PrikkVersion` from `TrustKeys`, and
                // a future Trust & Keys view (`FR-104`) has somewhere real to route it once it exists.
                self.banner = Some(detail);
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
            Target::Queue => self.open_queue(),
            Target::Glossary => self.overlays.push(Overlay::Glossary {
                offset: std::cell::Cell::new(0),
            }),
            // RFC 016: the full-queue refusal's own next-step lands here — the same entry point the
            // palette's `op.seal` command uses (`Self::begin_seal`'s own capability pre-check applies
            // either way, so a session that cannot seal sees why, not a silently-armed ceremony).
            Target::Seal => self.begin_seal(),
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
            "op.seal" => self.begin_seal(),
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

    /// Whether this session's prikk persists a commit message (RFC 015 F2) — the commit-message
    /// prompt's copy needs this. Defaults to `false` (the pre-0.32 claim) if orientation has not
    /// loaded, which cannot actually happen in practice: [`Self::begin_commit`] is unreachable before
    /// then, since every key/palette path to it requires a loaded, non-Viewer capability first.
    #[must_use]
    fn messages_persist(&self) -> bool {
        match &self.state {
            OrientationState::Loaded(view) => view.prikk_persists_messages,
            _ => false,
        }
    }

    /// The repository root this app opened.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// The ref the session is focused on, or `None` while focus is pending or unfocused (RFC 029 Handoff
    /// B §2). Never a HEAD (FR-055).
    #[must_use]
    pub fn focused_ref(&self) -> Option<&str> {
        match &self.ref_focus {
            RefFocus::Ref(name) => Some(name),
            RefFocus::Pending | RefFocus::Unfocused => None,
        }
    }

    /// Which of focus's three states the session is in (RFC 029 Handoff B §2), for the status bar.
    #[must_use]
    pub fn ref_focus(&self) -> &RefFocus {
        &self.ref_focus
    }

    /// The focused ref for an action that needs one, or — with none — `C-T4d`'s reason in the banner
    /// and `None`, so the caller dispatches nothing (RFC 029 Handoff B §2).
    fn require_focus(&mut self) -> Option<String> {
        let focused = self.focused_ref().map(str::to_string);
        if focused.is_none() {
            self.banner = Some(stikk_core::NO_FOCUSED_REF_REASON.to_string());
        }
        focused
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
            Some(Screen::BlockDetail { view, cursor, .. }) => Focus::BlockDetail(view, *cursor),
            Some(Screen::Changes {
                view,
                hide_untracked,
                history,
                ..
            }) => Focus::Changes(view, *hide_untracked, history),
            Some(Screen::Queue {
                view,
                offset,
                cursor,
                ..
            }) => Focus::Queue(view, offset, *cursor),
            Some(Screen::PatchDetail {
                view, raw, offset, ..
            }) => Focus::PatchDetail(view, *raw, offset),
            Some(Screen::MergePlan { view, offset }) => Focus::MergePlan(view, offset),
            Some(Screen::Bundle { view, offset }) => Focus::Bundle(view, offset),
            Some(Screen::Verify {
                view,
                raw,
                refreshing,
                offset,
            }) => Focus::Verify(view, *raw, refreshing.is_some(), offset),
            Some(Screen::Trust { view, offset }) => Focus::Trust(view, offset),
            Some(Screen::Compare {
                view,
                opened,
                cursor,
                raw,
                offset,
                ..
            }) => Focus::Compare(CompareFocus {
                view,
                opened,
                cursor: *cursor,
                raw: *raw,
                offset,
            }),
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
