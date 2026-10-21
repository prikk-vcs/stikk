//! Creating a branch, creating a tag and closing a branch (design `FR-070`/`FR-071`; RFC 044).
//!
//! **These write refs and nothing else.** No file in the worktree moves, no history is rewritten,
//! and nothing is deleted -- prikk has no delete, and closing reclaims nothing. What they change
//! is which names point where.
//!
//! They go through the machinery commit and seal already use, at seal's tier and capability
//! ([`stikk_model::RequestCategory::Publication`]): a preview that restates the facts, a
//! confirmation, then one prikk call. Three things are worth reading here.
//!
//! - **Readiness is checked before prikk runs** (decision 2). Without a ready MAINTAINER key all
//!   three refuse, and prikk's refusal names a signature mismatch -- true, and the wrong first
//!   thing for a reader to meet. stikk says what is actually missing, in its own words.
//! - **stikk adds no naming rules** (decision 5). An empty name is not sent, and that is the only
//!   rule. Everything else prikk polices, and its refusal is shown verbatim: a missing prefix, a
//!   name that exists, a `--from` that resolves to nothing. A space in a branch name is legal to
//!   prikk, so it is legal here.
//! - **No name is inside a `consequence`** (review v1 C1). That field is drawn unescaped, because
//!   [`crate::ConfirmationSummary`]'s own contract is that nothing a repository or a user wrote
//!   reaches it. Every name here is one or the other — prikk rejects `Cc` in a ref name but not
//!   `Cf`, bidi controls or default-ignorables — so the names are restated in `target_ids`, which
//!   the frontend renders through the identifier rule (RFC 036), and the sentences speak of *the
//!   new branch*, *the source* and *the target*.
//! - **Closing the branch you are on is not offered** (decision 7, F3). prikk allows it with an
//!   empty WAL, and the repository is then on a closed branch where even a default `branch
//!   create` refuses. prikk polices the queued-work case itself; it does not police this one.
//!   The refusal here is stikk's, and it names the way out.

use std::path::Path;

use stikk_model::{Capability, CurrentBranch, RequestCategory, Result};
use stikk_prikk::{Prikk, RefWrite};

use crate::confirm::{self, ConfirmationSummary, Evidence, Intent, Outcome, PreviewToken};

/// stikk's own short names for the three operations, used in `Intent` and in stale/declined
/// messages (never prikk's words).
pub const CREATE_BRANCH_OPERATION: &str = "create branch";
/// As [`CREATE_BRANCH_OPERATION`].
pub const CREATE_TAG_OPERATION: &str = "create tag";
/// As [`CREATE_BRANCH_OPERATION`].
pub const CLOSE_BRANCH_OPERATION: &str = "close branch";

/// The first prikk minor that offers these, re-exported so a view says the number the seam gates
/// on -- the one-constant rule every other band follows.
pub use stikk_prikk::CURRENT_BRANCH_FROM_MINOR as REF_WRITES_FROM_MINOR;

/// **Why stikk will not close the branch the repository is on** (RFC 044 decision 7).
///
/// Measured at 0.52.0: with an empty WAL prikk closes it, and the next default-source `branch
/// create` refuses because `.prikk/current-branch` names a closed branch. That is a repository a
/// reader has to dig out of, from a key press that looked ordinary.
///
/// **The way out is stikk's own, since RFC 045** (decision 9). It named `prikk branch switch`
/// while stikk could not switch; it names the key now, because sending a reader to the command
/// line for something two keys away would be stikk under-selling what it does.
pub const CLOSING_THE_CURRENT_BRANCH: &str = "this is the branch you are on; closing it would leave the repository on a closed branch. \
     Switch to another branch first (`s` in the ref picker).";

/// **stikk's own reason, below the band** (decision 3; `C-T4d`), given the minor prikk reported.
///
/// The three keys answer with this instead of previewing, so nothing is spawned and prikk is never
/// asked a question this version has no answer for.
#[must_use]
pub fn ref_writes_unavailable(minor: u32) -> String {
    format!(
        "Branches and tags need prikk ≥ 0.{REF_WRITES_FROM_MINOR} — this prikk is \
         0.{minor}. Before 0.{REF_WRITES_FROM_MINOR} it keeps no current-branch pointer, so stikk \
         cannot tell which branch closing one would strand you on. The rest of stikk works."
    )
}

/// What a ref write can say before it runs.
#[derive(Debug)]
pub enum RefWritePreview {
    /// stikk will not run it, and says why in its own voice.
    Blocked(String),
    /// Ready to confirm.
    Ready {
        /// The confirmation token.
        token: Box<PreviewToken>,
    },
}

/// Preview creating a branch at `from_ref`'s tip.
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises.
pub fn create_branch_preview(
    prikk: &impl Prikk,
    repo: &Path,
    name: &str,
    from_ref: &str,
) -> Result<RefWritePreview> {
    let name = name.to_string();
    let from_ref = from_ref.to_string();
    let resolve = Some(from_ref.clone());
    preview(
        prikk,
        repo,
        CREATE_BRANCH_OPERATION,
        resolve,
        move |facts| {
            if name.is_empty() {
                return Blocked("a branch needs a name.".to_string());
            }
            Summary {
                operation: "Create a branch".to_string(),
                // The new name, then the source and the block the source is on **right now**
                // (decision 6). The new branch has no id of its own to show: it does not exist
                // yet.
                target_ids: with_tip(vec![name.clone(), from_ref.clone()], facts),
                consequence: format!(
                    "The new branch will point at the source's tip. Nothing in the worktree \
                     changes, and no other ref moves.{}",
                    facts.unready
                ),
                message: None,
            }
            .into()
        },
    )
}

/// What a new tag names, points at and carries (RFC 044 decision 5).
///
/// **One argument, not three strings in a row.** `name`, `target` and `message` are all `&str`,
/// and two of them are ref-shaped; a caller that swapped a pair would compile and tag the wrong
/// thing. Naming them at the call site is what makes that impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewTag<'a> {
    /// The tag's full name, prefix included. stikk judges only whether it is empty.
    pub name: &'a str,
    /// A ref name or a block id -- prikk's `--target` takes either (F2).
    pub target: &'a str,
    /// The message, possibly empty. **prikk refuses an empty message, so an empty or blank one
    /// sends no `-m`** — a tag without one is prikk's own shape for a tag that carries no
    /// message, and a blank `-m` is a usage error (measured at 0.52.0).
    pub message: &'a str,
}

/// Preview creating a tag at `tag.target`, which may be a ref name or a block id.
///
/// # Errors
/// As [`create_branch_preview`].
pub fn create_tag_preview(
    prikk: &impl Prikk,
    repo: &Path,
    tag: NewTag<'_>,
) -> Result<RefWritePreview> {
    let name = tag.name.to_string();
    let target = tag.target.to_string();
    let message = tag.message.to_string();
    // **A block id needs no read; a ref does.** `--target` takes either (F2), and the two are
    // told apart by prikk's own object-id grammar, never by guessing at the shape of a name.
    let targets_a_block = stikk_model::ObjectId::parse(&target).is_ok();
    let resolve = (!targets_a_block).then(|| target.clone());
    preview(prikk, repo, CREATE_TAG_OPERATION, resolve, move |facts| {
        if name.is_empty() {
            return Blocked("a tag needs a name.".to_string());
        }
        let said = if message.trim().is_empty() {
            " It carries no message.".to_string()
        } else {
            String::new()
        };
        let targets = if targets_a_block {
            // Shown **once**, short: the target already is the block, so a second row would
            // repeat it rather than add anything.
            vec![name.clone(), short(&target)]
        } else {
            with_tip(vec![name.clone(), target.clone()], facts)
        };
        Summary {
            operation: "Create a tag".to_string(),
            target_ids: targets,
            consequence: format!(
                "The tag will point at the target.{said} Nothing in the worktree changes, and no \
                 other ref moves.{}",
                facts.unready
            ),
            message: (!message.trim().is_empty()).then(|| message.clone()),
        }
        .into()
    })
}

/// Preview closing `name`.
///
/// **Blocked when `name` is the current branch** (decision 7), before anything is read about it.
///
/// # Errors
/// As [`create_branch_preview`].
pub fn close_branch_preview(
    prikk: &impl Prikk,
    repo: &Path,
    name: &str,
) -> Result<RefWritePreview> {
    let name = name.to_string();
    let resolve = Some(name.clone());
    preview(prikk, repo, CLOSE_BRANCH_OPERATION, resolve, move |facts| {
        if name.is_empty() {
            return Blocked("a branch needs a name.".to_string());
        }
        if let CurrentBranch::Branch(current) = &facts.current_branch
            && current.as_str() == name
        {
            return Blocked(CLOSING_THE_CURRENT_BRANCH.to_string());
        }
        Summary {
            operation: "Close a branch".to_string(),
            target_ids: with_tip(vec![name.clone()], facts),
            // prikk's own promise (F1), in stikk's restatement (decision 6).
            consequence: format!(
                "The branch will be closed. Nothing is reclaimed: the pointer, its history and \
                 every object remain, and the branch is recoverable.{}",
                facts.unready
            ),
            message: None,
        }
        .into()
    })
}

/// Create the branch this token was previewed for.
///
/// # Errors
/// As [`create_branch_preview`].
pub fn create_branch_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    name: &str,
    from_ref: &str,
) -> Result<Outcome<RefWrite>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        prikk.create_branch(repo, name, from_ref)
    })
}

/// Create the tag this token was previewed for.
///
/// # Errors
/// As [`create_branch_preview`].
pub fn create_tag_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    tag: NewTag<'_>,
) -> Result<Outcome<RefWrite>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        prikk.create_tag(repo, tag.name, tag.target, tag.message)
    })
}

/// Close the branch this token was previewed for.
///
/// # Errors
/// As [`create_branch_preview`].
pub fn close_branch_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    name: &str,
) -> Result<Outcome<RefWrite>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || prikk.close_branch(repo, name))
}

// -------------------------------------------------------------------------------------------
// The shared shape. The three differ only in their words and their one guard.

/// What every one of the three needs to know before it can word itself.
struct Facts {
    current_branch: CurrentBranch,
    /// The sentence appended when MAINTAINER readiness is missing -- empty when it is present.
    unready: String,
    /// The **short block id the source or target is on**, read fresh inside this preview
    /// (decision 6). `None` when there is nothing to resolve, or when prikk would not say.
    ///
    /// **Read here, not carried from the picker.** The reason the id is on the card at all is
    /// that a ref can move between the list being read and the key being pressed; an id copied
    /// from that same stale list could never show the move. This is the read that can.
    ///
    /// **Not from the ref list's own id either.** Measured at 0.52.0
    /// (`.git-exclude/reports/044-branches-and-tags/measure044-c2-0.52.txt`): `branch list`
    /// reports a **RefState** id, which is not the tip's block id -- the two differ on the same
    /// branch in the same repository. `tag list` does report a block id, but one field filled
    /// from two commands with two meanings is not a fact this card may assert.
    tip: Option<String>,
}

/// An object id, shortened the way History and seal already shorten one.
fn short(id: &str) -> String {
    id.chars().take(crate::confirm::SHORT_ID_CHARS).collect()
}

/// Append the resolved tip to a target list, when there is one.
///
/// **Adjacent to the ref it belongs to**, which is the last row before it -- the ids are hex and
/// the refs are prefixed names, so the two never read as the same kind of row.
fn with_tip(mut targets: Vec<String>, facts: &Facts) -> Vec<String> {
    if let Some(tip) = &facts.tip {
        targets.push(tip.clone());
    }
    targets
}

/// The block a ref is on, short, or `None` if prikk will not say.
///
/// **A failure here is silence, never an error.** This id is a courtesy on the card; a ref that
/// has gone missing between the picker and the key press will be refused by the write itself, in
/// prikk's own words, and a cosmetic read must not pre-empt that with a failure of its own.
fn tip_of(prikk: &impl Prikk, repo: &Path, reff: &str) -> Option<String> {
    let history = prikk.history(repo, reff, 1).ok()?;
    history.blocks.first().map(|block| short(&block.block_id))
}

/// The parts of a summary each operation supplies; everything else is the same for all three.
struct Summary {
    operation: String,
    target_ids: Vec<String>,
    consequence: String,
    /// The tag message, when there is one (decision 6) -- `None` for the other two operations and
    /// for a tag without one.
    message: Option<String>,
}

enum Computed {
    Blocked(String),
    Ready(Summary),
}

#[expect(non_snake_case, reason = "reads as a variant at the call sites above")]
fn Blocked(reason: String) -> Computed {
    Computed::Blocked(reason)
}

impl From<Summary> for Computed {
    fn from(summary: Summary) -> Self {
        Self::Ready(summary)
    }
}

fn preview(
    prikk: &impl Prikk,
    repo: &Path,
    operation: &'static str,
    resolve: Option<String>,
    compute: impl FnOnce(&Facts) -> Computed,
) -> Result<RefWritePreview> {
    let intent = Intent {
        category: RequestCategory::Publication,
        operation,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        // Below the band the three are not offered at all; the frontend says so without
        // previewing. Reaching here anyway, the seam answers `Unavailable` and nothing runs.
        let orientation = prikk.orientation(repo)?;
        let report = prikk.readiness(repo)?;
        let ready = Capability::derive(report.readiness).may_publish();
        let facts = Facts {
            tip: resolve
                .as_deref()
                .and_then(|reff| tip_of(prikk, repo, reff)),
            current_branch: orientation.current_branch.clone(),
            unready: if ready {
                String::new()
            } else {
                // **stikk's own words, before prikk's** (decision 2). prikk would refuse with a
                // signature mismatch, which is true and is not what is wrong.
                " This needs a MAINTAINER signing key, which this session does not have."
                    .to_string()
            },
        };
        let (signing_key_id, claim) = crate::confirm::signing_key_claim(
            report.readiness.maintainer,
            &report.maintainer,
            stikk_prikk::key_id::maintainer_key_id(),
        );
        Ok(match compute(&facts) {
            Computed::Blocked(reason) => {
                (ReadView::Blocked(reason.clone()), blocked_summary(reason))
            }
            Computed::Ready(summary) => (
                ReadView::Ready,
                ConfirmationSummary {
                    operation: summary.operation,
                    // RFC 047 review v1 C1: these three writes each name one kind of thing and
                    // the card's own title says which, so every row is bare.
                    target_ids: summary
                        .target_ids
                        .into_iter()
                        .map(crate::confirm::TargetId::bare)
                        .collect(),
                    counts: Vec::new(),
                    capability: Some(Capability::Maintainer),
                    consequence: summary.consequence,
                    target_name: None,
                    signing_key_id,
                    signing_key_claim: claim,
                    signing_key_is_published_example: report
                        .maintainer
                        .public_key
                        .as_deref()
                        .and_then(stikk_model::published_example)
                        .is_some(),
                    branch_notice: None,
                    freezes: None,
                    history_notice: None,
                    rename_note: None,
                    declaration_notices: Vec::new(),
                    message_note: summary.message,
                    materializes: None,
                    merge_plan: None,
                    asides: Vec::new(),
                },
            ),
        })
    })?;
    // The token is attached to the `Ready` case only, as seal's preview does: a blocked preview
    // has nothing to confirm.
    Ok(match view {
        ReadView::Blocked(reason) => RefWritePreview::Blocked(reason),
        ReadView::Ready => RefWritePreview::Ready {
            token: Box::new(token),
        },
    })
}

/// The two shapes `compute` can produce -- never exposed; [`preview`] unwraps it, attaching the
/// token to `Ready` alone (seal's own shape, for its own reason).
enum ReadView {
    Blocked(String),
    Ready,
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked_summary(reason: String) -> ConfirmationSummary {
    ConfirmationSummary {
        operation: "Blocked".to_string(),
        target_ids: Vec::new(),
        counts: Vec::new(),
        capability: Some(Capability::Maintainer),
        consequence: reason,
        target_name: None,
        signing_key_id: None,
        signing_key_claim: crate::confirm::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
        history_notice: None,
        rename_note: None,
        declaration_notices: Vec::new(),
        message_note: None,
        materializes: None,
        merge_plan: None,
        asides: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
