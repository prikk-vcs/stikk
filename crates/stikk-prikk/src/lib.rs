//! The prikk seam — the only code in stikk that talks to prikk (design `stikk-04` AR-02, MOD-02).
//!
//! Everything above this crate depends on the [`Prikk`] trait, never on *how* prikk is reached. This
//! localizes four things to one place: the constraint that stikk drives prikk only through its public
//! surface (CON-1), the pre-1.0-library risk and machine-output gap (UD-02), the EPIPE guard (UD-04),
//! and version-skew handling (NFR-R03). The v1 implementor is [`cli_backend::CliBackend`], which
//! drives the `prikk` binary; a linked-library backend is deferred behind the same trait.
//!
//! Two security properties live here and nowhere else:
//! - [`mod@env`] reads signing-key **presence only, never values** (threat model C-I1, data model LC-13).
//! - No key material ever crosses the seam: prikk reads its own environment when it signs; stikk
//!   hands it nothing, because it holds nothing (design SEAM-06).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cli_backend;
pub mod env;
mod json;
pub mod key_id;
pub mod null_backend;
pub mod version;

pub use cli_backend::CliBackend;
pub use null_backend::NullBackend;
pub use version::{Version, validated_ceiling_display};

use std::path::Path;

use stikk_model::{ChangeToken, Result};

/// The result of the version-and-capability probe stikk performs when it first reaches prikk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    /// The parsed prikk version.
    pub version: Version,
    /// prikk's raw `--version` line, preserved verbatim for display and diagnostics (NFR-I03).
    pub raw_version: String,
    /// Whether this prikk version is at or above the floor stikk requires (NFR-R03). When false, the
    /// operation layer degrades to read-only rather than misrender an unknown format.
    pub supported: bool,
    /// Whether this prikk version is within the range stikk has actually checked its output shapes
    /// against — at or below the validated ceiling (RFC 009 decision 7). A `supported` prikk above
    /// this ceiling still runs (refusing it would break users the day prikk ships a minor), but stikk
    /// says its shapes are unverified rather than silently asserting knowledge it does not have.
    pub validated: bool,
}

/// A minimal read-only orientation of a repository — the summary the launcher and the (future) TUI
/// Orientation view show (design VW-01, FR-002). Deliberately small in this foundation increment;
/// more fields land as the read surfaces are built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orientation {
    /// Number of patches queued in the active WAL, not yet sealed.
    pub queued_patches: u64,
    /// The ref the queue targets, when prikk reports one (`queued patches: N targeting <ref>`) — absent
    /// only when the queue is empty. This is the same fact behind [`WorktreeStatus::queued_elsewhere`]'s
    /// warning (RFC 009 F1/F4): showing it here is strictly more honest than a bare count.
    pub queued_target: Option<String>,
    /// The current RefState object id of `heads/main`, if the ref is published.
    pub main_ref_state: Option<String>,
    /// Trailing partial WAL bytes, if any — a torn tail an interrupted commit left behind.
    pub trailing_partial_wal_bytes: u64,
    /// prikk's own active-patch threshold warning, verbatim, when the queue is at or above the warn or
    /// hard-limit threshold (`PRIKK_ACTIVE_PATCH_WARN`/`_LIMIT`; design `C-D2a`; RFC 014 F5). `None`
    /// below the warn threshold. Read from the same `status` report `Orientation` already parses — the
    /// parser has tolerated this line's presence since RFC 012, deferred to this increment (see
    /// `cli_backend::parse::orientation`'s doc). Never paraphrased (`ER-02`): stikk surfaces the limit
    /// it is about to hit using prikk's own words, computed against whatever thresholds are actually
    /// configured — including operator overrides stikk has no other way to know about.
    pub active_patch_warning: Option<String>,
}

/// One patch id and its message, as `prikk log` names it (design FR-011; RFC 015 F1). Only patches
/// authored by prikk ≥ 0.32 carry a line at all: `-m` has always been mandatory, but the message was
/// validated and discarded below that version (`UD-01`, retired at 0.32) — an older patch contributes
/// **no entry**, which is absence, not an empty message (RFC 015 F4). `BlockRow::patches` (the count)
/// can therefore legitimately exceed `messages.len()`; never render one without the other (`C-T4d`
/// via `T-T4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchMessage {
    /// The patch's object id.
    pub patch_id: String,
    /// The message, verbatim — repository content (`C-T2a`): render inert, never as a next-step or a
    /// glossary trigger (`C-T2b`).
    pub message: String,
}

/// One sealed block in a ref's lineage, as `prikk log` reports it (design FR-011; RFC 006). Block
/// granularity is prikk's ceiling — no per-patch *content* (operations, preimages) and no
/// `log --format json` (`UD-09`, narrowed RFC 015 F3) — but patch **ids and messages** are no longer
/// beyond it for a messaged patch (`messages`, RFC 015 F1); no author or date either way, by prikk's
/// no-clock design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRow {
    /// The block's object id.
    pub block_id: String,
    /// The RefState object id that points at this block.
    pub ref_state_id: String,
    /// Monotonic publication sequence.
    pub update_seq: u64,
    /// Block kind as prikk names it (`Root`, `Normal`, `Merge`, `Repair`, `Import`, …); kept as text
    /// so a future kind renders rather than breaks parsing.
    pub kind: String,
    /// Whether this block carries rollback patches.
    pub rollback_block: bool,
    /// Number of parent blocks.
    pub parents: u64,
    /// Number of patches sealed in this block. May legitimately exceed `messages.len()` — a patch
    /// authored below prikk 0.32 carries no message and so no entry there (RFC 015 F4); this count is
    /// the authoritative total either way.
    pub patches: u64,
    /// Number of rollback patches.
    pub rollback_patches: u64,
    /// Number of required attestations.
    pub required_attestations: u64,
    /// The patch id and message for every patch in this block that carries one (RFC 015 F1) — never
    /// the block's full patch count; see [`Self::patches`] for that. Empty for a block sealed entirely
    /// below prikk 0.32, which is normal, not an error.
    pub messages: Vec<PatchMessage>,
    /// The previous RefState in the chain, or `None` at genesis.
    pub previous_ref_state: Option<String>,
}

/// A ref's sealed lineage (tip first), as `prikk log` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    /// The ref this lineage belongs to.
    pub reff: String,
    /// Blocks newest-first (the order `prikk log` prints).
    pub blocks: Vec<BlockRow>,
}

/// The replayed state at a ref's tip, from `prikk checkout --patch-plan` (design FR-032 at tip
/// granularity; RFC 006). prikk replays to the ref tip, not to an arbitrary historical block, so this
/// describes the tip only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFiles {
    /// The block the state was replayed to (the ref tip).
    pub target_block: String,
    /// Repo-relative file paths present in the replayed state.
    pub files: Vec<String>,
    /// Total content bytes across those files, as prikk reports.
    pub total_bytes: u64,
}

/// One ref pointer — a branch (open, closed, or received) or a tag. `branch list --all` does not
/// reliably exclude tags (RFC 012 FR-014: `prikk`'s own ref-pointer index carries no namespace filter,
/// so a tag can appear there too), so stikk does not depend on it either including or excluding them;
/// `Prikk::tags` (`prikk tag list`) is the documented source, and `stikk_core::list_refs` is the merge
/// point that makes the result correct regardless of which way that unspecified behavior goes.
/// [`RefEntry::is_tag`] is sourced through both `Prikk::refs` and `Prikk::tags`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefEntry {
    /// The fully-qualified ref name (`heads/…`, `tags/…`, `remotes/…`).
    pub name: String,
    /// The RefState (or, for a received ref, the received pointer) id prikk printed.
    pub id: String,
    /// True for a closed branch (marked `(closed)`).
    pub closed: bool,
    /// True for a received ref (marked `(received)`) — read-only until adopted.
    pub received: bool,
}

impl RefEntry {
    /// True when this is a tag ref (`tags/…`).
    #[must_use]
    pub fn is_tag(&self) -> bool {
        self.name.starts_with("tags/")
    }
}

/// One worktree path that differs from the replay baseline, from `prikk worktree-status` (design
/// FR-034; RFC 008). Path-level only — prikk reports *that* a tracked file's bytes differ, never the
/// content difference (the UD-09 ceiling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// The change kind as prikk names it (`modified`, `missing`, `untracked`, `unsupported`); kept as
    /// text so a future kind renders rather than breaks parsing.
    pub kind: String,
    /// The repo-relative worktree path.
    pub path: String,
    /// prikk's own one-line description of why the path is listed (preserved verbatim, NFR-I03).
    pub note: String,
}

/// Worktree-vs-baseline status for a ref, from `prikk worktree-status` (design FR-034; RFC 008).
///
/// `worktree-status` reports a **non-zero exit when the tree is dirty** — that is a normal status, not
/// a refusal (RFC 008 finding 2 / UD-05); the seam reads this report from stdout regardless of exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeStatus {
    /// The ref whose replay baseline the worktree was compared against.
    pub reff: String,
    /// True when the worktree matches the baseline (`worktree: clean against baseline`).
    pub clean: bool,
    /// Tracked files in the baseline.
    pub tracked: u64,
    /// Files unchanged from the baseline.
    pub unchanged: u64,
    /// Tracked files absent from the worktree.
    pub missing: u64,
    /// Tracked files whose bytes differ from the baseline.
    pub modified: u64,
    /// Worktree files not in the baseline.
    pub untracked: u64,
    /// Paths prikk cannot represent against the baseline.
    pub unsupported: u64,
    /// The per-path entries (the counts above summarize these).
    pub entries: Vec<WorktreeEntry>,
    /// prikk's own warning, verbatim, when the active WAL holds queued patches for a **different** ref
    /// than the one asked about: paths listed "untracked" here may be committed-but-unsealed work
    /// (RFC 009 F4). `None` when prikk did not emit it. Never paraphrased (ER-02) — stikk transports
    /// this warning, it does not restate it.
    pub queued_elsewhere: Option<String>,
}

/// One file-level change `prikk commit` recorded, from its per-path output lines (design `FR-050`;
/// RFC 014). `operation` is prikk's own label (`create-file`, …), kept as text like
/// [`WorktreeEntry::kind`] so a future operation kind renders rather than breaks parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitChange {
    /// prikk's own operation label for this change.
    pub operation: String,
    /// The repo-relative path affected.
    pub path: String,
}

/// The result of `prikk commit --from-worktree` (design `FR-050`; RFC 014). Every field is prikk's own
/// fact, transported rather than summarised (`C-T4a`/`C-T4c`) — including `notes`, which carries **every**
/// `note:` line prikk printed, verbatim and in order, rather than two hardcoded slots. RFC 014 F4 found
/// exactly one such note (the message's fate); this seam was captured against real prikk 0.31.1 output,
/// but a note is prose, not a stable field — a later prikk (0.32+, once RFC 123 lands) removes it because
/// it becomes false, not because stikk's parser changed. A fixed two-note shape would misreport the
/// moment upstream ships that change; `Vec<String>` degrades honestly to whatever prikk actually said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitResult {
    /// The ref this patch was authored against (`baseline ref:`).
    pub baseline_ref: String,
    /// The new patch's object id.
    pub patch_id: String,
    /// The active WAL's sequence number for this patch.
    pub wal_sequence: u64,
    /// Number of operations the patch carries.
    pub operations: u64,
    /// Number of blobs the patch references.
    pub referenced_blobs: u64,
    /// Number of text-edit operations (always `0`: stikk never sends `--text-edits`, RFC 014 F1).
    pub text_edits: u64,
    /// The per-path changes captured (the indented lines under the summary).
    pub changes: Vec<CommitChange>,
    /// Every `note:` line prikk printed, verbatim, in the order printed (`ER-02`). Includes the
    /// perpetual multi-operation-diff-minimization note and, on a prikk that has not yet shipped RFC
    /// 123, the message's-fate note (RFC 014 F4) — never assumed present, never assumed absent.
    pub notes: Vec<String>,
}

/// The result of `prikk seal` (design `FR-052`; RFC 016 §5) — the second `Prikk` method that writes,
/// and the first that publishes history. Every field is prikk's own fact, transported rather than
/// summarised (`C-T4a`/`C-T4c`), following [`CommitResult`]'s own precedent: `notes` carries **every**
/// `note:` line prikk printed, verbatim and in order, never fixed named fields — captured at 0.28.0 and
/// 0.33.0, both print exactly one (`"audit plugins remain later PRs"`), but a note is prose, not a
/// stable field, and `CommitResult`'s own history is the reason this project never assumes a note set
/// is fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealResult {
    /// Number of patches sealed into the new block.
    pub patches: u64,
    /// The new block's object id.
    pub block_id: String,
    /// The ref that was sealed — echoes the seam's own argument, transported rather than merely
    /// assumed (the same discipline [`CommitResult::baseline_ref`] applies).
    pub reff: String,
    /// The ref's new `RefState` object id after this seal.
    pub ref_state: String,
    /// Every `note:` line prikk printed, verbatim, in the order printed (`ER-02`).
    pub notes: Vec<String>,
}

/// The entire prikk contract stikk depends on. Every method returns [`stikk_model::StikkError`] on
/// failure, classified into the presentation taxonomy the operation layer consumes.
///
/// Read-only surface so far (design CT-03 categories `read-history`, `read-state`); the mutating
/// categories grow against this trait later without disturbing callers.
///
/// **`Send + Sync`** (design SEAM-02; RFC 010): the frontend runs every seam call on a worker thread
/// borrowed via `std::thread::scope`, so an implementor must be safely shareable across that boundary.
/// This is additive for both existing implementors — `CliBackend` holds only an `OsString` and a
/// `OnceLock`, `NullBackend` holds only owned, `Clone` data — neither has interior mutability that
/// would need `unsafe` to satisfy the bound.
pub trait Prikk: Send + Sync {
    /// Probe prikk's version and whether it is supported (design SEAM-05).
    ///
    /// # Errors
    /// [`stikk_model::StikkError::Environment`] when `prikk` cannot be launched or its version line
    /// cannot be parsed.
    fn handshake(&self) -> Result<Handshake>;

    /// Read a minimal read-only orientation of the repository rooted at `repo`.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified: a refusal, an environment fault (unparseable output,
    /// missing binary), or a lock conflict, per the seam's classification rules.
    fn orientation(&self, repo: &Path) -> Result<Orientation>;

    /// Read a ref's sealed block lineage (design FR-010/011; category `read-history`). `limit` caps
    /// the number of blocks.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::orientation`].
    fn history(&self, repo: &Path, reff: &str, limit: usize) -> Result<History>;

    /// Read the replayed state file set at a ref's tip (design FR-032; category `read-state`).
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::orientation`].
    fn block_state(&self, repo: &Path, reff: &str) -> Result<StateFiles>;

    /// List every branch ref pointer in the repository (design FR-014; category `read-history`).
    ///
    /// **Does not reliably exclude tags** — `prikk branch list --all`'s own implementation lists every
    /// ref pointer regardless of namespace, undocumented behavior stikk must not depend on either way
    /// (RFC 012 FR-014, discovered empirically: RFC 009 F3 had claimed a tag could never appear here,
    /// an inference from an untested case, not a checked one). `stikk_core::list_refs` is the
    /// merge-and-deduplicate caller that makes tag coverage correct regardless of what this returns;
    /// call [`Prikk::tags`] for the documented, stable way to list tags.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::orientation`].
    fn refs(&self, repo: &Path) -> Result<Vec<RefEntry>>;

    /// List every tag pointer in the repository (design FR-014 completion; category `read-history`;
    /// RFC 012). prikk's own stable, documented way to list tags (`prikk tag list`), as distinct from
    /// [`Prikk::refs`]'s incidental (and not to be relied on) tag leakage.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::orientation`].
    fn tags(&self, repo: &Path) -> Result<Vec<RefEntry>>;

    /// Read worktree-vs-baseline status for `reff` (design FR-034; category `worktree-analysis`;
    /// RFC 008). A dirty worktree is reported as success (`clean == false`), not a refusal — prikk's
    /// non-zero dirty exit is handled inside the implementation (RFC 008 finding 2).
    ///
    /// # Errors
    /// [`stikk_model::StikkError`]: an unrecognized report shape is an environment fault (UD-02); a
    /// genuine failure (bad ref, not a repository) classifies as for [`Prikk::orientation`].
    fn worktree_status(&self, repo: &Path, reff: &str) -> Result<WorktreeStatus>;

    /// Compose a cheap "has anything changed?" signal from the ref pointers (branches **and** tags,
    /// merged and deduplicated by name — never `Prikk::refs` alone, since its tag coverage is
    /// unspecified) and queue state (design `LC-4`; category `read-history`; RFC 003, corrected by
    /// review C1). Composed from the same calls [`Prikk::refs`]/[`Prikk::tags`]/[`Prikk::orientation`]
    /// already make elsewhere in this trait — no *dedicated* prikk invocation is introduced beyond
    /// those three. Detection only, never a lock (`CT-05`/`NFR-R02`): the repository can still change
    /// between reading this token and acting on it.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::orientation`].
    fn change_token(&self, repo: &Path) -> Result<ChangeToken>;

    /// Author a worktree patch into the active WAL (design `FR-050`; category `QueueMutation`; RFC
    /// 014) — **the first method on this trait that writes**. Never sends `--text-edits`: prikk's own
    /// source documents it as a no-op retained for compatibility (RFC 014 F1), so offering it would be
    /// a control that changes nothing.
    ///
    /// **Single-shot: never retried** (`SEAM-04`/`NFR-S04`). A failure returns and the caller — the
    /// operation layer's `execute` — decides; no implementation of this method may retry internally.
    ///
    /// # Errors
    /// [`stikk_model::StikkError::CrossRef`] when the active WAL's queue targets a different ref than
    /// `reff` (a race: stikk prevents this client-side before arming a commit, via
    /// `Orientation::queued_target`, but the queue can move between preview and this call). A clean
    /// worktree is likewise prevented client-side and, reaching here anyway, degrades to a verbatim
    /// [`stikk_model::StikkError::Refusal`] (RFC 014 F6 — no dedicated class, since no misclassification
    /// risk exists to fix). [`stikk_model::StikkError::NotReady`] when AUTHOR signing readiness is
    /// absent (`OPL-04`'s seam-side re-check). Otherwise classified as for [`Prikk::orientation`].
    fn commit(&self, repo: &Path, reff: &str, message: &str) -> Result<CommitResult>;

    /// Seal the active WAL into a new, MAINTAINER-signed block (design `FR-052`; category
    /// `Publication`; RFC 016 §5) — **the second method on this trait that writes, and the first that
    /// publishes history**. There is no undo, in stikk or in prikk.
    ///
    /// **Passes `--allow-no-audit` unconditionally, every time, with no configuration to omit it.**
    /// prikk refuses without it as a **usage error (exit 2)** — per RFC 014 F6 an exit 2 from stikk is
    /// a stikk bug, never a user-facing refusal — so there is no code path that could omit the flag and
    /// still reach a real answer. The flag's own meaning is scaffolding prikk may retire; the reason
    /// stikk always passes it is structural, not a policy choice this trait exposes (RFC 016 F1).
    ///
    /// **Single-shot: never retried** (`SEAM-04`/`NFR-S04`), the same discipline [`Prikk::commit`]
    /// follows.
    ///
    /// # Errors
    /// [`stikk_model::StikkError::CrossRef`] when the active WAL's queue targets a different ref than
    /// `reff` — worded differently from commit's own cross-ref refusal (RFC 016 F4: seal says
    /// `"requested seal ref is"`, commit says `"requested ref"`, and at 0.33.0 seal's message carries
    /// no class prefix at all where commit's does), which is why the classifier matches the clause both
    /// share (`"active wal is owned by"`) rather than either's trailing phrase. An empty queue is
    /// likewise prevented client-side and, reaching here anyway, degrades to a verbatim
    /// [`stikk_model::StikkError::Refusal`]. [`stikk_model::StikkError::NotReady`] when MAINTAINER
    /// signing readiness is absent, or when prikk refuses the signer as untrusted (RFC 017 F5/F6 — the
    /// classification; the ceremony's guidance is RFC 016 §9). Otherwise classified as for
    /// [`Prikk::orientation`].
    fn seal(&self, repo: &Path, reff: &str) -> Result<SealResult>;
}

#[cfg(test)]
mod tests;
