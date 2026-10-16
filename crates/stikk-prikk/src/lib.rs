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
pub mod escape;
mod json;
pub mod key_id;
pub mod null_backend;
pub mod version;

pub use cli_backend::CliBackend;
pub use null_backend::NullBackend;
pub use version::{
    CURRENT_BRANCH_FROM_MINOR, DIFF_FROM_MINOR, JSON_REPORTS_FROM_MINOR,
    PATCH_PATH_AT_PATCH_FROM_MINOR, TREE_FROM_MINOR, UNPUBLISHED_BRANCH_FROM_MINOR, Version,
    validated_ceiling_display,
};

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
    /// prikk's interrupted-materialization line, **verbatim**, when a checkout or branch switch stopped
    /// part-way and left the worktree unverified (prikk ≥ 0.43; RFC 034 decision 4 §7).
    ///
    /// `None` when prikk reports none — **and below 0.43, where prikk reports the state nowhere at all**
    /// (measured: 0.42 prints no such line, its `status --format json` has no such field, and its
    /// `doctor` finds nothing; only its `commit` refuses, which `present()` glosses instead).
    ///
    /// **It is a state of the repository, not an error**, and prikk's sentence names both ways out, so
    /// it is carried whole rather than reworded (`ER-02`).
    pub interrupted_materialization: Option<String>,
    /// prikk's own active-patch threshold warning, verbatim, when the queue is at or above the warn or
    /// hard-limit threshold (`PRIKK_ACTIVE_PATCH_WARN`/`_LIMIT`; design `C-D2a`; RFC 014 F5). `None`
    /// below the warn threshold. Read from the same `status` report `Orientation` already parses — the
    /// parser has tolerated this line's presence since RFC 012, deferred to this increment (see
    /// `cli_backend::parse::orientation`'s doc). Never paraphrased (`ER-02`): stikk surfaces the limit
    /// it is about to hit using prikk's own words, computed against whatever thresholds are actually
    /// configured — including operator overrides stikk has no other way to know about.
    pub active_patch_warning: Option<String>,
    /// prikk's current branch, from the `current branch:` line prikk ≥ 0.42 prints in `status` (RFC 030
    /// decision 1): not reported below 0.42, prikk's unresolved text verbatim, or a validated branch.
    /// Read from the `status` report this struct already parses — never from `.prikk/current-branch`
    /// (`CON-1`). Nothing renders it yet; RFC 029 Handoff B does.
    pub current_branch: stikk_model::CurrentBranch,
}

/// The active queue, as [`Prikk::queue`] reads it (design `FR-051`; RFC 028 decision 1).
///
/// **Two variants, because below prikk 0.39 the list is not reported, not empty** (`C-T2c′`). An
/// [`QueueReport::Unreported`] queue with patches in it must never render as an empty list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueReport {
    /// prikk ≥ 0.39: `status --format json`'s `queue`, every patch listed.
    Listed(Queue),
    /// prikk < 0.39: no enumeration exists. The count and target come from the prose `status` line
    /// Orientation already reads; `target` is `None` for an empty queue, and for prikk's
    /// `<missing metadata>`/`<malformed metadata>` sentinels, which the prose form does not tell apart.
    Unreported {
        /// Patches queued in the active WAL.
        count: u64,
        /// The ref the queue targets, when prikk names one.
        target: Option<String>,
    },
}

/// An enumerated queue (`status-report-v1`'s `queue`, prikk ≥ 0.39).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    /// `count`, held equal to `patches.len()` by the reader.
    pub count: u64,
    /// The ref the queue targets, or why prikk cannot name one.
    pub target: QueueTarget,
    /// The active-patch thresholds. `None` exactly when the queue is empty.
    pub threshold: Option<QueueThreshold>,
    /// Each queued patch, in prikk's order.
    pub patches: Vec<QueuedPatch>,
}

/// `target_ref` with `target_ref_status`, as one state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueTarget {
    /// `target_ref` names a validated ref, and `target_ref_status` is `null`.
    Ref(String),
    /// `target_ref_status: "missing-metadata"` — prikk has no ref-name for the active session.
    MissingMetadata,
    /// `target_ref_status: "malformed-metadata"` — the active session's ref-name is not a valid ref.
    MalformedMetadata,
    /// Both `null`. prikk 0.42.0 emits this for an empty queue; it is held rather than refused, since
    /// the schema allows it for any queue.
    NotReported,
}

/// The active-patch thresholds prikk reports for a non-empty queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueThreshold {
    /// Where the queue stands.
    pub status: ThresholdStatus,
    /// `warn_threshold`.
    pub warn: u64,
    /// `hard_limit`.
    pub hard_limit: u64,
}

/// `threshold_status`, held to prikk's vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdStatus {
    /// `none` — below the warning threshold.
    None,
    /// `warn` — at or above the warning threshold.
    Warn,
    /// `hard-limit` — at or above the hard limit.
    HardLimit,
}

/// One queued patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPatch {
    /// The patch id, validated as an object id.
    pub patch_id: String,
    /// The patch's message, in three states that never collapse into each other.
    pub message: QueuedMessage,
    /// The patch's operations, in prikk's order.
    pub operations: Vec<QueuedOperation>,
}

/// A queued patch's message (RFC 028, decision 2 as revised at prikk 0.42.0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuedMessage {
    /// The field is absent: this prikk (0.39–0.41) does not report a queued patch's message. **Never
    /// "no message"** (`C-T2c′`).
    NotReported,
    /// `null`: the patch carries no message (a patch committed by an older prikk).
    None,
    /// The message, verbatim.
    Text(String),
}

/// One operation of a queued patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedOperation {
    /// prikk's operation kind, verbatim — a kind stikk does not know still renders (RFC 027 A).
    pub kind: String,
    /// The paths it touches, as reported.
    pub paths: Vec<QueuedPath>,
    /// The asserting author's key id. Always present on `rename-path`; carried on any other kind if
    /// prikk reports it.
    pub author_key_id: Option<String>,
}

/// A path as prikk's reports give it — **exactly one of the two, read from the element's shape**.
///
/// Shared by `status-report-v1`'s queue (where it landed, hence the name) and `show-report-v1`'s
/// operations (RFC 035), because it is one JSON shape with one reader.
///
/// **Never keyed on a field's absence** (RFC 035 decision 3). prikk 0.47 began writing a `path` where a
/// queued patch's `show` had written none, and filed it as `### Changed` — a reader that treated "no
/// `path`" as "unresolved" would have been reading a release note, not a report. An element carrying
/// neither field, or both, is stikk's environment error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuedPath {
    /// `{"path": …}`.
    Path(String),
    /// `{"unresolved_node_id": …}` — a node prikk did not resolve to a path.
    ///
    /// **stikk never says why, because it cannot know which reason applies** (RFC 035 A1 F7): a bare
    /// sealed patch id resolves nothing at any measured version, a queued patch resolved nothing below
    /// 0.47, and a sealed block resolves against its own end — where a file deleted later in the block
    /// has no path left to name. All three arrive here identically.
    UnresolvedNode(String),
}

/// What `prikk show <block-id|patch-id> --format json` reports (`show-report-v1`; design `FR-030`;
/// RFC 035 decision 1) — **the one read Patch detail is built on**.
///
/// **Two variants, because below prikk 0.39 the patches are not reported, not absent** (`C-T2c′`), the
/// same shape [`QueueReport`] takes for the same reason. 0.39 is stikk's JSON floor (`reads_json`), not
/// `show-report-v1`'s own: the schema exists from prikk 0.36, but stikk reads no JSON from prikk at all
/// between 0.36 and 0.38, and the project owner ruled that band unoccupied (RFC 035 A1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowReport {
    /// prikk ≥ 0.39: every patch the report lists, in prikk's order.
    Listed(Vec<ShownPatch>),
    /// prikk < 0.39: stikk asked prikk nothing and has nothing to report. **Never an empty list** — the
    /// view says, in stikk's own voice, that it reads a patch's content from prikk 0.39 on (`C-T4d`).
    /// Saying *prikk does not report it* would be false at 0.36–0.38.
    Unreported,
}

/// One patch, as `show-report-v1` reports it.
///
/// **A report asked about a block id carries every patch in that block** — that is how stikk enumerates
/// them, since `log` reports `patch_count` and `patch_messages` but no patch ids (RFC 035 F0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownPatch {
    /// The patch's object id, validated at the parse boundary.
    pub patch_id: String,
    /// **prikk's own word for whether this patch is still in the active WAL.** Never inferred from
    /// which id stikk asked with: asking about a *patch* id does not make a patch queued, and a sealed
    /// patch asked about by patch id answers `false` with its paths unresolved (RFC 035 A1 F7, row 2).
    pub queued: bool,
    /// The patch's operations, in prikk's order.
    pub operations: Vec<PatchOperation>,
}

/// One operation of a patch, with the content `FR-030` asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchOperation {
    /// prikk's operation kind, verbatim (`ER-02`) — `edit-text`, `create-file`, `delete-node`,
    /// `replace-binary`, `change-perm`, `rename-path` at every measured version. **A kind stikk does
    /// not know does not fail the report**: it arrives with [`OperationContent::Unrendered`] and keeps
    /// both prikk's word and its raw fields (RFC 035 §3 rule 3).
    pub kind: String,
    /// The paths it touches, as prikk reports them: one for most kinds, two — old then new — for
    /// `rename-path`.
    pub paths: Vec<QueuedPath>,
    /// The operation's content, read per kind as prikk's measured shape gives it.
    pub content: OperationContent,
    /// **prikk's own fields for this operation**, as JSON, for the raw view (RFC 035 decision 5). Keys
    /// in prikk's order, values as prikk gave them, re-indented for a terminal; it costs no second read
    /// because the report has already been parsed (§3 rule 6). The display bound is applied where it is
    /// rendered, not here — the seam reports what prikk said.
    pub raw: String,
}

/// An operation's content, by kind (RFC 035 §2's measured table, decision 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationContent {
    /// `edit-text`: the replaced span and its replacement, **as spans** — not line ranges, at any
    /// version (RFC 021 F2). Either may be empty, and empty is a measured state, not a missing one.
    TextEdit {
        /// `old_span_text`.
        old_span: String,
        /// `replacement_text`.
        replacement: String,
    },
    /// `create-file`: the created content and its mode.
    FileCreated {
        /// `content`.
        content: FileContent,
        /// `mode`, as prikk's integer (e.g. `33188`).
        mode: u64,
    },
    /// `delete-node`: the `preimage` — what the delete removed.
    NodeDeleted {
        /// `preimage.kind`, verbatim: `file` at every measured state. Anything else is carried as a
        /// word, not read as a shape.
        node_kind: String,
        /// `preimage.content`.
        content: FileContent,
    },
    /// `replace-binary`: both blobs, by id and size. **Never bytes.**
    BinaryReplaced {
        /// `old`.
        old: BinaryBlob,
        /// `new`.
        new: BinaryBlob,
    },
    /// `change-perm`: both modes, as prikk's integers.
    PermChanged {
        /// `old_mode`.
        old_mode: u64,
        /// `new_mode`.
        new_mode: u64,
    },
    /// `rename-path`: the key that asserted it. The two paths are in [`PatchOperation::paths`].
    PathRenamed {
        /// `author_key_id` — prikk cannot construct a rename without one (RFC 028 F1).
        author_key_id: String,
        /// `author_key_ids`, in prikk's order.
        author_key_ids: Vec<String>,
    },
    /// A kind stikk does not render, carrying prikk's kind word (on [`PatchOperation::kind`]) and its
    /// raw fields (`[PatchOperation::raw]`). **Not an error, and not empty** (`C-T2c′`; RFC 035 §3
    /// rule 3): a report stikk half-understands still shows every operation prikk listed.
    Unrendered,
}

/// A file's content as prikk reports it inside an operation — a `create-file`'s `content`, or a
/// `delete-node`'s `preimage.content` (RFC 035 §3 rule 5).
///
/// **Three kinds, measured, and a fourth state for anything else.** RFC 035 §2's table named only
/// `text`, because the measurement that produced it created only text files. prikk's own type has three
/// variants (`ShowBlobContent`, `prikk-store/src/show.rs`), and both of the others turn up in an
/// ordinary repository:
/// - `binary` — **any binary file's create or delete** (measured at 0.51.0, queued and sealed alike:
///   `{"kind": "binary", "blob_id", "size"}`). It carries the same two facts `replace-binary` carries,
///   and is rendered the same way: identity and size, **never bytes**.
/// - `unavailable` — a blob the object store does not hold, which is **by design**: a text node's
///   pre-edit blob is deliberately absent, so a `delete-node` whose preimage an earlier patch of the
///   same block authored reports `{"kind": "unavailable", "blob_id"}` and no content (measured at
///   0.51.0; review v1 §2.4 cites prikk's `DC-65`). The blob id is still reported, and still shown.
///
/// A kind beyond those three is **named, not dropped and not shown as empty** (`C-T2c′`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContent {
    /// `{"kind": "text", "text": …}`.
    Text(String),
    /// `{"kind": "binary", "blob_id": …, "size": …}` — the content is bytes, so stikk shows what
    /// `replace-binary` shows and nothing more.
    Binary(BinaryBlob),
    /// `{"kind": "unavailable", "blob_id": …}` — prikk names the blob it does not hold. **Not an
    /// error**: it is how prikk reports a blob that was never stored, and the id is the one fact there
    /// is to report.
    Unavailable(String),
    /// Any other `kind`, carried verbatim. The raw view shows whatever fields came with it.
    Unrendered(String),
}

/// One binary blob, as `replace-binary` names it. stikk never renders a blob's bytes (`FR-030`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryBlob {
    /// `blob_id`, validated as an object id at the parse boundary.
    pub blob_id: String,
    /// `size`, in bytes, as prikk reports it.
    pub size: u64,
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

/// One sealed block in a ref's lineage, as `prikk log` reports it (design FR-011; RFC 006).
///
/// **Block granularity is `log`'s ceiling, not prikk's.** This doc comment used to say per-patch
/// content was beyond prikk's reach; `UD-09`'s content half retired at prikk 0.36 and
/// [`Prikk::show`] reads it (RFC 035) — what `log` itself still does not carry is a patch **id**
/// (hence `show <block-id>`, which is how Block detail enumerates a block's patches) or any
/// per-operation content. Patch **messages** are no longer beyond it for a messaged patch
/// (`messages`, RFC 015 F1); `log --format json` arrived at 0.39. No author or date either way, by
/// prikk's no-clock design.
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

/// The files present at one block, from `prikk tree --ref <block-id> --format json` at prikk ≥ 0.46
/// and from `prikk checkout --patch-plan` below it (design FR-032; RFC 006, widened by RFC 037
/// decision 1).
///
/// **It used to describe a ref's tip only**, because that is all `checkout --patch-plan` can replay
/// to. `tree` takes a block id and needs no replay, so from [`crate::TREE_FROM_MINOR`] this describes
/// whichever block was asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFiles {
    /// The block the files belong to: `target_block_id` from `tree`, or the replay's `target block:`
    /// on the prose band. **The reader checks it against the id that was asked for** (RFC 037
    /// decision 1) — a report about another block is stikk's environment error, never a wrong file
    /// list.
    pub target_block: String,
    /// Every file present at that block, in prikk's order.
    pub files: Vec<StateFile>,
    /// Total content bytes: the sum of the entries' `size` from `tree`, and prikk's own `result
    /// content bytes:` on the prose band.
    pub total_bytes: u64,
}

/// One file at a block, as [`StateFiles`] lists it.
///
/// **Two of the three fields are banded, and `None` is "not reported", never a zero or a guess**
/// (`C-T2c′`): `checkout --patch-plan` prints a bare `file: <path>` line and nothing else, so below
/// [`crate::TREE_FROM_MINOR`] stikk has a path and no more. A `0` mode would be a claim — `0o000`,
/// a file nobody may read — that prikk never made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFile {
    /// The repo-relative path, as prikk reports it. Repository text (`C-T2a`): render it inert, or
    /// as the identifier it is.
    pub path: String,
    /// prikk's `kind`, **verbatim** (`ER-02`) — `file` at every measured state, and prikk's own type
    /// is `#[non_exhaustive]` with that one variant today. **A kind stikk has not measured is kept,
    /// not refused** (RFC 037 §2): a reader that rejected an unknown word would take the whole file
    /// list away over a field it does not even render for the only value it has seen.
    ///
    /// `None` on the prose band, which reports no kind at all.
    pub kind: Option<String>,
    /// The full file mode as prikk's integer (`33188` for `100644`, `33261` for `100755`) — the same
    /// spelling `create-file` and `change-perm` carry in a patch.
    ///
    /// `None` on the prose band. **Also expect `33188` for a file committed on Windows**, which has
    /// no executable bit for prikk to record; history committed on Linux or macOS keeps `33261` on
    /// every platform (prikk's own 0.46 changelog, and why no test may assert a mode it did not
    /// create on the platform it runs on).
    pub mode: Option<u32>,
}

impl StateFile {
    /// A file prikk named and said nothing else about — **the prose band's shape**, where
    /// `checkout --patch-plan` prints a path and no attributes.
    ///
    /// Deliberately not a `From<&str>` impl: a conversion that costs nothing to write is a
    /// conversion a caller reaches for without deciding, and the decision here is the whole point —
    /// this says prikk reported no mode, which is a different claim from any mode at all.
    #[must_use]
    pub fn path_only(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: None,
            mode: None,
        }
    }

    /// An ordinary file as `tree` reports one: `kind: "file"`, and its full mode.
    #[must_use]
    pub fn file(path: impl Into<String>, mode: u32) -> Self {
        Self {
            path: path.into(),
            kind: Some("file".to_string()),
            mode: Some(mode),
        }
    }
}

/// Which two things [`Prikk::diff`] compares (RFC 038 decision 1).
///
/// **Three comparisons are expressible and no others**, because those are the three RFC 038 decision
/// 3 offers and the fourth shape prikk accepts — one named point against the worktree — belongs to
/// no entry point stikk has. A pair of `Option<&str>` would have let a caller ask for it by
/// accident, and the bare form is not "both sides omitted" in any sense a caller should have to
/// know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// `diff --from <a> --to <b>`: two points, each a **block id** or a **ref name**.
    Points {
        /// The left side, shown first and older by the caller's ordering.
        from: String,
        /// The right side.
        to: String,
    },
    /// **The bare `diff`**, with no `--from` and no `--to`: the worktree against what `commit`
    /// would author against, which is the sealed tip **plus any queued patches** — a state no block
    /// id names, and the reason this is a variant rather than a point someone could spell.
    WorktreeAgainstBaseline,
}

/// What a ref-writing command printed, or why stikk did not run one (RFC 044 decision 1).
///
/// **Two variants, for the reason every banded read has two**: below prikk 0.42 stikk does not
/// run the command at all, and *unavailable* is not *failed*. A caller must not read silence as
/// a refusal, nor as a success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefWrite {
    /// prikk ran and exited 0. **Its printed lines, verbatim**, for the result screen.
    ///
    /// **Never parsed for state** (RFC 044 §6 trap 2). These commands have no `--format json`
    /// (F1), and prose is not a schema: what the repository now holds is read back from
    /// `branch list` and `tag list`, which stikk already parses.
    Done(Vec<String>),
    /// Below [`crate::CURRENT_BRANCH_FROM_MINOR`]. stikk spawned nothing and says so in its own
    /// voice (`C-T4d`).
    Unavailable,
}

/// What [`Prikk::verify`] can say (RFC 040 decision 1).
///
/// The same two-variant shape [`QueueReport`], [`ShowReport`], [`StateReport`] and [`DiffReport`]
/// take: below the band stikk spawns nothing, and *unreported* is not *sound* (`C-T2c′`). A
/// repository stikk never asked about must never read as one prikk found no fault in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyReport {
    /// prikk >= [`crate::JSON_REPORTS_FROM_MINOR`]: the report, as prikk wrote it.
    Listed(Verification),
    /// Below the band. stikk spawns nothing and says so in its own voice (`C-T4d`).
    Unreported,
}

/// One `verify-report-v1`, as prikk wrote it.
///
/// **Measured at 0.51.0 and at 0.39.0** (RFC 040 F1, re-measured for Handoff A): the same four
/// keys and the same verdict and stage shape at both ends. Only the stage *list* differs -- 17 at
/// 0.51, 14 at 0.39, and the three extra ones are new names, not renamed old ones -- which is
/// exactly why nothing here counts or names a stage (decision 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    /// prikk's verdict, and every condition it currently fails.
    pub verdict: Verdict,
    /// prikk's session coverage, when the report carries it. `None` is *not reported*, never zero
    /// sessions: `active_sessions` arrived in prikk 0.27, below stikk's floor, but the reader does
    /// not depend on a field being present in every version it reads.
    pub active_sessions: Option<ActiveSessions>,
    /// Every stage prikk ran, **in prikk's order**.
    pub stages: Vec<VerifyStage>,
    /// prikk's own JSON for the whole report, for the raw view (`v`). Bounded by the frontend,
    /// never here.
    pub raw: String,
}

/// prikk's verdict: whether the repository is sound, and what it currently fails.
///
/// **`ok` and `failed_conditions` cannot disagree.** prikk derives both from one declaration (RFC
/// 040 F1, from prikk's own source), so a report where they disagree is a report stikk does not
/// understand -- the reader rejects it rather than believing one over the other (`T-T4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// prikk's own boolean.
    pub ok: bool,
    /// Every condition currently failing, in prikk's order. Empty exactly when `ok`.
    pub failed_conditions: Vec<FailedCondition>,
}

/// One failing condition, in prikk's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedCondition {
    /// prikk's own identifier for the condition (`item-failure`,
    /// `commit-witness-integrity`, ...). A name the repository's tooling chose, rendered as an
    /// identifier.
    pub id: String,
    /// prikk's sentence, verbatim (`ER-02`). Never parsed for anything (RFC 040 §7 trap 4).
    pub message: String,
}

/// How many active sessions prikk saw, and how many its verification covered.
///
/// At 0.51 `verified_count` is always `1`: only the default session is covered, by construction.
/// stikk reports both numbers as prikk gives them and claims nothing about the difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveSessions {
    /// Sessions prikk found.
    pub count: u64,
    /// Sessions its sealed-history verification covered.
    pub verified_count: u64,
}

/// One verification stage, in prikk's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyStage {
    /// prikk's stage name (`objects`, `wal-replay`, ...), rendered as an identifier.
    ///
    /// **Never matched against a list.** 0.39 reports 14 stages and 0.51 reports 17 (measured);
    /// stikk shows what prikk lists.
    pub stage: String,
    /// What prikk said about it.
    pub status: StageStatus,
}

/// A stage's outcome, with the field prikk pairs with it.
///
/// **A status stikk has not measured is kept as prikk's word** ([`Self::Other`]), not folded into
/// one of the four below: a new outcome shown as `evaluated` would be stikk inventing a clean
/// result (`T-T4`). Its own fields stay visible in the raw view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageStatus {
    /// prikk ran the stage and it raised nothing.
    Evaluated,
    /// prikk ran the stage and it failed, with prikk's own message.
    Failed {
        /// prikk's sentence, verbatim.
        message: String,
    },
    /// prikk did not run the stage, because of the stage it names.
    NotEvaluated {
        /// prikk's own word for what blocked it.
        blocked_by: String,
    },
    /// prikk stopped after the stage it names.
    Halted {
        /// prikk's own word for where it stopped.
        after: String,
    },
    /// A status stikk has not measured, kept exactly as prikk wrote it.
    Other(String),
}

/// What [`Prikk::diff`] can say (RFC 038 decision 1).
///
/// **Two variants, because below prikk 0.46 a comparison is not reported, not empty** (`C-T2c′`) —
/// the shape [`QueueReport`], [`ShowReport`] and [`StateReport`] all take, for the same reason. An
/// empty [`Diff`] means prikk compared the two points and found nothing; [`Self::Unreported`] means
/// stikk never asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffReport {
    /// prikk ≥ [`crate::DIFF_FROM_MINOR`]: the comparison, as prikk computed it.
    Listed(Diff),
    /// Below the band. stikk spawns nothing and says so in its own voice (`C-T4d`).
    Unreported,
}

/// One comparison, as `diff-report-v1` reports it. **stikk computes no part of this** (RFC 038 §7
/// trap 1): no renumbering, no re-hunking, no rename inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diff {
    /// The left side.
    pub from: DiffSide,
    /// The right side.
    pub to: DiffSide,
    /// Every differing path, in prikk's order.
    pub entries: Vec<DiffEntry>,
    /// Paths `commit` would refuse, with prikk's own refusal (`ER-02`). **The worktree form only**;
    /// always empty between two points, measured.
    pub unsupported_paths: Vec<UnsupportedPath>,
}

/// One side of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSide {
    /// `point`, **as the caller gave it** — a block id, a ref name, or the word `worktree`.
    pub point: String,
    /// What that point resolved to.
    pub target: DiffTarget,
    /// The queue folded into this side.
    pub queued: QueuedOnSide,
}

/// A side's `target_block_id`: an id, **or the word `worktree`, which is not one** (RFC 038 §7
/// trap 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffTarget {
    /// A 64-hex block id, validated at the parse boundary.
    Block(String),
    /// The literal `worktree`, accepted **only** where `point` is also `worktree`.
    Worktree,
}

/// What a side says about queued patches (RFC 038 §7 trap 2).
///
/// **The absence of `queued_patches` means "none queued" on exactly one side of one comparison** —
/// a bare `diff`'s left side, where prikk omits the key when the queue is empty. Anywhere else the
/// key never appears and its absence says nothing at all, so stikk says nothing. Collapsing the two
/// into `Option<u64>` would have made every other side read as "0 queued", which is a claim prikk
/// never made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedOnSide {
    /// Not a bare `diff`'s left side: prikk reports no queue here and stikk shows none.
    NotApplicable,
    /// A bare `diff`'s left side: how many patches are folded in, `0` where prikk omitted the key.
    Queued(u64),
}

/// One differing path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    /// The path on the right side (the new path, for a rename). Repository text (`C-T2a`).
    pub path: String,
    /// prikk's status word, **verbatim** (`ER-02`): `added`, `deleted`, `modified`, `renamed`,
    /// `mode`, `binary` at every measured version; prikk's schema also names `symlink`, which it
    /// says is *"not produced today"*. **A word stikk has not measured is kept and shown with its
    /// hunks**, never refused and never interpreted (RFC 038 decision 2).
    pub status: String,
    /// The left-side path, present exactly on a `renamed` entry.
    pub from_path: Option<String>,
    /// The left side's file metadata. **Absent on an `added` entry** — absence is read per status,
    /// never as a general rule.
    pub from: Option<DiffFile>,
    /// The right side's file metadata. **Absent on a `deleted` entry.**
    pub to: Option<DiffFile>,
    /// prikk's own word for whether the line diff reached the shortest edit script. `false` means
    /// the hunks are larger than they need to be and **still correct** — prikk's guide: *"You are
    /// always told."*
    pub minimal: bool,
    /// prikk's unified hunks, **each one whole and exactly as prikk wrote it**. stikk splits them
    /// into rows for display and changes no byte.
    pub hunks: Vec<String>,
    /// prikk's own fields for this entry, as JSON, for the raw view — the same thing
    /// [`PatchOperation::raw`] carries, built the same way and for the same reason.
    pub raw: String,
}

/// One side's file metadata, as `diff-report-v1` gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFile {
    /// `kind`, verbatim (`file` at every measured state).
    pub kind: String,
    /// `encoding`: `text` or `binary`, verbatim.
    pub encoding: String,
    /// The full file mode as prikk's integer (`33188`), the spelling `tree` and `create-file` use.
    pub mode: u32,
    /// The content's size in bytes.
    pub size: u64,
    /// `content_id`, **on a binary side only**. stikk shows it and never the bytes (`FR-030`).
    pub content_id: Option<String>,
}

/// A worktree path `commit` would refuse, with prikk's reason verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedPath {
    /// The path, as prikk reports it.
    pub path: String,
    /// `commit`'s own refusal — never stikk's paraphrase (`ER-02`).
    pub refusal: String,
}

/// What [`Prikk::block_state`] can say about a block's files (RFC 037 decision 2).
///
/// **Two variants, because below prikk 0.46 an older block's files are not reported, not absent**
/// (`C-T2c′`) — the shape [`QueueReport`] and [`ShowReport`] take, for the same reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateReport {
    /// The files at the block asked about: from `tree` at prikk ≥ 0.46, and from the tip's prose
    /// replay below it.
    Listed(StateFiles),
    /// **Below prikk 0.46, for a block that is not the ref's tip.** prikk replays only to a tip
    /// there, so there is no answer to be had and stikk spawns nothing to look for one. The view
    /// says so in stikk's own voice (`C-T4d`); an empty file list would say the block is empty.
    Unreported,
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

/// What [`Prikk::refs`] reads: the ref pointers, **and the current branch beside them** (RFC 037
/// decision 3).
///
/// # Why the branch is not in the list
///
/// RFC 032 decision 5 reads *"has this ref published history?"* as **membership in the ref list**.
/// prikk saw stikk's answer and deliberately kept `branches[]` unchanged, adding a separate
/// top-level field instead (prikk reply 020), because a `RefEntry` for a branch with no published
/// history would make every caller of that test say it was published. Keeping it a sibling of the
/// list, not a member, is what makes RFC 032's test stay true without being rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchList {
    /// Every ref pointer prikk listed — **unchanged by this increment**, in prikk's order.
    pub refs: Vec<RefEntry>,
    /// The branch the user is on, before its first seal.
    pub unpublished_current: UnpublishedBranch,
}

/// The current branch before its first seal (`branch-list-v1`'s `unpublished_current_branch`,
/// prikk ≥ [`crate::UNPUBLISHED_BRANCH_FROM_MINOR`]).
///
/// **Three states, and the reader never keys on the field's absence** (`C-T2c′`; RFC 037 §2).
/// Whether prikk can answer is decided **by the version**, because *absent* and `null` are two
/// different facts that a presence check reads as one — and prikk has already shipped a release
/// that started writing a field where it previously wrote none (0.47's queued-patch `path`), which
/// is the exact way a presence check turns a release note into a behaviour change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpublishedBranch {
    /// Below the band: this prikk does not report the field at all. **Never [`Self::None`]** — a
    /// prikk that cannot answer has not answered "there is none".
    NotReported,
    /// `null`: prikk answered, and the current branch is published (it is in
    /// [`BranchList::refs`] instead).
    None,
    /// The branch, validated through `RefName::parse` like every other ref name (`INV-9`).
    Branch(String),
}

impl UnpublishedBranch {
    /// The branch's name when prikk named one, else `None`.
    ///
    /// **Collapses the two "no branch" states on purpose, and only for a caller that genuinely
    /// does not distinguish them** — the picker, which draws a row or does not. A caller that says
    /// anything *about* the absence must match the three variants instead, because "prikk reports
    /// none" and "this prikk cannot say" are not one sentence.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Branch(name) => Some(name),
            Self::NotReported | Self::None => None,
        }
    }
}

/// One worktree path that differs from the replay baseline, from `prikk worktree-status` (design
/// FR-034; RFC 008). Path-level only — prikk reports *that* a tracked file's bytes differ, never the
/// content difference (the UD-09 ceiling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// The change kind as prikk names it (`modified`, `missing`, `untracked`, `unsupported-path`); kept
    /// as text so a future kind renders rather than breaks parsing — the reader takes any first word
    /// (RFC 027 F0).
    pub kind: String,
    /// The repo-relative worktree path.
    pub path: String,
    /// prikk's own one-line description of why the path is listed (preserved verbatim, NFR-I03). At
    /// prikk ≥ 0.39 this is JSON's `detail`, which carries no `[refused: …]` suffix — the verdict is in
    /// [`Self::authoring`] instead.
    pub note: String,
    /// Whether prikk's `commit` would author this entry, as prikk states it (RFC 027 decision 3).
    pub authoring: Authoring,
}

/// prikk's verdict on one worktree entry: would `commit` author it? (RFC 027 decision 3; prikk ≥ 0.39.)
///
/// **Three states, because the third is real.** Below prikk 0.39 no verdict is reported, and "no entry
/// is refused" is not knowable there — only unreported. `Unreported` is never `Authored` (`C-T2c′`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authoring {
    /// prikk reports that `commit` would author this entry.
    Authored,
    /// prikk reports that `commit` would refuse this entry, and the whole commit with it. The reason is
    /// prikk's, verbatim — the same string `commit` prints (measured at 0.41) — and is never classified.
    Refused(String),
    /// This prikk does not report a verdict (below 0.39).
    Unreported,
}

/// prikk's report that the active WAL holds queued patches for a **different** ref than the one asked
/// about (RFC 009 F4; RFC 027 F6).
///
/// **Two different things, kept apart** rather than one string meaning either (`C-T2c′`): prikk's own
/// sentence, which only the prose report carries, and prikk's queued ref, which is all the JSON report
/// carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuedElsewhere {
    /// prikk's warning sentence, verbatim (prose, below prikk 0.39). Never paraphrased (`ER-02`).
    Note(String),
    /// The ref the queued patches belong to, from `queued_elsewhere` (JSON, prikk ≥ 0.39), validated as
    /// a ref name at the parse boundary. A front-end words the warning itself and labels it as its own.
    Ref(String),
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
    /// How many entries prikk reports `commit` would refuse (RFC 027 decision 3): `Some(n)` from prikk
    /// ≥ 0.39's JSON report, checked there against the entries themselves; **`None` below 0.39, never
    /// `Some(0)`** — a report that cannot say is not a report of zero (`C-T2c′`).
    pub refused: Option<u64>,
    /// How many **declarations** prikk reports `commit` would refuse (RFC 034 decision 4): `Some(n)` at
    /// prikk ≥ 0.44, **`None` below it, never `Some(0)`** — the same posture as [`Self::refused`].
    ///
    /// **A different fact from [`Self::refused`], and neither implies the other** (RFC 034 §8 trap 1):
    /// a symlink destination measures `refused: Some(1)` with `refused_declarations: Some(0)`, and a
    /// declaration whose source is back measures the reverse — with `clean: true` besides, which is why
    /// commit's preview must read this before it decides there is nothing to commit.
    pub refused_declarations: Option<u64>,
    /// The per-path entries (the counts above summarize these).
    pub entries: Vec<WorktreeEntry>,
    /// Present when the active WAL holds queued patches for a **different** ref than the one asked
    /// about: paths listed "untracked" here may be committed-but-unsealed work (RFC 009 F4). `None`
    /// when prikk did not report it. Below 0.39 it is prikk's sentence verbatim; at ≥ 0.39 it is prikk's
    /// queued ref (RFC 027 F6) — see [`QueuedElsewhere`].
    pub queued_elsewhere: Option<QueuedElsewhere>,
    /// prikk's live rename declarations (RFC 030 amendment A1). Each is authored into the next `prikk
    /// commit` as a `RenamePath`, so a declaration made after a preview changes what `commit` authors
    /// **even when the listed entries do not move**. Read from JSON at prikk ≥ 0.39, and from the `live
    /// rename declarations:` prose section at 0.38.
    ///
    /// **Below 0.38 this is empty as a version fact**, not an unreported zero: `prikk mv` does not exist
    /// there, so nothing can be declared.
    pub declarations: Vec<RenameDeclaration>,
}

/// One rename recorded with `prikk mv`, which prikk authors into the next commit as a `RenamePath` (RFC
/// 030 amendment A1; prikk ≥ 0.38). Both paths are carried as prikk reports them; render them inert
/// wherever they are ever shown, as entries' paths are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDeclaration {
    /// The path the declaration renames from.
    pub old_path: String,
    /// The path it renames to.
    pub new_path: String,
    /// **prikk's own verdict on what the next `commit` will do with this declaration** (RFC 034
    /// decision 4), replacing the inference RFC 032 had to make because prikk would not say.
    ///
    /// **`None` below prikk 0.44 — including at 0.43, where the field exists and stikk does not read
    /// it.** 0.43's classifier is wrong in a measured state: a directory at the destination resolved
    /// `rename` while `commit` recorded a deletion (stikk letter 014, fixed in 0.44; re-measured here at
    /// 0.43.0 and 0.44.0). One band, `≥ 0.44`.
    pub resolution: Option<DeclarationResolution>,
    /// prikk's verbatim refusal for this declaration, present exactly when [`Self::resolution`] is
    /// [`DeclarationResolution::Refused`] (`ER-02`: prikk's words, never stikk's paraphrase).
    pub refusal: Option<String>,
    /// Whether the renamed file's content changed too, when prikk reports it.
    ///
    /// **`None` is unknown, never "unchanged"** (`C-T2c′`): below 0.44 prikk does not report it at all,
    /// and at ≥ 0.44 a destination that is not a regular file reports `null` (measured: a symlink
    /// destination at 0.44.0 and 0.46.0).
    pub content_changed: Option<bool>,
    /// Whether the renamed file's mode changed too, on the same terms as [`Self::content_changed`].
    pub mode_changed: Option<bool>,
}

/// What prikk's own classifier says the next `commit` will do with a rename declaration — the
/// `resolution` field of `worktree-status-report-v1` (prikk ≥ 0.43; **read by stikk from 0.44**, see
/// [`RenameDeclaration::resolution`]).
///
/// Every variant below was provoked against real 0.44.0 and 0.46.0 binaries and matched against what
/// `prikk commit` then printed. [`Self::Other`] keeps a word stikk does not model rather than dropping
/// it, exactly as [`WorktreeEntry::kind`] does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationResolution {
    /// `commit` authors a `rename-path`.
    Rename,
    /// The destination is not a file prikk will author: gone, or a directory. `commit` records a
    /// deletion of the source instead.
    Deletion,
    /// The destination is ignored (`.prikkignore`), so `commit` records a deletion of the source.
    DeletionIgnored,
    /// The source was never a tracked node, so there is nothing to rename: `commit` creates the
    /// destination and drops the declaration.
    NeverTracked,
    /// prikk will refuse the whole commit over this declaration; [`RenameDeclaration::refusal`] carries
    /// its words.
    Refused,
    /// A word stikk does not model, kept verbatim (render it inert).
    Other(String),
}

impl DeclarationResolution {
    /// prikk's own word for this resolution — including an unmodelled one, so a frontend shows what
    /// prikk said rather than a stikk paraphrase (`ER-02`).
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Rename => "rename",
            Self::Deletion => "deletion",
            Self::DeletionIgnored => "deletion-ignored",
            Self::NeverTracked => "never-tracked",
            Self::Refused => "refused",
            Self::Other(word) => word,
        }
    }

    fn from_label(label: &str) -> Self {
        match label {
            "rename" => Self::Rename,
            "deletion" => Self::Deletion,
            "deletion-ignored" => Self::DeletionIgnored,
            "never-tracked" => Self::NeverTracked,
            "refused" => Self::Refused,
            other => Self::Other(other.to_string()),
        }
    }
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

/// What one signing role's `key status` row says, beyond the part that decides capability.
///
/// The capability-deciding half lives on [`stikk_model::Readiness`], which stays `Copy` and threads
/// through every gate; this carries the display half. **The split is RFC 023 Handoff B's precedent**,
/// applied again: a `String` field on `Readiness` would remove `Copy` from a type with fifty-odd call
/// sites, for values most of them do not want.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoleDetail {
    /// prikk's own `reason` when the key is unusable — `missing`, `override-missing`,
    /// `readable-by-others (mode 0644)`, `undecodable`, or whatever it adds next. **Verbatim**
    /// (`ER-02`): four reasons do not become one "not ready".
    pub reason: Option<String>,
    /// The key id that would sign. **prikk always has one** — it defaults to the role's own name when
    /// `PRIKK_<ROLE>_KEY_ID` is unset — which is why reading this from `key status` rather than from
    /// the environment is what stops the confirmation card showing nothing on a default setup
    /// (RFC 026 Handoff B §0).
    pub key_id: Option<String>,
    /// `environment` when the operator chose the id, `default` when prikk did.
    pub key_id_source: Option<String>,
    /// The public key that would sign. Public, never secret — this is the value `C-S2` compares.
    pub public_key: Option<String>,
    /// `PRIKK_<ROLE>_SEED` is set on a prikk that no longer reads it (RFC 026 §4). Not from prikk:
    /// `key-status-v1` has no such field, and `stikk_prikk::env` establishes it by presence.
    pub stale_seed_variable: bool,
}

/// Signing readiness for both roles: the capability fold, plus what to say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessReport {
    /// The `Copy` fold every capability gate reads.
    pub readiness: stikk_model::Readiness,
    /// AUTHOR's display detail.
    pub author: RoleDetail,
    /// MAINTAINER's display detail.
    pub maintainer: RoleDetail,
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

    /// Read this session's signing readiness for both roles, against the repository at `repo`.
    ///
    /// **Takes a repository because `binding` needs one**: prikk computes it only when asked about a
    /// repository, and `binding` is the whole point (RFC 026 §2).
    ///
    /// Version-banded behind this one method, so nothing above the seam learns that readiness has
    /// versions: environment presence at ≤ 0.39, `Unverifiable` at 0.40 exactly, and
    /// `key status --format json` at ≥ 0.41.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as any other seam read.
    fn readiness(&self, repo: &Path) -> Result<ReadinessReport>;

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

    /// Read the files present at one block (design FR-032; category `read-state`; RFC 037
    /// decision 1).
    ///
    /// **Version-banded behind this one method**, in [`Prikk::queue`]'s and [`Prikk::show`]'s
    /// pattern:
    /// - **At prikk ≥ [`crate::TREE_FROM_MINOR`]** it runs `tree --ref <block_id> --format json`,
    ///   with the **block's own id, never `reff`** (RFC 037 §7 trap 1): a ref moves, and the block
    ///   the user opened does not. Every block answers, tip or not, and the reader checks the report
    ///   names the block it asked about.
    /// - **Below it**, only the tip has an answer: `checkout --patch-plan --ref <reff>`, which is
    ///   the one thing prikk can replay to there. For any other block it **spawns nothing** and
    ///   returns [`StateReport::Unreported`] — never an empty list (`C-T2c′`).
    ///
    /// `is_tip` is the caller's, not this method's to infer: it is the same fact
    /// `stikk_core::block_detail` is already given, and only above the seam is a block known to be a
    /// ref's tip.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`]: a `tree-listing-v1` report that breaks the measured shape — a
    /// wrong schema, a `target_block_id` that is not the id asked for, an entry missing a field or
    /// carrying an empty path — is stikk's environment error (UD-02). A genuine failure classifies
    /// as for [`Prikk::orientation`].
    fn block_state(
        &self,
        repo: &Path,
        reff: &str,
        block_id: &str,
        is_tip: bool,
    ) -> Result<StateReport>;

    /// List every branch ref pointer in the repository (design FR-014; category `read-history`).
    ///
    /// **Does not reliably exclude tags** — `prikk branch list --all`'s own implementation lists every
    /// ref pointer regardless of namespace, undocumented behavior stikk must not depend on either way
    /// (RFC 012 FR-014, discovered empirically: RFC 009 F3 had claimed a tag could never appear here,
    /// an inference from an untested case, not a checked one). `stikk_core::list_refs` is the
    /// merge-and-deduplicate caller that makes tag coverage correct regardless of what this returns;
    /// call [`Prikk::tags`] for the documented, stable way to list tags.
    ///
    /// **Returns the unpublished current branch beside the list, never inside it** (RFC 037
    /// decision 3): `BranchList { refs, unpublished_current }`. The list itself is unchanged, so
    /// RFC 032's *published* test — membership in it — means exactly what it meant before.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::orientation`]. At prikk ≥
    /// [`crate::UNPUBLISHED_BRANCH_FROM_MINOR`] the field is **required**: missing, or neither
    /// `null` nor a valid ref name, is stikk's environment error (UD-02).
    fn refs(&self, repo: &Path) -> Result<BranchList>;

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

    /// Read the active queue (design `FR-051`; category `read-history`; RFC 028 decision 1).
    ///
    /// **Version-banded behind this one method.** At prikk ≥ 0.39 it reads `status --format json` and
    /// lists every queued patch. Below 0.39 it spawns nothing new: it answers from the prose `status` read
    /// Orientation makes, as [`QueueReport::Unreported`] — never as an empty list.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`]: a JSON report that breaks the queue's rules is stikk's environment
    /// error (RFC 027 B, C1); a genuine failure classifies as for [`Prikk::orientation`].
    fn queue(&self, repo: &Path) -> Result<QueueReport>;

    /// Read what one patch, or every patch of one block, changed (design `FR-030`; category
    /// `read-history`; RFC 035 decision 1) — `show <id> --format json`.
    ///
    /// **`id` is the id that resolves the most, and the caller chooses it, not this method**: a
    /// **block** id for a sealed patch, whose report carries every patch in the block with their paths
    /// resolved; a **patch** id for a queued patch, which has no block to resolve against (RFC 035 F2).
    /// A bare *sealed* patch id resolves nothing at any measured version (A1 F7), which is why Block
    /// detail asks with the block's id and renders one patch from that one report.
    ///
    /// **Version-banded behind this one method**, in [`Prikk::queue`]'s pattern. At prikk ≥ 0.39
    /// (`reads_json`, where that number lives) it reads. **Below 0.39 it spawns nothing** and answers
    /// [`ShowReport::Unreported`] — never an empty list (`C-T2c′`).
    ///
    /// # Errors
    /// [`stikk_model::StikkError`]: a report that breaks `show-report-v1`'s measured shape is stikk's
    /// environment error (UD-02); a genuine failure — an id in no object store and no WAL — classifies
    /// as for [`Prikk::orientation`].
    fn show(&self, repo: &Path, id: &str) -> Result<ShowReport>;

    /// Compare two points, or the worktree with what `commit` would author against (design
    /// `FR-033`; category `read-state`; RFC 038 decision 1) — `diff [--from <a>] [--to <b>]
    /// --format json`.
    ///
    /// **prikk computes the difference and its hunks; stikk shows them** and computes no part of
    /// it (§7 trap 1). `prikk diff` is what retired RFC 021 F3's "arbitrary-point comparison is
    /// refused, not deferred": that finding predates the command.
    ///
    /// **Version-banded behind this one method**, in [`Prikk::queue`]'s pattern. At prikk ≥
    /// [`crate::DIFF_FROM_MINOR`] it reads. **Below it nothing is spawned** and the answer is
    /// [`DiffReport::Unreported`] — never an empty comparison, which would say the two points are
    /// the same (`C-T2c′`). **No prose `diff` is parsed, at any version.**
    ///
    /// # Errors
    /// [`stikk_model::StikkError`]: a report that breaks `diff-report-v1`'s measured shape is
    /// stikk's environment error (UD-02). A refusal — an absent ref, or a live `prikk mv` that
    /// `commit` would refuse — classifies as for [`Prikk::orientation`] and carries prikk's words
    /// verbatim (decision 6), never an empty comparison.
    fn diff(&self, repo: &Path, comparison: &Comparison) -> Result<DiffReport>;

    /// Whether the repository is sound, in prikk's own verdict (design `FR-100`; category
    /// `read-state`; RFC 040 decision 1) — `verify --format json`.
    ///
    /// **Version-banded behind this one method**, in [`Prikk::queue`]'s pattern. At prikk ≥
    /// [`crate::JSON_REPORTS_FROM_MINOR`] it reads. **Below it nothing is spawned** and the answer
    /// is [`VerifyReport::Unreported`] — never a sound verdict, which would say stikk had checked
    /// (`C-T2c′`). **No prose `verify` is parsed, at any version** (§7 trap 4): the prose is
    /// current CLI vocabulary, not a schema, and the counts it carries are the ones F3 found
    /// missing from the JSON.
    ///
    /// **This method reads `exit 1` as success, and it is the only one that may.** A damaged
    /// repository makes `verify` exit 1 **with a complete report** (RFC 040 F2, measured twice at
    /// 0.51): a byte flipped in a container, and a torn WAL record. Were this classified like
    /// every other non-zero exit, Verify would say *prikk refused* at exactly the moment prikk is
    /// reporting damage — the most wrong thing stikk could say (`T-T4`). The rule is narrow:
    /// **exit 0 or exit 1 whose stdout parses as `verify-report-v1` is a report**, whatever stderr
    /// says (stderr only repeats the condition); anything else classifies as it does today.
    ///
    /// **Verify changes nothing** (decision 4): no repair, no lock clearing, and it never runs on
    /// its own — it scans the whole repository, so only the reader starts it.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`]: a report that breaks `verify-report-v1`'s measured shape — or
    /// one whose `verdict.ok` disagrees with its own `failed_conditions`, which prikk derives from
    /// one declaration — is stikk's environment error (UD-02), never a guess at which half to
    /// believe. An exit 1 **without** a parseable report is prikk's refusal, verbatim, classified
    /// as for [`Prikk::orientation`]; exit 2 is stikk's bug, as everywhere.
    fn verify(&self, repo: &Path) -> Result<VerifyReport>;

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

    /// Publish a branch at an existing ref's tip (design `FR-070`; category `Publication`; RFC 044
    /// decision 1) — `branch create <name> --from <from_ref>`.
    ///
    /// **`--from` is always passed**, never omitted (§6 trap 1). prikk's default source is the
    /// *current branch*, which is a different question from the one the user answered: they chose
    /// a ref in the picker, and that is the ref this branch must start at. Omitting the flag would
    /// silently substitute prikk's answer for theirs.
    ///
    /// **`from_ref` is a ref name, never a block id.** prikk refuses a block id here —
    /// *`--from ref <id> does not resolve to a published ref`* (F2, measured) — and stikk does not imitate the
    /// missing feature by tagging the block first and branching from the tag.
    ///
    /// **Banded at [`crate::CURRENT_BRANCH_FROM_MINOR`]**: below it nothing is spawned and the
    /// answer is [`RefWrite::Unavailable`]. The command exists further back, but [`Prikk::close_branch`]
    /// cannot be offered safely without knowing the current branch, and the three travel together.
    ///
    /// # Errors
    /// [`stikk_model::StikkError`], classified as for [`Prikk::seal`]: a name that already exists,
    /// a missing `heads/` prefix and a `--from` that resolves to nothing are prikk's refusals,
    /// verbatim (`ER-02`). An absent MAINTAINER key is [`stikk_model::StikkError::NotReady`],
    /// raised by stikk **before prikk runs** (decision 2).
    fn create_branch(&self, repo: &Path, name: &str, from_ref: &str) -> Result<RefWrite>;

    /// Publish a tag at a block (design `FR-071`; category `Publication`; RFC 044 decision 1) —
    /// `tag create <name> --target <target>`, with `-m <message>` only when `message` is non-empty.
    ///
    /// **`target` may be a ref name or a bare block id**, unlike [`Prikk::create_branch`]'s
    /// `--from`: prikk accepts both here (F2, measured), which is what lets History tag the block
    /// a reader is looking at.
    ///
    /// **An empty message sends no `-m` at all**, rather than an empty one: a tag with no message
    /// and a tag with an empty message are not the same object, and only the user says which they
    /// meant.
    ///
    /// # Errors
    /// As [`Prikk::create_branch`].
    fn create_tag(&self, repo: &Path, name: &str, target: &str, message: &str) -> Result<RefWrite>;

    /// Close a branch (design `FR-070`; category `Publication`; RFC 044 decision 1) —
    /// `branch close <name>`.
    ///
    /// **Closing reclaims nothing**: prikk's own line says the pointer, its history and every
    /// object remain, and the branch is recoverable (F1). There is no delete.
    ///
    /// **This method will close the branch the repository is on, and the caller must not ask it
    /// to** (§6 trap 4). Measured at 0.52.0 with an empty WAL: the close succeeds, and the
    /// repository is then on a closed branch where a default-source `branch create` refuses
    /// (F3). prikk polices the *queued-work* case itself and refuses; it does not police this
    /// one. The frontend therefore does not offer it, and says why.
    ///
    /// # Errors
    /// As [`Prikk::create_branch`]. A branch that is already closed, one that does not exist, and
    /// one owning a non-empty active WAL are prikk's refusals, verbatim.
    fn close_branch(&self, repo: &Path, name: &str) -> Result<RefWrite>;
}

#[cfg(test)]
mod tests;
