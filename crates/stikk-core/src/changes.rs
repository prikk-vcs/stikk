//! Worktree-vs-baseline (Changes) operation (design FR-034; RFC 008; RFC 009 F4).
//!
//! `changes_view` drives prikk's `worktree-status` — verified fixed as of prikk 0.28 (RFC 008; the
//! audit's UD-03 defect was 0.27.x). The operation is **version-gated**: below 0.28 it returns
//! stikk-authored guidance rather than invoking the pre-fix command, honoring FR-034's "must not
//! present the broken command's error to users." Everything here is read-only and path-level — prikk
//! reports *that* a file's bytes differ, never the content difference (the UD-09 ceiling), so no
//! per-file diff is fabricated (threat T-T4).
//!
//! [`ChangesView::queued_elsewhere`] carries prikk's own warning verbatim when the active WAL holds
//! queued work for a **different** ref (RFC 009 F4): shipped stikk silently dropped this warning and
//! then added its own contradicting "a commit still captures them" banner, which is exactly the
//! confident-but-wrong picture (`T-T4`) the threat model names as this project's worst failure. This
//! operation only transports the field; the frontend decides how to render it.

use std::path::Path;

use stikk_model::{Result, StikkError};
use stikk_prikk::{Prikk, WorktreeStatus};

/// The lowest prikk version where `worktree-status` is reliable (RFC 008; UD-03 fixed at 0.28).
const WORKTREE_STATUS_MIN: (u32, u32, u32) = (0, 28, 0);

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
    /// A path prikk cannot represent against the baseline.
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
            "unsupported" => Self::Unsupported,
            other => Self::Other(other.to_string()),
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
    /// The changed paths (the counts summarize these).
    pub entries: Vec<ChangeEntry>,
    /// prikk's own warning, verbatim, when the active WAL holds queued patches for a **different** ref
    /// than the one asked about — paths listed "untracked" above may be committed-but-unsealed work
    /// (RFC 009 F4). `None` when prikk did not emit it. Never paraphrased (ER-02): while this is
    /// present, the UD-08 untracked filter's "a commit still captures them" claim is suppressed and
    /// replaced by a pointer to this warning, because the two would otherwise contradict each other and
    /// prikk's is the true one (RFC 009 decision 3).
    pub queued_elsewhere: Option<String>,
}

/// Produce the Changes view for `reff` (design FR-034; RFC 008).
///
/// # Errors
/// [`StikkError::NotReady`] when the prikk version predates the `worktree-status` fix (< 0.28), so the
/// caller shows guidance rather than the broken command's output (FR-034/UD-03). Otherwise propagates
/// any [`StikkError`] the seam raises (a bad ref, an unrecognized shape).
pub fn changes_view(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<ChangesView> {
    let handshake = prikk.handshake()?;
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
    let status = prikk.worktree_status(repo, reff)?;
    Ok(from_status(status))
}

/// Map the seam's [`WorktreeStatus`] into the view-model (labels → [`ChangeKind`]).
fn from_status(status: WorktreeStatus) -> ChangesView {
    let entries = status
        .entries
        .into_iter()
        .map(|entry| ChangeEntry {
            kind: ChangeKind::from_label(&entry.kind),
            path: entry.path,
            note: entry.note,
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
        entries,
        queued_elsewhere: status.queued_elsewhere,
    }
}

#[cfg(test)]
mod tests;
