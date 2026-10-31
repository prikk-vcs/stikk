//! Switching the repository to another branch (design `FR-053`; RFC 045).
//!
//! **The first stikk operation that changes files on disk.** prikk makes the switch itself safe
//! (F2, F3): every refusal writes nothing, a tracked file is replaced only after the worktree was
//! found clean, an old file is deleted only if its bytes still match the branch being left, an
//! untracked file is never read or deleted, and the pointer is written last. Switching back
//! restores every byte.
//!
//! What is left for stikk is the half prikk cannot do: **the user sees exactly which files will
//! change before saying yes** (`FR-053`), and the one case prikk allows but gets wrong is not
//! offered. Three things are worth reading here.
//!
//! - **The plan is stikk's own Compare, read inside the preview** (decision 5). Measured at
//!   0.52.0: `diff --from <current> --to <target>` listed `a.txt modified`, `b.txt deleted`,
//!   `c.txt added`, and the switch then reported written 2, deleted 1, with the worktree holding
//!   exactly the target's files (F4). Read in the preview, not earlier, so the plan the user
//!   confirms is the plan as it stands at that moment.
//! - **A plan that is not whole is not a plan** (§6 trap 2). A status stikk has not measured, or
//!   any `unsupported_paths`, blocks the switch and says so. `FR-053` asks the user to confirm
//!   what will happen; a list with a silent hole in it is not that.
//! - **No signing key is asked for** (decision 2). prikk needs none (F1, measured with every key
//!   variable unset), and [`stikk_model::RequestCategory::Materialization`] says so: the gate
//!   reads the requirement from the category, so a Tier Three operation can need no key.

use std::path::Path;

use stikk_model::{CurrentBranch, RequestCategory, Result, StikkError};
use stikk_prikk::{Comparison, DiffReport, Prikk, SwitchResult};

use crate::confirm::{
    self, ConfirmationSummary, Evidence, FilePlan, Intent, Outcome, PreviewToken,
};

/// stikk's own short name for the operation, used in `Intent` and in stale/declined messages.
pub const SWITCH_OPERATION: &str = "switch branch";

/// The first prikk minor that offers a switch **with its file plan** (decision 4), re-exported so
/// a view says the number the seam gates on -- the one-constant rule every other band follows.
///
/// `branch switch` itself arrived at 0.42, with the current-branch pointer. It is not offered
/// below 0.46 because the plan comes from `diff --from --to`, which stikk reads from there: a
/// switch without its file list is the idea, not the plan, and `FR-053` asks for the plan.
pub use stikk_prikk::DIFF_FROM_MINOR as SWITCH_FROM_MINOR;

/// **Already there** (decision 6). prikk prints `already on heads/main` and does nothing; stikk
/// says so without spawning, because a confirmation for a no-op is a question not worth asking.
pub const ALREADY_THE_CURRENT_BRANCH: &str = "this is already the current branch.";

/// **Why stikk will not switch to a branch the queue holds work for** (decision 6, F5).
///
/// Measured at 0.52.0: prikk allows it and writes the target's **sealed** files only. The queued
/// files are then absent from the worktree, `worktree-status` reports them missing, and the next
/// `commit --from-worktree` would queue their deletion. Nothing is lost yet, but the picture is
/// wrong, and stikk can reach this case by itself -- it commits to its focused ref, which may
/// differ from prikk's current branch (`FR-055`).
pub const WORK_IS_QUEUED_FOR_THIS_BRANCH: &str =
    "work is queued for this branch; seal it first, or the worktree would not show it.";

/// **Why stikk will not switch without a file plan** (decision 6).
///
/// `FR-053` is that the user confirms the plan, not the idea. Where the current branch cannot be
/// compared -- prikk reports no current branch, or one it cannot resolve, or the branch has no
/// published history -- there is nothing to compare from, so there is no plan to show and no
/// switch offered. prikk's own command is the way out, and it writes only files that are absent
/// or already identical (F3).
pub const NO_FILE_PLAN: &str = "stikk cannot list what this switch would write, so it does not offer it. prikk's own \
     `prikk branch switch heads/<name>` does the switch; it writes only files that are absent or \
     already identical, and refuses rather than overwrite your work.";

/// What a switch can say before it runs.
#[derive(Debug)]
pub enum SwitchPreview {
    /// stikk will not run it, and says why in its own voice.
    Blocked(String),
    /// Ready to confirm.
    Ready {
        /// The confirmation token.
        token: Box<PreviewToken>,
    },
}

/// **stikk's own reason, below the band** (decision 4; `C-T4d`), given the minor prikk reported.
#[must_use]
pub fn switch_unavailable(minor: u32) -> String {
    format!(
        "Switching branch needs prikk ≥ 0.{SWITCH_FROM_MINOR} — this prikk is 0.{minor}. Below \
         0.{SWITCH_FROM_MINOR} stikk cannot read the comparison the preview is made of, and a \
         switch is not offered without the list of files it would write. The rest of stikk works."
    )
}

/// Preview switching to `target`.
///
/// **Reads the plan itself**, so what the confirmation restates is what the repository holds now.
/// The three blocked cases that need no plan are decided before the comparison is read, so a
/// switch stikk will not make never costs a `diff`.
///
/// # Errors
/// Propagates any [`StikkError`] the orientation read raises, and every one the comparison
/// raises **except a refusal**. A comparison prikk *refused* is the fourth blocked case, because
/// a switch stikk cannot describe is one stikk does not offer; a comparison a **lock** or the
/// environment stopped is not a fact about the plan at all, and travels as itself.
pub fn switch_preview(prikk: &impl Prikk, repo: &Path, target: &str) -> Result<SwitchPreview> {
    let target = target.to_string();
    let intent = Intent {
        category: RequestCategory::Materialization,
        operation: SWITCH_OPERATION,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        let orientation = prikk.orientation(repo)?;
        let CurrentBranch::Branch(current) = &orientation.current_branch else {
            // prikk names no current branch, or one it cannot resolve: nothing to compare from.
            return Ok(blocked(NO_FILE_PLAN));
        };
        let current = current.as_str().to_string();
        if current == target {
            return Ok(blocked(ALREADY_THE_CURRENT_BRANCH));
        }
        // **F5, and only F5** (§6 trap 4). Work queued for *another* ref is prikk's own refusal,
        // which already names the way out; widening this guard would take a decision from prikk
        // that prikk makes better.
        if orientation.queued_patches > 0
            && orientation.queued_target.as_deref() == Some(target.as_str())
        {
            return Ok(blocked(WORK_IS_QUEUED_FOR_THIS_BRANCH));
        }
        let comparison = Comparison::Points {
            from: current.clone(),
            to: target.clone(),
        };
        let diff = match prikk.diff(repo, &comparison) {
            Ok(DiffReport::Listed(diff)) => diff,
            // **Below the band, or a comparison prikk refused** -- an unpublished current branch
            // among them, which refuses with *"does not exist"*. Either way there is no plan,
            // and stikk says that rather than passing on a sentence about the branch the reader
            // is standing on.
            Ok(DiffReport::Unreported) | Err(StikkError::Refusal { .. }) => {
                return Ok(blocked(NO_FILE_PLAN));
            }
            // **Everything else travels as itself** (review v1 C1). A lock conflict, an
            // environment fault or an internal error says nothing about whether a plan exists:
            // the way out of a held lock is not `prikk branch switch`, and a lock has its own
            // handling everywhere else in stikk. Answering *no file plan* here would hide a
            // failure behind a sentence about something that is not wrong.
            Err(other) => return Err(other),
        };
        let Some(plan) = FilePlan::classify(&diff) else {
            return Ok(blocked(INCOMPLETE_PLAN));
        };
        let tip = prikk
            .history(repo, &target, 1)
            .ok()
            .and_then(|history| history.blocks.first().map(|block| short(&block.block_id)));
        // RFC 047 review v1 C1: a switch names where it is going, and `Switch branch` plus the
        // file plan below say the rest -- there is no pair whose order could be read backwards.
        let mut target_ids = vec![
            crate::confirm::TargetId::bare(current),
            crate::confirm::TargetId::bare(target.clone()),
        ];
        target_ids.extend(tip.map(crate::confirm::TargetId::bare));
        Ok((
            ReadView::Ready,
            ConfirmationSummary {
                operation: "Switch branch".to_string(),
                target_ids,
                // **`written` and `deleted`, not `files written`** — the renderer draws
                // `{count} {label}`, and "1 files deleted" is a sentence nobody wrote on
                // purpose. The row marks below use the same two words.
                counts: vec![
                    ("written", plan.written.len() as u64),
                    ("deleted", plan.deleted.len() as u64),
                ],
                // **No key** (decision 2). prikk needs none, so the card says so rather than
                // naming a capability the operation never consumes.
                capability: None,
                consequence: consequence(plan.is_empty()),
                target_name: None,
                signing_key_id: None,
                signing_key_claim: crate::confirm::KeyClaim::None,
                signing_key_is_published_example: false,
                branch_notice: None,
                freezes: None,
                history_notice: None,
                rename_note: None,
                declaration_notices: Vec::new(),
                notices: Vec::new(),
                message_note: None,
                materializes: Some(plan),
                merge_plan: None,
                preamble: None,
                undo_plan: None,
                asides: Vec::new(),
            },
        ))
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => SwitchPreview::Blocked(reason),
        ReadView::Ready => SwitchPreview::Ready {
            token: Box::new(token),
        },
    })
}

/// Switch to the branch this token was previewed for.
///
/// # Errors
/// As [`switch_preview`], plus every refusal prikk raises — each of which writes nothing (F2).
pub fn switch_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    target: &str,
) -> Result<Outcome<SwitchResult>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || prikk.switch_branch(repo, target))
}

/// **Why stikk will not switch on a plan with a hole in it** (§6 trap 2).
pub const INCOMPLETE_PLAN: &str = "stikk cannot list every file this switch would change, so it does not offer it: prikk's \
     comparison holds a path or a kind of change stikk has not measured. A plan you confirm has \
     to be the whole plan.";

/// The sentence under the counts, in stikk's own words — **with no name in it** (RFC 044 review
/// C1): `consequence` is drawn unescaped, and every name is already restated in `target_ids`.
fn consequence(empty: bool) -> String {
    let opening = if empty {
        "The two branches hold the same files, so the switch moves the pointer and writes nothing."
    } else {
        "The worktree will be rewritten to the target's files."
    };
    format!(
        "{opening} Untracked files are not touched. prikk refuses, writing nothing, if a tracked \
         file has changes or an untracked file is in the way, and switching back restores every \
         byte."
    )
}

/// An object id, shortened the way History, seal and RFC 044's previews already shorten one.
fn short(id: &str) -> String {
    id.chars().take(crate::confirm::SHORT_ID_CHARS).collect()
}

/// The two shapes the read can produce -- never exposed; [`switch_preview`] unwraps it, attaching
/// the token to `Ready` alone (seal's own shape, for its own reason).
enum ReadView {
    Blocked(String),
    Ready,
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked(reason: &str) -> (ReadView, ConfirmationSummary) {
    (
        ReadView::Blocked(reason.to_string()),
        ConfirmationSummary {
            operation: "Blocked".to_string(),
            target_ids: Vec::new(),
            counts: Vec::new(),
            capability: None,
            consequence: reason.to_string(),
            target_name: None,
            signing_key_id: None,
            signing_key_claim: crate::confirm::KeyClaim::None,
            signing_key_is_published_example: false,
            branch_notice: None,
            freezes: None,
            history_notice: None,
            rename_note: None,
            declaration_notices: Vec::new(),
            notices: Vec::new(),
            message_note: None,
            materializes: None,
            merge_plan: None,
            preamble: None,
            undo_plan: None,
            asides: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests;
