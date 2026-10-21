//! Bringing one branch into another: the baseline stikk may name, and the plan prikk prints for
//! it (design `FR-080`/`FR-081`; RFC 047).
//!
//! **This half reads and offers nothing else.** The merge itself is RFC 047's Handoff B.
//!
//! Two things prikk leaves to its caller shape everything here.
//!
//! - **The baseline block is the caller's**, and prikk exposes no parents to find it from (F2):
//!   `log --format json` lists the mainline only, with `kind` and `parent_count` but no parent
//!   ids, and prikk's own guide defers merge-base discovery. So [`merge_baseline`] names one only
//!   where it can name the **true** one, and says why when it cannot. **A wrong baseline is not
//!   harmless**: measured at 0.52, a baseline older than the true merge base turned a pair the
//!   true base proves `Confluent` into a `Conflict` — and prikk records the baseline on the merge
//!   block, where `verify` re-derives the real one and reports the disagreement.
//! - **A merge into the branch you are on leaves the worktree behind its new tip** (F3). prikk
//!   allows it and writes no file; `worktree-status` then reports the adopted file *missing*, the
//!   next commit would queue its deletion, and `branch switch` refuses because the worktree is
//!   not clean — so the way out is not even a switch. stikk does not offer it.

use std::path::Path;

use stikk_model::{CurrentBranch, RequestCategory, Result};
use stikk_prikk::{BlockRow, History, MergeOutcome, MergePlan, MergeResult, Prikk};

use crate::confirm::{self, ConfirmationSummary, Evidence, Intent, Outcome, PreviewToken};

/// stikk's own short name for the operation, used in stale and declined messages.
pub const MERGE_OPERATION: &str = "merge";

/// The first prikk minor that offers this, re-exported so a view says the number the seam gates
/// on -- the one-constant rule every other band follows.
///
/// Decision 7: the first guard needs prikk's current branch (0.42), and the baseline finder needs
/// `log`'s `kind` (0.39), so the later of the two bands it.
pub use stikk_prikk::CURRENT_BRANCH_FROM_MINOR as MERGE_FROM_MINOR;

/// How many blocks of each lineage the baseline finder reads.
///
/// **The same number History lists**, so a baseline stikk declines for a truncated read is one
/// the reader could not have seen the shared block of either.
pub const MERGE_LINEAGE_LIMIT: usize = 200;

/// Whether stikk can name the baseline block truthfully, and the reason when it cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeBaseline {
    /// The latest block both lineages share, with no merge above it on either side.
    Found(String),
    /// stikk will not name one, and says why in its own voice.
    None(String),
}

/// **Why stikk will not name a baseline after an earlier merge** (decision 3).
///
/// The rule that makes the latest shared block *provably* the merge base is that every block
/// above it has one parent. A merge block above it breaks that: the true base could be older,
/// and prikk publishes no parent ids for stikk to follow. Rather than guess — and a guess here
/// is a plan that may say `Conflict` where the truth is `Confluent` — stikk declines.
pub const MERGED_ALREADY: &str = "one of these branches has already merged, and prikk does not report which blocks a merge \
     joined. stikk will not guess a baseline, because the wrong one makes prikk's plan wrong. \
     `prikk merge-plan --baseline-block <id>` takes one you choose yourself.";

/// **Why stikk will not name a baseline for two lineages with nothing in common.**
pub const NO_SHARED_BLOCK: &str =
    "these two branches share no block, so there is no point to merge from.";

/// **Why stikk will not name a baseline it did not read far enough to find.**
///
/// The alternative is to call the oldest block it happened to read the base, which would be a
/// guess dressed as a fact — and `FR-080`'s whole point is that the baseline shown is the one
/// the plan was computed from.
pub const LINEAGE_TRUNCATED: &str = "stikk read as much of these branches' history as it lists at once and found no shared block \
     yet, so it cannot say which one they parted from.";

/// The baseline block for merging `from` into `into`, or stikk's reason for naming none.
///
/// **The latest block both lineages list, provided no block above it on either side is a merge**
/// (decision 3). Where that holds, every block above the shared one has a single parent, so the
/// latest common block is the *only* candidate and naming it is a fact rather than a guess.
///
/// `limit` is the number of blocks the caller asked prikk for. A lineage that came back full may
/// have more behind it, so a search that found nothing in it has not proved the two share
/// nothing — it has only run out of history, and says so. A lineage of exactly `limit` blocks
/// that happens to be complete is called truncated too; that errs towards naming no baseline,
/// which is the safe direction here.
#[must_use]
pub fn merge_baseline(into: &History, from: &History, limit: usize) -> MergeBaseline {
    let shared = into.blocks.iter().enumerate().find_map(|(at, block)| {
        from.blocks
            .iter()
            .position(|other| other.block_id == block.block_id)
            .map(|other_at| (at, other_at, block.block_id.clone()))
    });
    let Some((into_at, from_at, block_id)) = shared else {
        let truncated = into.blocks.len() >= limit || from.blocks.len() >= limit;
        return MergeBaseline::None(
            if truncated {
                LINEAGE_TRUNCATED
            } else {
                NO_SHARED_BLOCK
            }
            .to_string(),
        );
    };
    // **Above it on either side**, not including it: a merge *at* the shared block is fine, since
    // everything above that block is then linear on both lineages.
    let above = into
        .blocks
        .get(..into_at)
        .unwrap_or_default()
        .iter()
        .chain(from.blocks.get(..from_at).unwrap_or_default());
    if above.into_iter().any(is_merge) {
        return MergeBaseline::None(MERGED_ALREADY.to_string());
    }
    MergeBaseline::Found(block_id)
}

/// Everything the Merge plan view shows: the two branches, the baseline stikk found, and prikk's
/// own plan for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergePlanView {
    /// The branch whose work would be brought in -- the marked one, `--from` (decision 4).
    pub from: String,
    /// The branch it would be brought into -- the selected one, `--into`.
    pub into: String,
    /// The baseline block the plan was computed from, **as prikk's own full id**.
    ///
    /// Full, not shortened: this is the string the merge itself is run with, and a short id is a
    /// display form, not an argument. [`Self::baseline_short`] is what the reader sees.
    pub baseline: String,
    /// prikk's printed plan, **verbatim and in order** (decision 2). Shown, never parsed.
    pub lines: Vec<String>,
    /// The verdict, from the plan's one `evidence outcome:` line. **Read here and nowhere else.**
    pub outcome: MergeOutcome,
}

impl MergePlanView {
    /// The baseline as the reader sees it, shortened the way History and every preview shorten
    /// an object id -- so a reader can check it against the plan's own `baseline block:` line.
    #[must_use]
    pub fn baseline_short(&self) -> String {
        short(&self.baseline)
    }
}

/// What the Merge plan view has to show: prikk's plan, or stikk's reason for not asking for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergePlanRead {
    /// prikk's plan for the pair.
    Read(Box<MergePlanView>),
    /// stikk did not ask prikk, and says why in its own voice.
    Declined(String),
}

/// **Why stikk will not merge into the branch the repository is on** (decision 5, F3) -- the one
/// guard a measurement wrote.
///
/// prikk allows this merge and writes no file. The worktree is then behind the branch's new tip:
/// `worktree-status` reports the adopted file **missing**, the next commit would queue its
/// deletion, and `branch switch` refuses because the worktree is not clean -- so the way out is
/// not even a switch.
pub const INTO_THE_CURRENT_BRANCH: &str = "merging into the branch you are on would leave the worktree without the merged files, and \
     prikk would then refuse a switch. Switch to another branch first.";

/// **Why stikk will not merge into a branch with work queued** (decision 5, F4).
///
/// Measured: `merge-plan` judges the branch's **sealed** tip and the merge succeeds, after which
/// the queued work seals on top of the merge block. prikk allows it; whether a patch queued
/// against the old tip still means what it did is not something stikk can see.
pub const WORK_IS_QUEUED_FOR_INTO: &str = "work is queued for this branch; seal it first. A plan is judged against the sealed tip, not \
     against what is waiting.";

/// **Why stikk will not merge a tag** (decision 5). A merge moves a branch.
pub const TAGS_ARE_NOT_MERGED: &str =
    "only branches are merged; a tag names a point in history, not a line of work.";

/// **Why stikk will not merge a closed branch** (decision 5), from the ref list's own flag.
pub const CLOSED_IS_NOT_MERGED: &str = "this branch is closed; merge between open ones.";

/// **Why stikk will not merge a branch into itself** (decision 5).
pub const THE_SAME_BRANCH_TWICE: &str =
    "this is the branch you marked; mark one branch and select another.";

/// **Why stikk will not show a plan it cannot band-check.**
///
/// The seam answers [`MergePlan::Unavailable`] below the band without spawning. The picker
/// refuses the key before this is reached, so the only way here is a prikk that changed version
/// between the two reads -- rare, and still not a reason to show nothing at all.
pub const PLAN_UNAVAILABLE: &str = "this prikk does not offer a merge plan.";

/// prikk's plan for merging `from` into `into`, or stikk's reason for not asking.
///
/// **The baseline is found first, and a plan is asked for only when it is truthful** (decision 3):
/// a plan computed from the wrong baseline is not a weaker answer than none, it is a wrong one
/// (F2, measured).
///
/// `limit` is how many blocks of each lineage to read; [`merge_baseline`] treats a lineage that
/// came back full as possibly truncated.
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the two history reads and the plan raise. A
/// refusal is **not** turned into a decline here: a branch prikk will not list is a fact the
/// reader should see in prikk's own words (`ER-02`), not behind a sentence of stikk's.
pub fn merge_plan_view(
    prikk: &impl Prikk,
    repo: &Path,
    into: &str,
    from: &str,
    limit: usize,
) -> Result<MergePlanRead> {
    let into_lineage = prikk.history(repo, into, limit)?;
    let from_lineage = prikk.history(repo, from, limit)?;
    let baseline = match merge_baseline(&into_lineage, &from_lineage, limit) {
        MergeBaseline::Found(block_id) => block_id,
        MergeBaseline::None(why) => return Ok(MergePlanRead::Declined(why)),
    };
    // **`--left-ref` is `into` and `--right-ref` is `from`**, the order the merge itself takes
    // them in (`--into`, `--from`), so the plan shown is the plan of the merge offered.
    Ok(match prikk.merge_plan(repo, &baseline, into, from)? {
        MergePlan::Read { printed, outcome } => MergePlanRead::Read(Box::new(MergePlanView {
            from: from.to_string(),
            into: into.to_string(),
            baseline,
            lines: printed,
            outcome,
        })),
        MergePlan::Unavailable => MergePlanRead::Declined(PLAN_UNAVAILABLE.to_string()),
    })
}

/// An object id, shortened the way History, seal and RFC 044's previews already shorten one.
fn short(id: &str) -> String {
    id.chars().take(crate::confirm::SHORT_ID_CHARS).collect()
}

/// Whether a block joined two lineages.
///
/// **Either signal is enough.** Measured at 0.53.0 they agree exactly — a merge block is
/// `kind: "Merge"` with `parent_count: 2`, a normal one `"Normal"` with 1, the root `"Root"` with
/// 0 — and `the_two_merge_signals_agree` pins that. Checking both is the safe direction: if they
/// ever disagree, the block is one stikk does not understand, and not understanding a block is
/// not a reason to merge across it.
fn is_merge(block: &BlockRow) -> bool {
    block.kind == "Merge" || block.parents > 1
}

/// **stikk's own reason, below the band** (decision 7; `C-T4d`), given the minor prikk reported.
#[must_use]
pub fn merge_unavailable(minor: u32) -> String {
    format!(
        "Merging needs prikk ≥ 0.{MERGE_FROM_MINOR} — this prikk is 0.{minor}. Below \
         0.{MERGE_FROM_MINOR} it keeps no current-branch pointer, so stikk cannot tell whether a \
         merge would leave the worktree behind the branch you are on. The rest of stikk works."
    )
}

// -------------------------------------------------------------------------------------------
// The merge itself (RFC 047 decision 6; Handoff B)
// -------------------------------------------------------------------------------------------

/// What a merge can say before it runs.
#[derive(Debug)]
pub enum MergePreview {
    /// stikk will not run it, and says why in its own voice.
    Blocked(String),
    /// Ready to confirm.
    Ready {
        /// The confirmation token.
        token: Box<PreviewToken>,
    },
}

/// **Why stikk will not merge from a plan that is no longer confluent**, given prikk's word.
///
/// prikk merges only what it has proven confluent, and refuses everything else without writing
/// (F1). Offering a confirmation for a merge prikk will refuse would make the card a question
/// whose only answer is a refusal.
#[must_use]
pub fn not_confluent(outcome: MergeOutcome) -> String {
    format!(
        "prikk now reports these two branches as {} rather than Confluent, and it merges only \
         what it has proven confluent. Nothing was written. The Merge plan view shows prikk's \
         full reasoning.",
        outcome.name()
    )
}

/// **Why stikk will not merge from a baseline that moved** (§2).
///
/// The plan the reader saw was computed from one block. If either branch has sealed since, the
/// latest shared block can be another one, and confirming the old card would confirm a plan that
/// no longer describes anything.
pub const THE_BASELINE_MOVED: &str = "these branches have moved since this plan was read, so it was computed from a block that is \
     no longer the one they parted from. Nothing was written. Open the Merge plan again for a \
     current one.";

/// Preview merging `from` into `into`, from the baseline the view showed.
///
/// **Everything is read again here** (§2): the guards, the baseline and prikk's plan. The current
/// branch, the queue and a branch's closed flag can all change between the view opening and the
/// reader pressing `Enter`, and a confirmation restates what *is* true, not what was.
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the orientation, ref, history and plan reads
/// raise. A refusal is not turned into a block: a branch prikk will not list is a fact the reader
/// should meet in prikk's own words (`ER-02`).
pub fn merge_preview(
    prikk: &impl Prikk,
    repo: &Path,
    into: &str,
    from: &str,
    shown_baseline: &str,
    limit: usize,
) -> Result<MergePreview> {
    let (into, from) = (into.to_string(), from.to_string());
    let shown_baseline = shown_baseline.to_string();
    let intent = Intent {
        category: RequestCategory::Publication,
        operation: MERGE_OPERATION,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        if let Some(reason) = blocked_reason(prikk, repo, &into, &from)? {
            return Ok(blocked(&reason));
        }
        let plan = match merge_plan_view(prikk, repo, &into, &from, limit)? {
            MergePlanRead::Read(plan) => plan,
            MergePlanRead::Declined(reason) => return Ok(blocked(&reason)),
        };
        // **The baseline the reader saw, or none at all** -- compared in the short form the card
        // and the view both show, which is the form the caller was handed.
        if plan.baseline_short() != shown_baseline {
            return Ok(blocked(THE_BASELINE_MOVED));
        }
        if !plan.outcome.is_confluent() {
            return Ok(blocked(&not_confluent(plan.outcome)));
        }
        // The adopted side's tip: what this merge brings in, named so the card can be checked
        // against prikk's own plan above it.
        let adopted = prikk
            .history(repo, &from, 1)
            .ok()
            .and_then(|history| history.blocks.first().map(|block| short(&block.block_id)));
        // **Labelled, because direction is the whole operation** (review v1 C1). Four bare ids
        // leave a reader at Tier Three unable to tell which branch gains the merge block, and
        // the measured cost of the wrong `into` is a worktree left behind its branch (F3). The
        // words are stikk's; the values stay bare ids for the renderer to escape.
        let mut target_ids = vec![
            confirm::TargetId::named("from", from.clone()),
            confirm::TargetId::named("into", into.clone()),
            confirm::TargetId::named("baseline", plan.baseline_short()),
        ];
        target_ids.extend(adopted.map(|tip| confirm::TargetId::named("from's tip", tip)));
        Ok((
            ReadView::Ready,
            ConfirmationSummary {
                operation: "Merge".to_string(),
                target_ids,
                counts: Vec::new(),
                capability: Some(stikk_model::Capability::Maintainer),
                consequence: CONSEQUENCE.to_string(),
                target_name: None,
                signing_key_id: None,
                signing_key_claim: confirm::KeyClaim::None,
                signing_key_is_published_example: false,
                branch_notice: None,
                freezes: None,
                history_notice: None,
                rename_note: None,
                declaration_notices: Vec::new(),
                message_note: None,
                materializes: None,
                merge_plan: Some(plan.lines.clone()),
                asides: Vec::new(),
            },
        ))
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => MergePreview::Blocked(reason),
        ReadView::Ready => MergePreview::Ready {
            token: Box::new(token),
        },
    })
}

/// The sentence under the card, in stikk's own words -- **with no name in it** (RFC 044 C1):
/// `consequence` is drawn unescaped, and every name is already restated in `target_ids`.
///
/// **It opens with the verdict** (review v1 C2). The card is armed *because* stikk read
/// `evidence outcome: Confluent` under decision 2's pinned rule, and at 80 columns that line can
/// be among the ones prikk's block below gives up for height -- so the one fact the reader is
/// confirming must not depend on that block fitting. It names nothing, so it is safe here.
const CONSEQUENCE: &str = "prikk's plan reports the two sides confluent from this baseline. A merge block is sealed on \
     the branch merged into, adopting the other side's patches. Nothing is written to the \
     worktree: switch to that branch afterwards to have its files. prikk merges only what it has \
     proven confluent, and a refusal writes nothing.";

/// The three names a merge is run with, kept together.
///
/// **One value, not three `&str` in a row**: `into` and `from` are the same type, mean opposite
/// things, and are passed through four layers. A caller that swaps them merges the wrong way
/// round and nothing but a test would notice — the measured difference between the two is a
/// worktree left behind its branch (F3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTarget {
    /// The block both branches parted from, as [`merge_baseline`] named it.
    pub baseline: String,
    /// The branch merged into: it gains the merge block. Never the current branch (F3).
    pub into: String,
    /// The branch merged from: it is read and not moved.
    pub from: String,
}

/// Merge the pair this token was previewed for.
///
/// # Errors
/// As [`merge_preview`], plus every refusal prikk raises -- each of which writes nothing (F1).
pub fn merge_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    target: &MergeTarget,
) -> Result<Outcome<MergeResult>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        // **The full baseline, not the short one the card showed.** prikk takes a block id; the
        // short form is for the reader. `merge_plan` was given this same string.
        prikk.merge(repo, &target.baseline, &target.into, &target.from)
    })
}

/// **stikk's one line after a merge lands**: prikk wrote no file, and the reader should know
/// where the merged files are rather than discover their absence.
pub const SWITCH_TO_SEE_THE_FILES: &str = "The worktree is unchanged. Switch to the branch you merged into (s in the ref picker) to \
     have its files.";

/// Every Handoff A guard, re-asked from freshly read state (§2).
///
/// **Read once, here, for both sides.** The ref list and the orientation each cost one spawn, and
/// asking per guard would cost several for an answer that cannot differ between them.
fn blocked_reason(
    prikk: &impl Prikk,
    repo: &Path,
    into: &str,
    from: &str,
) -> Result<Option<String>> {
    if into == from {
        return Ok(Some(THE_SAME_BRANCH_TWICE.to_string()));
    }
    let refs = prikk.refs(repo)?;
    for name in [into, from] {
        match refs
            .refs
            .iter()
            .find(|entry| entry.name == name)
            .map_or((false, false), |entry| (entry.is_tag(), entry.closed))
        {
            (true, _) => return Ok(Some(TAGS_ARE_NOT_MERGED.to_string())),
            (_, true) => return Ok(Some(CLOSED_IS_NOT_MERGED.to_string())),
            _ => {}
        }
    }
    let orientation = prikk.orientation(repo)?;
    // **F3 and F4, and only about `into`.** Both are facts about the branch merged *into*; the
    // branch merged *from* is read, not moved, and neither applies to it.
    if let CurrentBranch::Branch(current) = &orientation.current_branch
        && current.as_str() == into
    {
        return Ok(Some(INTO_THE_CURRENT_BRANCH.to_string()));
    }
    if orientation.queued_patches > 0 && orientation.queued_target.as_deref() == Some(into) {
        return Ok(Some(WORK_IS_QUEUED_FOR_INTO.to_string()));
    }
    Ok(None)
}

/// The two shapes the read can produce -- never exposed; [`merge_preview`] unwraps it, attaching
/// the token to `Ready` alone, as switch and seal do.
enum ReadView {
    Blocked(String),
    Ready,
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked(reason: &str) -> (ReadView, ConfirmationSummary) {
    (
        ReadView::Blocked(reason.to_string()),
        ConfirmationSummary {
            operation: "Merge".to_string(),
            target_ids: Vec::new(),
            counts: Vec::new(),
            capability: None,
            consequence: String::new(),
            target_name: None,
            signing_key_id: None,
            signing_key_claim: confirm::KeyClaim::None,
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
        },
    )
}

#[cfg(test)]
mod tests;
