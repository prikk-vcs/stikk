//! The commit operation (design `FR-050`/`FL-05`; RFC 014) — **the first operation that writes**.
//!
//! `FL-05`'s order is normative and built exactly as specified: a message step happens before any
//! confirmation exists to restate (`crate::confirm`'s [`Intent`]/[`ConfirmationSummary`]), the preview
//! re-reads the worktree and the queue fresh (RFC 013's ruling on this increment: reusing an on-screen
//! [`ChangesView`] would let the change token assert freshness the view does not have), and execution
//! is the one and only seam call that writes ([`stikk_prikk::Prikk::commit`]).
//!
//! Two refusals prikk would otherwise give are **prevented**, not merely classified (RFC 014 F2/F6,
//! decisions 1/5b): a cross-ref commit (the focused ref is not the active WAL's queue target) and a
//! clean-worktree commit (nothing to author) both make commit **unavailable with a reason**
//! ([`CommitPreviewOutcome::Blocked`], `C-T4d`) before any [`PreviewToken`] exists — never offered and
//! then refused. Both reads (`orientation`, `worktree-status`) happen inside [`preview`]'s `compute`
//! closure, i.e. **after** the change token is stamped, so the token and the blocking decision describe
//! the same instant.

use std::path::Path;

use stikk_model::{Capability, RequestCategory, Result};
use stikk_prikk::{CommitResult, Prikk};

use crate::changes::{self, ChangesView};
use crate::confirm::{self, ConfirmationSummary, Evidence, Intent, Outcome, PreviewToken};

/// stikk's own short name for this operation — used verbatim in [`stikk_model::StikkError::Stale`]/
/// [`stikk_model::StikkError::Declined`] messages (never prikk's words) and as the palette/`Intent` operation id.
pub const COMMIT_OPERATION: &str = "commit";

/// The commit preview (design `FL-05` step 3; RFC 014 F3) — the Changes view, **labelled as stikk's own
/// derivation**, since prikk has no commit dry-run. Well-founded, not a guess: prikk's own guide states
/// `worktree-status` "answers 'what would the next commit author?', not merely 'what differs from the
/// last seal.'" — the same replay baseline `commit` authors against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPreview {
    /// The whole-worktree capture this commit would author (`UD-06`): there is no staging, so this is
    /// every difference against the baseline, not a selection.
    pub changes: ChangesView,
    /// prikk's own active-patch threshold warning, verbatim, when the queue is near or at a limit
    /// (`C-D2a`; RFC 014 F5) — surfaced here, in the preview, before any refusal.
    pub active_patch_notice: Option<String>,
}

/// What [`commit_preview`] found (RFC 014 decisions 1/5b). `Blocked` carries a reason in stikk's own
/// words — never routed through [`stikk_model::StikkError`] and `present()`, since neither a cross-ref queue target
/// nor a clean worktree is a fault or a refusal: they are ordinary, expected preview outcomes, exactly
/// the way a disabled palette entry is not an error (`C-T4d`).
#[derive(Debug)]
pub enum CommitPreviewOutcome {
    /// Commit is unavailable, with the reason to show (disabled-with-reason, `C-T4d`) — no
    /// [`PreviewToken`] exists for this outcome; nothing was armed.
    Blocked(String),
    /// Commit is available: the preview to show and the token to carry into confirmation. `token` is
    /// boxed only to keep this enum's two variants close in size (`clippy::large_enum_variant`) — no
    /// meaning attaches to the indirection.
    Ready {
        /// The preview to restate to the user before they confirm.
        preview: CommitPreview,
        /// Feed this into [`commit_confirm_and_execute`] once the user supplies evidence.
        token: Box<PreviewToken>,
    },
}

/// Build the commit preview for `reff` (design `FL-05` step 3; RFC 014 §3).
///
/// Reads `orientation` and `worktree-status` **fresh, inside the change-token-gated `compute` step**
/// (RFC 013's ruling for this increment) — never the `ChangesView` already on screen. Two conditions
/// are checked before anything is armed (decisions 1/5b): the active WAL's queue must target `reff`
/// (RFC 014 F2), and the worktree must not be clean (RFC 014 F6). Neither reaches
/// [`stikk_prikk::Prikk::commit`] itself —
/// prevention, not classification.
///
/// # Errors
/// [`stikk_model::StikkError::NotReady`] below prikk 0.28 (`worktree-status` is unreliable, UD-03). Otherwise
/// propagates any [`stikk_model::StikkError`] the seam raises.
pub fn commit_preview(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<CommitPreviewOutcome> {
    let intent = Intent {
        category: RequestCategory::QueueMutation,
        operation: COMMIT_OPERATION,
    };
    let reff_owned = reff.to_string();
    let (view, token) =
        confirm::preview(prikk, repo, intent, || compute(prikk, repo, &reff_owned))?;
    Ok(match view {
        CommitReadView::Blocked(reason) => CommitPreviewOutcome::Blocked(reason),
        CommitReadView::Ready(preview) => CommitPreviewOutcome::Ready {
            preview,
            token: Box::new(token),
        },
    })
}

/// The two shapes [`compute`] can produce — never exposed outside this module; [`commit_preview`]
/// unwraps this into [`CommitPreviewOutcome`], attaching the token only to the `Ready` case.
enum CommitReadView {
    Blocked(String),
    Ready(CommitPreview),
}

/// The `compute` closure `preview()` runs after stamping the change token (RFC 013 `OPL-02`).
fn compute(
    prikk: &impl Prikk,
    repo: &Path,
    reff: &str,
) -> Result<(CommitReadView, ConfirmationSummary)> {
    let handshake = prikk.handshake()?;
    changes::ensure_worktree_status_supported(&handshake)?;

    // RFC 014 decision 1 / F2: prevent a cross-ref commit before arming anything. Both ref names are
    // stikk's own authoritative sources (the intent's `reff`, and `Orientation::queued_target`) —
    // never parsed from a prikk refusal (`C-T2b`).
    let orientation = prikk.orientation(repo)?;
    if let Some(target) = orientation.queued_target.as_deref() {
        if target != reff {
            let reason = format!(
                "the active queue belongs to {target}, but you are focused on {reff} — seal {target} \
                 first, or choose {target} to continue its queue"
            );
            return Ok((CommitReadView::Blocked(reason), placeholder_summary()));
        }
    }

    let status = prikk.worktree_status(repo, reff)?;
    if status.clean {
        return Ok((
            CommitReadView::Blocked(
                "the worktree matches this ref's replay baseline — there is nothing to commit"
                    .to_string(),
            ),
            placeholder_summary(),
        ));
    }

    let view = changes::from_status(status);
    let summary = ConfirmationSummary {
        operation: "Commit worktree changes".to_string(),
        target_ids: vec![reff.to_string()],
        counts: vec![
            ("modified", view.modified),
            ("missing", view.missing),
            ("untracked", view.untracked),
            ("unsupported", view.unsupported),
        ],
        capability: Capability::Author,
        consequence: consequence(orientation.active_patch_warning.as_deref()),
        target_name: None,
    };
    let preview = CommitPreview {
        changes: view,
        active_patch_notice: orientation.active_patch_warning,
    };
    Ok((CommitReadView::Ready(preview), summary))
}

/// A [`ConfirmationSummary`] never shown — [`CommitPreviewOutcome::Blocked`] discards it along with the
/// [`PreviewToken`] `preview()` mints for it (harmless: nothing bad happens by minting an unused token,
/// only by *using* one that skipped a step — RFC 013's guarantee is about what `execute` can reach, not
/// about never constructing a value nobody looks at).
fn placeholder_summary() -> ConfirmationSummary {
    ConfirmationSummary {
        operation: String::new(),
        target_ids: Vec::new(),
        counts: Vec::new(),
        capability: Capability::Author,
        consequence: String::new(),
        target_name: None,
    }
}

/// What becomes permanent, in stikk's own words (`TU-09`) — with prikk's own active-patch threshold
/// warning appended verbatim when present (`C-D2a`; RFC 014 F5), never paraphrased (`ER-02`).
fn consequence(active_patch_warning: Option<&str>) -> String {
    const BASE: &str = "Queues this worktree capture as a new patch in the active WAL; nothing is sealed until you \
         run Seal.";
    match active_patch_warning {
        Some(warning) => format!("{BASE} {warning}"),
        None => BASE.to_string(),
    }
}

/// Confirm and execute a commit in one step (design `OPL-01…05`; RFC 014 §3 step 4) — there is no user
/// action between confirmation succeeding and execution starting, so this is one seam-worker round
/// trip, not two. `readiness` is taken as an explicit parameter, matching [`confirm::confirm`]'s own
/// signature, rather than read internally: the caller (the worker thread) reads it immediately before
/// this call, and passing it explicitly is what let this function's gate be tested with an arbitrary
/// [`stikk_model::Readiness`] rather than the real process environment (`confirm/tests.rs`'s own
/// precedent, `ready(author, maintainer, read_only)`).
///
/// # Errors
/// [`stikk_model::StikkError::NotReady`] if read-only mode or AUTHOR capability is insufficient;
/// [`stikk_model::StikkError::Stale`] if the repository changed since the preview or since confirmation;
/// [`stikk_model::StikkError::Declined`] if `evidence` does not satisfy tier 2 (an explicit yes); otherwise whatever
/// [`stikk_prikk::Prikk::commit`] or a change-token read raises — including
/// [`stikk_model::StikkError::CrossRef`] for the RFC 014 F2 race.
pub fn commit_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    reff: &str,
    message: &str,
) -> Result<Outcome<CommitResult>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || prikk.commit(repo, reff, message))
}

#[cfg(test)]
mod tests;
