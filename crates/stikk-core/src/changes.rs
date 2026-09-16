//! Worktree-vs-baseline (Changes) operation (design FR-034; RFC 008; RFC 009 F4).
//!
//! `changes_view` drives prikk's `worktree-status` — verified fixed as of prikk 0.28 (RFC 008; the
//! audit's UD-03 defect was 0.27.x). The operation is **version-gated**: below 0.28 it returns
//! stikk-authored guidance rather than invoking the pre-fix command, honoring FR-034's "must not
//! present the broken command's error to users." Everything here is read-only and path-level — prikk
//! reports *that* a file's bytes differ, never the content difference (the UD-09 ceiling), so no
//! per-file diff is fabricated (threat T-T4).
//!
//! [`ChangesView::queued_elsewhere`] carries prikk's queued-elsewhere report when the active WAL holds
//! queued work for a **different** ref (RFC 009 F4): shipped stikk silently dropped this warning and
//! then added its own contradicting "a commit still captures them" banner, which is exactly the
//! confident-but-wrong picture (`T-T4`) the threat model names as this project's worst failure. Below
//! prikk 0.39 it is prikk's sentence verbatim; at ≥ 0.39 prikk reports only the queued ref, and
//! [`queued_elsewhere_clauses`] words the warning as stikk's own, keeping every claim prikk's sentence
//! makes (RFC 027 F6). This operation transports the field; the frontend decides how to render it.
//!
//! Each entry carries prikk's `commit` verdict ([`Authoring`], RFC 027 decision 3) — `Unreported` below
//! prikk 0.39, never read as "authored".

use std::path::Path;

use stikk_model::{Result, StikkError};
use stikk_prikk::{Handshake, Orientation, Prikk, WorktreeStatus};

/// prikk's per-entry verdict and its queued-elsewhere report, re-exported so a front-end reads the view
/// model without reaching past this crate to the seam (RFC 027 decision 3, F6).
pub use stikk_prikk::{Authoring, QueuedElsewhere, RenameDeclaration};

/// The lowest prikk version where `worktree-status` is reliable (RFC 008; UD-03 fixed at 0.28).
/// `pub(crate)`: the commit preview (RFC 014 §3) is derived from the same read and needs the same
/// gate — a preview built atop known-unreliable worktree-status would inherit UD-03's defect.
pub(crate) const WORKTREE_STATUS_MIN: (u32, u32, u32) = (0, 28, 0);

/// A change kind, mapped from prikk's per-path label so the view can group and style it. `Other`
/// preserves any future kind rather than dropping it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    /// A tracked file whose bytes differ from the baseline.
    Modified,
    /// A tracked file absent from the worktree.
    Missing,
    /// A worktree file not in the baseline.
    Untracked,
    /// A path prikk cannot represent against the baseline (prikk's `unsupported-path`).
    Unsupported,
    /// A kind stikk does not yet model (kept verbatim, never dropped).
    Other(String),
}

impl ChangeKind {
    fn from_label(label: &str) -> Self {
        match label {
            "modified" => Self::Modified,
            "missing" => Self::Missing,
            "untracked" => Self::Untracked,
            "unsupported-path" => Self::Unsupported,
            other => Self::Other(other.to_string()),
        }
    }

    /// prikk's own word for this kind — the label `worktree-status` printed, including an unmodelled
    /// kind's word, so a front-end can show what prikk said rather than a stikk paraphrase (`ER-02`).
    /// Render it inert: an `Other` word is prikk's text.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Modified => "modified",
            Self::Missing => "missing",
            Self::Untracked => "untracked",
            Self::Unsupported => "unsupported-path",
            Self::Other(word) => word,
        }
    }

    /// True for the untracked kind — the group the UD-08 display filter hides.
    #[must_use]
    pub fn is_untracked(&self) -> bool {
        matches!(self, Self::Untracked)
    }
}

/// One changed path, for the Changes view (design FR-034).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeEntry {
    /// The change kind.
    pub kind: ChangeKind,
    /// The repo-relative worktree path.
    pub path: String,
    /// prikk's own one-line description (preserved verbatim).
    pub note: String,
    /// Whether prikk's `commit` would author this entry — authored, refused with prikk's reason, or
    /// unreported below prikk 0.39 (RFC 027 decision 3). Orthogonal to [`Self::kind`].
    pub authoring: Authoring,
    /// Which half of a **paired** declared rename this entry is (RFC 032 decision 1), or `None`. Set only
    /// when prikk lists both halves beside the declaration; a declaration alone never marks an entry.
    pub rename: Option<RenameHalf>,
}

/// Which half of a paired declared rename an entry is (RFC 032 decision 1; Q1 ruled (a): prikk's two
/// rows stay, each annotated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameHalf {
    /// The `missing` entry at the declaration's source; carries the destination it is renamed to.
    Source {
        /// The declaration's `new_path`.
        new_path: String,
    },
    /// The `untracked` entry at the declaration's destination; carries the source it is renamed from.
    Destination {
        /// The declaration's `old_path`.
        old_path: String,
    },
}

impl RenameHalf {
    /// stikk's annotation, rendered after prikk's note (RFC 032 handoff §5). Carries repository text:
    /// render it inert.
    #[must_use]
    pub fn annotation(&self) -> String {
        match self {
            Self::Source { new_path } => format!("· declared rename → {new_path}"),
            Self::Destination { old_path } => format!("· declared rename ← {old_path}"),
        }
    }

    /// True for the destination half, which the untracked filter never hides (RFC 032 decision 3).
    #[must_use]
    pub fn is_destination(&self) -> bool {
        matches!(self, Self::Destination { .. })
    }
}

/// What prikk's report shows about one rename declaration (RFC 032 decisions 1 and 2), classified **in
/// this order**, from the worktree report alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationState {
    /// No `missing` entry at the source: the source is present again (rows 2 and 5). prikk's `commit`
    /// refuses. Checked first, because row 5 lists no entries at all.
    SourcePresent {
        /// Whether an `untracked` entry at the destination is listed. When it is not (row 5's state),
        /// `prikk mv {new} {old}` was measured at 0.42.0 to drop the declaration (RFC 032 A3).
        destination_listed: bool,
    },
    /// A `missing` entry at the source, but no `untracked` entry at the destination (rows 1 and 3):
    /// prikk authors a deletion, not a rename.
    DestinationAbsent,
    /// Both halves listed (rows A, B and C): prikk authors a rename.
    Paired,
}

/// One of prikk's rename declarations with what the report shows about it (RFC 032 handoff §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredRename {
    /// The path the declaration renames from.
    pub old_path: String,
    /// The path it renames to.
    pub new_path: String,
    /// Paired, destination absent, or source present again.
    pub state: DeclarationState,
}

impl DeclaredRename {
    /// stikk's sentence for a declaration prikk will not author as a rename, or `None` when paired (RFC
    /// 032 decision 2; A3). Carries repository text: render it inert.
    #[must_use]
    pub fn notice(&self) -> Option<String> {
        let (old, new) = (&self.old_path, &self.new_path);
        match self.state {
            DeclarationState::Paired => None,
            DeclarationState::DestinationAbsent => Some(format!(
                "declared rename {old} → {new}: {new} is not a file in the worktree, so prikk will not author it as a rename"
            )),
            DeclarationState::SourcePresent { destination_listed } => {
                let sentence = format!(
                    "declared rename {old} → {new}: {old} is present again, and prikk refuses to commit until the declaration is resolved"
                );
                // A3: offered only where measured — the destination gone. With both copies present
                // (row 2) prikk refuses the move, and stikk does not advise moving a user's files.
                Some(if destination_listed {
                    sentence
                } else {
                    format!("{sentence}; in a terminal, prikk mv {new} {old} drops it")
                })
            }
        }
    }
}

/// Said once under the entries when any declared rename is paired and prikk refuses nothing (RFC 032
/// decision 1; F3; amendment A7). Emitted through [`ChangesView::content_note`], never directly.
pub const RENAME_CONTENT_NOTE: &str = "a declared rename is authored as a rename; prikk does not report whether its content also changed";

/// Said once under a confirmation's counts when any declared rename is paired (RFC 032 decision 4).
pub const RENAMES_ALSO_COUNTED: &str =
    "each rename is also counted above as one missing and one untracked path";

/// Whether a ref has published history, and when not, what the queue holds for it (RFC 032 decision 5,
/// amendment A2). **Never inside [`ChangesView`]**: commit's confirmation re-reads the worktree and
/// requires an equal view, and this is not read from the worktree report (handoff §4, trap 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefHistory {
    /// The ref appears in prikk's `refs()`.
    Published,
    /// The ref does not appear in prikk's `refs()`.
    Unpublished(UnpublishedQueue),
}

/// What the active queue holds for an unpublished ref, from Orientation's `queued_patches` and
/// `queued_target` alone (RFC 032 A2). **Nothing is inferred from `tracked`.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpublishedQueue {
    /// No queued patch.
    Empty,
    /// `n` queued patches, and prikk reports this ref as their target.
    ForThisRef(u64),
    /// Queued patches for another ref, or a count with no target reported.
    NotThisRef,
}

impl RefHistory {
    /// Decide from prikk's facts: `refs()` membership, then the queue (RFC 032 A2).
    #[must_use]
    pub fn from_facts(
        reff: &str,
        refs: &[stikk_prikk::RefEntry],
        queued_patches: u64,
        queued_target: Option<&str>,
    ) -> Self {
        if refs.iter().any(|entry| entry.name == reff) {
            return Self::Published;
        }
        Self::Unpublished(match (queued_patches, queued_target) {
            (0, _) => UnpublishedQueue::Empty,
            (n, Some(target)) if target == reff => UnpublishedQueue::ForThisRef(n),
            _ => UnpublishedQueue::NotThisRef,
        })
    }

    /// The Changes headline in place of `N change(s) against baseline` or `clean against baseline`, or
    /// `None` when the ref is published (RFC 032 A2). Carries a ref name: render it inert.
    #[must_use]
    pub fn changes_headline(&self, reff: &str, clean: bool) -> Option<String> {
        let Self::Unpublished(queue) = self else {
            return None;
        };
        Some(match (queue, clean) {
            (UnpublishedQueue::Empty, false) => format!(
                "{reff} has no published history — every file is listed as untracked, and a commit would be its first"
            ),
            (UnpublishedQueue::Empty, true) => {
                format!("{reff} has no published history, and nothing in the worktree to commit")
            }
            (UnpublishedQueue::ForThisRef(n), false) => format!(
                "{reff} has no published history yet — its {n} queued patch(es) are the baseline here, and nothing is sealed"
            ),
            (UnpublishedQueue::ForThisRef(n), true) => format!(
                "{reff} has no published history yet — nothing in the worktree beyond its {n} queued patch(es), and nothing is sealed"
            ),
            (UnpublishedQueue::NotThisRef, _) => format!("{reff} has no published history"),
        })
    }

    /// Commit's card line under the targets, or `None` when the ref is published (RFC 032 A2). A clean
    /// worktree never reaches the card. Carries a ref name: render it inert.
    #[must_use]
    pub fn card_line(&self, reff: &str) -> Option<String> {
        let Self::Unpublished(queue) = self else {
            return None;
        };
        Some(match queue {
            UnpublishedQueue::Empty => {
                format!("{reff} has no published history: this would be its first commit")
            }
            UnpublishedQueue::ForThisRef(n) => format!(
                "{reff} has no published history: this adds to its {n} queued patch(es), and nothing is sealed until the queue is sealed"
            ),
            UnpublishedQueue::NotThisRef => format!("{reff} has no published history"),
        })
    }
}

/// What the Changes operation returns (RFC 032 handoff §4): the view, and **beside it**, never inside it,
/// whether the ref has published history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesRead {
    /// The worktree-vs-baseline view, a pure function of prikk's worktree report.
    pub view: ChangesView,
    /// Whether the ref has published history, and what the queue holds for it.
    pub history: RefHistory,
}

/// Worktree-vs-baseline status for the focused ref (design FR-034; RFC 008). Path-level: the counts
/// and the changed paths, never a per-file content diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesView {
    /// The ref whose replay baseline the worktree was compared against.
    pub reff: String,
    /// True when the worktree matches the baseline.
    pub clean: bool,
    /// Tracked files in the baseline.
    pub tracked: u64,
    /// Files unchanged from the baseline.
    pub unchanged: u64,
    /// Tracked files absent from the worktree.
    pub missing: u64,
    /// Tracked files whose bytes differ.
    pub modified: u64,
    /// Worktree files not in the baseline.
    pub untracked: u64,
    /// Paths prikk cannot represent.
    pub unsupported: u64,
    /// Entries prikk reports `commit` would refuse: `Some(n)` at prikk ≥ 0.39, **`None` below it —
    /// never `Some(0)`** (RFC 027 decision 3, `C-T2c′`).
    pub refused: Option<u64>,
    /// The changed paths (the counts summarize these).
    pub entries: Vec<ChangeEntry>,
    /// Present when the active WAL holds queued patches for a **different** ref than the one asked
    /// about — paths listed "untracked" above may be committed-but-unsealed work (RFC 009 F4). Below
    /// prikk 0.39 it is prikk's sentence, verbatim and never paraphrased (`ER-02`); at ≥ 0.39 it is
    /// prikk's queued ref, which a front-end words with [`queued_elsewhere_clauses`] as its own (RFC 027
    /// F6). Either way, while this is present the UD-08 untracked filter's "a commit still captures
    /// them" claim is suppressed and replaced by a pointer to the warning, because the two would
    /// otherwise contradict each other and prikk's fact is the true one (RFC 009 decision 3).
    pub queued_elsewhere: Option<QueuedElsewhere>,
    /// prikk's live rename declarations, each authored into the next commit as a `RenamePath` (RFC 030
    /// amendment A1). **Carried so that equality covers them**: commit's confirmation compares the view
    /// it re-reads against the one it previewed, and a declaration made in between leaves every entry
    /// unchanged. Empty below prikk 0.38 as a version fact. **Nothing renders them yet** — showing a
    /// declaration in the preview is roadmap item 11.
    pub declarations: Vec<RenameDeclaration>,
    /// Each declaration, classified from this report alone (RFC 032 handoff §3), in `declarations`' order.
    /// Derived from the report, so commit's preview and its re-read still build equal views.
    pub declared_renames: Vec<DeclaredRename>,
    /// The number of paired declared renames (RFC 032 decision 1).
    pub renames: u64,
}

impl ChangesView {
    /// [`RENAME_CONTENT_NOTE`], when this report has a paired declared rename to say it about — and
    /// **`None` while prikk reports that `commit` would refuse something** (RFC 032 amendment A7).
    ///
    /// **Why a refusal withholds it.** The sentence promises an outcome: *authored as a rename*. One refused
    /// entry refuses the whole commit (RFC 027 F2), so prikk would author **nothing** — and a paired rename
    /// whose own destination is the refused entry is exactly the measured case (a symlink at `{new}`). A
    /// `refused —` marker elsewhere on the screen does not make the promise true (`C-T2b`).
    ///
    /// **prikk's verdict only.** Below prikk 0.39 [`Self::refused`] is `None`: the verdict is **unreported,
    /// never a zero** (`C-T2c′`), so the sentence stays exactly as it was. At prikk 0.43 this becomes
    /// `resolution`, prikk's own word for what each declaration will do.
    #[must_use]
    pub fn content_note(&self) -> Option<&'static str> {
        if self.renames == 0 || self.refused.is_some_and(|refused| refused >= 1) {
            return None;
        }
        Some(RENAME_CONTENT_NOTE)
    }
}

/// Produce the Changes view for `reff` (design FR-034; RFC 008), with whether `reff` has published history
/// beside it (RFC 032 decision 5, A2).
///
/// **Three reads, and a race accepted, not hidden:** `worktree-status`, `refs()` and Orientation. A
/// terminal `prikk commit` or `prikk seal` landing between them can show the no-published-history words
/// once for a state that has just moved. RFC 031's check notices and refreshes within five seconds, and
/// RFC 030's change token stops a confirmation armed on the old state.
///
/// # Errors
/// [`StikkError::NotReady`] when the prikk version predates the `worktree-status` fix (< 0.28), so the
/// caller shows guidance rather than the broken command's output (FR-034/UD-03). Otherwise propagates
/// any [`StikkError`] the seam raises (a bad ref, an unrecognized shape), including a failed `refs()` or
/// Orientation read: publication is never guessed.
pub fn changes_view(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<ChangesRead> {
    let handshake = prikk.handshake()?;
    ensure_worktree_status_supported(&handshake)?;
    let status = prikk.worktree_status(repo, reff)?;
    let view = from_status(status);
    let history = ref_history(prikk, repo, reff)?;
    Ok(ChangesRead { view, history })
}

/// Read whether `reff` has published history (RFC 032 decision 5, A2): `refs()` membership, closed and
/// received refs included, then Orientation's queue facts.
///
/// # Errors
/// Propagates a failed `refs()` or Orientation read.
pub(crate) fn ref_history(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<RefHistory> {
    let refs = prikk.refs(repo)?;
    let orientation = prikk.orientation(repo)?;
    Ok(history_from(reff, &refs, &orientation))
}

pub(crate) fn history_from(
    reff: &str,
    refs: &[stikk_prikk::RefEntry],
    orientation: &Orientation,
) -> RefHistory {
    RefHistory::from_facts(
        reff,
        refs,
        orientation.queued_patches,
        orientation.queued_target.as_deref(),
    )
}

/// Refuse with stikk-authored guidance, rather than invoke the pre-fix command, below prikk 0.28
/// (RFC 008; UD-03). Shared with the commit preview (RFC 014 §3), which is derived from the same
/// `worktree-status` read and would otherwise inherit the same unreliability silently.
///
/// # Errors
/// [`StikkError::NotReady`] below the floor.
pub(crate) fn ensure_worktree_status_supported(handshake: &Handshake) -> Result<()> {
    let version = handshake.version;
    if (version.major, version.minor, version.patch) < WORKTREE_STATUS_MIN {
        return Err(StikkError::NotReady {
            detail: format!(
                "Worktree review needs prikk ≥ 0.28 — this prikk is {version}. Before 0.28, \
                 worktree-status is unreliable (audit UD-03); the rest of stikk works. Update prikk \
                 to review changes.",
            ),
        });
    }
    Ok(())
}

/// Map the seam's [`WorktreeStatus`] into the view-model (labels → [`ChangeKind`]).
///
/// **Pure: nothing but the report goes in** (RFC 032 handoff §3), so commit's preview and its re-read at
/// Enter build equal views from equal reports (RFC 030).
pub(crate) fn from_status(status: WorktreeStatus) -> ChangesView {
    let mut entries: Vec<ChangeEntry> = status
        .entries
        .into_iter()
        .map(|entry| ChangeEntry {
            kind: ChangeKind::from_label(&entry.kind),
            path: entry.path,
            note: entry.note,
            authoring: entry.authoring,
            rename: None,
        })
        .collect();
    let declared_renames = classify(&status.declarations, &mut entries);
    let renames = declared_renames
        .iter()
        .filter(|d| d.state == DeclarationState::Paired)
        .count() as u64;
    ChangesView {
        reff: status.reff,
        clean: status.clean,
        tracked: status.tracked,
        unchanged: status.unchanged,
        missing: status.missing,
        modified: status.modified,
        untracked: status.untracked,
        unsupported: status.unsupported,
        refused: status.refused,
        entries,
        queued_elsewhere: status.queued_elsewhere,
        declarations: status.declarations,
        declared_renames,
        renames,
    }
}

/// RFC 032 handoff §3, **in this order**: source present again, then destination absent, then paired.
/// Marks the two entries of each pair, and nothing else.
fn classify(
    declarations: &[RenameDeclaration],
    entries: &mut [ChangeEntry],
) -> Vec<DeclaredRename> {
    let listed = |entries: &[ChangeEntry], kind: ChangeKind, path: &str| {
        entries
            .iter()
            .position(|e| e.kind == kind && e.path == path)
    };
    declarations
        .iter()
        .map(|declaration| {
            let (old, new) = (&declaration.old_path, &declaration.new_path);
            let source = listed(entries, ChangeKind::Missing, old);
            let destination = listed(entries, ChangeKind::Untracked, new);
            let state = match (source, destination) {
                (None, destination) => DeclarationState::SourcePresent {
                    destination_listed: destination.is_some(),
                },
                (Some(_), None) => DeclarationState::DestinationAbsent,
                (Some(s), Some(d)) => {
                    if let Some(entry) = entries.get_mut(s) {
                        entry.rename = Some(RenameHalf::Source {
                            new_path: new.clone(),
                        });
                    }
                    if let Some(entry) = entries.get_mut(d) {
                        entry.rename = Some(RenameHalf::Destination {
                            old_path: old.clone(),
                        });
                    }
                    DeclarationState::Paired
                }
            };
            DeclaredRename {
                old_path: old.clone(),
                new_path: new.clone(),
                state,
            }
        })
        .collect()
}

/// **stikk's wording of prikk's queued-elsewhere fact**, for prikk ≥ 0.39, where the report carries only
/// the queued ref (RFC 027 F6, ruled by the architect).
///
/// One sentence per safety claim in prikk's own sentence, **in its order, adding none**:
///
/// | # | prikk says (prose, 0.28 and 0.41 alike) | stikk says |
/// |---|---|---|
/// | 1 | the active WAL has queued (unsealed) patches for `<queued>`, not `<focused>` | the queue holds unsealed patches for `<queued>`, not for `<focused>` |
/// | 2 | that is real, committed work, not shown above | that is real, committed work, not shown here |
/// | 3 | any "untracked" file here may be exactly that work seen from this ref's own baseline | an untracked entry here may be exactly that work, seen from this ref's own baseline |
/// | 4 | so do not delete based on this report alone | do not delete anything on the strength of this view alone |
///
/// These are **stikk's words**, and a front-end must render them as stikk's — never inside a
/// "prikk reported" quote band, which stays reserved for prikk's own text (`C-T2b`). The clause-by-clause
/// test in `changes/tests.rs` holds each sentence to prikk's captured one, so an edit that drops a claim
/// fails by name.
#[must_use]
pub fn queued_elsewhere_clauses(queued_ref: &str, focused_ref: &str) -> [String; 4] {
    [
        format!("The active queue holds unsealed patches for {queued_ref}, not for {focused_ref}."),
        "That is real, committed work, and it is not shown here.".to_string(),
        "An untracked entry here may be exactly that work, seen from this ref's own baseline."
            .to_string(),
        "Do not delete anything on the strength of this view alone.".to_string(),
    ]
}

#[cfg(test)]
mod tests;
