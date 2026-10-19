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

use stikk_model::Result;
use stikk_prikk::{BlockRow, History, MergeOutcome, MergePlan, Prikk};

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
    /// The baseline block the plan was computed from, shortened for the reader to check.
    pub baseline: String,
    /// prikk's printed plan, **verbatim and in order** (decision 2). Shown, never parsed.
    pub lines: Vec<String>,
    /// The verdict, from the plan's one `evidence outcome:` line. **Read here and nowhere else.**
    pub outcome: MergeOutcome,
}

/// What the Merge plan view has to show: prikk's plan, or stikk's reason for not asking for one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergePlanRead {
    /// prikk's plan for the pair.
    Read(Box<MergePlanView>),
    /// stikk did not ask prikk, and says why in its own voice.
    Declined(String),
}

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
            baseline: short(&baseline),
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

#[cfg(test)]
mod tests;
