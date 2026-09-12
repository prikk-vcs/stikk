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
use stikk_prikk::{Handshake, Prikk, WorktreeStatus};

/// prikk's per-entry verdict and its queued-elsewhere report, re-exported so a front-end reads the view
/// model without reaching past this crate to the seam (RFC 027 decision 3, F6).
pub use stikk_prikk::{Authoring, QueuedElsewhere};

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
}

/// Produce the Changes view for `reff` (design FR-034; RFC 008).
///
/// # Errors
/// [`StikkError::NotReady`] when the prikk version predates the `worktree-status` fix (< 0.28), so the
/// caller shows guidance rather than the broken command's output (FR-034/UD-03). Otherwise propagates
/// any [`StikkError`] the seam raises (a bad ref, an unrecognized shape).
pub fn changes_view(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<ChangesView> {
    let handshake = prikk.handshake()?;
    ensure_worktree_status_supported(&handshake)?;
    let status = prikk.worktree_status(repo, reff)?;
    Ok(from_status(status))
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
pub(crate) fn from_status(status: WorktreeStatus) -> ChangesView {
    let entries = status
        .entries
        .into_iter()
        .map(|entry| ChangeEntry {
            kind: ChangeKind::from_label(&entry.kind),
            path: entry.path,
            note: entry.note,
            authoring: entry.authoring,
        })
        .collect();
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
    }
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
